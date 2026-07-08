# Quickstart: AW Window Menu List

## Prerequisites

- macOS 또는 native app menu를 확인할 수 있는 데스크톱 환경
- repository dependencies 설치 완료
- AW에 최소 1개 project/worktree가 등록되어 여러 session window를 열 수 있는 상태

## Automated Verification

```bash
pnpm --dir apps/agentic-workbench check-types
pnpm --dir apps/agentic-workbench test
cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml window_menu
cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml
rustfmt --edition 2024 --check \
  apps/agentic-workbench/src-tauri/src/lib.rs \
  apps/agentic-workbench/src-tauri/src/domain/window_menu.rs \
  apps/agentic-workbench/src-tauri/src/application/window_menu_service.rs \
  apps/agentic-workbench/src-tauri/src/infrastructure/native_window_menu.rs \
  apps/agentic-workbench/src-tauri/src/infrastructure/window_manager.rs
```

Expected outcomes:

- TypeScript typecheck passes if frontend files changed.
- Vitest passes if title helper or app title synchronization files changed.
- Rust tests cover menu id parsing, duplicate-title handling, fallback title, stale target selection, and focus command construction.
- Rust formatting check passes for changed backend files.

## Manual Native Menu Validation

Start AW:

```bash
pnpm --dir apps/agentic-workbench tauri:dev
```

### Scenario 1: 열린 창 목록 표시

1. AW main window에서 같은 project의 worktree session window를 2개 이상 연다.
2. native `Window` 메뉴를 연다.
3. 열린 session windows가 각각 표시되는지 확인한다.

Expected:

- 메뉴에는 열린 AW 창이 모두 표시된다.
- 각 항목은 사용자가 구분 가능한 제목을 가진다.

### Scenario 2: 메뉴 항목으로 창 전환

1. session window A를 전면에 둔다.
2. session window B가 뒤에 있거나 최소화된 상태가 되도록 만든다.
3. native `Window` 메뉴에서 session window B 항목을 선택한다.

Expected:

- session window B가 표시되고 활성화된다.
- 새 창이 중복 생성되지 않는다.
- 오류 dialog가 표시되지 않는다.

### Scenario 3: 창 닫힘 반영

1. session window를 3개 연다.
2. 하나를 닫는다.
3. native `Window` 메뉴를 다시 연다.

Expected:

- 닫힌 창 항목은 목록에서 제거된다.
- 남아 있는 창 항목은 계속 선택 가능하다.

### Scenario 4: 창 제목 변경 반영

1. session window에서 agent run을 시작한다.
2. title control tool 또는 기존 title 변경 경로로 창 제목을 바꾼다.
3. native `Window` 메뉴를 다시 연다.

Expected:

- 해당 창 항목은 변경된 제목으로 표시된다.
- 같은 제목의 창이 둘 이상이어도 각 항목 선택은 대응하는 창으로 전환된다.

## Non-Goals To Preserve

- 새 창 생성 메뉴를 추가하지 않는다.
- 창 정렬, tile, tab overview 같은 OS-specific window management feature를 추가하지 않는다.
- `packages/*`나 다른 앱에 native menu state를 공유하지 않는다.

## References

- [spec.md](./spec.md)
- [data-model.md](./data-model.md)
- [contracts/native-window-menu.md](./contracts/native-window-menu.md)

## Implementation Verification Log

- 2026-07-08: `cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml window_menu` passed, 12 window menu tests.
- 2026-07-08: `cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml` passed, 185 library tests.
- 2026-07-08: `npm exec --yes -- pnpm --dir apps/agentic-workbench test` passed, 32 files / 154 tests.
- 2026-07-08: `npm exec --yes -- pnpm --dir apps/agentic-workbench check-types` passed.
- 2026-07-08: `rustfmt --edition 2024 --check ...` passed for the changed backend files.
- 2026-07-08: `npm exec --yes -- pnpm --dir apps/agentic-workbench tauri:dev` built and launched the AW dev app successfully.
- 2026-07-08: Manual native `Window` menu scenarios 1 through 4 were confirmed in the desktop app.
