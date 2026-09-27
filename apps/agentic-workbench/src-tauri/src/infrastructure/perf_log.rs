use std::{
    sync::OnceLock,
    time::{Duration, Instant},
};

/// `AW_PERF_LOG=1` 환경 변수로 켜는 성능 계측 로그.
/// 포맷: `perf kind=<command|git|watcher> name=<..> wait_ms=<n> run_ms=<n> [extra]`
/// stderr 전용이며 외부 소비 계약이 아니다(specs/007 research R12). `kind=git` 줄은 Git 어댑터와 함께
/// `workbench-core`(`infrastructure::perf`)로 옮겼다(038 US2) — 같은 환경 변수·형식이다.
pub fn perf_log_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("AW_PERF_LOG").is_ok_and(|value| value == "1"))
}

pub fn log_command(name: &str, wait: Duration, run: Duration) {
    if perf_log_enabled() {
        eprintln!(
            "perf kind=command name={name} wait_ms={} run_ms={}",
            wait.as_millis(),
            run.as_millis()
        );
    }
}

/// `Workbench.call` 경유 command용. blocking pool 대기(wait_ms)는 handler 안 `spawn_blocking`으로 옮겨져
/// 여기서 측정할 대상이 없으므로 `run_ms`만 남긴다(038 research R10).
pub fn log_async_command_run(name: &str, run: Duration) {
    if perf_log_enabled() {
        eprintln!("perf kind=command name={name} run_ms={}", run.as_millis());
    }
}

/// 시그니처상 오류를 돌려줄 수 없는 command(`list_agents`)가 삼킨 오류를 남긴다.
pub fn log_async_command_error(name: &str, error: &str) {
    if perf_log_enabled() {
        eprintln!("perf kind=command name={name} error={error:?}");
    }
}

/// `Workbench.call`을 거치는 command를 감싸 `run_ms`를 perf 로그로 남긴다.
pub async fn log_async_command<T>(
    name: &'static str,
    task: impl std::future::Future<Output = T>,
) -> T {
    let started_at = Instant::now();
    let result = task.await;
    log_async_command_run(name, started_at.elapsed());
    result
}

/// blocking 작업(git 프로세스, WalkDir 등)을 Tauri async runtime의 blocking
/// thread pool에서 실행한다. 동기 command가 main thread를 점유해 IPC 전체를
/// 직렬화하던 문제를 해소한다(specs/007 research R1). invoke 도착→실행 시작
/// 대기 시간(wait_ms)과 실행 시간(run_ms)을 perf 로그로 남긴다.
pub async fn run_blocking_command<T, F>(name: &'static str, task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    let enqueued_at = Instant::now();
    tauri::async_runtime::spawn_blocking(move || {
        let started_at = Instant::now();
        let result = task();
        log_command(name, started_at - enqueued_at, started_at.elapsed());
        result
    })
    .await
    .map_err(|error| format!("Failed to run {name}: {error}"))?
}
