//! JSON 문서 저장. AW `infrastructure/json_store.rs`를 기반으로 하되 **읽기와 복구를 분리**했다(research R14).
//!
//! - `load_json_vec`는 읽기 전용이다. primary가 손상되면 `StoreError::PrimaryCorrupt`를 돌려주고 파일을 쓰지 않는다.
//! - `recover_from_backup`은 aggregate lock을 잡은 호출자만 부른다. temp + rename으로 교체한다.
//!
//! 오류 문구는 AW 원본과 같은 형식을 유지한다(Tauri compat 골든).

use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use serde::{de::DeserializeOwned, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StoreError {
    /// 파일을 읽을 수 없음(권한, I/O).
    #[error("Failed to read {label} store {path}: {cause}")]
    Read {
        label: String,
        path: String,
        cause: String,
    },
    /// 파일은 있으나 JSON으로 파싱할 수 없음. 복구 후보.
    #[error("Failed to parse {label} store {path}: {cause}")]
    PrimaryCorrupt {
        label: String,
        path: String,
        cause: String,
    },
    #[error("{0}")]
    Write(String),
    /// primary가 손상됐는데 `.bak`도 없거나 손상됨.
    #[error("{primary}; backup recovery failed: {backup}")]
    RecoveryUnavailable { primary: String, backup: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryOutcome {
    /// primary가 이미 정상이라 아무 것도 하지 않았다(다른 쓰기가 먼저 고쳤을 때).
    PrimaryHealthy,
    /// `.bak` 내용으로 primary를 교체했다.
    Recovered,
}

/// 읽기 전용. 파일이 없으면 빈 벡터.
pub fn load_json_vec<T>(store_path: &Path, label: &str) -> Result<Vec<T>, StoreError>
where
    T: DeserializeOwned,
{
    if !store_path.exists() {
        return Ok(Vec::new());
    }
    read_json(store_path, label)
}

pub fn save_json_vec<T>(store_path: &Path, label: &str, values: &[T]) -> Result<(), StoreError>
where
    T: Serialize,
{
    let contents = serde_json::to_vec_pretty(values)
        .map_err(|error| StoreError::Write(format!("Failed to serialize {label}: {error}")))?;
    atomic_write(store_path, label, &contents)
}

/// primary가 손상됐을 때만 `.bak`으로 교체한다. 호출 전에 반드시 aggregate lock을 잡는다.
///
/// `T`는 `load_json_vec`가 쓰는 것과 **같은 문서 타입**(예: `Vec<Project>`)이어야 한다. `serde_json::Value`로
/// 검증하면 "JSON 문법은 맞지만 필드가 빠진" 파일을 정상으로 보아 복구를 건너뛰고, 이후 읽기가 계속 실패한다.
pub fn recover_from_backup<T>(store_path: &Path, label: &str) -> Result<RecoveryOutcome, StoreError>
where
    T: DeserializeOwned,
{
    // 다른 쓰기가 이미 고쳤을 수 있으므로 lock 안에서 다시 확인한다.
    let primary_error = match read_json::<T>(store_path, label) {
        Ok(_) => return Ok(RecoveryOutcome::PrimaryHealthy),
        Err(error @ StoreError::PrimaryCorrupt { .. }) if store_path.exists() => error,
        Err(error) => return Err(error),
    };

    let backup_path = backup_path(store_path);
    if !backup_path.exists() {
        return Err(StoreError::RecoveryUnavailable {
            primary: primary_error.to_string(),
            backup: format!("backup {} does not exist", backup_path.display()),
        });
    }

    let backup_contents =
        fs::read(&backup_path).map_err(|error| StoreError::RecoveryUnavailable {
            primary: primary_error.to_string(),
            backup: error.to_string(),
        })?;
    // backup도 같은 타입으로 검증한다. 구조가 어긋난 backup으로 primary를 덮으면 복구가 아니라 손상 전파다.
    serde_json::from_slice::<T>(&backup_contents).map_err(|error| {
        StoreError::RecoveryUnavailable {
            primary: primary_error.to_string(),
            backup: error.to_string(),
        }
    })?;

    let temp_path = temp_path(store_path);
    write_temp_file(&temp_path, label, &backup_contents)?;
    replace_file(&temp_path, store_path).map_err(|error| {
        let _ = fs::remove_file(&temp_path);
        StoreError::Write(format!(
            "Recovered {label} store from backup, but failed to restore {}: {error}",
            store_path.display()
        ))
    })?;
    Ok(RecoveryOutcome::Recovered)
}

fn read_json<T>(store_path: &Path, label: &str) -> Result<T, StoreError>
where
    T: DeserializeOwned,
{
    let contents = fs::read_to_string(store_path).map_err(|error| StoreError::Read {
        label: label.to_owned(),
        path: store_path.display().to_string(),
        cause: error.to_string(),
    })?;

    serde_json::from_str(&contents).map_err(|error| StoreError::PrimaryCorrupt {
        label: label.to_owned(),
        path: store_path.display().to_string(),
        cause: error.to_string(),
    })
}

fn atomic_write(store_path: &Path, label: &str, contents: &[u8]) -> Result<(), StoreError> {
    let parent = store_path.parent().ok_or_else(|| {
        StoreError::Write(format!(
            "Failed to resolve parent directory for {}",
            store_path.display()
        ))
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        StoreError::Write(format!("Failed to create {label} store directory: {error}"))
    })?;

    let temp_path = temp_path(store_path);
    let backup_path = backup_path(store_path);

    write_temp_file(&temp_path, label, contents)?;

    if store_path.exists() {
        fs::copy(store_path, &backup_path).map_err(|error| {
            StoreError::Write(format!("Failed to backup {label} store: {error}"))
        })?;
    }

    if let Err(error) = replace_file(&temp_path, store_path) {
        if backup_path.exists() {
            let _ = fs::copy(&backup_path, store_path);
        }
        let _ = fs::remove_file(&temp_path);
        return Err(StoreError::Write(format!(
            "Failed to write {label} store atomically: {error}"
        )));
    }

    Ok(())
}

fn write_temp_file(temp_path: &Path, label: &str, contents: &[u8]) -> Result<(), StoreError> {
    let mut file = File::create(temp_path).map_err(|error| {
        StoreError::Write(format!("Failed to create {label} store temp file: {error}"))
    })?;
    file.write_all(contents)
        .map_err(|error| StoreError::Write(format!("Failed to write {label} store: {error}")))?;
    file.sync_all()
        .map_err(|error| StoreError::Write(format!("Failed to flush {label} store: {error}")))?;
    Ok(())
}

fn replace_file(temp_path: &Path, store_path: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        if store_path.exists() {
            fs::remove_file(store_path)?;
        }
    }
    fs::rename(temp_path, store_path)
}

fn temp_path(store_path: &Path) -> PathBuf {
    with_suffix(store_path, ".tmp")
}

pub(crate) fn backup_path(store_path: &Path) -> PathBuf {
    with_suffix(store_path, ".bak")
}

fn with_suffix(store_path: &Path, suffix: &str) -> PathBuf {
    let mut name = store_path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    name.push(suffix);
    store_path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn store(dir: &tempfile::TempDir) -> PathBuf {
        dir.path().join("projects.json")
    }

    #[test]
    fn missing_file_loads_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        let loaded: Vec<serde_json::Value> = load_json_vec(&store(&dir), "projects").unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn saves_json_with_backup_and_loads_current_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = store(&dir);
        save_json_vec(&path, "projects", &[1, 2]).unwrap();
        save_json_vec(&path, "projects", &[3]).unwrap();
        assert_eq!(load_json_vec::<i32>(&path, "projects").unwrap(), vec![3]);
        assert_eq!(
            serde_json::from_str::<Vec<i32>>(&fs::read_to_string(backup_path(&path)).unwrap())
                .unwrap(),
            vec![1, 2]
        );
        assert!(!temp_path(&path).exists());
    }

    #[test]
    fn corrupt_primary_is_reported_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let path = store(&dir);
        save_json_vec(&path, "projects", &[1]).unwrap();
        fs::write(&path, b"{not json").unwrap();
        let before = fs::metadata(&path).unwrap().modified().unwrap();
        std::thread::sleep(Duration::from_millis(20));

        let error = load_json_vec::<i32>(&path, "projects").unwrap_err();
        assert!(
            matches!(error, StoreError::PrimaryCorrupt { .. }),
            "{error}"
        );
        assert!(error
            .to_string()
            .starts_with("Failed to parse projects store"));
        assert_eq!(fs::read(&path).unwrap(), b"{not json");
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), before);
    }

    #[test]
    fn recover_replaces_corrupt_primary_with_backup_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let path = store(&dir);
        save_json_vec(&path, "projects", &[1]).unwrap();
        save_json_vec(&path, "projects", &[1, 2]).unwrap(); // .bak = [1]
        fs::write(&path, b"{not json").unwrap();

        assert_eq!(
            recover_from_backup::<Vec<i32>>(&path, "projects").unwrap(),
            RecoveryOutcome::Recovered
        );
        assert_eq!(load_json_vec::<i32>(&path, "projects").unwrap(), vec![1]);
        assert!(!temp_path(&path).exists());
    }

    #[test]
    fn recover_is_noop_when_primary_is_healthy() {
        let dir = tempfile::tempdir().unwrap();
        let path = store(&dir);
        save_json_vec(&path, "projects", &[7]).unwrap();
        assert_eq!(
            recover_from_backup::<Vec<i32>>(&path, "projects").unwrap(),
            RecoveryOutcome::PrimaryHealthy
        );
        assert_eq!(load_json_vec::<i32>(&path, "projects").unwrap(), vec![7]);
    }

    #[test]
    fn recover_fails_when_backup_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = store(&dir);
        fs::write(&path, b"{not json").unwrap();
        let error = recover_from_backup::<Vec<i32>>(&path, "projects").unwrap_err();
        assert!(
            matches!(error, StoreError::RecoveryUnavailable { .. }),
            "{error}"
        );
        assert!(error.to_string().contains("backup recovery failed"));
    }
}

/// Codex 리뷰(2026-09-26) 반영: 문법은 맞지만 구조가 틀린 문서도 typed 검증으로 복구 대상이어야 한다.
#[cfg(test)]
mod recovery_shape_tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Doc {
        id: String,
        name: String,
    }

    fn write(path: &Path, contents: &str) {
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn structurally_invalid_primary_is_recovered_from_typed_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("docs.json");
        write(&backup_path(&path), r#"[{"id":"a","name":"A"}]"#);
        write(&path, r#"[{"id":"x"}]"#); // JSON 문법은 맞지만 name이 없다

        assert!(matches!(
            load_json_vec::<Doc>(&path, "docs").unwrap_err(),
            StoreError::PrimaryCorrupt { .. }
        ));
        assert_eq!(
            recover_from_backup::<Vec<Doc>>(&path, "docs").unwrap(),
            RecoveryOutcome::Recovered
        );
        assert_eq!(
            load_json_vec::<Doc>(&path, "docs").unwrap(),
            vec![Doc {
                id: "a".into(),
                name: "A".into()
            }]
        );
    }

    #[test]
    fn structurally_invalid_backup_is_not_used() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("docs.json");
        write(&backup_path(&path), r#"[{"id":"b"}]"#); // backup도 구조가 틀림
        write(&path, "{ not json");

        let error = recover_from_backup::<Vec<Doc>>(&path, "docs").unwrap_err();
        assert!(
            matches!(error, StoreError::RecoveryUnavailable { .. }),
            "{error}"
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "{ not json",
            "primary는 건드리지 않는다"
        );
    }

    #[test]
    fn value_typed_check_would_have_missed_the_shape_error() {
        // 회귀 방지 문서화: Value로 파싱하면 정상으로 보인다.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("docs.json");
        write(&path, r#"[{"id":"x"}]"#);
        assert!(read_json::<serde_json::Value>(&path, "docs").is_ok());
        assert!(read_json::<Vec<Doc>>(&path, "docs").is_err());
    }
}
