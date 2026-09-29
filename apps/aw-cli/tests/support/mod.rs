#![allow(dead_code)]
#[path = "../../../../crates/workbench-client/tests/support/mod.rs"]
pub mod peer;
use std::{
    process::{Output, Stdio},
    time::Duration,
};
use tokio::{
    io::AsyncReadExt,
    process::{Child, Command},
};
pub fn spawn(args: &[&str]) -> Child {
    Command::new(env!("CARGO_BIN_EXE_aw"))
        .args(args)
        .kill_on_drop(true)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}
pub async fn finish(child: Child, input: &[u8]) -> Output {
    finish_inner(child, Some(input)).await
}
// Keep the input producer alive through wait/reap: no EOF can unblock the child.
pub async fn finish_with_open_input<T>(child: Child, input_owner: T) -> Output {
    let result = finish_inner(child, None).await;
    drop(input_owner);
    result
}
async fn finish_inner(mut child: Child, input: Option<&[u8]>) -> Output {
    use tokio::io::AsyncWriteExt;
    let stdin = child.stdin.take();
    let mut stdout = child
        .stdout
        .take()
        .map(|out| out.take(16 * 1024 * 1024 + 1));
    let mut stderr = child.stderr.take().unwrap().take(16 * 1024 * 1024 + 1);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = tokio::time::timeout(Duration::from_secs(3), async {
        let (_, read_out, read_err, status) = tokio::join!(
            async {
                if let (Some(mut stdin), Some(input)) = (stdin, input) {
                    let _ = stdin.write_all(input).await;
                }
            },
            async {
                match stdout.as_mut() {
                    Some(stdout) => stdout.read_to_end(&mut out).await,
                    None => Ok(0),
                }
            },
            stderr.read_to_end(&mut err),
            child.wait()
        );

        read_out.unwrap();
        read_err.unwrap();
        assert!(out.len() <= 16 * 1024 * 1024 && err.len() <= 16 * 1024 * 1024);
        Output {
            status: status.unwrap(),
            stdout: out,
            stderr: err,
        }
    })
    .await;
    match result {
        Ok(output) => output,
        Err(_) => {
            child.start_kill().ok();
            tokio::time::timeout(Duration::from_secs(1), child.wait())
                .await
                .expect("CLI cleanup timed out")
                .expect("CLI reap failed");
            panic!("CLI fixture deadline exceeded after kill/reap");
        }
    }
}
pub async fn run(args: &[&str], input: &[u8]) -> Output {
    finish(spawn(args), input).await
}
pub fn state_directory() -> tempfile::TempDir {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    dir
}
pub fn assert_error(output: &Output, exit: i32) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(exit));
    assert!(output.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["ok"], false);
    assert!(!String::from_utf8_lossy(&output.stderr).contains("private-sentinel"));
    value
}
