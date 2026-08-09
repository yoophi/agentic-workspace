use super::*;
#[test]
fn missing_trash_is_safe() {
    let base = std::env::temp_dir().join(format!("ma-data-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    assert_eq!(DataManagementService::new(base).purge_expired().unwrap(), 0);
}
#[test]
fn quota_removes_snapshot_before_live_sessions() {
    let base = std::env::temp_dir().join(format!("ma-quota-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("reviews/snapshots/s")).unwrap();
    std::fs::create_dir_all(base.join("reviews/sessions")).unwrap();
    std::fs::write(base.join("reviews/snapshots/s/1.json"), vec![0; 20]).unwrap();
    std::fs::write(base.join("reviews/sessions/live.json"), vec![0; 20]).unwrap();
    DataManagementService::new(&base).enforce_quota(0).unwrap();
    assert!(base.join("reviews/sessions/live.json").exists());
    assert!(!base.join("reviews/snapshots/s/1.json").exists());
    std::fs::remove_dir_all(base).unwrap();
}
