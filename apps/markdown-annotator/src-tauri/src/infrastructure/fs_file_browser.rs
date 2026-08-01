use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use crate::{
    application::launch_target_service::root_identity_for_canonical,
    domain::{
        document_identity::DocumentIdentity,
        file_browser::{
            FileBrowserError, FileBrowserErrorCode, FileBrowserWarning, RootIdentity,
            RootScanResult, ScannedEntry, ScannedEntryKind, validate_relative_path,
        },
    },
    infrastructure::sha256_fingerprint::sha256_hex,
    ports::file_browser::{DocumentReadResult, FileBrowser},
};

const MAX_DOCUMENT_BYTES: u64 = 5 * 1024 * 1024;

pub struct FsFileBrowser;

impl FileBrowser for FsFileBrowser {
    fn canonical_root(&self, path: &Path) -> Result<RootIdentity, FileBrowserError> {
        let canonical = path.canonicalize().map_err(|cause| io_error(cause, path))?;
        if !canonical.is_dir() {
            return Err(error(FileBrowserErrorCode::NotRegularFile, path));
        }
        Ok(root_identity_for_canonical(canonical))
    }

    fn scan_root(
        &self,
        root: &RootIdentity,
        excluded_directory_names: &[String],
    ) -> Result<RootScanResult, FileBrowserError> {
        let exclusions = excluded_directory_names
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let mut files = BTreeMap::<String, ScannedEntry>::new();
        let mut warnings = Vec::new();
        let mut visited_entries = 0;
        let mut canonical_files = HashSet::new();
        scan_directory(
            &root.canonical_path,
            &root.canonical_path,
            &exclusions,
            &mut files,
            &mut warnings,
            &mut visited_entries,
            &mut canonical_files,
        )?;
        let file_entries = files.values().cloned().collect::<Vec<_>>();
        for file in &file_entries {
            let mut parent = Path::new(&file.relative_path).parent();
            while let Some(path) = parent.filter(|path| !path.as_os_str().is_empty()) {
                let relative_path = display_relative(path);
                files.entry(relative_path.clone()).or_insert(ScannedEntry {
                    relative_path,
                    kind: ScannedEntryKind::Directory,
                    size: 0,
                    modified_at_ms: None,
                });
                parent = path.parent();
            }
        }
        Ok(RootScanResult {
            entries: files.into_values().collect(),
            warnings,
            visited_entries,
        })
    }

    fn read_document(
        &self,
        root: &RootIdentity,
        relative_path: &str,
    ) -> Result<DocumentReadResult, FileBrowserError> {
        validate_relative_path(relative_path)?;
        let requested = root.canonical_path.join(relative_path);
        if !is_markdown(&requested) {
            return Err(error(
                FileBrowserErrorCode::UnsupportedExtension,
                &requested,
            ));
        }
        let link_metadata =
            fs::symlink_metadata(&requested).map_err(|cause| io_error(cause, &requested))?;
        if link_metadata.file_type().is_dir() {
            return Err(error(FileBrowserErrorCode::NotRegularFile, &requested));
        }
        let canonical = requested
            .canonicalize()
            .map_err(|cause| io_error(cause, &requested))?;
        if !canonical.starts_with(&root.canonical_path) {
            return Err(error(FileBrowserErrorCode::OutsideRootSymlink, &requested));
        }
        let metadata = fs::metadata(&canonical).map_err(|cause| io_error(cause, &canonical))?;
        if !metadata.is_file() {
            return Err(error(FileBrowserErrorCode::NotRegularFile, &canonical));
        }
        if metadata.len() > MAX_DOCUMENT_BYTES {
            return Err(error(FileBrowserErrorCode::TooLarge, &canonical));
        }
        let bytes = fs::read(&canonical).map_err(|cause| io_error(cause, &canonical))?;
        let content = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
        let markdown_text = String::from_utf8(content.to_vec())
            .map_err(|_| error(FileBrowserErrorCode::InvalidUtf8, &canonical))?;
        let modified_at_ms = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|duration| u64::try_from(duration.as_millis()).ok());
        Ok(DocumentReadResult {
            identity: DocumentIdentity {
                root_id: root.root_id.clone(),
                relative_path: relative_path.to_string(),
                fingerprint: sha256_hex(&bytes),
                byte_length: metadata.len(),
                modified_at_ms,
            },
            markdown_text,
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn scan_directory(
    root: &Path,
    directory: &Path,
    exclusions: &HashSet<&str>,
    files: &mut BTreeMap<String, ScannedEntry>,
    warnings: &mut Vec<FileBrowserWarning>,
    visited_entries: &mut u64,
    canonical_files: &mut HashSet<PathBuf>,
) -> Result<(), FileBrowserError> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(cause) if directory != root => {
            warnings.push(FileBrowserWarning {
                relative_path: display_relative(directory.strip_prefix(root).unwrap_or(directory)),
                code: io_error(cause, directory).code,
            });
            return Ok(());
        }
        Err(cause) => return Err(io_error(cause, directory)),
    };
    for entry in entries {
        *visited_entries += 1;
        let entry = match entry {
            Ok(entry) => entry,
            Err(cause) => {
                warnings.push(FileBrowserWarning {
                    relative_path: display_relative(
                        directory.strip_prefix(root).unwrap_or(directory),
                    ),
                    code: io_error(cause, directory).code,
                });
                continue;
            }
        };
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let file_type = match entry.file_type() {
            Ok(kind) => kind,
            Err(cause) => {
                warnings.push(FileBrowserWarning {
                    relative_path: display_relative(path.strip_prefix(root).unwrap_or(&path)),
                    code: io_error(cause, &path).code,
                });
                continue;
            }
        };
        if file_type.is_dir() {
            if !exclusions.contains(name.as_ref()) {
                scan_directory(
                    root,
                    &path,
                    exclusions,
                    files,
                    warnings,
                    visited_entries,
                    canonical_files,
                )?;
            }
            continue;
        }
        let canonical = match path.canonicalize() {
            Ok(path) => path,
            Err(cause) => {
                warnings.push(FileBrowserWarning {
                    relative_path: display_relative(path.strip_prefix(root).unwrap_or(&path)),
                    code: io_error(cause, &path).code,
                });
                continue;
            }
        };
        if !canonical.starts_with(root)
            || !canonical.is_file()
            || !is_markdown(&path)
            || !canonical_files.insert(canonical.clone())
        {
            continue;
        }
        let metadata = match fs::metadata(&canonical) {
            Ok(metadata) => metadata,
            Err(cause) => {
                warnings.push(FileBrowserWarning {
                    relative_path: display_relative(path.strip_prefix(root).unwrap_or(&path)),
                    code: io_error(cause, &path).code,
                });
                continue;
            }
        };
        let relative_path = display_relative(path.strip_prefix(root).unwrap_or(&path));
        let modified_at_ms = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|duration| u64::try_from(duration.as_millis()).ok());
        files.insert(
            relative_path.clone(),
            ScannedEntry {
                relative_path,
                kind: ScannedEntryKind::File,
                size: metadata.len(),
                modified_at_ms,
            },
        );
    }
    Ok(())
}

fn display_relative(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
        })
}

fn error(code: FileBrowserErrorCode, path: &Path) -> FileBrowserError {
    FileBrowserError {
        code,
        path: Some(path.to_string_lossy().into_owned()),
    }
}

fn io_error(cause: std::io::Error, path: &Path) -> FileBrowserError {
    let code = match cause.kind() {
        std::io::ErrorKind::NotFound => FileBrowserErrorCode::NotFound,
        std::io::ErrorKind::PermissionDenied => FileBrowserErrorCode::PermissionDenied,
        _ => FileBrowserErrorCode::IoError,
    };
    error(code, path)
}

#[cfg(test)]
#[path = "fs_file_browser_test.rs"]
mod tests;
