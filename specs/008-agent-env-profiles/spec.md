# Feature Specification: Agent 프로필과 환경변수 주입

**Feature Branch**: `008-agent-env-profiles`

**Created**: 2026-07-02

**Status**: Draft

**Input**: User description: "https://github.com/yoophi/agentic-workspace/issues/121 이슈 진행" (AW: ACP command override에서 환경변수 주입 지원 + agent 프로필 복수 등록 확장)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Agent 실행에 환경변수를 구조적으로 주입 (Priority: P1)

사용자가 특정 agent를 실행할 때 API key, feature flag, provider별 설정 같은 환경변수를 명령 문자열에 우회적으로 끼워 넣지 않고, 설정 화면에서 key/value 목록으로 등록한다. 등록된 환경변수는 해당 agent 실행 시 실행 환경에 주입된다.

**Why this priority**: 이슈의 핵심 요청이다. 현재 `env FOO=bar ...` 우회 방식은 quoting 오류, secret 노출, agent별 관리 불가 문제를 만들며, 이를 해소하는 것만으로 독립적 가치가 있다.

**Independent Test**: 설정에서 특정 agent에 환경변수를 등록하고 해당 agent를 실행해, 실행된 프로세스가 그 환경변수를 갖는지 확인한다(예: 환경변수를 읽는 프롬프트/동작으로 검증).

**Acceptance Scenarios**:

1. **Given** 설정에서 agent 프로필에 환경변수 `FOO=bar`를 저장한 상태, **When** 그 프로필로 agent를 실행하면, **Then** agent 프로세스는 환경변수 `FOO=bar`를 가진다.
2. **Given** 공통(global) 환경변수와 프로필별 환경변수에 동일 key가 있는 상태, **When** agent를 실행하면, **Then** 프로필별 값이 우선 적용되고 나머지 공통 key는 병합된다.
3. **Given** 환경변수 편집기에서 key가 비어 있거나 공백뿐인 항목을 입력한 상태, **When** 저장하면, **Then** 해당 항목은 저장되지 않고 정리(normalization)된다.
4. **Given** 환경변수 설정이 없는 agent, **When** 실행하면, **Then** 기존과 동일하게 동작한다(회귀 없음).

---

### User Story 2 - 동일 type의 agent를 프로필로 복수 등록 (Priority: P2)

사용자가 같은 종류의 agent(예: claude_code)를 서로 다른 실행 명령/환경변수 조합으로 여러 개 등록한다 — 예: "Claude (기본)", "Claude (프록시 경유)", "Claude (실험 플래그)". 세션 시작 시 등록된 프로필 목록에서 하나를 선택해 실행한다.

**Why this priority**: 환경변수 주입(US1)이 프로필 단위로 동작하므로 그 기반 위에서 자연스럽게 확장되는 가치다. 하나의 agent 종류를 상황별 구성으로 나눠 쓰는 것이 이슈의 확장 요청이다.

**Independent Test**: 같은 type으로 커스텀 프로필 2개를 서로 다른 command/env로 등록하고, 세션 시작 목록에 둘 다 표시되는지, 각각 선택해 실행하면 해당 구성으로 실행되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 설정 화면, **When** type, 이름, command, 환경변수를 지정해 커스텀 프로필을 추가하면, **Then** 프로필 목록에 저장되고 세션 시작 목록에 나타난다.
2. **Given** 동일 type의 프로필이 이미 있는 상태, **When** 같은 type으로 다른 이름의 프로필을 추가하면, **Then** 두 프로필이 공존한다.
3. **Given** 커스텀 프로필, **When** 삭제하면, **Then** 목록과 세션 시작 선택지에서 제거된다.
4. **Given** command를 지정하지 않은 프로필, **When** 실행하면, **Then** 해당 type의 기본 실행 명령으로 동작한다.
5. **Given** 세션 시작 화면, **When** 프로필을 선택해 실행하면, **Then** 그 프로필의 command와 환경변수로 agent가 실행된다.

---

### User Story 3 - 기본 프로필 관리와 안전장치 (Priority: P3)

최초 실행 시 4개 agent 종류(codex, claude_code, opencode, pi)가 기본 프로필로 자동 등록되어 있다. 사용자는 기본 프로필의 command/환경변수를 수정하거나 비활성화(disable)할 수 있지만 삭제할 수는 없다. 기본 프로필 중 최소 1개는 항상 활성 상태로 유지된다.

**Why this priority**: 프로필 모델(US2)의 안정성 장치다. 사용자가 실수로 모든 실행 수단을 지워 앱을 못 쓰게 되는 상황을 막는다.

**Independent Test**: 새 환경(설정 없음)에서 앱을 열어 기본 프로필 4개가 있는지 확인하고, 기본 프로필의 삭제 불가·수정 가능·disable 가능을 확인한 뒤, 마지막 남은 활성 기본 프로필의 disable 시도가 차단되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 저장된 프로필 설정이 없는 새 환경, **When** 설정 화면을 열면, **Then** codex, claude_code, opencode, pi 기본 프로필 4개가 등록되어 있다.
2. **Given** 기본 프로필, **When** 편집하면, **Then** command/환경변수 수정과 enable/disable은 가능하지만 삭제 동작은 제공되지 않는다.
3. **Given** 활성 기본 프로필이 1개만 남은 상태, **When** 그 프로필을 disable하려 하면, **Then** 차단되고 이유가 안내된다.
4. **Given** disable된 프로필, **When** 세션 시작 화면을 열면, **Then** 그 프로필은 선택 목록에 표시되지 않는다.
5. **Given** 기본 프로필 일부가 누락된 기존 저장 데이터, **When** 로드하면, **Then** 누락된 기본 프로필이 자동으로 채워진다.

---

### Edge Cases

- 기존 command-only override 데이터(공통 command, agent별 command)만 저장된 사용자가 업데이트 후 앱을 열면, 별도 조치 없이 해당 설정이 프로필 구성에 반영되어 동일하게 동작해야 한다.
- 실행 중이던 프로필을 disable하거나 삭제해도 진행 중인 세션은 영향받지 않아야 하며, 다음 세션 시작 목록에서만 빠진다.
- 세션 재사용(기존 세션 이어가기) 목록은 프로필이 아닌 agent 종류 기준으로 저장된 이력과 연결되므로, 프로필 도입 후에도 기존 세션 재사용이 동작해야 한다.
- 환경변수 value가 빈 문자열인 항목의 처리: key가 유효하면 빈 값 그대로 주입한다(일부 도구는 "빈 값 설정"과 "미설정"을 구분한다).
- 동일 프로필 이름 중복 입력 시에도 저장은 허용하되 목록에서 구분 가능해야 한다(이름은 표시용, 식별은 내부 고유값).
- 환경변수로 시스템 필수 변수(예: PATH)를 지정한 경우, 실행에 필요한 기본 실행 환경이 깨지지 않아야 한다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 사용자는 설정에서 agent 프로필 단위로 환경변수 key/value 목록을 저장·수정·초기화할 수 있어야 한다.
- **FR-002**: 사용자는 모든 프로필에 공통 적용되는 공통(global) 환경변수를 별도로 저장할 수 있어야 하며, agent 실행 시 공통 환경변수와 프로필 환경변수가 병합되고 동일 key는 프로필 값이 우선해야 한다.
- **FR-003**: agent 실행 시 병합된 환경변수가 agent 프로세스의 실행 환경에 주입되어야 한다.
- **FR-004**: 환경변수 key가 비어 있거나 공백뿐인 항목은 저장 시 제거되어야 한다. key 앞뒤 공백은 정리한다.
- **FR-005**: 사용자는 type(codex/claude_code/opencode/pi), 표시 이름, 실행 command(선택), 환경변수(선택)를 지정해 커스텀 프로필을 추가·수정·삭제할 수 있어야 한다.
- **FR-006**: 동일 type의 프로필을 여러 개 등록할 수 있어야 한다.
- **FR-007**: 프로필의 command가 지정되지 않으면 해당 type의 기본 실행 명령을 사용해야 한다. 공통(global) command가 설정되어 있으면 기본 실행 명령보다 우선한다(기존 동작 유지).
- **FR-008**: 최초 실행(프로필 저장 데이터 없음) 시 4개 type의 기본 프로필이 자동 등록되어야 하며, 저장 데이터에서 기본 프로필이 누락된 경우 로드 시 자동으로 채워져야 한다.
- **FR-009**: 기본 프로필은 수정(command/env)과 enable/disable만 가능하고 삭제할 수 없어야 한다. 커스텀 프로필은 삭제할 수 있어야 한다.
- **FR-010**: 기본 프로필 중 최소 1개는 항상 활성 상태여야 한다. 마지막 활성 기본 프로필의 disable 시도는 차단되고 사용자에게 이유가 안내되어야 한다.
- **FR-011**: 세션 시작 화면의 agent 선택 목록에는 활성(enabled) 프로필만 표시되어야 하며, 선택한 프로필의 command/환경변수로 agent가 실행되어야 한다.
- **FR-012**: 기존 command-only override 저장 데이터(공통 command, agent별 command)는 별도 migration 절차 없이 로드되어 동일한 실행 동작을 유지해야 한다.
- **FR-013**: command override와 환경변수가 모두 없는 프로필은 기존 기본 동작(카탈로그 기본 명령, 표준 실행 환경)을 그대로 유지해야 한다.

### Key Entities

- **Agent 프로필**: 고유 식별자, 표시 이름, agent type(codex/claude_code/opencode/pi), 실행 command(선택), 환경변수 목록(선택), 활성 여부, 기본 프로필 여부. 기본 프로필은 삭제 불가.
- **공통 override 설정**: 모든 프로필에 적용되는 공통 command(선택)와 공통 환경변수(선택). 프로필 값이 동일 항목보다 우선.
- **환경변수 항목**: key(비공백 필수)와 value(빈 문자열 허용)의 쌍.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: `apps/agentic-workbench` 단독 범위(frontend + Tauri backend). 공유 패키지/crate 변경 없음.
- **Frontend layering**: 프로필 모델·normalization은 `features/agent-command-override/model`(순수 로직, 단위 테스트), 설정 편집 UI는 `features/agent-command-override/ui`, agent 실행 선택은 `features/agent-run`, 타입·저장 API는 `entities/agent-run`.
- **Backend boundary**: 프로필/환경변수 도메인 모델은 `domain`(agent_run_settings), 병합·seed·하위 호환 로드 규칙은 `application` 서비스, 설정 저장은 기존 JSON repository(`infrastructure`), 프로세스 실행 환경 주입은 ACP runner(`infrastructure`). Tauri command는 위임만 한다.
- **Shared core vs UI**: 공유 없음(앱 전용 기능).
- **Persistence and safety**: 환경변수 값에 secret이 포함될 수 있으므로 로그·오류 메시지에 value를 노출하지 않는다. 실행 환경 주입 시 실행에 필요한 기본 환경(예: 실행 경로 탐색)이 깨지지 않아야 한다. 저장 데이터는 기존 설정 저장소 범위를 벗어나지 않는다.
- **Documentation and Storybook**: 프로필 편집기·env 편집기 UI 상태(기본/커스텀, disable 차단 안내, 빈 목록)의 Storybook 스토리 추가. 설정 동작 문서(docs) 갱신 검토.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 사용자가 설정 화면에서 환경변수 1개를 등록하고 agent 실행으로 반영을 확인하기까지 명령 문자열 편집 없이 2분 이내에 완료할 수 있다.
- **SC-002**: 동일 type의 프로필 2개를 등록하고 세션 시작에서 각각 선택 실행했을 때, 두 실행이 각자의 command/환경변수 구성으로 동작한다(100% 재현).
- **SC-003**: 기존 command-only 설정을 가진 사용자가 업데이트 후 아무 조치 없이 기존과 동일한 agent 실행 결과를 얻는다(회귀 0건).
- **SC-004**: 새 환경 최초 실행 시 기본 프로필 4개가 항상 존재하며, 어떤 조작 순서로도 활성 기본 프로필 0개 상태를 만들 수 없다.
- **SC-005**: 비활성 프로필은 세션 시작 목록에 나타나지 않으며, disable 차단 시 사용자에게 이유가 표시된다.
- **SC-006**: 빈 key/공백 key 환경변수는 저장 후 다시 열었을 때 존재하지 않는다.

## Assumptions

- 이슈의 1부(env 주입)와 확장 요청(프로필)을 하나의 기능으로 구현하며, 최종 모델은 프로필 리스트다. 1부의 "agent별 env" 요구는 프로필별 env로 충족된다.
- 공통(global) command/환경변수는 유지한다: 이슈 1부 수용 기준이 global env와의 병합을 명시하고, 기존 globalCommand 데이터의 하위 호환에도 자연스럽다. 우선순위는 프로필 값 > 공통 값 > 기본값.
- 환경변수 value는 평문으로 저장한다(기존 설정 저장 방식과 동일). secret 전용 암호화 저장은 범위 밖이며, 로그 비노출로 완화한다.
- 세션 재사용(reuse) 목록은 agent type 기준으로 동작을 유지한다. 프로필-세션 매핑 고도화는 범위 밖.
- agent type 목록(codex/claude_code/opencode/pi)은 현재 카탈로그와 동일하며, type 추가는 범위 밖.
- 기본 프로필의 "수정 초기화"(카탈로그 기본값으로 되돌리기)는 command/env를 비우는 것으로 충분하다.
