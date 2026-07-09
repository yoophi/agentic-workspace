# Research: 세션 정보 메타데이터 표시

## 결정 1: `title`의 primary target은 AW window title이다

- **Decision**: `session_info_update.title`은 현재 Worktree Session window title에 반영한다.
- **Rationale**: 사용자는 여러 worktree/session 창을 오가며 작업하므로 OS window title이 session identity를 가장 빠르게 드러낸다. 기존 `App.tsx`에는 Tauri window title 상태와 `normalizeAgentWindowTitle` 검증 경로가 이미 있다.
- **Alternatives considered**: run header에만 표시, session list에만 표시. 두 방식은 창 전환과 OS-level context에서 title을 확인할 수 없어 primary target으로 부족하다.

## 결정 2: `updatedAt`은 window title이 아닌 보조 metadata로 표시한다

- **Decision**: 유효한 `updatedAt`은 run/session header 또는 session summary의 보조 metadata 영역에 표시한다.
- **Rationale**: timestamp는 session freshness 판단에는 유용하지만 window title에 넣으면 짧은 주기로 변하는 잡음이 된다. #113의 active/idle title 정책과도 섞이지 않게 한다.
- **Alternatives considered**: window title suffix, timeline message. suffix는 noisy하고 timeline message는 #145의 raw suppression 보장을 깨뜨린다.

## 결정 3: #145의 typed `sessionInfo` event contract를 유지한다

- **Decision**: backend가 보존하는 `SessionInfo { threadStatus, title, updatedAt }` event를 primary input으로 사용하고, raw fallback parser는 회귀 방지용으로 유지한다.
- **Rationale**: 이미 `session_update_mapper.rs`와 frontend types/formatter가 `title/updatedAt`을 읽는다. 구현은 raw JSON을 새로 노출하지 않고 기존 suppression 경로를 확장하면 된다.
- **Alternatives considered**: frontend에서 raw event만 직접 파싱. typed event contract를 우회해 backend/frontend 계약 검증이 약해진다.

## 결정 4: live metadata는 run panel 상태와 window-title event bridge로 처리한다

- **Decision**: `AgentRunPanel`이 active run의 `sessionInfo` update를 수신하면 thread status를 유지/갱신하고, 의미 있는 title은 기존 window-title event/fallback으로 전달한다. `updatedAt`은 panel state에 보관해 header metadata로 렌더링한다.
- **Rationale**: `App.tsx`가 standalone window title 적용 책임을 이미 가진다. panel은 session event를 가장 먼저 관찰하므로 metadata source로 적합하다.
- **Alternatives considered**: global store 추가, backend command 추가. 현재 범위에는 과하다.

## 결정 5: malformed metadata는 표시하지 않고 기존 UI를 유지한다

- **Decision**: 빈 title, control character 포함 title, 너무 긴 title은 기존 `normalizeAgentWindowTitle` 규칙으로 거른다. invalid `updatedAt`은 표시하지 않거나 이전 유효 값을 유지한다.
- **Rationale**: metadata-only event가 status indicator나 panel rendering을 망가뜨리면 안 된다.
- **Alternatives considered**: malformed 값을 그대로 표시. OS window title과 header UI에 깨진 값이 보일 수 있다.

## 결정 6: 새 shared package나 Storybook component는 만들지 않는다

- **Decision**: 표시 영역은 기존 run panel/header 내부에서 확장한다.
- **Rationale**: AW 단일 기능이고 cross-app 재사용 요구가 없다.
- **Alternatives considered**: shared session metadata component. 현재는 재사용 근거가 없다.
