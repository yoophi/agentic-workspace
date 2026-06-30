//! git-core: git-explorer(정본)와 agentic-workbench가 공유하는 Git history /
//! commit-graph / commit-detail / file-diff core.
//!
//! Phase 0 spike: 루트 Cargo workspace 도입과 두 Tauri 앱에서의 빌드를 검증하기
//! 위한 골격이다. 실제 domain/ports/git_cli는 Phase 2에서 git-explorer에서
//! working_directory(path) 기반으로 이관한다.
