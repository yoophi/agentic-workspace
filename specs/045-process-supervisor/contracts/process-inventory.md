# Contract: Production process inventory

| Source | Category | Owner | Stream contract | Supervisor |
|---|---|---|---|---|
| `crates/acp-agent-core/src/infrastructure/acp/runner.rs` | ServerOwned | Run attempt | stdin/stdout protocol, stderr display | required |
| `crates/acp-agent-core/src/infrastructure/acp/terminal.rs` | ServerOwned | Terminal | stdout/stderr display | required |
| `crates/acp-agent-core/src/infrastructure/agent_catalog.rs` | ServerOwned | CatalogHelper | parsed capture | required |
| `crates/acp-agent-core/src/infrastructure/acp/util.rs` | ServerOwned | ShellProbe | parsed capture | required |
| `crates/git-core/src/git_cli.rs` | ServerOwned | Git | parsed capture | required |
| `crates/workbench-core/src/infrastructure/git/cli_branch_provider.rs` | ServerOwned | Git | parsed capture | required |
| `crates/workbench-core/src/infrastructure/git/cli_remote_provider.rs` | ServerOwned | Git | parsed capture | required |
| `crates/workbench-core/src/infrastructure/git/cli_worktree_provider.rs` | ServerOwned | Git | parsed capture | required |
| `crates/workbench-core/src/infrastructure/git/cli_worktree_change_provider.rs` | ServerOwned | Git | parsed capture | required |
| `crates/workbench-core/src/infrastructure/fs/worktree_watcher.rs`의 Git probe | ServerOwned | WatcherHelper | parsed capture | required |
| `crates/workbench-core/src/infrastructure/orchestration/worktree_guard.rs` | ServerOwned | Git | parsed capture | required |
| `crates/workbench-host/src/lifecycle/ensure.rs` | DaemonBootstrap | Startup lifecycle | server log/null | excluded: launched server cannot own its launcher |
| `apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs` native opener | DesktopNative | Desktop shell | null | excluded: Tauri native boundary |
| `apps/agentic-workbench/src-tauri/build.rs` | Build | Build process | capture | excluded: server runtime 아님 |
| `tests/**`, `#[cfg(test)]` child | Fixture | Test | fixture-specific | excluded from production gate; used for contract tests |
| `apps/git-explorer/src-tauri/src/adapters/outbound/git_cli.rs` | OtherApp | GE Git adapter | parsed capture | 045 AW server scope 밖; GE lifecycle |
| `apps/git-explorer/src-tauri/src/adapters/outbound/fs_repository_watcher.rs` | OtherApp | GE watcher helper | parsed capture | 045 AW server scope 밖; GE lifecycle |
| `apps/hushline/src-tauri/src/adapters/system.rs` | OtherApp | HL model/download/transcription helper | helper-specific | 045 AW server scope 밖; HL lifecycle |
| `apps/markdown-annotator/src-tauri/src/cli_launcher.rs` | OtherApp | MA CLI launcher | launcher-specific | 045 AW server scope 밖; MA lifecycle |

## Gate

production Rust source에서 process creation API를 찾은 결과가 이 표의 pattern과 정확히 대응해야 한다.

- ServerOwned가 direct std/tokio `Command`를 사용하면 실패한다.
- allowlist는 platform module, daemon bootstrap, desktop-native, build와 위에 정확히 명시된 other-app source path만 허용한다. app 디렉터리 전체 wildcard는 금지한다.
- test source는 production violation 수에 섞지 않고 fixture inventory로 별도 출력한다.
- 경로가 이동하거나 새 helper가 생기면 표와 source gate fixture를 같은 commit에서 갱신한다.
