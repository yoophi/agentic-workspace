use std::{path::PathBuf, process::Command};

#[test]
fn production_spawn_sources_match_the_exact_inventory() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root");
    let output = Command::new("python3")
        .arg(root.join("scripts/check-process-spawn-inventory.py"))
        .arg("--json")
        .current_dir(root)
        .output()
        .expect("inventory script runs");
    assert!(
        output.status.success(),
        "inventory mismatch:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn inventory_gate_rejects_alias_and_same_file_drift_fixtures() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root");
    let output = Command::new("python3")
        .args([
            "-m",
            "unittest",
            "scripts/tests/test_process_spawn_inventory.py",
        ])
        .current_dir(root)
        .output()
        .expect("inventory negative fixtures run");
    assert!(
        output.status.success(),
        "negative inventory fixtures failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
