//! 파일시스템 어댑터(038 US2: AW `fs_worktree_file_provider.rs`에서 이동).

pub mod worktree_file_provider;

/// 파일 목록 스캔(`worktree_file_provider`)과 AW worktree watcher가 공유하는 제외 디렉터리 목록.
/// 화면에 표시되지 않는 디렉터리의 변경이 rescan을 유발하지 않도록 단일 소스로 관리한다(AW specs/007 research R3).
pub const WORKSPACE_EXCLUDED_DIRS: &[&str] = &[
    ".git",
    ".next",
    ".turbo",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
];
