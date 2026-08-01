use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RootIdentity {
    pub root_id: String,
    pub canonical_path: PathBuf,
    pub display_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchTarget {
    pub root: RootIdentity,
    pub selected_document: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanStatus {
    Scanning,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSession {
    pub scan_id: String,
    pub root_id: String,
    pub exclusion_revision: u64,
    pub sequence: u64,
    pub status: ScanStatus,
    pub visited_entries: u64,
    pub matched_documents: u64,
    pub warnings: Vec<FileBrowserWarning>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileBrowserWarning {
    pub relative_path: String,
    pub code: FileBrowserErrorCode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ScannedEntryKind {
    Directory,
    File,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedEntry {
    pub relative_path: String,
    pub kind: ScannedEntryKind,
    pub size: u64,
    pub modified_at_ms: Option<u64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RootScanResult {
    pub entries: Vec<ScannedEntry>,
    pub warnings: Vec<FileBrowserWarning>,
    pub visited_entries: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileBrowserErrorCode {
    OutsideRoot,
    InvalidRelativePath,
    UnsupportedExtension,
    NotFound,
    NotRegularFile,
    DirectorySymlink,
    OutsideRootSymlink,
    InvalidUtf8,
    TooLarge,
    PermissionDenied,
    IoError,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileBrowserError {
    pub code: FileBrowserErrorCode,
    pub path: Option<String>,
}

impl std::fmt::Display for FileBrowserError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}", self.code)
    }
}

impl std::error::Error for FileBrowserError {}

pub fn validate_relative_path(path: &str) -> Result<(), FileBrowserError> {
    let candidate = std::path::Path::new(path);
    if path.is_empty()
        || path.contains('\0')
        || candidate.is_absolute()
        || candidate.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::CurDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(FileBrowserError {
            code: FileBrowserErrorCode::InvalidRelativePath,
            path: Some(path.to_string()),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_relative_paths() {
        for path in [
            "",
            "../secret.md",
            "a/../../secret.md",
            "/tmp/a.md",
            "a\0b.md",
        ] {
            assert_eq!(
                validate_relative_path(path).unwrap_err().code,
                FileBrowserErrorCode::InvalidRelativePath
            );
        }
    }
}
