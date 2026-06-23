# feat/support-multi-window Handoff

## 목표

현재 PR #7의 목표는 worktree session을 현재 창뿐 아니라 별도 Tauri window 또는 macOS native tab으로 열 수 있게 하고, agent 실행 흐름에 permission mode와 permission 응답 UI를 연결하는 것이다.

핵심 사용 흐름은 다음과 같다.

- 사용자가 worktree 목록에서 작업 화면 열기 버튼을 누르면 `새 창`, `새 탭`, `현재 창` 중 하나를 선택할 수 있다.
- 새 창/탭으로 열린 session은 `/session/:projectId/:worktreePath` standalone route를 사용해 앱 상단 navigation 없이 작업 화면만 표시한다.
- 각 session window에서 시작한 agent run 이벤트는 해당 window에만 전달되어 다른 session 화면을 오염시키지 않아야 한다.
- agent 실행 전 permission mode를 선택하고, ACP agent가 permission 요청을 보내면 dialog에서 승인/거절 옵션을 선택해 run을 계속 진행할 수 있어야 한다.
- Finder/Launchpad에서 실행한 GUI 앱에서도 agent/terminal command가 사용자의 login shell PATH를 반영해 `codex`, `node`, `npx` 등을 찾을 수 있어야 한다.

## 수행한 작업

- `PermissionMode`를 frontend/backend run request 모델에 추가했다.
- `AgentRunPanel`에 permission mode selector를 추가하고 `startAgentRun` 요청에 `permissionMode`를 전달하도록 했다.
- ACP permission 요청을 `RunEvent::Permission`으로 변환하고, pending permission을 dialog로 보여준 뒤 `respond_agent_permission` command로 응답하도록 연결했다.
- `PermissionBroker`를 `AppState`에 추가해 permission waiter를 run 단위로 관리하도록 했다.
- agent run event sink를 target window label 기반 `emit_to` 방식으로 바꿔 multi-window 환경에서 이벤트 전달 범위를 좁혔다.
- `open_worktree_window` Tauri command와 `window_manager`를 추가해 worktree session을 새 window 또는 macOS tab으로 열 수 있게 했다.
- worktree 목록의 단일 agent 실행 버튼을 dropdown menu로 바꿔 `새 창에서 열기`, `새 탭에서 열기`, `현재 창에서 열기`를 선택할 수 있게 했다.
- `/session/:projectId/:worktreePath` standalone route를 추가하고 session window에서는 full-width layout을 사용하도록 했다.
- Tauri bundle 설정과 icon asset을 추가했다.
- agent process와 terminal command 실행 시 login shell PATH를 조회해 `PATH`를 보강하고, 실행 파일을 가능한 경우 절대경로로 resolve하도록 했다.
- PR #6 이후 `origin/main` 위로 rebase하면서 prompt queue, usage bar, fixed-height session layout, `scrollHeader` 구조와 permission/multi-window 변경을 결합했다.

## 잘 된 점

- window별 event routing을 backend sink에서 처리해 frontend의 active run filter만으로는 막기 어려운 multi-window event 섞임을 구조적으로 줄였다.
- 새 창과 새 탭 열기 기능이 Tauri adapter인 `window_manager`에 분리되어 있어 UI route 코드가 native window 생성 세부사항을 직접 알지 않는다.
- 같은 project/worktree 조합에 deterministic session label을 사용해 중복 window를 무한히 만드는 대신 이미 열린 window를 focus하는 정책이 명확하다.
- permission mode 선택이 run 시작 요청에 명시적으로 들어가고, agent가 지원하지 않는 mode는 diagnostic으로 남기고 기본 동작을 유지하는 방향이라 실패 범위가 작다.
- GUI 앱 PATH 문제를 agent process와 terminal command 양쪽에 적용해 실제 desktop 실행 환경에서 흔한 command-not-found 문제를 줄인다.
- rebase 후 PR #6의 queue/usage/fixed layout 개선을 잃지 않고 PR #7 기능과 결합했다.

## 부족한 점

- 실제 Tauri window/tab 생성, macOS tab grouping, focus 재사용은 native runtime에서 수동 확인이 필요하다.
- permission request round-trip은 broker 단위 테스트와 frontend 타입 검증은 있지만, 실제 ACP agent가 permission 요청을 보내는 end-to-end 화면 검증은 아직 없다.
- login shell PATH 조회는 timeout을 갖지만, Finder/Launchpad에서 실행한 packaged 앱 환경에서 실제 `codex`, `node`, `npx` resolution이 되는지는 수동 smoke test가 필요하다.
- Tauri bundle/icon 변경이 기능 PR에 함께 들어와 있어 review 범위가 넓고, icon asset 생성 의도와 품질 검증이 분리되어 있지 않다.

## 남은 작업

- 실제 Tauri 앱에서 worktree session 열기 dropdown의 세 가지 경로를 확인한다.
  - 현재 창에서 열기
  - 새 창에서 열기
  - macOS 새 탭에서 열기
- 경로에 `/`, 공백, 한글, `#`, `%`가 포함된 worktree path를 실제 Tauri window에서 열어 query-string 기반 route 복원이 정확한지 확인한다.
- 같은 worktree를 새 창/탭으로 다시 열 때 새 window를 만들지 않고 기존 session window가 focus되는지 확인한다.
- 서로 다른 두 session window에서 agent run을 동시에 시작해 이벤트, usage, permission dialog, cancel 상태가 서로 섞이지 않는지 확인한다.
- permission 요청이 필요한 agent/tool 호출을 실제로 실행해 다음 상태를 확인한다.
  - permission dialog 표시
  - allow/reject 선택 후 dialog close
  - 선택 결과가 agent에 전달되어 run이 계속되거나 중단되는지
  - 이미 완료/취소된 run의 permission waiter가 정리되는지
- bundle/icon 변경을 별도 PR로 분리할지, 현재 PR에 유지한다면 icon asset 출처와 생성 방법을 문서화한다.
- native 변경이 있으므로 rebase 후 실제 앱을 다시 실행하고 smoke test를 수행한다.

## 검증 현황

- `pnpm run check-types` 통과.
- `pnpm run test` 통과.
- `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` 통과.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml permission -- --nocapture` 통과.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml window_manager -- --nocapture` 통과.
- Tauri dev 앱을 native rebuild와 함께 재실행했다.
- worktree session route는 `worktreePath` query string 기반으로 변경했고, `/`, 공백, 한글, `#`, `%` 인코딩을 Rust unit test로 고정했다.
- 현재 창 session route도 frontend helper로 분리했고, 동일한 특수문자 worktree path round-trip을 Vitest로 고정했다.
- session label은 SHA-256 prefix 기반으로 변경했고 route-safe/stable 속성을 Rust unit test로 고정했다.
- permission 응답은 run id와 owner window를 확인하도록 좁혔고, wrong-run 응답이 waiter를 제거하지 않는지 Rust unit test로 확인했다.
- permission dialog는 응답 선택 직후 optimistic하게 닫히며, command 실패 시 다시 열릴 수 있도록 answered set에서 제거한다.
- `dangerouslySkipAllPermissions` 선택 시 확인 dialog를 거치도록 했다.
- login shell PATH 조회는 2초 timeout을 둬 느린 shell init에 무기한 막히지 않도록 했다.
- 실제 Tauri multi-window/tab, permission request round-trip, packaged/Finder 실행 PATH 동작은 아직 수동 검증이 필요하다.

## 참고 변경 파일

- `apps/desktop/src-tauri/src/infrastructure/window_manager.rs`
- `apps/desktop/src-tauri/src/infrastructure/acp/runner.rs`
- `apps/desktop/src-tauri/src/infrastructure/acp/permission_flow.rs`
- `apps/desktop/src-tauri/src/infrastructure/permission_broker.rs`
- `apps/desktop/src-tauri/src/infrastructure/tauri_run_event_sink.rs`
- `apps/desktop/src-tauri/src/inbound/tauri_commands.rs`
- `apps/desktop/src-tauri/src/infrastructure/acp/util.rs`
- `apps/desktop/src/app/App.tsx`
- `apps/desktop/src/features/project-worktree/ui/project-worktree-card.tsx`
- `apps/desktop/src/features/agent-run/ui/agent-run-panel.tsx`
- `apps/desktop/src/pages/project-worktree-session/ui/project-worktree-session-page.tsx`
