# PR #7 Review TODO

## 개요

현재 PR은 worktree session을 현재 창, 별도 Tauri window, macOS native tab으로 열 수 있게 하고, ACP agent 실행 흐름에 permission mode와 permission 응답 UI를 연결한다.

핵심 변경 범위는 다음과 같다.

- worktree 목록의 실행 버튼을 dropdown으로 바꿔 `새 창`, `새 탭`, `현재 창` 진입 경로를 제공한다.
- standalone session route를 추가해 새 window/tab에서 navigation shell 없이 작업 화면만 표시한다.
- Tauri backend에 deterministic session window label과 window/tab 생성 adapter를 추가한다.
- agent run event를 target window label로 emit해 session window 간 이벤트 섞임을 줄인다.
- run request에 permission mode를 추가하고 ACP `session/request_permission`을 dialog 응답으로 왕복시킨다.
- GUI 실행 환경에서도 agent/terminal command가 login shell PATH를 반영하도록 command resolution을 보강한다.
- Tauri bundle/icon 설정을 추가하고 debug DevTools 자동 open을 opt-in으로 바꾼다.

## 리뷰 결과

현재까지 확인한 범위에서 merge를 막을 만한 기능 결함은 발견하지 못했다.

잘 된 점:

- window routing 책임이 backend event sink와 window manager 쪽에 있어 frontend 상태 필터만으로 multi-window isolation을 떠안지 않는다.
- session route는 `worktreePath`를 query string으로 넘겨 `/`, 공백, 한글, `#`, `%` 같은 path 문자를 안정적으로 다룬다.
- permission 응답은 run id와 owner window를 함께 확인해 다른 run/window의 waiter를 잘못 해제하지 않도록 좁혀져 있다.
- `dangerouslySkipAllPermissions`는 확인 dialog를 거치도록 되어 있어 실수로 높은 권한 mode를 선택하는 위험이 줄었다.
- smoke 검증용 ACP agent가 opt-in catalog로 추가되어 실제 permission dialog round-trip을 재현할 수 있다.

주의할 점:

- `ACP_AGENT_CATALOG_PATH` smoke catalog는 root `pnpm run tauri:dev`의 Turborepo 경유 실행에서는 Rust 앱까지 전달되지 않았다. smoke 검증 시에는 `apps/agentic-workbench`에서 직접 `ACP_AGENT_CATALOG_PATH=... pnpm tauri dev`로 실행해야 했다.
- smoke catalog command는 linked worktree cwd에서도 main worktree의 smoke script를 찾도록 `git worktree list --porcelain` 기반으로 보강했다.
- production Vite build는 chunk size warning을 출력한다. 기존 번들 크기 경고이며 이번 PR 기능 검증을 막지는 않는다.

## 완료한 검증

- `pnpm run check-types` 통과.
- `pnpm run test` 통과.
- `pnpm run build` 통과.
- `cargo check --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml` 통과.
- `cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml -- --nocapture` 통과.
- `node --check apps/agentic-workbench/scripts/acp-permission-smoke-agent.mjs` 통과.
- permission smoke agent JSON-RPC harness 통과: `initialize`, `session/new`, `session/prompt`, `session/request_permission` 응답까지 완료.
- Tauri dev 앱을 native rebuild와 함께 재실행했다.
- `ACP_OPEN_DEVTOOLS` 미설정 dev 실행에서 DevTools가 자동으로 열리지 않는 것을 확인했다.
- worktree dropdown에 `새 창에서 열기`, `새 탭에서 열기`, `현재 창에서 열기`가 표시되는 것을 확인했다.
- `현재 창에서 열기`가 main window를 session 화면으로 전환하고 선택 worktree cwd를 표시하는 것을 확인했다.
- `새 창에서 열기`가 `ACP Worktree Session` window를 추가하고 session 화면을 표시하는 것을 확인했다.
- `/private/tmp/작업 tree/a#b%c` 임시 worktree를 새 window로 열어 cwd가 정확히 복원되는 것을 확인했다.
- `새 탭에서 열기`가 macOS native tab bar에 session tab을 추가하는 것을 확인했다.
- 같은 worktree를 다시 `새 창에서 열기`로 호출했을 때 session window count가 증가하지 않아 중복 window가 생성되지 않는 것을 확인했다.
- 실제 Codex ACP run에서 `pwd` tool call이 완료되고 usage bar가 갱신되는 것을 확인했다.
- 실제 Codex ACP run에서 `/tmp/acp-permission-roundtrip-test.txt` 생성 tool call이 완료되는 것을 확인했고, 검증 후 임시 파일은 제거했다.
- `Permission Smoke` catalog로 실제 Tauri 앱을 실행해 permission dialog 표시를 확인했다.
- 실제 permission dialog에서 `Allow once` 선택 시 dialog가 닫히고 agent message에 `optionId: "allow-once"` 응답이 전달되는 것을 확인했다.
- 실제 permission dialog에서 `Reject` 선택 시 dialog가 닫히고 agent message에 `optionId: "reject"` 응답이 전달되는 것을 확인했다.
- 같은 worktree를 다시 `새 창에서 열기`로 호출했을 때 창 수가 main + session 2개로 유지되고 기존 `ACP Worktree Session`이 foreground로 올라오는 것을 확인했다.
- pending permission dialog가 열린 session window를 닫았을 때 main window만 남고 `acp-permission-smoke-agent.mjs` child process가 정리되는 것을 확인했다.
- 서로 다른 두 session window에서 동시에 `Permission Smoke` run을 시작했고, main worktree와 linked worktree 각각의 permission dialog가 서로 다른 workspace를 표시하는 것을 확인했다.
- 동시 permission 상태에서 main worktree run은 `Reject`, linked worktree run은 `Allow once`를 선택했고 각 window timeline에 해당 선택과 agent response가 서로 섞이지 않고 기록되는 것을 확인했다.
- smoke catalog command가 linked worktree cwd에서도 smoke agent를 초기화하는 것을 별도 harness로 확인했다.
- `cargo check --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml`을 release warning 정리 후 다시 통과했다.
- `pnpm --filter @yoophi/agentic-workbench tauri build` 통과. `.app`와 `.dmg` bundle이 생성됐고 release Rust warning은 없다.
- Finder에서 packaged `.app`를 실행해 기본 Codex agent의 `npx -y @agentclientprotocol/codex-acp`가 `npm exec`, `codex-acp`, Node, Codex app-server 프로세스로 시작되는 것을 확인했다.
- Finder-launched packaged 앱에서 Codex ACP `pwd` tool call이 `Completed`로 끝나고 `/Users/yoophi/project/agentic-workbench`와 usage bar가 표시되는 것을 확인했다.

## 남은 작업

- 남은 작업 없음.

## 재검증 명령

```bash
pnpm run check-types
pnpm run test
pnpm run build
cargo check --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml
cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml -- --nocapture
node --check apps/agentic-workbench/scripts/acp-permission-smoke-agent.mjs
pnpm --filter @yoophi/agentic-workbench tauri build
```

Permission smoke catalog로 앱을 실행할 때:

```bash
cd apps/agentic-workbench
ACP_AGENT_CATALOG_PATH=/Users/yoophi/project/agentic-workbench/apps/agentic-workbench/scripts/acp-smoke-agents.json pnpm tauri dev
```

## 참고 파일

- `apps/agentic-workbench/src-tauri/src/infrastructure/window_manager.rs`
- `apps/agentic-workbench/src-tauri/src/infrastructure/acp/runner.rs`
- `apps/agentic-workbench/src-tauri/src/infrastructure/acp/permission_flow.rs`
- `apps/agentic-workbench/src-tauri/src/infrastructure/permission_broker.rs`
- `apps/agentic-workbench/src-tauri/src/infrastructure/tauri_run_event_sink.rs`
- `apps/agentic-workbench/src-tauri/src/infrastructure/acp/util.rs`
- `apps/agentic-workbench/src/app/App.tsx`
- `apps/agentic-workbench/src/app/model/session-route.ts`
- `apps/agentic-workbench/src/features/project-worktree/ui/project-worktree-card.tsx`
- `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- `apps/agentic-workbench/scripts/acp-permission-smoke-agent.mjs`
