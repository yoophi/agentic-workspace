# Contract: Settings Window UI

## Scope

이 contract는 Agentic Workbench에서 설정 화면을 별도 창으로 여는 사용자-visible 동작과 app-shell 경계를 정의한다. 구현 세부 코드가 아니라 테스트 가능한 인터페이스와 상태 기대값을 기록한다.

## Native Menu Contract

### Menu Item: Preferences

- **Location**: macOS app menu under `Agentic Workbench`.
- **Visible Label**: `Preferences...`
- **Accelerator**: `Cmd+,`
- **Enabled State**: 앱 실행 중 항상 enabled.
- **Action**: 설정 창 열기 동작을 실행한다.

### Expected Behavior

1. 설정 창이 닫혀 있으면 새 설정 창을 열고 앞으로 가져온다.
2. 설정 창이 이미 있으면 새 창을 만들지 않고 기존 창을 앞으로 가져온다.
3. 설정 창이 최소화되어 있으면 복원 후 앞으로 가져온다.
4. 동작 실패 시 앱은 실패를 삼키지 않고 사용자-visible 오류 또는 command error로 남긴다.

## Tauri Command Contract

### Command: `open_settings_window`

**Request**: No payload.

**Response**: `void` on success.

**Errors**:

- 창 생성 실패
- 기존 창 표시/포커스 실패
- 설정 route URL 생성 실패

**Invariants**:

- 같은 요청을 반복해도 active settings window count는 1개 이하여야 한다.
- command는 설정 저장 데이터를 읽거나 쓰지 않는다.
- command는 agent run, worktree watcher, permission broker 상태를 변경하지 않는다.

## Frontend API Contract

### Function: `openSettingsWindow()`

**Owner Layer**: `entities/settings-window/api`

**Request**: No arguments.

**Response**: `Promise<void>`

**Expected Consumers**:

- App-level Settings toolbar button
- Worktree session page `onOpenSettings`
- Agent run panel override error action

**Invariants**:

- Consumers do not hard-code Tauri command names.
- Consumers do not navigate the current window to `/settings` when they intend to open preferences.
- Errors are surfaced by the calling screen in its existing error channel.

## Route Contract

### Route: `/settings-window`

**Purpose**: Render settings UI inside the dedicated settings window.

**Expected UI**:

- Displays the same settings groups, values, defaults, validation, and save behavior as the existing settings screen.
- Does not require `returnTo` query parameter.
- Provides a window-appropriate close action only if needed; native window close remains sufficient.

**Non-Goals**:

- It does not display project dashboard navigation.
- It does not replace the main window route.
- It does not create a separate settings data model.

## Acceptance Matrix

| Scenario | Action | Expected Result |
|----------|--------|-----------------|
| Settings closed | Click toolbar Settings | One settings window opens |
| Settings closed | Press `Cmd+,` | One settings window opens within 1 second |
| Settings open | Press `Cmd+,` five times | Existing settings window is focused; no duplicates |
| Settings minimized | Press `Cmd+,` | Existing settings window is restored and focused |
| Session active | Open settings from run error action | Session route, selected panel, and prompt draft remain intact |
| Save failure | Save invalid or failing settings | Error appears in settings window; main/session window remains usable |

## Accessibility and UX Expectations

- Settings toolbar/error actions keep accessible labels.
- Native menu item uses the conventional Preferences label and accelerator.
- Settings window title clearly identifies the window as settings.
- Text and controls fit within the settings window minimum size without overlapping.
