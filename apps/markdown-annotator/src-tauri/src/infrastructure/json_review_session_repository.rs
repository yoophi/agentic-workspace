use crate::{
    domain::review_session::ReviewSession,
    ports::review_session_repository::{ReviewRepositoryError, ReviewSessionRepository},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const SCHEMA_VERSION: u32 = 1;
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Envelope {
    schema_version: u32,
    revision: u64,
    payload: ReviewSession,
}

pub struct JsonReviewSessionRepository {
    base: PathBuf,
}
impl JsonReviewSessionRepository {
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self { base: base.into() }
    }
    fn sessions(&self) -> PathBuf {
        self.base.join("reviews/sessions")
    }
    fn snapshots(&self, id: &str) -> PathBuf {
        self.base.join("reviews/snapshots").join(safe_id(id))
    }
    fn path(&self, id: &str) -> PathBuf {
        self.sessions().join(format!("{}.json", safe_id(id)))
    }
    fn read_envelope(path: &Path) -> Result<Envelope, ReviewRepositoryError> {
        let bytes = fs::read(path).map_err(map_io)?;
        let mut value: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|error| ReviewRepositoryError::Corrupt(error.to_string()))?;
        let version = value
            .get("schemaVersion")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| ReviewRepositoryError::Corrupt("schemaVersion is missing".into()))?
            as u32;
        if version > SCHEMA_VERSION {
            return Err(ReviewRepositoryError::UnsupportedSchema(version));
        }
        if version == 0 {
            value["schemaVersion"] = serde_json::json!(1);
            if value.get("revision").is_none() {
                value["revision"] = serde_json::json!(0);
            }
            let revision = value["revision"].clone();
            if let Some(payload) = value.get_mut("payload") {
                payload["schemaVersion"] = serde_json::json!(1);
                if payload.get("revision").is_none() {
                    payload["revision"] = revision;
                }
            }
        }
        serde_json::from_value(value)
            .map_err(|error| ReviewRepositoryError::Corrupt(error.to_string()))
    }
    fn recover_snapshot(&self, id: &str) -> Result<ReviewSession, ReviewRepositoryError> {
        let directory = self.snapshots(id);
        let mut paths = fs::read_dir(directory)
            .map_err(map_io)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        paths.sort();
        paths.reverse();
        for path in paths {
            if let Ok(envelope) = Self::read_envelope(&path) {
                return Ok(envelope.payload);
            }
        }
        Err(ReviewRepositoryError::Corrupt(
            "valid snapshot not found".into(),
        ))
    }
}
impl ReviewSessionRepository for JsonReviewSessionRepository {
    fn load(&self, id: &str) -> Result<ReviewSession, ReviewRepositoryError> {
        match Self::read_envelope(&self.path(id)) {
            Ok(value) => Ok(value.payload),
            Err(ReviewRepositoryError::Corrupt(_)) => self.recover_snapshot(id),
            Err(error) => Err(error),
        }
    }
    fn save(
        &self,
        session: &ReviewSession,
        expected_revision: u64,
    ) -> Result<ReviewSession, ReviewRepositoryError> {
        fs::create_dir_all(self.sessions()).map_err(map_io)?;
        let path = self.path(&session.session_id);
        let actual = if path.exists() {
            Self::read_envelope(&path)?.revision
        } else {
            0
        };
        if actual != expected_revision {
            return Err(ReviewRepositoryError::RevisionConflict {
                expected: expected_revision,
                actual,
            });
        }
        if path.exists() {
            let snapshots = self.snapshots(&session.session_id);
            fs::create_dir_all(&snapshots).map_err(map_io)?;
            fs::copy(&path, snapshots.join(format!("{actual:020}.json"))).map_err(map_io)?;
            retain_latest(&snapshots, 5)?;
        }
        let mut saved = session.clone();
        saved.revision = actual + 1;
        let envelope = Envelope {
            schema_version: SCHEMA_VERSION,
            revision: saved.revision,
            payload: saved.clone(),
        };
        let temp = self.sessions().join(format!(
            ".{}.{}-{}.tmp",
            safe_id(&session.session_id),
            std::process::id(),
            now_nanos()
        ));
        let mut file = fs::File::create(&temp).map_err(map_io)?;
        serde_json::to_writer_pretty(&mut file, &envelope)
            .map_err(|error| ReviewRepositoryError::Io(error.to_string()))?;
        file.write_all(b"\n").map_err(map_io)?;
        file.sync_all().map_err(map_io)?;
        fs::rename(&temp, &path).map_err(map_io)?;
        if let Ok(directory) = fs::File::open(self.sessions()) {
            let _ = directory.sync_all();
        }
        Ok(saved)
    }
    fn trash(&self, id: &str) -> Result<(), ReviewRepositoryError> {
        let path = self.path(id);
        if !path.exists() {
            return Err(ReviewRepositoryError::NotFound);
        }
        let trash = self.base.join("reviews/trash");
        fs::create_dir_all(&trash).map_err(map_io)?;
        fs::rename(
            path,
            trash.join(format!("{}-{}.json", now_nanos(), safe_id(id))),
        )
        .map_err(map_io)
    }
}
fn safe_id(id: &str) -> String {
    id.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}
fn now_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
fn map_io(error: std::io::Error) -> ReviewRepositoryError {
    if error.kind() == std::io::ErrorKind::NotFound {
        ReviewRepositoryError::NotFound
    } else {
        ReviewRepositoryError::Io(error.to_string())
    }
}
fn retain_latest(directory: &Path, count: usize) -> Result<(), ReviewRepositoryError> {
    let mut paths = fs::read_dir(directory)
        .map_err(map_io)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    paths.sort();
    let remove = paths.len().saturating_sub(count);
    for path in paths.into_iter().take(remove) {
        fs::remove_file(path).map_err(map_io)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "json_review_session_repository_test.rs"]
mod tests;
