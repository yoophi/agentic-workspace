//! ACP 세션 기록 저장소(`acp-sessions.json`). 040: AW `infrastructure/json_acp_session_store.rs`에서 이동 —
//! run 기계(`AcpRunEngine`)와 041 전 AW orchestration이 **같은 인스턴스**를 쓴다(같은 파일을 두 인스턴스가
//! 쓰지 않게). 인스턴스 안에서는 읽기-수정-쓰기를 `io_lock`으로 직렬화하고(여러 run·orchestration task가 서로 다른
//! thread에서 동시에 기록한다), 쓰기는 임시 파일에 쓴 뒤 rename해 읽는 쪽이 잘린 파일을 보지 않게 한다.

use std::{
    fs,
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};

use acp_agent_core::{
    domain::acp_session::{AcpSessionListQuery, AcpSessionLookup, AcpSessionRecord},
    ports::acp_session_store::AcpSessionStore,
};
use anyhow::{Context, Result};

use crate::infrastructure::data_paths::DataPaths;

pub struct JsonAcpSessionStore {
    store_path: PathBuf,
    io_lock: Mutex<()>,
}

impl JsonAcpSessionStore {
    pub fn from_paths(paths: &DataPaths) -> Self {
        Self::new(paths.app_data_dir().join("acp-sessions.json"))
    }

    pub fn new(store_path: PathBuf) -> Self {
        Self {
            store_path,
            io_lock: Mutex::new(()),
        }
    }

    /// 임계 구역 안에는 await가 없다(동기 파일 I/O뿐). 앞선 기록이 panic했어도 파일은 rename 단위로 온전하다.
    fn io(&self) -> MutexGuard<'_, ()> {
        self.io_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn load_records(&self) -> Result<Vec<AcpSessionRecord>> {
        if !self.store_path.exists() {
            return Ok(Vec::new());
        }

        let contents = fs::read_to_string(&self.store_path).with_context(|| {
            format!("Failed to read ACP session store at {:?}", self.store_path)
        })?;

        serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse ACP session store at {:?}", self.store_path))
    }

    fn save_records(&self, records: &[AcpSessionRecord]) -> Result<()> {
        if let Some(parent) = self.store_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create ACP session store directory at {:?}",
                    parent
                )
            })?;
        }

        let contents =
            serde_json::to_string_pretty(records).context("Failed to serialize ACP sessions")?;

        let temp_path = self.store_path.with_extension("json.tmp");
        fs::write(&temp_path, contents)
            .with_context(|| format!("Failed to write ACP session store at {:?}", temp_path))?;
        fs::rename(&temp_path, &self.store_path)
            .with_context(|| format!("Failed to write ACP session store at {:?}", self.store_path))
    }
}

impl AcpSessionStore for JsonAcpSessionStore {
    fn record_session<'a>(
        &'a self,
        mut record: AcpSessionRecord,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let _io = self.io();
            let mut records = self.load_records()?;

            if let Some(existing) = records
                .iter_mut()
                .find(|existing| existing.run_id == record.run_id)
            {
                record.created_at.clone_from(&existing.created_at);
                record.updated_at = timestamp();
                *existing = record;
            } else {
                records.push(record);
            }

            self.save_records(&records)
        })
    }

    fn latest_session<'a>(
        &'a self,
        lookup: AcpSessionLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<AcpSessionRecord>>> + Send + 'a>> {
        Box::pin(async move {
            let records = {
                let _io = self.io();
                self.load_records()?
            };

            Ok(records
                .into_iter()
                .filter(|record| matches_lookup(record, &lookup))
                .max_by_key(record_sort_key))
        })
    }

    fn list_sessions<'a>(
        &'a self,
        query: AcpSessionListQuery,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<AcpSessionRecord>>> + Send + 'a>> {
        Box::pin(async move {
            let loaded = {
                let _io = self.io();
                self.load_records()?
            };
            let mut records: Vec<_> = loaded
                .into_iter()
                .filter(|record| matches_query(record, &query))
                .collect();

            records.sort_by_key(record_sort_key);
            records.reverse();

            if let Some(limit) = query.limit {
                records.truncate(limit as usize);
            }

            Ok(records)
        })
    }

    fn clear_session<'a>(
        &'a self,
        run_id: String,
    ) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'a>> {
        Box::pin(async move {
            let _io = self.io();
            let mut records = self.load_records()?;
            let before_len = records.len();
            records.retain(|record| record.run_id != run_id);

            let removed = records.len() != before_len;
            if removed {
                self.save_records(&records)?;
            }

            Ok(removed)
        })
    }
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn record_sort_key(record: &AcpSessionRecord) -> u64 {
    record
        .updated_at
        .parse::<u64>()
        .or_else(|_| record.created_at.parse::<u64>())
        .unwrap_or(0)
}

fn matches_lookup(record: &AcpSessionRecord, lookup: &AcpSessionLookup) -> bool {
    record.agent_id == lookup.agent_id
        && option_filter_matches(&lookup.workspace_id, &record.workspace_id)
        && option_filter_matches(&lookup.checkout_id, &record.checkout_id)
        && option_filter_matches(&lookup.workdir, &record.workdir)
        && option_filter_matches(&lookup.agent_command, &record.agent_command)
}

fn matches_query(record: &AcpSessionRecord, query: &AcpSessionListQuery) -> bool {
    option_filter_matches(&query.workspace_id, &record.workspace_id)
        && option_filter_matches(&query.checkout_id, &record.checkout_id)
        && option_filter_matches(&query.workdir, &record.workdir)
        && option_filter_matches(&query.agent_id, &Some(record.agent_id.clone()))
        && option_filter_matches(&query.agent_command, &record.agent_command)
}

fn option_filter_matches(filter: &Option<String>, value: &Option<String>) -> bool {
    filter
        .as_ref()
        .is_none_or(|filter| value.as_ref().is_some_and(|value| value == filter))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(
        run_id: &str,
        session_id: &str,
        agent_id: &str,
        updated_at: &str,
    ) -> AcpSessionRecord {
        AcpSessionRecord {
            run_id: run_id.to_string(),
            session_id: session_id.to_string(),
            workspace_id: Some("workspace-1".to_string()),
            checkout_id: Some("checkout-1".to_string()),
            workdir: Some("/tmp/project".to_string()),
            agent_id: agent_id.to_string(),
            agent_command: Some("codex".to_string()),
            task: "Implement feature".to_string(),
            created_at: updated_at.to_string(),
            updated_at: updated_at.to_string(),
        }
    }

    fn store_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "acp-session-store-{name}-{}.json",
            uuid::Uuid::new_v4()
        ))
    }

    #[tokio::test]
    async fn records_lists_and_clears_sessions() {
        let store = JsonAcpSessionStore::new(store_path("crud"));

        store
            .record_session(record("run-1", "session-1", "codex", "100"))
            .await
            .unwrap();
        store
            .record_session(record("run-2", "session-2", "codex", "200"))
            .await
            .unwrap();
        store
            .record_session(record("run-3", "session-3", "other", "300"))
            .await
            .unwrap();

        let latest = store
            .latest_session(AcpSessionLookup {
                workspace_id: Some("workspace-1".to_string()),
                checkout_id: Some("checkout-1".to_string()),
                workdir: Some("/tmp/project".to_string()),
                agent_id: "codex".to_string(),
                agent_command: Some("codex".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(latest.unwrap().run_id, "run-2");

        let sessions = store
            .list_sessions(AcpSessionListQuery {
                agent_id: Some("codex".to_string()),
                limit: Some(1),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].run_id, "run-2");

        assert!(store.clear_session("run-2".to_string()).await.unwrap());
        assert!(!store.clear_session("run-2".to_string()).await.unwrap());
    }

    #[tokio::test]
    async fn upserts_sessions_by_run_id() {
        let store = JsonAcpSessionStore::new(store_path("upsert"));

        store
            .record_session(record("run-1", "session-1", "codex", "100"))
            .await
            .unwrap();
        store
            .record_session(record("run-1", "session-2", "codex", "200"))
            .await
            .unwrap();

        let sessions = store
            .list_sessions(AcpSessionListQuery {
                agent_id: Some("codex".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, "session-2");
        assert_eq!(sessions[0].created_at, "100");
    }
}
