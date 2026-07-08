# Research: AW Window Menu List

## R1. Native `Window` 메뉴 갱신 위치

**Decision**: 기존 `build_native_menu`의 static `Window` submenu를 유지하되, 열린 창 목록 영역만 갱신하는 `native_window_menu` adapter를 `apps/agentic-workbench/src-tauri/src/infrastructure`에 둔다.

**Rationale**: Tauri native menu와 window handle은 framework 객체이므로 infrastructure 경계에 두는 것이 constitution의 hexagonal boundary에 맞다. `lib.rs`에 모든 로직을 넣으면 bootstrap 파일이 메뉴 정책과 window focus 정책까지 갖게 되므로 테스트와 유지보수가 어려워진다.

**Alternatives considered**:

- `lib.rs`에 직접 구현: 변경량은 적지만 menu id parsing, fallback title, stale window 처리 테스트가 어렵다.
- frontend에서 custom window switcher 제공: native `Windows` 메뉴 요구를 충족하지 못한다.

## R2. 메뉴 항목 ID 정책

**Decision**: 창 전환 항목은 `window-focus:<window-label>` namespace를 사용한다. parsing helper는 label이 비어 있거나 namespace가 다르면 선택 명령을 만들지 않는다.

**Rationale**: 기존 메뉴 ID인 `about-agentic-workbench`, `preferences-agentic-workbench`와 충돌하지 않고, 같은 제목의 창이 여러 개 있어도 Tauri window label로 대상을 구분할 수 있다.

**Alternatives considered**:

- 제목을 ID에 포함: 같은 제목, title 변경, control character 문제에 취약하다.
- index 기반 ID: 창 목록 정렬이나 갱신 사이에 대상이 바뀔 수 있다.

## R3. 창 제목 표시 기준

**Decision**: native menu label은 현재 window title을 우선 사용하고, 비어 있거나 읽을 수 없는 경우 session/default title fallback을 사용한다. title validation은 기존 80자/control-character 제한과 일관되게 유지한다.

**Rationale**: frontend `App.tsx`와 MCP title control은 이미 window title을 갱신한다. native menu가 window title을 읽으면 agent가 바꾼 제목도 반영할 수 있고, fallback은 로딩 중/빈 제목 edge case를 처리한다.

**Alternatives considered**:

- session metadata만 사용: MCP title override를 놓친다.
- frontend가 별도 command로 menu title을 전송: 중복 state가 생기고 실패 지점이 늘어난다.

## R4. 동기화 trigger

**Decision**: 창 생성 직후, 창 destroyed event, 메뉴 항목 선택 실패/성공 후, 그리고 title 변경 경로에서 메뉴 재동기화를 호출한다. 필요하면 frontend title effect 이후 lightweight command/event를 추가하지만, 우선 backend `set_title` 경로와 lifecycle hook을 기준으로 설계한다.

**Rationale**: feature의 핵심은 사용자가 native menu를 열 때 최신 창 목록을 보는 것이다. 생성/닫힘/선택 후 sync는 stale item을 줄이고, title 변경 sync는 issue 완료 조건을 충족한다.

**Alternatives considered**:

- 메뉴를 열 때마다 lazy rebuild: native menu open hook이 명확하지 않고 플랫폼 차이가 크다.
- 주기적 polling: 창 수가 적어도 불필요한 work가 생기며 상태 변경 지점이 이미 존재한다.

## R5. 창 focus 동작

**Decision**: 메뉴 선택 시 대상 window가 있으면 최소화 해제, 표시, focus 순서로 전환을 시도한다. 대상이 없으면 오류 dialog 없이 메뉴 상태를 갱신하고 무시한다.

**Rationale**: 사용자는 메뉴 선택을 창 전환으로 기대한다. 닫힌 창 race는 edge case로 명시되어 있으므로 앱 중단이나 불필요한 오류 표시보다 silent recovery가 적절하다.

**Alternatives considered**:

- 대상이 없을 때 오류 dialog 표시: 사용자가 이미 닫은 창에 대한 race를 불필요하게 방해한다.
- focus만 호출: 최소화된 창이나 숨겨진 창에서 기대 동작이 약하다.

## R6. 검증 전략

**Decision**: 순수 helper는 Rust unit test로 검증하고, native menu rendering/focus는 macOS Tauri dev app에서 수동 검증한다. frontend 변경이 발생하면 AW typecheck/test를 함께 수행한다.

**Rationale**: native OS menu는 headless unit test로 완전히 검증하기 어렵다. 하지만 ID parsing, entry ordering, fallback title, stale selection은 pure logic으로 분리해 자동화할 수 있다.

**Alternatives considered**:

- 전체 native menu e2e 자동화: 현재 프로젝트에 안정적인 native menu automation 기반이 없다.
- 수동 검증만 수행: regression surface가 있는 parsing/fallback 로직까지 수동에 의존하게 된다.
