//! fixture용 결정적 Git 저장소 빌더(research R8). 고정 author/committer와 커밋 순번 기반 날짜를 써서
//! 같은 seed는 같은 커밋 해시를 만든다.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Deserialize;

const AUTHOR_NAME: &str = "Fixture Author";
const AUTHOR_EMAIL: &str = "fixture@example.com";
/// 2026-01-01T00:00:00Z. 커밋 i는 이 값 + i초.
const BASE_EPOCH: u64 = 1_767_225_600;

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FileContent {
    Text(String),
    Bytes { bytes: Vec<u8> },
    Repeat { repeat: String, count: usize },
}

impl FileContent {
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            FileContent::Text(text) => text.as_bytes().to_vec(),
            FileContent::Bytes { bytes } => bytes.clone(),
            FileContent::Repeat { repeat, count } => repeat.repeat(*count).into_bytes(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommitSeed {
    pub message: String,
    #[serde(default)]
    pub files: BTreeMap<String, FileContent>,
    /// 있으면 이 커밋 전에 `checkout -b <branch>`.
    #[serde(default)]
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteSeed {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorktreeSeed {
    /// 저장소 **부모** 디렉터리 기준 상대 경로.
    pub path: String,
    pub branch: String,
    /// worktree를 만든 뒤 그 안에 쓰는 커밋하지 않은 파일(삭제 전 검사 `dirty` 시나리오용).
    #[serde(default)]
    pub files: BTreeMap<String, FileContent>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRepoSeed {
    #[serde(default = "default_repo_name")]
    pub name: String,
    #[serde(default)]
    pub commits: Vec<CommitSeed>,
    /// 마지막 HEAD를 가리키는 추가 브랜치.
    #[serde(default)]
    pub branches: Vec<String>,
    #[serde(default)]
    pub remotes: Vec<RemoteSeed>,
    #[serde(default)]
    pub worktrees: Vec<WorktreeSeed>,
    /// 커밋하지 않는 working tree 변경. `null`이면 파일 삭제.
    #[serde(default)]
    pub working_changes: BTreeMap<String, Option<FileContent>>,
}

fn default_repo_name() -> String {
    "fixture-repo".into()
}

#[derive(Debug, Clone)]
pub struct BuiltRepo {
    pub root: PathBuf,
    pub name: String,
    /// 오래된 것부터.
    pub commits: Vec<String>,
}

impl BuiltRepo {
    pub fn head(&self) -> &str {
        self.commits.last().map(String::as_str).unwrap_or("")
    }

    pub fn parent(&self) -> &Path {
        self.root.parent().expect("repo has a parent")
    }
}

fn git(root: &Path, args: &[&str], commit_index: Option<u64>) -> String {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(root)
        .args(["-c", "commit.gpgsign=false"])
        .args(["-c", &format!("user.name={AUTHOR_NAME}")])
        .args(["-c", &format!("user.email={AUTHOR_EMAIL}")])
        .args(args)
        .env("GIT_AUTHOR_NAME", AUTHOR_NAME)
        .env("GIT_AUTHOR_EMAIL", AUTHOR_EMAIL)
        .env("GIT_COMMITTER_NAME", AUTHOR_NAME)
        .env("GIT_COMMITTER_EMAIL", AUTHOR_EMAIL)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE");
    if let Some(index) = commit_index {
        let date = format!("@{} +0000", BASE_EPOCH + index);
        command
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date);
    }
    let output = command.output().expect("git available");
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn write_file(root: &Path, relative: &str, content: &FileContent) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create dirs");
    }
    fs::write(path, content.to_bytes()).expect("write file");
}

/// `parent/<name>`에 저장소를 만든다. `parent`는 비어 있는 디렉터리여야 한다.
pub fn build(seed: &GitRepoSeed, parent: &Path) -> BuiltRepo {
    let root = parent.join(&seed.name);
    fs::create_dir_all(&root).expect("create repo dir");
    let mut init = Command::new("git");
    init.args(["init", "-q", "-b", "main"]).arg(&root);
    let output = init.output().expect("git available");
    assert!(
        output.status.success(),
        "git init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mut commits = Vec::new();
    for (index, commit) in seed.commits.iter().enumerate() {
        if let Some(branch) = &commit.branch {
            git(&root, &["checkout", "-q", "-b", branch], None);
        }
        for (relative, content) in &commit.files {
            write_file(&root, relative, content);
        }
        git(&root, &["add", "-A"], None);
        git(
            &root,
            &["commit", "-q", "--allow-empty", "-m", &commit.message],
            Some(index as u64),
        );
        commits.push(git(&root, &["rev-parse", "HEAD"], None));
    }
    for branch in &seed.branches {
        git(&root, &["branch", branch], None);
    }
    for remote in &seed.remotes {
        git(&root, &["remote", "add", &remote.name, &remote.url], None);
    }
    for worktree in &seed.worktrees {
        let path = parent.join(&worktree.path);
        git(
            &root,
            &[
                "worktree",
                "add",
                "-q",
                path.to_str().expect("utf8 path"),
                "-b",
                &worktree.branch,
            ],
            None,
        );
        for (relative, content) in &worktree.files {
            write_file(&path, relative, content);
        }
    }
    for (relative, content) in &seed.working_changes {
        match content {
            Some(content) => write_file(&root, relative, content),
            None => {
                let _ = fs::remove_file(root.join(relative));
            }
        }
    }

    BuiltRepo {
        root,
        name: seed.name.clone(),
        commits,
    }
}

/// `git worktree list --porcelain`의 항목 수(main 포함).
pub fn worktree_count(root: &Path) -> usize {
    git(root, &["worktree", "list", "--porcelain"], None)
        .lines()
        .filter(|line| line.starts_with("worktree "))
        .count()
}

/// `git worktree list --porcelain`의 경로들(실제 경로).
pub fn worktree_paths(root: &Path) -> Vec<String> {
    git(root, &["worktree", "list", "--porcelain"], None)
        .lines()
        .filter_map(|line| line.strip_prefix("worktree ").map(str::to_owned))
        .collect()
}

/// 저장소 root의 canonical 문자열. Git이 `worktree list --porcelain`에 내는 형식과 같다.
pub fn canonical_string(path: &Path) -> String {
    fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed() -> GitRepoSeed {
        serde_json::from_value(serde_json::json!({
            "commits": [
                { "message": "init", "files": { "README.md": "hello\n" } },
                { "message": "feat", "files": { "src/a.rs": "fn a() {}\n" }, "branch": "feature/a" }
            ],
            "workingChanges": { "README.md": "hello world\n" }
        }))
        .unwrap()
    }

    #[test]
    fn same_seed_builds_same_hashes() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let repo_a = build(&seed(), a.path());
        let repo_b = build(&seed(), b.path());
        assert_eq!(repo_a.commits.len(), 2);
        assert_eq!(repo_a.commits, repo_b.commits);
        assert_eq!(repo_a.head(), repo_b.head());
        assert_eq!(
            fs::read_to_string(repo_a.root.join("README.md")).unwrap(),
            "hello world\n"
        );
    }
}
