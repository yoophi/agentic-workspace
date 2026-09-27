//! 세대 범위 멱등성(040, research R7, ADR core 0005). 메모리 상태를 바꾸는 command(작업대·run 제어·교환)의
//! 재시도를 같은 세대 안에서 걸러 같은 결과를 돌려준다. 기록은 **작업대별**로 두고, 작업대가 닫히면 버린다 —
//! 그 뒤의 재시도는 대상이 없어 `notFound`로 끝나므로 효과가 두 번 나지 않는다.
//!
//! 한 작업대 안에서 결과 기록은 최근 `max_results`개만 두고, 넘치면 오래된 것부터 payload hash만 남긴 요약으로
//! 강등한다. 요약에 걸린 재시도는 다시 실행하지 않고 `conflict(applied)`로 답한다. 요약이 `max_summaries`에
//! 이르면 그 작업대의 새 command를 거절한다(받으면 중복 보장을 깰 수밖에 없다).

use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    sync::{Arc, Mutex, MutexGuard},
};

use workbench_protocol::{
    CallReply, FaultCode, IdempotencyKey, OperationId, Outcome, PrincipalSubject, RequestId,
    WorkbenchFault,
};

use crate::application::idempotency::{fingerprint, MESSAGE_DIFFERENT_PAYLOAD};

pub const MESSAGE_RESULT_EXPIRED: &str =
    "idempotency result is no longer available; the request was already applied.";
pub const MESSAGE_CAPACITY_EXHAUSTED: &str =
    "bench idempotency capacity exhausted; close and reopen the bench.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpochIdempotencyLimits {
    pub max_results: usize,
    pub max_summaries: usize,
}

impl Default for EpochIdempotencyLimits {
    fn default() -> Self {
        Self {
            max_results: 1_024,
            max_summaries: 65_536,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Key {
    subject: PrincipalSubject,
    operation: OperationId,
    idempotency_key: String,
}

enum Entry {
    Result {
        fingerprint: String,
        reply: CallReply,
    },
    Summary {
        fingerprint: String,
    },
}

#[derive(Default)]
struct ScopeTable {
    entries: HashMap<Key, Entry>,
    /// 결과 기록의 삽입 순서(강등 대상 선정).
    results: VecDeque<Key>,
    summaries: usize,
}

/// 키별 진행 중 요청의 직렬화 lock.
type InFlight = HashMap<(String, Key), Arc<tokio::sync::Mutex<()>>>;

#[derive(Default)]
pub struct EpochIdempotency {
    limits: EpochIdempotencyLimits,
    scopes: Mutex<HashMap<String, ScopeTable>>,
    in_flight: Mutex<InFlight>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 호출 한 번의 멱등 문맥.
pub struct EpochCall<'a> {
    pub scope: String,
    pub subject: &'a PrincipalSubject,
    pub operation: OperationId,
    pub key: &'a IdempotencyKey,
    pub input: &'a serde_json::Value,
    pub request_id: &'a RequestId,
}

impl EpochIdempotency {
    pub fn new(limits: EpochIdempotencyLimits) -> Self {
        Self {
            limits,
            ..Self::default()
        }
    }

    /// 같은 키의 진행 중 요청은 끝날 때까지 기다린 뒤 판정한다. `run`은 기록이 없을 때만 실행된다.
    /// 성공만 기록한다 — 실패한 요청의 재시도는 다시 실행된다(실패는 효과가 없었으므로).
    pub async fn run<F, Fut>(
        &self,
        call: EpochCall<'_>,
        run: F,
    ) -> Result<CallReply, WorkbenchFault>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<CallReply, WorkbenchFault>>,
    {
        let key = Key {
            subject: call.subject.clone(),
            operation: call.operation,
            idempotency_key: call.key.as_str().to_owned(),
        };
        let print = fingerprint(call.operation, call.input);
        let slot = {
            let mut in_flight = lock(&self.in_flight);
            Arc::clone(
                in_flight
                    .entry((call.scope.clone(), key.clone()))
                    .or_default(),
            )
        };
        let guard = slot.lock().await;

        let decided = {
            let scopes = lock(&self.scopes);
            let table = scopes.get(&call.scope);
            match table.and_then(|table| table.entries.get(&key)) {
                Some(Entry::Result { fingerprint, reply }) if *fingerprint == print => {
                    Some(Ok(reply.clone()))
                }
                Some(Entry::Summary { fingerprint }) if *fingerprint == print => {
                    Some(Err(WorkbenchFault::conflict(
                        call.request_id.clone(),
                        MESSAGE_RESULT_EXPIRED,
                        Outcome::Applied,
                    )))
                }
                Some(_) => Some(Err(WorkbenchFault::conflict(
                    call.request_id.clone(),
                    MESSAGE_DIFFERENT_PAYLOAD,
                    Outcome::NotApplied,
                ))),
                None if table.is_some_and(|table| table.summaries >= self.limits.max_summaries) => {
                    Some(Err(WorkbenchFault::new(
                        FaultCode::RateLimited,
                        call.request_id.clone(),
                        MESSAGE_CAPACITY_EXHAUSTED,
                    )
                    .with_retryable(false)))
                }
                None => None,
            }
        };
        let result = match decided {
            Some(result) => result,
            None => {
                let result = run().await;
                if let Ok(reply) = &result {
                    self.record(&call.scope, key.clone(), print, reply.clone());
                }
                result
            }
        };
        drop(guard);
        {
            let mut in_flight = lock(&self.in_flight);
            if Arc::strong_count(&slot) <= 2 {
                in_flight.remove(&(call.scope, key));
            }
        }
        result
    }

    fn record(&self, scope: &str, key: Key, fingerprint: String, reply: CallReply) {
        let mut scopes = lock(&self.scopes);
        let table = scopes.entry(scope.to_owned()).or_default();
        table
            .entries
            .insert(key.clone(), Entry::Result { fingerprint, reply });
        table.results.push_back(key);
        while table.results.len() > self.limits.max_results {
            let Some(oldest) = table.results.pop_front() else {
                break;
            };
            if let Some(Entry::Result { fingerprint, .. }) = table.entries.remove(&oldest) {
                table.entries.insert(oldest, Entry::Summary { fingerprint });
                table.summaries += 1;
            }
        }
    }

    /// 작업대가 닫힐 때 그 작업대의 기록을 버린다.
    pub fn drop_scope(&self, scope: &str) {
        lock(&self.scopes).remove(scope);
    }

    /// 주체별 `bench.open` 기록에서 닫힌 작업대를 만든 항목을 버린다.
    pub fn forget_open_of(&self, scope: &str, bench_id: &str) {
        let mut scopes = lock(&self.scopes);
        if let Some(table) = scopes.get_mut(scope) {
            table.entries.retain(|_, entry| match entry {
                Entry::Result { reply, .. } => {
                    reply.output().and_then(|output| output.get("benchId"))
                        != Some(&serde_json::Value::String(bench_id.to_owned()))
                }
                Entry::Summary { .. } => true,
            });
            let entries = &table.entries;
            table.results.retain(|key| entries.contains_key(key));
        }
    }
}

/// 작업대 id를 멱등 기록 범위로.
pub fn bench_scope(bench_id: &str) -> String {
    format!("bench:{bench_id}")
}

/// `bench.open`처럼 작업대가 아직 없는 호출의 범위(주체별).
pub fn open_scope(subject: &PrincipalSubject) -> String {
    format!("open:{subject}")
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use serde_json::json;

    use super::*;

    fn call<'a>(
        subject: &'a PrincipalSubject,
        key: &'a IdempotencyKey,
        input: &'a serde_json::Value,
        request_id: &'a RequestId,
    ) -> EpochCall<'a> {
        EpochCall {
            scope: bench_scope("b1"),
            subject,
            operation: OperationId::BenchClose,
            key,
            input,
            request_id,
        }
    }

    #[tokio::test]
    async fn replays_demotes_and_limits() {
        let store = EpochIdempotency::new(EpochIdempotencyLimits {
            max_results: 2,
            max_summaries: 1,
        });
        let subject = PrincipalSubject::new("desktop");
        let rid = RequestId::new("r").unwrap();
        let input = json!({"a": 1});
        let runs = AtomicUsize::new(0);
        let exec = || async {
            runs.fetch_add(1, Ordering::SeqCst);
            Ok(CallReply::complete(json!({"n": 1}), None))
        };
        let k1 = IdempotencyKey::new("k1").unwrap();
        store
            .run(call(&subject, &k1, &input, &rid), exec)
            .await
            .unwrap();
        // 같은 키·같은 payload → 저장된 결과, 재실행 없음.
        store
            .run(call(&subject, &k1, &input, &rid), exec)
            .await
            .unwrap();
        assert_eq!(runs.load(Ordering::SeqCst), 1);
        // 다른 payload → conflict.
        let other = json!({"a": 2});
        let fault = store
            .run(call(&subject, &k1, &other, &rid), exec)
            .await
            .unwrap_err();
        assert_eq!(
            (fault.code, fault.outcome),
            (FaultCode::Conflict, Outcome::NotApplied)
        );

        // k2·k3로 k1을 요약으로 강등.
        for name in ["k2", "k3"] {
            let key = IdempotencyKey::new(name).unwrap();
            store
                .run(call(&subject, &key, &input, &rid), exec)
                .await
                .unwrap();
        }
        let fault = store
            .run(call(&subject, &k1, &input, &rid), exec)
            .await
            .unwrap_err();
        assert_eq!(
            (fault.code, fault.outcome),
            (FaultCode::Conflict, Outcome::Applied)
        );
        assert_eq!(fault.message, MESSAGE_RESULT_EXPIRED);
        assert_eq!(
            runs.load(Ordering::SeqCst),
            3,
            "demoted retry must not execute"
        );

        // 요약 한도(1) 도달 → 새 키 거절.
        let k4 = IdempotencyKey::new("k4").unwrap();
        let fault = store
            .run(call(&subject, &k4, &input, &rid), exec)
            .await
            .unwrap_err();
        assert_eq!(fault.code, FaultCode::RateLimited);
        assert!(!fault.retryable);

        // 작업대 닫힘 → 기록 폐기.
        store.drop_scope(&bench_scope("b1"));
        store
            .run(call(&subject, &k4, &input, &rid), exec)
            .await
            .unwrap();
        assert_eq!(runs.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn concurrent_same_key_runs_once() {
        let store = Arc::new(EpochIdempotency::default());
        let runs = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();
        for _ in 0..8 {
            let store = Arc::clone(&store);
            let runs = Arc::clone(&runs);
            tasks.push(tokio::spawn(async move {
                let subject = PrincipalSubject::new("desktop");
                let key = IdempotencyKey::new("same").unwrap();
                let input = json!({});
                let rid = RequestId::new("r").unwrap();
                store
                    .run(call(&subject, &key, &input, &rid), || async {
                        runs.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                        Ok(CallReply::complete(json!(null), None))
                    })
                    .await
                    .unwrap();
            }));
        }
        for task in tasks {
            task.await.unwrap();
        }
        assert_eq!(runs.load(Ordering::SeqCst), 1);
    }
}
