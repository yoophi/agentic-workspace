use serde_json::Value;
mod support;
use std::process::{Command, Output};
fn run(args: &[&str], input: &[u8]) -> Output {
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(support::run(args, input))
}
fn finite_error(output: Output, exit: i32) -> Value {
    assert_eq!(output.status.code(), Some(exit));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr.iter().filter(|b| **b == b'\n').count(), 1);
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["ok"], false);
    assert!(!String::from_utf8_lossy(&output.stderr).contains("private-sentinel"));
    value
}
#[test]
fn operations_returns_one_catalog_json_and_no_diagnostics() {
    let output = run(&["operations"], b"");
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout.iter().filter(|b| **b == b'\n').count(), 1);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"].as_array().unwrap().len(), 94);
}
#[test]
fn unknown_operation_and_secret_argv_are_safe_usage_errors() {
    for args in [
        &["operations", "future.operation"][..],
        &["call", "project.list", "--token", "private-sentinel"][..],
        &[][..],
    ] {
        finite_error(run(args, b""), 2);
    }
}
#[test]
fn invalid_utf8_json_schema_and_input_quota_fail_before_endpoint_lookup() {
    for input in [
        &[255u8][..],
        b"not-json-private-sentinel",
        b"{\"private-sentinel\":1}",
        &vec![b' '; 1024 * 1024 + 1],
    ] {
        finite_error(run(&["call", "project.list", "--input", "-"], input), 2);
    }
}
#[test]
fn explicit_and_generic_admission_have_identical_outcomes() {
    let explicit = finite_error(run(&["run", "start", "--input", "-"], b"{}"), 8);
    let generic = finite_error(run(&["call", "run.start", "--input", "-"], b"{}"), 8);
    assert_eq!(explicit["error"], generic["error"]);
}
#[test]
fn missing_server_has_unavailable_outcome_without_raw_path() {
    let output = run(
        &[
            "project",
            "list",
            "--descriptor",
            "/private/missing-private-sentinel/server.json",
        ],
        b"",
    );
    let value = finite_error(output, 8);
    assert_eq!(value["error"]["code"], "unavailable");
    assert_eq!(value["error"]["outcome"], "notApplied");
}

#[test]
fn explicit_and_generic_cancel_preserve_current_prerequisite_gate() {
    let explicit = finite_error(
        run(&["run", "cancel", "r", "--idempotency-key", "key"], b""),
        8,
    );
    let generic = finite_error(
        run(
            &[
                "call",
                "run.cancel",
                "--input",
                "-",
                "--idempotency-key",
                "key",
            ],
            br#"{"benchId":"b","runId":"r"}"#,
        ),
        8,
    );
    assert_eq!(explicit["error"], generic["error"]);
    assert_eq!(explicit["error"]["code"], "prerequisiteUnavailable");
}

#[test]
fn non_utf8_argv_is_a_safe_finite_usage_error() {
    use std::os::unix::ffi::OsStringExt;
    let output = Command::new(env!("CARGO_BIN_EXE_aw"))
        .arg(std::ffi::OsString::from_vec(vec![255]))
        .output()
        .unwrap();
    finite_error(output, 2);
}

#[tokio::test]
async fn all_server_fault_families_have_safe_subprocess_exits_and_exact_outcome() {
    use workbench_protocol::{FaultCode, Outcome};
    let exits = [2, 3, 3, 4, 5, 10, 5, 9, 9, 8, 8, 8, 7, 1];
    for (code, exit) in FaultCode::ALL.into_iter().zip(exits) {
        let mut peer = support::peer::Peer::spawn(
            vec![support::peer::Action::Fault(code, Outcome::Unknown)],
            true,
        )
        .await;
        let output = support::run(
            &[
                "project",
                "list",
                "--descriptor",
                peer.descriptor.to_str().unwrap(),
            ],
            b"",
        )
        .await;
        let error = finite_error(output, exit);
        assert_eq!(error["error"]["code"], code.as_str());
        assert_eq!(error["error"]["outcome"], "unknown");
        assert!(error["requestId"].is_string());
        assert!(
            !String::from_utf8_lossy(&serde_json::to_vec(&error).unwrap())
                .contains(support::peer::TOKEN)
        );
        peer.settled().await;
        assert_eq!(peer.requests.lock().unwrap().len(), 3);
    }
}
