use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Serialize, de::DeserializeOwned};

pub fn load_json_vec<T>(store_path: &Path, label: &str) -> Result<Vec<T>, String>
where
    T: DeserializeOwned,
{
    match read_json_vec(store_path, label) {
        Ok(value) => Ok(value),
        Err(primary_error) if store_path.exists() => {
            let backup_path = backup_path(store_path);
            if !backup_path.exists() {
                return Err(primary_error);
            }

            let backup_value = read_json_vec(&backup_path, label).map_err(|backup_error| {
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
    let contents = serde_json::to_vec_pretty(values)
        .map_err(|error| format!("Failed to serialize {label}: {error}"))?;
    atomic_write(store_path, label, &contents)
}

fn read_json_vec<T>(store_path: &Path, label: &str) -> Result<Vec<T>, String>
where
    T: DeserializeOwned,
{
    if !store_path.exists() {
        return Ok(Vec::new());
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
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    struct TestRecord {
        id: String,
    }

    #[test]
    fn saves_json_with_backup_and_loads_current_file() {
        let store_path = test_store_path("current");

        save_json_vec(&store_path, "test records", &[record("first")]).expect("save first");
        save_json_vec(&store_path, "test records", &[record("second")]).expect("save second");

        let current: Vec<TestRecord> =
            load_json_vec(&store_path, "test records").expect("load current");
        let backup: Vec<TestRecord> =
            load_json_vec(&backup_path(&store_path), "test records").expect("load backup");

        assert_eq!(current, vec![record("second")]);
        assert_eq!(backup, vec![record("first")]);
    }

    #[test]
    fn recovers_from_backup_when_current_json_is_corrupt() {
        let store_path = test_store_path("recovery");

        save_json_vec(&store_path, "test records", &[record("backup")]).expect("save backup");
        save_json_vec(&store_path, "test records", &[record("current")]).expect("save current");
        fs::write(&store_path, "{ broken").expect("corrupt current");

        let recovered: Vec<TestRecord> =
            load_json_vec(&store_path, "test records").expect("recover from backup");
        let restored = fs::read_to_string(&store_path).expect("read restored current");

        assert_eq!(recovered, vec![record("backup")]);
        assert!(restored.contains("backup"));
    }

    fn record(id: &str) -> TestRecord {
        TestRecord { id: id.into() }
    }

    fn test_store_path(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("acp-json-store-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create test dir");
        dir.join("store.json")
    }
}
