# Quickstart: MCP Session Title Control

**Feature**: 011-mcp-session-ui-control | **Date**: 2026-07-06

이 문서는 구현 완료 후 기능을 end-to-end로 검증하는 절차다. 세부 interface는 [contracts/mcp-title-control.md](./contracts/mcp-title-control.md)를 기준으로 한다.

## Prerequisites

- 의존성 설치 완료: `pnpm install`
- Rust toolchain available
- AW Tauri app 실행 가능
- agent profile이 ACP `mcpServers` session config를 지원해야 함

## Static Verification

### TypeScript

```bash
pnpm --filter @yoophi/agentic-workbench check-types
pnpm --filter @yoophi/agentic-workbench test
```

Expected:

- title formatting/helper tests pass
- Worktree Session title override handling tests pass
- no TypeScript errors

Observed during implementation:

- `pnpm --filter @yoophi/agentic-workbench check-types` passed.
- `pnpm --filter @yoophi/agentic-workbench test` passed with 21 test files and 108 tests.

### Rust

```bash
cargo test -p agentic-workbench
```

If package selection is unavailable in the local Cargo setup:

```bash
cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml
```

Expected:

- title validation tests pass
- MCP tools/list exposes only `set_window_title`
- unauthorized request tests fail closed
- unknown/inactive run tests fail without window event
- valid owner request emits target window title command/result

Observed during implementation:

- `cargo test -p agentic-workbench` passed with 154 library tests, 0 main tests, and 0 doc tests.

## Manual End-to-End Validation

Implementation note: the automated checks passed, but the GUI/agent manual scenarios below still require launching AW and an actual Worktree Session agent run.

### 1. Start AW

```bash
pnpm tauri:dev:workbench
```

Expected:

- Agentic Workbench opens.
- A Worktree Session window can be opened from an existing project/worktree.

### 2. Start an agent run from the Worktree Session

Use any prompt that keeps the agent session active long enough to inspect environment and call MCP.

Expected:

- Agent run starts normally.
- Existing agent timeline and permission behavior remain unchanged.
- The agent process receives MCP compatibility environment variables: `AW_MCP_URL`, `AW_MCP_TOKEN`, `AW_MCP_RUN_ID`.
- The ACP `session/new` request includes an HTTP MCP server in `mcpServers` with name `agentic_workbench`, the local MCP URL, and an `Authorization` bearer header.

### 3. Verify capability list

From the agent, run `/mcp` or request the MCP tool list. If the selected agent does not expose a UI command for MCP listing, use a local test client with the injected MCP URL/token.

Expected:

- The agent recognizes the `agentic_workbench` MCP server without editing `~/.codex/config.toml`.
- The capability list contains `set_window_title`.
- The capability list does not contain file display, Markdown display, Git change display, workspace refresh, file modification, staging, commit, or permission tools.

### 4. Apply a valid title

Call `set_window_title` with the active run id and a readable title:

```json
{
  "runId": "<AW_MCP_RUN_ID>",
  "title": "Review session title control"
}
```

Expected:

- The owning Worktree Session window title changes to `Review session title control` within 1 second.
- Other Worktree Session windows are unchanged.
- Agent conversation, prompt input, selected workspace tab, and permission state are unchanged.
- Tool result is success and includes the applied title.

### 5. Reject invalid title

Call `set_window_title` with an empty or whitespace-only title.

Expected:

- No window title changes.
- Tool result is failure with `invalidTitle`.
- Existing title remains visible.

### 6. Reject cross-session request

Open two Worktree Session windows with active runs. Use the first run's token/context with the second run id, or otherwise attempt to target a non-owned run.

Expected:

- Request fails.
- Neither unintended window title changes nor cross-session side effects occur.
- Failure result is `unauthorized`, `unknownRun`, or `inactiveRun` depending on the attempted mismatch.

### 7. Closed-session behavior

Close the owning Worktree Session or finish/cancel the run, then call `set_window_title` again with the old run id.

Expected:

- Request fails.
- No window title changes.
- Result is `inactiveRun`, `unknownRun`, or `windowUnavailable`.

## Documentation Verification

Confirm that implementation adds Korean documentation:

```text
docs/mcp-session-title-control.md
```

Expected:

- File name is English.
- Content is Korean.
- It includes a Mermaid diagram for agent-to-window-title flow.
