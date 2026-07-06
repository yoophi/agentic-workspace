# Tasks: MCP Session Title Control

**Input**: Design documents from `/specs/011-mcp-session-ui-control/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/mcp-title-control.md](./contracts/mcp-title-control.md), [quickstart.md](./quickstart.md)

**Tests**: Constitution-required tests are included for pure title validation, MCP safety boundaries, session-owner authorization, and frontend title application.

**Organization**: Tasks are grouped by the single P1 user story so the MVP can be implemented and tested independently.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add dependency and module entry points needed before implementation.

- [X] T001 Add `axum` and `tower-http` dependencies for the local MCP HTTP service in `apps/agentic-workbench/src-tauri/Cargo.toml`
- [X] T002 Create MCP infrastructure module declarations in `apps/agentic-workbench/src-tauri/src/infrastructure/mod.rs` and `apps/agentic-workbench/src-tauri/src/infrastructure/mcp/mod.rs`
- [X] T003 Create title-control domain and application module declarations in `apps/agentic-workbench/src-tauri/src/domain/mod.rs` and `apps/agentic-workbench/src-tauri/src/application/mod.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Establish testable domain/application contracts and app-managed MCP runtime state before the user story wiring.

**CRITICAL**: No user story work can begin until this phase is complete.

- [X] T004 [P] Add failing unit tests for title normalization, blank/control-character rejection, and 80-character maximum in `apps/agentic-workbench/src-tauri/src/domain/mcp_title_control.rs`
- [X] T005 [P] Add failing unit tests for MCP tool list shape, unsupported tool rejection, and token validation in `apps/agentic-workbench/src-tauri/src/infrastructure/mcp/title_tool.rs`
- [X] T006 [P] Add failing unit tests for active owner lookup, unknown run rejection, and window-unavailable result in `apps/agentic-workbench/src-tauri/src/application/mcp_title_control_service.rs`
- [X] T007 Implement `TitleChangeRequest`, `TitleChangeResult`, failure codes, and title validation helpers in `apps/agentic-workbench/src-tauri/src/domain/mcp_title_control.rs`
- [X] T008 Implement `McpTitleControlService` interface for run-owner validation and target window command creation in `apps/agentic-workbench/src-tauri/src/application/mcp_title_control_service.rs`
- [X] T009 Expose active run owner lookup and inactive-run distinction needed by title control in `apps/agentic-workbench/src-tauri/src/infrastructure/agent_session_registry.rs`
- [X] T010 Define app-managed MCP runtime state with base URL, runtime token, and env key constants in `apps/agentic-workbench/src-tauri/src/infrastructure/mcp/mod.rs`

**Checkpoint**: Foundation ready. Domain validation, safety boundaries, and runtime state contracts are defined and tested.

---

## Phase 3: User Story 1 - Agent labels a Worktree Session window (Priority: P1) MVP

**Goal**: An authorized agent can change only its owning Worktree Session window title through the local MCP HTTP service.

**Independent Test**: Start a Worktree Session agent run, call `set_window_title` with the injected run id/token, verify that only the owning window title changes within 1 second and invalid/cross-session requests fail closed.

### Tests for User Story 1

- [X] T011 [P] [US1] Add failing MCP HTTP request/response tests for `initialize`, `tools/list`, valid `tools/call`, invalid token, and unsupported tool in `apps/agentic-workbench/src-tauri/src/infrastructure/mcp/protocol.rs`
- [X] T012 [P] [US1] Add failing tests for agent environment injection of `AW_MCP_URL`, `AW_MCP_TOKEN`, and `AW_MCP_RUN_ID` in `apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs`
- [X] T013 [P] [US1] Add failing frontend title override tests for default title, valid override, and invalid/empty event handling in `apps/agentic-workbench/src/entities/project/lib/worktree-window-title.test.ts`

### Implementation for User Story 1

- [X] T014 [US1] Implement minimal JSON-RPC/MCP request and response parsing for `initialize`, `tools/list`, and `tools/call` in `apps/agentic-workbench/src-tauri/src/infrastructure/mcp/protocol.rs`
- [X] T015 [US1] Implement `set_window_title` tool schema, success payload, failure payload, and capability-list-only boundary in `apps/agentic-workbench/src-tauri/src/infrastructure/mcp/title_tool.rs`
- [X] T016 [US1] Implement localhost HTTP router, bearer token extraction, Origin validation, and JSON response handling in `apps/agentic-workbench/src-tauri/src/infrastructure/mcp/mod.rs`
- [X] T017 [US1] Start and manage the MCP HTTP service during Tauri app setup and register its state in `apps/agentic-workbench/src-tauri/src/lib.rs`
- [X] T018 [US1] Inject MCP URL, runtime token, and run id into agent launch environment without overwriting user-provided non-MCP env values in `apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs`
- [X] T019 [US1] Build a session-window title command only for the run owner window from `apps/agentic-workbench/src-tauri/src/application/mcp_title_control_service.rs`
- [X] T020 [US1] Listen for the title command and apply runtime title override in the Worktree Session route in `apps/agentic-workbench/src/app/App.tsx`
- [X] T021 [US1] Add title override formatting and validation helpers used by the route in `apps/agentic-workbench/src/entities/project/lib/worktree-window-title.ts`
- [X] T022 [US1] Ensure closed-window cleanup prevents later MCP title requests from changing any title in `apps/agentic-workbench/src-tauri/src/lib.rs`

**Checkpoint**: User Story 1 is fully functional and independently testable.

---

## Phase 4: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, verification, and safety review for the completed MVP.

- [X] T023 [P] Add Korean architecture documentation with a Mermaid agent-to-window-title flow in `docs/mcp-session-title-control.md`
- [X] T024 [P] Document final MCP env key names and failure codes in `specs/011-mcp-session-ui-control/contracts/mcp-title-control.md`
- [X] T025 Run Rust verification command `cargo test -p agentic-workbench` and record any deviations in `specs/011-mcp-session-ui-control/quickstart.md`
- [X] T026 Run frontend verification commands from `apps/agentic-workbench/package.json`: `pnpm --filter @yoophi/agentic-workbench check-types` and `pnpm --filter @yoophi/agentic-workbench test`
- [ ] T027 Run quickstart manual validation for tools/list, valid title, invalid title, cross-session rejection, and closed-session rejection using `specs/011-mcp-session-ui-control/quickstart.md`
- [X] T028 Verify no file display, Markdown display, Git change display, file modification, staging, commit, or permission tools are exposed by MCP in `apps/agentic-workbench/src-tauri/src/infrastructure/mcp/title_tool.rs`
- [X] T029 Review the final diff for app boundary compliance across `apps/agentic-workbench`, `docs/mcp-session-title-control.md`, and `specs/011-mcp-session-ui-control/tasks.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup completion. Blocks User Story 1.
- **User Story 1 (Phase 3)**: Depends on Foundational completion.
- **Polish (Phase 4)**: Depends on User Story 1 completion.

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational. It is the complete MVP and has no dependency on other stories.

### Within User Story 1

- Write constitution-required tests T011-T013 before implementation tasks T014-T022.
- Implement domain/application validation before HTTP and frontend integration.
- Implement MCP protocol/tool routing before agent env injection validation.
- Implement backend window event emission before frontend listener wiring.
- Run story checkpoint validation before Phase 4 polish tasks.

### Parallel Opportunities

- T004, T005, and T006 can be written in parallel after Phase 1 because they target different modules.
- T011, T012, and T013 can be written in parallel because they target backend protocol, inbound env injection, and frontend helper tests respectively.
- T023 and T024 can run in parallel after implementation stabilizes because they update different documentation files.

---

## Parallel Example: User Story 1

```bash
# Write independent tests in parallel:
Task: "T011 [US1] Add failing MCP HTTP request/response tests in apps/agentic-workbench/src-tauri/src/infrastructure/mcp/protocol.rs"
Task: "T012 [US1] Add failing agent environment injection tests in apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs"
Task: "T013 [US1] Add failing frontend title override tests in apps/agentic-workbench/src/entities/project/lib/worktree-window-title.test.ts"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1 setup.
2. Complete Phase 2 foundation and ensure safety tests fail before implementation.
3. Complete Phase 3 User Story 1.
4. Stop and validate the single MVP story with Rust tests, TypeScript tests, and quickstart manual checks.

### Incremental Delivery

1. Deliver title validation and owner-scope service.
2. Add MCP tools/list and tools/call with only `set_window_title`.
3. Add agent env injection and HTTP service startup.
4. Add frontend title override listener.
5. Verify that no non-title MCP capabilities are exposed.

### Notes

- `[P]` tasks use different files and can run in parallel after their phase dependencies are met.
- `[US1]` maps to "Agent labels a Worktree Session window".
- No Storybook task is required because no reusable UI component is introduced.
- Avoid adding file display, Markdown preview, Git changes, permission approval, or file modification tools in this feature.
