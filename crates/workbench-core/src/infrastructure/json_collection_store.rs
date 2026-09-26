//! `Vec<T>` 하나를 담는 JSON 저장 파일의 generic 어댑터(research R3). `json_store`의 load/save/recover 분리를
//! 저장 단위 4개(projects·saved-prompts·goals·agent-run-settings)가 복사 없이 공유한다.
//!
//! - `load`는 읽기 전용이다. 손상이면 `StoreError::PrimaryCorrupt`를 돌려주고 파일을 쓰지 않는다.
//! - `recover_from_backup`은 aggregate lock을 잡은 호출자(`StorageCoordinator::with_aggregate`)만 부른다.

use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
};

use serde::{de::DeserializeOwned, Serialize};

use crate::infrastructure::json_store::{self, RecoveryOutcome, StoreError};

pub struct JsonCollectionStore<T> {
    path: PathBuf,
    label: &'static str,
    // `fn() -> T`로 두어 T가 Send/Sync가 아니어도 store는 Send + Sync다.
    _marker: PhantomData<fn() -> T>,
}

impl<T> JsonCollectionStore<T>
where
    T: Serialize + DeserializeOwned,
{
    pub fn new(path: PathBuf, label: &'static str) -> Self {
        Self {
            path,
            label,
            _marker: PhantomData,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn label(&self) -> &'static str {
        self.label
    }

    /// 읽기 전용. 파일이 없으면 빈 벡터.
    pub fn load(&self) -> Result<Vec<T>, StoreError> {
        json_store::load_json_vec(&self.path, self.label)
    }

    /// temp + rename. 이전 파일은 `.bak`.
    pub fn save(&self, items: &[T]) -> Result<(), StoreError> {
        json_store::save_json_vec(&self.path, self.label, items)
    }

    /// 손상된 primary를 `.bak`으로 교체한다(`Vec<T>`로 검증). **lock 안에서만** 부른다.
    pub fn recover_from_backup(&self) -> Result<RecoveryOutcome, StoreError> {
        json_store::recover_from_backup::<Vec<T>>(&self.path, self.label)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, time::Duration};

    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Item {
        id: String,
        n: u32,
    }

    fn item(id: &str, n: u32) -> Item {
        Item { id: id.into(), n }
    }

    fn store(dir: &tempfile::TempDir) -> JsonCollectionStore<Item> {
        JsonCollectionStore::new(dir.path().join("items.json"), "items")
    }

    #[test]
    fn round_trips_vec_and_missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(&dir);
        assert!(store.load().unwrap().is_empty());
        store.save(&[item("a", 1), item("b", 2)]).unwrap();
        assert_eq!(store.load().unwrap(), vec![item("a", 1), item("b", 2)]);
        assert_eq!(store.label(), "items");
    }

    #[test]
    fn load_reports_primary_corrupt_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(&dir);
        store.save(&[item("a", 1)]).unwrap();
        fs::write(store.path(), b"{ nope").unwrap();
        let before = fs::metadata(store.path()).unwrap().modified().unwrap();
        std::thread::sleep(Duration::from_millis(20));

        let error = store.load().unwrap_err();
        assert!(
            matches!(error, StoreError::PrimaryCorrupt { .. }),
            "{error}"
        );
        assert_eq!(fs::read(store.path()).unwrap(), b"{ nope");
        assert_eq!(
            fs::metadata(store.path()).unwrap().modified().unwrap(),
            before
        );
    }

    #[test]
    fn save_writes_backup_of_previous_and_recover_validates_as_typed_vec() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(&dir);
        store.save(&[item("a", 1)]).unwrap();
        store.save(&[item("a", 1), item("b", 2)]).unwrap(); // .bak = [a]
        fs::write(store.path(), br#"[{"id":"x"}]"#).unwrap(); // 문법은 맞지만 n이 없다

        assert!(matches!(
            store.load().unwrap_err(),
            StoreError::PrimaryCorrupt { .. }
        ));
        assert_eq!(
            store.recover_from_backup().unwrap(),
            RecoveryOutcome::Recovered
        );
        assert_eq!(store.load().unwrap(), vec![item("a", 1)]);
    }

    #[test]
    fn recover_rejects_structurally_invalid_backup() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(&dir);
        fs::write(
            store.path().with_file_name("items.json.bak"),
            br#"[{"id":"b"}]"#,
        )
        .unwrap();
        fs::write(store.path(), b"{ nope").unwrap();
        let error = store.recover_from_backup().unwrap_err();
        assert!(
            matches!(error, StoreError::RecoveryUnavailable { .. }),
            "{error}"
        );
        assert_eq!(fs::read(store.path()).unwrap(), b"{ nope");
    }
}
