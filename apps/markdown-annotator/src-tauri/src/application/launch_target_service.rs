use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::domain::file_browser::{
    FileBrowserError, FileBrowserErrorCode, LaunchTarget, RootIdentity,
};

pub struct LaunchTargetService;

impl LaunchTargetService {
    pub fn resolve(input: Option<&Path>, cwd: &Path) -> Result<LaunchTarget, FileBrowserError> {
        let candidate = input.unwrap_or(cwd);
        let canonical = candidate.canonicalize().map_err(|error| FileBrowserError {
            code: if error.kind() == std::io::ErrorKind::NotFound {
                FileBrowserErrorCode::NotFound
            } else if error.kind() == std::io::ErrorKind::PermissionDenied {
                FileBrowserErrorCode::PermissionDenied
            } else {
                FileBrowserErrorCode::IoError
            },
            path: Some(candidate.to_string_lossy().into_owned()),
        })?;

        let (root_path, selected_document) = if canonical.is_dir() {
            (canonical, None)
        } else if canonical.is_file() && is_markdown(&canonical) {
            let file_name = canonical
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| invalid_target(&canonical))?
                .to_string();
            let parent = canonical
                .parent()
                .ok_or_else(|| invalid_target(&canonical))?;
            (parent.to_path_buf(), Some(file_name))
        } else {
            return Err(FileBrowserError {
                code: FileBrowserErrorCode::UnsupportedExtension,
                path: Some(canonical.to_string_lossy().into_owned()),
            });
        };

        Ok(LaunchTarget {
            root: root_identity_for_canonical(root_path),
            selected_document,
        })
    }
}

pub(crate) fn root_identity_for_canonical(canonical_path: PathBuf) -> RootIdentity {
    let display_path = canonical_path.to_string_lossy().into_owned();
    let mut hasher = Sha256::new();
    hasher.update(b"ma-root-v1\0");
    hasher.update(display_path.as_bytes());
    RootIdentity {
        root_id: format!("root-{:x}", hasher.finalize()),
        canonical_path,
        display_path,
    }
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
        })
}

fn invalid_target(path: &Path) -> FileBrowserError {
    FileBrowserError {
        code: FileBrowserErrorCode::InvalidRelativePath,
        path: Some(path.to_string_lossy().into_owned()),
    }
}

#[cfg(test)]
#[path = "launch_target_service_test.rs"]
mod tests;
