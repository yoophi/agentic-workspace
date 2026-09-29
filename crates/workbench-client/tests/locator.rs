use serde_json::json;
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
};
use workbench_client::{
    application::admission::CallerProfile, infrastructure::locator::read_descriptor,
};
fn fixture() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path().canonicalize().unwrap();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
    let path = parent.join("server.json");
    fs::write(&path, json!({"formatVersion":1,"mode":"server","instanceId":"i","serverEpoch":"e","baseUrl":"http://127.0.0.1:32123","protocolVersions":[1],"storageSchemaVersion":2,"ownerToken":"private-sentinel"}).to_string()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    (dir, path)
}
#[test]
fn owner_private_descriptor_is_readonly_and_redacted() {
    let (_dir, path) = fixture();
    let before = fs::read(&path).unwrap();
    let endpoint = read_descriptor(&path, CallerProfile::Owner).unwrap();
    assert_eq!(endpoint.identity().instance(), "i");
    assert!(!format!("{endpoint:?}").contains("private-sentinel"));
    assert_eq!(fs::read(&path).unwrap(), before);
}
#[test]
fn symlink_file_and_parent_are_rejected() {
    let (_dir, path) = fixture();
    let link = path.with_file_name("link");
    symlink(&path, &link).unwrap();
    assert!(read_descriptor(&link, CallerProfile::Owner).is_err());
    let parent_link = path.parent().unwrap().join("parent-link");
    symlink(path.parent().unwrap(), &parent_link).unwrap();
    assert!(read_descriptor(&parent_link.join("server.json"), CallerProfile::Owner).is_err());
}
#[test]
fn permissions_type_size_and_traversal_are_rejected() {
    let (_dir, path) = fixture();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(read_descriptor(&path, CallerProfile::Owner).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&path, vec![b' '; 65537]).unwrap();
    assert!(read_descriptor(&path, CallerProfile::Owner).is_err());
    assert!(read_descriptor(path.parent().unwrap(), CallerProfile::Owner).is_err());
    assert!(read_descriptor(
        &path.parent().unwrap().join("../server.json"),
        CallerProfile::Owner
    )
    .is_err());
    fs::set_permissions(path.parent().unwrap(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(read_descriptor(&path, CallerProfile::Owner).is_err());
}
#[test]
fn agent_refuses_before_open_even_when_descriptor_is_missing() {
    assert!(read_descriptor(
        std::path::Path::new("/missing/server.json"),
        CallerProfile::AgentScoped
    )
    .is_err());
}
#[test]
fn endpoint_url_and_compatibility_are_closed() {
    for url in [
        "http://localhost:80",
        "https://127.0.0.1:80",
        "http://192.168.1.1:80",
        "http://user@127.0.0.1:80",
        "http://127.0.0.1:80/?token=secret",
        "http://127.0.0.1:80/a",
        "http://127.0.0.1:80#secret",
    ] {
        let (_dir, path) = fixture();
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        value["baseUrl"] = json!(url);
        fs::write(&path, value.to_string()).unwrap();
        assert!(
            read_descriptor(&path, CallerProfile::Owner).is_err(),
            "{url}"
        );
    }
    let (_dir, path) = fixture();
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["storageSchemaVersion"] = json!(999);
    fs::write(&path, value.to_string()).unwrap();
    assert!(read_descriptor(&path, CallerProfile::Owner).is_err());
}

#[test]
fn missing_descriptor_is_unavailable_but_permission_and_symlink_are_private_state() {
    use workbench_client::ports::ClientError;
    let (_dir, path) = fixture();
    let link = path.with_file_name("link");
    symlink(&path, &link).unwrap();
    assert!(matches!(
        read_descriptor(&link, CallerProfile::Owner),
        Err(ClientError::PrivateState)
    ));
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(
        read_descriptor(&path, CallerProfile::Owner),
        Err(ClientError::PrivateState)
    ));
    fs::remove_file(&path).unwrap();
    let error = read_descriptor(&path, CallerProfile::Owner).unwrap_err();
    assert!(matches!(error, ClientError::Unavailable));
    assert!(!format!("{error:?} {error}").contains(path.to_str().unwrap()));
}
