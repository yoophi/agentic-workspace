//! CLI composition root; command wiring follows the reviewed implementation tasks.
fn main() -> std::process::ExitCode {
    eprintln!(
        r#"{{"ok":false,"error":{{"code":"unavailable","outcome":"notApplied","retryable":false}}}}"#
    );
    std::process::ExitCode::from(8)
}
