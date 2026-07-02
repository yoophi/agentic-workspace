use std::{
    sync::OnceLock,
    time::{Duration, Instant},
};

/// `AW_PERF_LOG=1` 환경 변수로 켜는 성능 계측 로그.
/// 포맷: `perf kind=<command|git|watcher> name=<..> wait_ms=<n> run_ms=<n> [extra]`
/// stderr 전용이며 외부 소비 계약이 아니다(specs/007 research R12).
pub fn perf_log_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("AW_PERF_LOG").is_ok_and(|value| value == "1")
    })
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

pub fn log_git(name: &str, run: Duration) {
    if perf_log_enabled() {
        eprintln!("perf kind=git name={name} run_ms={}", run.as_millis());
    }
}

pub fn log_watcher(name: &str, extra: &str) {
    if perf_log_enabled() {
        eprintln!("perf kind=watcher name={name} {extra}");
    }
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
