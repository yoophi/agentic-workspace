# Phase 0 Research: 설정 별도 창과 단축어 실행

## Decision: 설정 창은 고정 label의 Tauri WebviewWindow로 관리한다

**Rationale**: 요구사항은 앱 전체에서 설정 창을 하나만 유지하고 반복 실행 시 기존 창을 앞으로 가져오는 것이다. Tauri window manager가 이미 세션 창 생성과 macOS tabbing 처리를 맡고 있으므로, 같은 infrastructure boundary에 `settings` 고정 label과 `index.html#/settings-window` URL을 추가하면 단일 창 재사용, 포커스, 최소화 복구를 한곳에서 관리할 수 있다.

**Alternatives considered**:

- 메인 창 내부 `/settings` route 유지. 거절: 메인 작업 화면을 전환하므로 현재 세션/입력 맥락 보존 요구를 직접 만족하지 못한다.
- 프론트엔드에서만 `window.open` 사용. 거절: Tauri 앱 창 lifecycle, label 기반 중복 방지, native menu accelerator와의 연결을 안정적으로 다루기 어렵다.
- 매번 새 설정 창 생성. 거절: FR-004와 SC-002의 단일 창 요구를 위반한다.

## Decision: 기존 `/settings` route는 설정 창 전용 route로 분리한다

**Rationale**: 현재 `SettingsPage`는 메인 `App.tsx`의 `/settings` route에서 렌더링되며 `returnTo` 기반 뒤로 가기를 가진다. 별도 창에서는 메인 창으로 돌아가는 버튼보다 창 닫기 동작이 자연스럽다. 따라서 설정 창 URL은 `/settings-window` 같은 전용 route로 분리하고, 메인 창의 기존 설정 버튼과 agent-run 오류 진입점은 route 이동 대신 `openSettingsWindow()`를 호출한다.

**Alternatives considered**:

- `/settings` route를 그대로 별도 창 URL로 사용. 고려 가능하지만 메인 창에서 실수로 직접 접근했을 때 기존 return flow와 창 전용 flow가 섞인다.
- 설정 페이지 컴포넌트를 복제. 거절: 설정 항목 누락과 저장 동작 divergence 위험이 크다.
- 설정 화면을 dialog로 유지. 거절: 별도 window 요구와 macOS Preferences 관례에 맞지 않는다.

## Decision: `Cmd+,`는 native app menu의 Preferences 항목 accelerator로 연결한다

**Rationale**: 사용자가 명시한 단축어는 macOS의 Preferences 관례다. Tauri native menu에서 `Preferences...` 항목에 `CmdOrCtrl+,` accelerator를 부여하고 menu event에서 `open_settings_window`와 같은 window manager 함수를 호출하면 텍스트 입력 포커스와 무관하게 앱 수준 단축어로 동작한다.

**Alternatives considered**:

- React keydown listener로 `metaKey && comma` 처리. 거절: WebView focus 상태, iframe/portal, native 메뉴 관례와의 차이 때문에 앱 전역 동작으로 약하다.
- Tauri global shortcut plugin 사용. 거절: 앱이 비활성화된 상태까지 잡는 global shortcut은 요구 범위를 넘고 권한/플러그인 복잡도를 늘린다.
- 단축어 없이 기존 버튼만 제공. 거절: FR-003을 충족하지 못한다.

## Decision: 설정 저장 모델과 query key는 유지한다

**Rationale**: 기능의 핵심은 설정 화면의 위치와 접근 방식 변경이다. 현재 설정 페이지는 `APP_COMMAND_OVERRIDE_SETTINGS_KEY`로 agent command override 설정을 불러오고 저장한다. 별도 창에서도 같은 저장소와 query key를 사용하면 기존 데이터, 저장 의미, validation을 바꾸지 않으면서 창 분리만 수행할 수 있다.

**Alternatives considered**:

- 설정 창 전용 저장 모델 도입. 거절: 사용자 가치 없이 migration과 regression 위험만 늘어난다.
- 설정 값을 메인 창 state로 끌어올려 공유. 거절: 별도 Tauri 창 간 메모리 상태 공유가 목적에 비해 복잡하며 기존 persistence가 이미 source of truth다.

## Decision: 설정 창은 세션 owner cleanup 대상에서 제외한다

**Rationale**: backend `on_window_event`는 label이 `session-`으로 시작하는 창이 닫힐 때 해당 창의 run과 watcher를 정리한다. 설정 창은 agent run owner가 아니며 닫혀도 메인 작업이나 세션을 중단하면 안 된다. 고정 label을 `settings`처럼 `session-` prefix와 분리하면 기존 cleanup 조건을 그대로 유지할 수 있다.

**Alternatives considered**:

- 설정 창 label을 세션 창과 유사한 prefix로 생성. 거절: cleanup 조건과 충돌할 수 있고 디버깅 비용이 커진다.
- cleanup 조건을 더 넓은 window registry로 재작성. 거절: 요구 범위를 넘으며 세션 안정성에 불필요한 위험을 만든다.

## Decision: Storybook은 `SettingsPage`의 창형 화면 상태를 page category에 추가한다

**Rationale**: 설정 화면은 screen-level UI이며 기존 Storybook page registry가 `pages.stories.tsx`에 있다. 별도 창으로 이동해도 SettingsPage 자체는 재사용되므로 기본, loading/error, long-content 상태를 pages category에 등록해 창 너비에서 레이아웃과 텍스트 overflow를 검증한다.

**Alternatives considered**:

- Storybook 생략. 거절: 프로젝트 constitution이 재사용 UI와 상태ful UI의 대표 상태 등록을 요구하고, 설정 창은 화면 크기/긴 내용 regression 위험이 있다.
- organism story로 등록. 거절: SettingsPage는 page-level composition이다.
