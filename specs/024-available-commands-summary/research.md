# Research: 사용 가능한 명령 요약과 조회

## 결정 1: `available_commands_update`는 session-level metadata로 처리한다

- **Decision**: `available_commands_update`는 timeline content가 아니라 현재 session의 command metadata로 분류한다.
- **Rationale**: command 목록은 사용자가 대화처럼 읽어야 하는 이벤트가 아니며, 길어질수록 timeline noise가 된다. #150의 주요 수용 기준도 raw payload suppression이다.
- **Alternatives considered**: raw timeline에 접힌 JSON으로 표시. 접힌 상태여도 raw diagnostic 성격이 남고 command 상세 조회 UX와 연결하기 어렵다.

## 결정 2: 기존 autocomplete parser를 확장한다

- **Decision**: `availableCommandCandidatesFromSessionUpdate` 주변의 unwrap/parsing 로직을 command metadata/detail 모델로 확장한다.
- **Rationale**: 이미 payload shape 처리, name/description normalization, `$` command source 분류가 존재한다. 같은 source를 재사용하면 autocomplete와 detail 조회의 command 목록이 어긋나지 않는다.
- **Alternatives considered**: AgentRunPanel 안에서 별도 ad hoc parsing. 중복과 edge case divergence 위험이 크다.

## 결정 3: summary는 run/session header에 compact하게 표시한다

- **Decision**: command count와 갱신 상태는 run/session header의 보조 metadata 영역에 표시한다.
- **Rationale**: 사용자는 command 목록 존재를 빠르게 확인하면 되고, timeline 흐름을 방해하면 안 된다. header는 #148의 session freshness와 같은 session-level metadata 위치다.
- **Alternatives considered**: timeline status message. #101과 유사하지만 #150의 primary goal은 raw noise 제거와 조회 가능성이고, 반복 update가 timeline을 채울 수 있다.

## 결정 4: detail 조회는 compact list 또는 popover/details panel로 제한한다

- **Decision**: 상세 조회에는 command 이름, 설명, 입력 힌트만 표시한다. 전체 input schema는 펼치지 않는다.
- **Rationale**: spec은 `input` 또는 입력 힌트를 보존하되 사용자가 인자 필요 여부를 알 수 있으면 충분하다고 정의한다. 전체 schema 렌더링은 layout risk가 크다.
- **Alternatives considered**: 모든 input schema JSON 표시. 구현 범위가 커지고 raw noise 제거 목표와 충돌한다.

## 결정 5: malformed command는 유효 항목만 표시하고 안전하게 fallback한다

- **Decision**: 이름이 없는 command는 detail list에서 제외하고, description/input hint가 없거나 잘못되면 해당 필드를 생략한다. command 목록 자체가 invalid이면 `0 commands available` 또는 empty 상태를 표시한다.
- **Rationale**: provider payload 품질이 일정하지 않을 수 있으므로 화면 안정성이 우선이다.
- **Alternatives considered**: invalid 항목도 "Unknown command"로 표시. 사용자가 실행 가능한 command로 오해할 수 있다.

## 결정 6: backend typed event는 선택 사항이다

- **Decision**: 우선 frontend parser/reducer에서 raw event를 metadata로 승격하고 timeline append 전에 suppress한다. backend mapper 변경은 typed contract가 필요해질 때만 한다.
- **Rationale**: 현재 backend는 unknown update를 raw로 보존하고, frontend에는 이미 session/update parsing 흐름이 있다. 최소 변경으로 value를 낼 수 있다.
- **Alternatives considered**: backend에 `AvailableCommands` run event 추가. 더 명확한 장점은 있지만 Rust/TS event contract를 함께 넓혀야 하므로 계획상 optional로 둔다.
