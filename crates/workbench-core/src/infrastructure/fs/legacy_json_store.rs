//! orchestration 저장 전용 JSON 입출력(041: AW `infrastructure/json_store.rs`에서 tauri 의존만 빼고 복사). `load_json`은
//! 읽기 경로에서 `.bak`을 복구한다 — orchestration 저장소는 이 복구를 aggregate lock 안에서만 부른다(research R1).
//! 오류 문자열은 오늘과 같다(`OrchestrationError::WorkerUnavailable` 메시지로 노출).
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use serde::{de::DeserializeOwned, Serialize};

pub fn load_json_vec<T>(store_path: &Path, label: &str) -> Result<Vec<T>, String>
where
    T: DeserializeOwned,
{
    load_json(store_path, label)
}

pub fn load_json<T>(store_path: &Path, label: &str) -> Result<T, String>
where
    T: DeserializeOwned + Default,
{
    match read_json(store_path, label) {
        Ok(value) => Ok(value),
        Err(primary_error) if store_path.exists() => {
            let backup_path = backup_path(store_path);
            if !backup_path.exists() {
                return Err(primary_error);
            }

            let backup_value = read_json(&backup_path, label).map_err(|backup_error| {
                format!("{primary_error}; backup recovery failed: {backup_error}")
            })?;
            fs::copy(&backup_path, store_path).map_err(|error| {
                format!(
                    "Recovered {label} store from backup, but failed to restore {}: {error}",
                    store_path.display()
                )
            })?;
            Ok(backup_value)
        }
        Err(error) => Err(error),
    }
}

pub fn save_json_vec<T>(store_path: &Path, label: &str, values: &[T]) -> Result<(), String>
where
    T: Serialize,
{
    save_json(store_path, label, values)
}

pub fn save_json<T>(store_path: &Path, label: &str, value: &T) -> Result<(), String>
where
    T: Serialize + ?Sized,
{
    let contents = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("Failed to serialize {label}: {error}"))?;
    atomic_write(store_path, label, &contents)
}

fn read_json<T>(store_path: &Path, label: &str) -> Result<T, String>
where
    T: DeserializeOwned + Default,
{
    if !store_path.exists() {
        return Ok(T::default());
    }

    let contents = fs::read_to_string(store_path).map_err(|error| {
        format!(
            "Failed to read {label} store {}: {error}",
            store_path.display()
        )
    })?;

    serde_json::from_str(&contents).map_err(|error| {
        format!(
            "Failed to parse {label} store {}: {error}",
            store_path.display()
        )
    })
}

fn atomic_write(store_path: &Path, label: &str, contents: &[u8]) -> Result<(), String> {
    let parent = store_path.parent().ok_or_else(|| {
        format!(
            "Failed to resolve parent directory for {}",
            store_path.display()
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Failed to create {label} store directory: {error}"))?;

    let temp_path = temp_path(store_path);
    let backup_path = backup_path(store_path);

    write_temp_file(&temp_path, label, contents)?;

    if store_path.exists() {
        fs::copy(store_path, &backup_path)
            .map_err(|error| format!("Failed to backup {label} store: {error}"))?;
    }

    if let Err(error) = replace_file(&temp_path, store_path) {
        if backup_path.exists() {
            let _ = fs::copy(&backup_path, store_path);
        }
        let _ = fs::remove_file(&temp_path);
        return Err(format!("Failed to write {label} store atomically: {error}"));
    }

    sync_directory(parent);
    Ok(())
}

fn write_temp_file(temp_path: &Path, label: &str, contents: &[u8]) -> Result<(), String> {
    let mut file = File::create(temp_path)
        .map_err(|error| format!("Failed to create temporary {label} store: {error}"))?;
    file.write_all(contents)
        .map_err(|error| format!("Failed to write temporary {label} store: {error}"))?;
    file.write_all(b"\n")
        .map_err(|error| format!("Failed to finish temporary {label} store: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("Failed to sync temporary {label} store: {error}"))
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

fn backup_path(store_path: &Path) -> PathBuf {
    store_path.with_extension(format!(
        "{}bak",
        store_path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| format!("{extension}."))
            .unwrap_or_default()
    ))
}

fn temp_path(store_path: &Path) -> PathBuf {
    store_path.with_extension(format!(
        "{}tmp",
        store_path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| format!("{extension}."))
            .unwrap_or_default()
    ))
}

fn sync_directory(path: &Path) {
    if let Ok(directory) = File::open(path) {
        let _ = directory.sync_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 042 research R13: orchestration 작업 영역 쓰기가 중간에 끊기면(임시 파일만 남음) 이전 판이 그대로 읽히고,
    /// 다음 저장은 정상이다. 본 파일이 깨졌다면 `.bak`에서 복구한다.
    #[test]
    fn interrupted_writes_leave_the_previous_version_readable() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("orchestration-sessions.json");
        save_json(&store, "orchestration", &vec!["v1".to_owned()]).unwrap();

        // rename 전에 끊긴 쓰기: 임시 파일만 반쯤 쓰여 있다.
        fs::write(temp_path(&store), b"[\"v2\", \"trunc").unwrap();
        let read: Vec<String> = load_json(&store, "orchestration").unwrap();
        assert_eq!(read, vec!["v1".to_owned()]);
        save_json(&store, "orchestration", &vec!["v2".to_owned()]).unwrap();
        let read: Vec<String> = load_json(&store, "orchestration").unwrap();
        assert_eq!(read, vec!["v2".to_owned()]);

        // 본 파일 자체가 깨진 경우(비원자 매체): 직전 판 `.bak`에서 복구한다.
        fs::write(&store, b"{broken").unwrap();
        let read: Vec<String> = load_json(&store, "orchestration").unwrap();
        assert_eq!(read, vec!["v1".to_owned()], "recovered from the backup");
    }
}
