//! 데스크톱의 Workbench 모드(044 T028, research R11, contracts/desktop-client.md §1).
//!
//! - `external`(기본): 앱 안에 런타임을 두지 않는다. 안내 파일로 독립 서버를 찾거나 띄워 붙는다. 호환 command는 쓸 수 없다.
//! - `embedded`(`AW_WORKBENCH_MODE=embedded`, 개발·시험): 043 경로(앱 안 런타임 + HTTP + 호환 command). 같은 데이터
//!   디렉터리의 `owner.lock`을 잡고 안내 파일(`mode: embedded`)을 쓴다 — 외부 서버와 동시에 쓰지 못한다. 8단계에서 지운다.

use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use workbench_host::lifecycle::{
    descriptor::{Descriptor, remove_descriptor_if, write_descriptor},
    identity::OwnerIdentity,
    lock::{HeldLock, ensure_server_dir, try_owner_lock},
};

pub const MODE_ENV: &str = "AW_WORKBENCH_MODE";
pub const MESSAGE_EXTERNAL_UNAVAILABLE: &str =
    "Workbench server is external; this command is unavailable.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkbenchMode {
    External,
    Embedded,
}

impl WorkbenchMode {
    pub fn from_env() -> Self {
        Self::parse(std::env::var(MODE_ENV).ok().as_deref())
    }

    pub fn parse(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some(value) if value.eq_ignore_ascii_case("embedded") => Self::Embedded,
            _ => Self::External,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::External => "external",
            Self::Embedded => "embedded",
        }
    }
}

/// embedded 모드의 단일 writer 소유: `owner.lock`을 쥐고, 준비되면 안내 파일을 쓴다. 앱이 끝날 때 자기 안내만 지운다.
pub struct EmbeddedOwnership {
    _lock: HeldLock,
    server_dir: PathBuf,
    identity: OwnerIdentity,
    owner_token: Mutex<Option<String>>,
}

impl EmbeddedOwnership {
    /// 소유 잠금을 잡는다. 이미 서버(외부·다른 embedded)가 쥐고 있으면 부팅 실패 이유를 돌려준다.
    pub fn claim(data_dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(data_dir).map_err(|error| error.to_string())?;
        let server_dir = ensure_server_dir(data_dir).map_err(|error| error.to_string())?;
        let lock = try_owner_lock(data_dir)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| {
                format!(
                    "Another Workbench server owns this data directory ({}); stop it or unset {MODE_ENV}.",
                    data_dir.display()
                )
            })?;
        let (identity, owner_token) = OwnerIdentity::generate().into_parts();
        Ok(Self {
            _lock: lock,
            server_dir,
            identity,
            owner_token: Mutex::new(Some(owner_token)),
        })
    }

    pub fn identity(&self) -> &OwnerIdentity {
        &self.identity
    }

    pub fn publish(
        &self,
        server_epoch: &str,
        base_url: &str,
        version: &str,
    ) -> std::io::Result<()> {
        let mut owner_token = self
            .owner_token
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let token = owner_token
            .as_deref()
            .ok_or_else(|| std::io::Error::other("embedded descriptor was already published"))?;
        let result = write_descriptor(
            &self.server_dir,
            &Descriptor::for_endpoint(
                "embedded",
                &self.identity,
                token,
                server_epoch,
                base_url,
                version,
            ),
        );
        if result.is_ok() {
            *owner_token = None;
        }
        result
    }

    pub fn withdraw(&self) {
        let _ = remove_descriptor_if(&self.server_dir, self.identity.instance_id());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_mode_is_external() {
        assert_eq!(WorkbenchMode::parse(None), WorkbenchMode::External);
        assert_eq!(
            WorkbenchMode::parse(Some("external")),
            WorkbenchMode::External
        );
        assert_eq!(
            WorkbenchMode::parse(Some("Embedded")),
            WorkbenchMode::Embedded
        );
        assert_eq!(WorkbenchMode::parse(Some("junk")), WorkbenchMode::External);
    }

    #[test]
    fn embedded_ownership_is_exclusive_and_publishes_an_embedded_descriptor() {
        let dir = tempfile::tempdir().unwrap();
        let first = EmbeddedOwnership::claim(dir.path()).expect("first claim");
        assert!(
            EmbeddedOwnership::claim(dir.path()).is_err(),
            "the lock is exclusive"
        );
        first.publish("epoch", "http://127.0.0.1:1", "v").unwrap();
        let server_dir = ensure_server_dir(dir.path()).unwrap();
        let written = workbench_host::lifecycle::descriptor::read_descriptor(&server_dir)
            .unwrap()
            .unwrap();
        assert_eq!(written.mode, "embedded");
        first.withdraw();
        assert!(
            workbench_host::lifecycle::descriptor::read_descriptor(&server_dir)
                .unwrap()
                .is_none()
        );
        drop(first);
        assert!(
            EmbeddedOwnership::claim(dir.path()).is_ok(),
            "released with the ownership"
        );
    }
}
