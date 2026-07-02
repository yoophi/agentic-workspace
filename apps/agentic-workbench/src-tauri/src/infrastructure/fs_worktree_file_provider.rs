use std::{
    fs,
    path::{Component, Path, PathBuf},
    time::UNIX_EPOCH,
};

use walkdir::{DirEntry, WalkDir};

use crate::domain::{
    worktree_file::{WorktreeFileEntry, WorktreeFileListKind, WorktreeFileListScope, WorktreeTextFile},
    worktree_file_provider::WorktreeFileProvider,
};

use crate::infrastructure::WORKSPACE_EXCLUDED_DIRS;

const MAX_PREVIEW_BYTES: u64 = 512 * 1024;

pub struct FsWorktreeFileProvider;

impl WorktreeFileProvider for FsWorktreeFileProvider {
    fn list_files(
        &self,
        working_directory: &str,
        scope: &WorktreeFileListScope,
    ) -> Result<Vec<WorktreeFileEntry>, String> {
        let root = canonical_root(working_directory)?;
        // dir이 있으면 그 하위만 순회한다(폴더 펼침용 lazy loading, R10).
        // 상대 경로는 항상 worktree root 기준으로 유지해 트리 표시가 안정적이다.
        let base = match scope.dir.as_deref().map(str::trim).filter(|dir| !dir.is_empty()) {
            Some(dir) => resolve_worktree_path(&root, dir)?,
            None => root.clone(),
        };
        let mut walker = WalkDir::new(&base).min_depth(1);
        if let Some(depth) = scope.depth {
            walker = walker.max_depth(depth.max(1));
        }
        let mut entries = Vec::new();

        for entry in walker
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

        if scope.kind == WorktreeFileListKind::Markdown {
            entries = filter_markdown_entries(entries);
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

/// markdown 파일과 그 조상 디렉터리만 남긴다(응답 크기 축소, R10).
fn filter_markdown_entries(entries: Vec<WorktreeFileEntry>) -> Vec<WorktreeFileEntry> {
    let mut markdown_dir_paths = std::collections::HashSet::new();

    for entry in &entries {
        if entry.is_dir || !is_markdown_path(&entry.relative_path) {
            continue;
        }

        // 파일 경로의 모든 조상 디렉터리를 기록한다(경로 자신은 파일이므로 제외).
        let mut ancestor = String::new();
        for segment in entry.relative_path.split('/').filter(|segment| !segment.is_empty()) {
            if ancestor.is_empty() {
                ancestor = segment.to_string();
            } else {
                ancestor = format!("{ancestor}/{segment}");
            }
            if ancestor != entry.relative_path {
                markdown_dir_paths.insert(ancestor.clone());
            }
        }
    }

    entries
        .into_iter()
        .filter(|entry| {
            if entry.is_dir {
                markdown_dir_paths.contains(&entry.relative_path)
            } else {
                is_markdown_path(&entry.relative_path)
            }
        })
        .collect()
}

fn is_markdown_path(path: &str) -> bool {
    let lowered = path.to_lowercase();
    lowered.ends_with(".md") || lowered.ends_with(".markdown") || lowered.ends_with(".mdx")
}

fn should_descend(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return true;
    }

    let name = entry.file_name().to_string_lossy();
    if !entry.file_type().is_dir() {
        return !name.starts_with('.');
    }

    !name.starts_with('.') && !WORKSPACE_EXCLUDED_DIRS.contains(&name.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::worktree_file::{WorktreeFileListKind, WorktreeFileListScope};
    use crate::domain::worktree_file_provider::WorktreeFileProvider;

    fn init_fixture() -> tempfile::TempDir {
        let fixture = tempfile::tempdir().expect("fixture dir should be created");
        let path = fixture.path();
        fs::create_dir_all(path.join("docs/nested")).expect("fixture dirs");
        fs::create_dir_all(path.join("src/deep")).expect("fixture dirs");
        fs::write(path.join("README.md"), "# readme").expect("fixture file");
        fs::write(path.join("main.ts"), "export {}").expect("fixture file");
        fs::write(path.join("docs/guide.md"), "# guide").expect("fixture file");
        fs::write(path.join("docs/nested/deep.md"), "# deep").expect("fixture file");
        fs::write(path.join("src/app.ts"), "export {}").expect("fixture file");
        fs::write(path.join("src/deep/inner.ts"), "export {}").expect("fixture file");
        fixture
    }

    fn relative_paths(entries: &[crate::domain::worktree_file::WorktreeFileEntry]) -> Vec<String> {
        entries.iter().map(|entry| entry.relative_path.clone()).collect()
    }

    #[test]
    fn rejects_parent_dir_preview_paths() {
        let root = Path::new("/tmp/worktree");
        let error = resolve_worktree_path(root, "../secret.txt").expect_err("path should fail");

        assert_eq!(error, "File path must stay inside the worktree.");
    }

    #[test]
    fn markdown_scope_returns_markdown_files_and_their_ancestor_dirs() {
        let fixture = init_fixture();

        let entries = FsWorktreeFileProvider
            .list_files(
                fixture.path().to_str().expect("utf-8 path"),
                &WorktreeFileListScope {
                    kind: WorktreeFileListKind::Markdown,
                    ..Default::default()
                },
            )
            .expect("markdown scope should list");
        let paths = relative_paths(&entries);

        assert!(paths.contains(&"README.md".to_string()));
        assert!(paths.contains(&"docs".to_string()));
        assert!(paths.contains(&"docs/guide.md".to_string()));
        assert!(paths.contains(&"docs/nested/deep.md".to_string()));
        assert!(!paths.contains(&"main.ts".to_string()));
        assert!(!paths.contains(&"src".to_string()), "markdown 없는 디렉터리는 제외");
    }

    #[test]
    fn dir_scope_with_depth_lists_direct_children_only() {
        let fixture = init_fixture();

        let entries = FsWorktreeFileProvider
            .list_files(
                fixture.path().to_str().expect("utf-8 path"),
                &WorktreeFileListScope {
                    dir: Some("src".to_string()),
                    depth: Some(1),
                    ..Default::default()
                },
            )
            .expect("dir scope should list");
        let paths = relative_paths(&entries);

        assert!(paths.contains(&"src/app.ts".to_string()));
        assert!(paths.contains(&"src/deep".to_string()));
        assert!(!paths.contains(&"src/deep/inner.ts".to_string()), "depth 1은 직계만");
        assert!(!paths.contains(&"README.md".to_string()));
    }

    #[test]
    fn dir_scope_rejects_paths_escaping_the_worktree() {
        let fixture = init_fixture();

        let error = FsWorktreeFileProvider
            .list_files(
                fixture.path().to_str().expect("utf-8 path"),
                &WorktreeFileListScope {
                    dir: Some("../outside".to_string()),
                    ..Default::default()
                },
            )
            .expect_err("escape should fail");

        assert_eq!(error, "File path must stay inside the worktree.");
    }
}
