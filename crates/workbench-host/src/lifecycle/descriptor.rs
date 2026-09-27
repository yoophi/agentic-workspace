//! 서버 안내 파일 `server.json`(044 research R4, contracts/server-lifecycle.md §2). 준비된 서버만 쓰고, 소유 사용자만
//! 읽는다(0600). 같은 디렉터리의 임시 파일(0600)에 쓴 뒤 fsync → rename으로 원자적으로 바꾸고 디렉터리도 fsync한다.
//! 서버는 정지할 때 자기 인스턴스의 것일 때만 지운다. `pid`는 진단용이며 살아 있음·동일성 판단에 쓰지 않는다.

use std::{
    fs::{self, File},
    io::{self, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::identity::OwnerIdentity;

pub const DESCRIPTOR: &str = "server.json";
pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Descriptor {
    pub format_version: u32,
    /// `server`(독립 서버) 또는 `embedded`(AW 개발 모드, T028).
    pub mode: String,
    pub instance_id: String,
    pub server_epoch: String,
    pub pid: u32,
    pub base_url: String,
    pub server_version: String,
    pub protocol_versions: Vec<u16>,
    pub storage_schema_version: i64,
    pub owner_token: String,
    pub started_at: String,
}

impl Descriptor {
    /// 자격 증명을 뺀 안내 JSON(표준 출력·오류 보고용).
    pub fn public_json(&self) -> serde_json::Value {
        let mut value = serde_json::to_value(self).expect("descriptor json");
        if let Some(object) = value.as_object_mut() {
            object.remove("ownerToken");
        }
        value
    }

    pub fn identity(&self) -> OwnerIdentity {
        OwnerIdentity::from_parts(&self.instance_id, &self.owner_token)
    }

    /// 시험용: 신원과 끝점만 채운 안내.
    pub fn for_test(identity: &OwnerIdentity, base_url: &str) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            mode: "server".into(),
            instance_id: identity.instance_id().to_owned(),
            server_epoch: "test".into(),
            pid: std::process::id(),
            base_url: base_url.to_owned(),
            server_version: "test".into(),
            protocol_versions: vec![workbench_protocol::PROTOCOL_VERSION],
            storage_schema_version: workbench_core::infrastructure::sqlite_ledger::SCHEMA_VERSION,
            owner_token: identity.token().to_owned(),
            started_at: "1970-01-01T00:00:00Z".into(),
        }
    }
}

pub fn descriptor_path(server_dir: &Path) -> PathBuf {
    server_dir.join(DESCRIPTOR)
}

/// 원자적 쓰기: 0600 임시 파일 → fsync → rename → 디렉터리 fsync.
pub fn write_descriptor(server_dir: &Path, descriptor: &Descriptor) -> io::Result<()> {
    let temp = server_dir.join(format!(
        ".{DESCRIPTOR}.{}.tmp",
        uuid::Uuid::new_v4().simple()
    ));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)?;
        file.write_all(&serde_json::to_vec_pretty(descriptor).map_err(io::Error::other)?)?;
        file.sync_all()?;
        fs::rename(&temp, descriptor_path(server_dir))?;
        File::open(server_dir)?.sync_all()
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

/// 안내 파일을 읽는다. 없거나 읽을 수 없는 형식이면 `None`(시작 절차가 판별한다).
pub fn read_descriptor(server_dir: &Path) -> io::Result<Option<Descriptor>> {
    match fs::read(descriptor_path(server_dir)) {
        Ok(bytes) => Ok(serde_json::from_slice::<Descriptor>(&bytes)
            .ok()
            .filter(|descriptor| descriptor.format_version == FORMAT_VERSION)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// 자기 인스턴스의 안내 파일일 때만 지운다.
pub fn remove_descriptor_if(server_dir: &Path, instance_id: &str) -> io::Result<bool> {
    match read_descriptor(server_dir)? {
        Some(descriptor) if descriptor.instance_id == instance_id => {
            fs::remove_file(descriptor_path(server_dir))?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// 남은(죽은 서버의) 안내 파일을 지운다. 호출자는 `owner.lock`을 쥔 상태여야 한다.
pub fn remove_stale_descriptor(server_dir: &Path) -> io::Result<()> {
    match fs::remove_file(descriptor_path(server_dir)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    #[test]
    fn descriptors_are_written_owner_only_and_removed_only_by_their_instance() {
        let dir = tempfile::tempdir().unwrap();
        let identity = OwnerIdentity::generate();
        let descriptor = Descriptor::for_test(&identity, "http://127.0.0.1:1");
        write_descriptor(dir.path(), &descriptor).unwrap();
        let mode = fs::metadata(descriptor_path(dir.path()))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(
            read_descriptor(dir.path()).unwrap(),
            Some(descriptor.clone())
        );
        assert!(
            !descriptor
                .public_json()
                .to_string()
                .contains(identity.token())
        );
        assert!(!remove_descriptor_if(dir.path(), "other").unwrap());
        assert!(remove_descriptor_if(dir.path(), identity.instance_id()).unwrap());
        assert_eq!(read_descriptor(dir.path()).unwrap(), None);
        let leftovers: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
        assert!(leftovers.is_empty(), "no temp files left behind");
    }
}
