# Worktree Session 창 상태 복원 시도 기록

## 목적

Worktree Session 창의 크기와 위치를 Worktree별로 저장·복원하려는 현재 구현이 실제 앱에서 동작하지 않는 상태를 기록한다. 다음 작업자는 이 문서를 기준으로 원인을 재현·분석한 뒤 안정적인 복원 방식을 구현해야 한다.

## 요구 동작

- 사용자가 Worktree Session 창을 이동하거나 크기를 조절하면, 해당 Worktree의 창 위치와 내부 크기를 저장한다.
- 같은 Worktree를 다시 열면 마지막 위치와 크기로 창을 생성한다.
- 패널 레이아웃 저장이 창 상태를 지우거나, 창 상태 저장이 패널 레이아웃을 지우면 안 된다.
- 화면 구성이나 모니터가 달라진 경우에도 사용자가 접근 가능한 위치와 최소 크기로 창을 표시한다.

## 시도한 구현

1. `WorkspaceLayoutSettings`에 `windowX`, `windowY`, `windowWidth`, `windowHeight` 필드를 추가했다.
2. `window_manager::build_window`에서 Worktree 경로로 저장된 레이아웃을 조회해 `WebviewWindowBuilder`의 위치와 내부 크기에 적용했다.
3. `WindowEvent::Moved`, `WindowEvent::Resized`, `WindowEvent::CloseRequested`에서 현재 창의 위치와 내부 크기를 읽어 저장하도록 했다.
4. 패널 폭을 저장할 때 기존 창 상태 필드를 병합해 덮어쓰기를 막도록 했다.

## 확인된 결과

- Rust `cargo check`와 프런트엔드 TypeScript 검사는 통과했다.
- 실제 개발용 AW 앱에서 창을 이동·리사이즈한 뒤 닫고 다시 열어도 위치와 크기가 복원되지 않는다고 보고되었다.
- 따라서 위 구현은 완료로 간주하지 않는다.

## 우선 조사할 지점

1. `WindowEvent`에서 전달되는 `Window`와 실제 `WebviewWindow`의 URL/쿼리 값이 이벤트 시점에도 올바르게 조회되는지 로그로 확인한다.
2. `worktreePath` 쿼리 값과 JSON 레코드의 `workingDirectory`가 동일하게 정규화되는지 확인한다.
3. 앱 데이터 디렉터리의 `worktree-workspace-layouts.json`에 창 상태 필드가 실제로 기록되는지 확인한다.
4. `build_window`가 복원 값을 읽는지, 이후 운영체제 또는 Tauri 창 탭 처리 코드가 값을 덮어쓰는지 확인한다.
5. macOS 탭 모드와 새 창 모드 각각에서 창 생성 경로가 `build_window`를 통과하는지 확인한다.
6. 모니터 범위를 벗어난 좌표, 최소 크기, 최대화 상태를 포함한 창 상태 정책을 별도로 설계한다.

## 권장 재구현 방향

- 창 상태만을 위한 전용 도메인 모델·저장소를 레이아웃 패널 설정과 분리하는 방안을 우선 검토한다.
- 창 이벤트가 아니라 창 생성·종료 책임을 가진 window manager 내부에서 Worktree 경로를 명시적으로 전달해 저장한다.
- 저장 직후 JSON 파일 내용을 테스트로 검증하고, 동일 Worktree 재생성 시 builder에 적용되는 값을 단위 테스트 또는 통합 테스트로 검증한다.
- 복원 실패 시 기본 위치·크기로 안전하게 열되, 원인 파악에 필요한 오류 로그를 남긴다.

## 관련 파일

- `apps/agentic-workbench/src-tauri/src/infrastructure/window_manager.rs`
- `apps/agentic-workbench/src-tauri/src/application/worktree_workspace_layout_service.rs`
- `apps/agentic-workbench/src-tauri/src/infrastructure/json_worktree_workspace_layout_repository.rs`
- `apps/agentic-workbench/src-tauri/src/lib.rs`
- `specs/031-hide-workspace-panels/spec.md`
