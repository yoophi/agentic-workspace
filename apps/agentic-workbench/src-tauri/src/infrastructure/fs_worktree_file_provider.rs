use std::{
    fs,
    path::{Component, Path, PathBuf},
    time::UNIX_EPOCH,
};

use walkdir::{DirEntry, WalkDir};

use crate::domain::{
    worktree_file::{WorktreeFileEntry, WorktreeTextFile},
    worktree_file_provider::WorktreeFileProvider,
};

const MAX_PREVIEW_BYTES: u64 = 512 * 1024;
const EXCLUDED_DIRS: &[&str] = &[
    ".git",
    ".next",
    ".turbo",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
];

pub struct FsWorktreeFileProvider;

impl WorktreeFileProvider for FsWorktreeFileProvider {
    fn list_files(&self, working_directory: &str) -> Result<Vec<WorktreeFileEntry>, String> {
        let root = canonical_root(working_directory)?;
        let mut entries = Vec::new();

        for entry in WalkDir::new(&root)
            .min_depth(1)
            .into_iter()
            .filter_entry(|entry| should_descend(entry))
        {
            let entry = entry.map_err(|error| format!("Failed to read worktree entry: {error}"))?;
            let path = entry.path();
            let metadata = entry.metadata().map_err(|error| {
                format!("Failed to read metadata for {}: {error}", path.display())
            })?;
            let relative_path = relative_path(&root, path)?;

            entries.push(WorktreeFileEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: path.to_string_lossy().into_owned(),
                relative_path,
                is_dir: metadata.is_dir(),
                size: if metadata.is_dir() { 0 } else { metadata.len() },
                modified_ms: metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|duration| duration.as_millis() as u64),
            });
        }

        entries.sort_by(|left, right| {
            left.relative_path
                .to_lowercase()
                .cmp(&right.relative_path.to_lowercase())
                .then_with(|| left.relative_path.cmp(&right.relative_path))
        });

        Ok(entries)
    }

    fn read_text_file(
        &self,
        working_directory: &str,
        requested_relative_path: &str,
    ) -> Result<WorktreeTextFile, String> {
        let root = canonical_root(working_directory)?;
        let file_path = resolve_worktree_path(&root, requested_relative_path)?;
        let metadata = fs::metadata(&file_path).map_err(|error| {
            format!(
                "Failed to read metadata for {}: {error}",
                file_path.display()
            )
        })?;

        if !metadata.is_file() {
            return Err("Only regular files can be previewed.".into());
        }

        let bytes = fs::read(&file_path)
            .map_err(|error| format!("Failed to read {}: {error}", file_path.display()))?;
        let truncated = metadata.len() > MAX_PREVIEW_BYTES;
        let preview_bytes = if truncated {
            &bytes[..MAX_PREVIEW_BYTES as usize]
        } else {
            &bytes
        };
        let content = std::str::from_utf8(preview_bytes)
            .map_err(|_| "Only UTF-8 text files can be previewed.".to_string())?
            .to_owned();

        Ok(WorktreeTextFile {
            path: file_path.to_string_lossy().into_owned(),
            relative_path: relative_path(&root, &file_path)?,
            content,
            size: metadata.len(),
            truncated,
        })
    }
}

fn canonical_root(path: &str) -> Result<PathBuf, String> {
    let root = fs::canonicalize(path)
        .map_err(|error| format!("Failed to resolve worktree path {path}: {error}"))?;

    if !root.is_dir() {
        return Err("Working directory must be a directory.".into());
    }

    Ok(root)
}

fn resolve_worktree_path(root: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let requested = Path::new(relative_path);

    if requested.is_absolute()
        || requested
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err("File path must stay inside the worktree.".into());
    }

    let resolved = fs::canonicalize(root.join(requested))
        .map_err(|error| format!("Failed to resolve file path {relative_path}: {error}"))?;

    if !resolved.starts_with(root) {
        return Err("File path must stay inside the worktree.".into());
    }

    Ok(resolved)
}

fn relative_path(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map_err(|_| "File path must stay inside the worktree.".to_string())
        .map(|path| path.to_string_lossy().replace('\\', "/"))
}

fn should_descend(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return true;
    }

    let name = entry.file_name().to_string_lossy();
    if !entry.file_type().is_dir() {
        return !name.starts_with('.');
    }

    !name.starts_with('.') && !EXCLUDED_DIRS.contains(&name.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_dir_preview_paths() {
        let root = Path::new("/tmp/worktree");
        let error = resolve_worktree_path(root, "../secret.txt").expect_err("path should fail");

        assert_eq!(error, "File path must stay inside the worktree.");
    }
}
