# Contract: 불변 Tauri command 29개 (호환 어댑터)

프론트엔드(`apps/agentic-workbench/src/**`)는 변경하지 않는다. command 이름·파라미터 이름(camelCase)·반환 JSON 형태·오류 문자열이 이전과 같아야 한다. 037 [tauri-compat-commands.md](../../037-workbench-seam/contracts/tauri-compat-commands.md)의 Fault→String 규칙(message만)을 그대로 쓴다.

## 불변 시그니처

| command | 파라미터 | 반환 | operation |
|---|---|---|---|
| `update_project` | `id: String, input: ProjectInput` | `Project` | `project.update` |
| `delete_project` | `id: String` | `()` | `project.delete` |
| `list_saved_prompts` | — | `Vec<SavedPrompt>` | `savedPrompt.list` |
| `create_saved_prompt` | `input: SavedPromptInput` | `SavedPrompt` | `savedPrompt.create` |
| `update_saved_prompt` | `id, input: SavedPromptInput` | `SavedPrompt` | `savedPrompt.update` |
| `delete_saved_prompt` | `id` | `()` | `savedPrompt.delete` |
| `get_goal` | `working_directory` | `Option<ThreadGoal>` | `goal.get` |
| `create_goal` | `input: GoalInput` | `ThreadGoal` | `goal.create` |
| `update_goal` | `working_directory, input: GoalUpdateInput` | `ThreadGoal` | `goal.update` |
| `clear_goal` | `working_directory` | `()` | `goal.clear` |
| `record_goal_progress` | `working_directory, input: GoalProgressInput` | `ThreadGoal` | `goal.recordProgress` |
| `get_agent_run_settings` | `working_directory` | `Option<AgentRunSettings>` | `agentRunSettings.get` |
| `save_agent_run_settings` | `settings: AgentRunSettings` | `AgentRunSettings` | `agentRunSettings.save` |
| `list_git_remotes` | `working_directory` | `Vec<GitRemote>` | `git.listRemotes` |
| `list_git_branches` | `working_directory` | `Vec<GitBranch>` | `git.listBranches` |
| `list_git_worktrees` | `working_directory, include_status: Option<bool>` | `Vec<GitWorktree>` | `git.listWorktrees` |
| `create_git_worktree` | `working_directory, input: GitWorktreeCreateDraft` | `()` | `git.createWorktree` |
| `delete_git_worktree` | `working_directory, path` | `()` | `git.deleteWorktree` |
| `list_worktree_changes` | `working_directory` | `Vec<WorktreeChange>` | `worktree.listChanges` |
| `get_worktree_changes` | `working_directory` | `GitWorktreeChanges` | `worktree.getChanges` |
| `get_worktree_file_diff` | `working_directory, path` | `GitWorktreeFileDiff` | `worktree.getFileDiff` |
| `list_worktree_files` | `working_directory, scope: Option<WorktreeFileListScope>` | `Vec<WorktreeFileEntry>` | `worktree.listFiles` |
| `read_worktree_text_file` | `working_directory, path` | `WorktreeTextFile` | `worktree.readTextFile` |
| `list_worktree_git_history` | `working_directory, max_count?, offset?, cursor?` | `GitCommitHistory` | `worktree.listHistory` |
| `get_worktree_git_graph` | 같음 | `GitCommitGraph` | `worktree.getGraph` |
| `get_worktree_commit_detail` | `working_directory, commit_hash` | `GitCommitDetail` | `worktree.getCommitDetail` |
| `get_worktree_commit_file_diff` | `working_directory, commit_hash, path` | `GitFileDiff` | `worktree.getCommitFileDiff` |
| `list_agents` | — | `Vec<AgentDescriptor>` (오늘은 infallible; 어댑터는 `Result` 실패 시 빈 목록이 아니라 `Err(message)` — 실제로는 발생하지 않음) | `agent.list` |
| `list_provider_sessions` | `agent_id, cwd: Option<String>` | `Vec<ProviderSession>` | `agent.listProviderSessions` |

반환 타입은 core로 이동한 도메인 타입(AW `domain/mod.rs`가 `pub use workbench_core::domain::*`로 재노출) 또는 git-core 타입이며, 어댑터는 `CallReply.output`(DTO JSON)을 그 타입으로 역직렬화한다. DTO와 도메인 타입의 JSON이 같음은 wire 동일성 테스트가 보장한다(R1).

## 변환 규칙

- `*Input` → `serde_json::Value`: 필드명 camelCase, `Option::None`은 필드 생략(현재 프론트가 보내는 형태 유지).
- `list_git_worktrees(include_status: None)` → `includeStatus` 생략(서버 기본 false).
- `list_worktree_files(scope: None)` → `scope` 생략.
- `save_agent_run_settings(settings)` → `{ "settings": <AgentRunSettings JSON> }`.
- 변경 command는 호출마다 `IdempotencyKey::random()`, `expectedRevision: None`.
- 조회 command는 `idempotencyKey: None`.
- 오류: `fault.message`만. 코드·outcome·requestId를 문자열에 붙이지 않는다.
- `Ok(())` 반환 command는 `output: null`을 `()`로 받는다.

## perf 로그

`run_blocking_command` 경로가 남기던 `[perf] <name> wait_ms run_ms` 대신 `perf_log::log_async_command(name, future)`가 `run_ms`를 남긴다(R10).

## 검증

- `workbench_compat.rs` 유닛 테스트: fixture 29개 이상의 request를 어댑터 변환 결과와 대조(operation·input JSON 동일), 대표 fault 3건(invalidArgument·notFound·forbidden)이 message만 남기는지, `null` 출력이 `()`로 풀리는지.
- AW `cargo test -p agentic-workbench`: 기존 command 관련 테스트 무수정 통과.
- `git diff --stat origin/main -- apps/agentic-workbench/src` = 0.
