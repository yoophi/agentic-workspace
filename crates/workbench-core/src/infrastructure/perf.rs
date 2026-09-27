//! AW와 같은 `AW_PERF_LOG=1` 계측 로그(stderr 전용, 외부 계약 아님). Git 어댑터가 core로 옮겨 오면서
//! `kind=git` 한 줄을 여기서 남긴다. 형식은 AW `perf_log::log_git`과 같다.

use std::{sync::OnceLock, time::Duration};

fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("AW_PERF_LOG").is_ok_and(|value| value == "1"))
}

pub fn log_git(name: &str, run: Duration) {
    if enabled() {
        eprintln!("perf kind=git name={name} run_ms={}", run.as_millis());
    }
}
