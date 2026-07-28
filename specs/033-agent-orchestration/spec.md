# Feature Specification: Main Coordinator 기반 에이전트 오케스트레이션

**Feature Branch**: `033-agent-orchestration`

**Created**: 2026-07-27

**Status**: Draft

**Input**: User description: "각 Worktree Session 창의 main agent-run을 부모이자 Main Coordinator로 두고, 이후 생성되는 모든 에이전트를 직접 자식으로 관리한다. 하나의 프롬프트 영역에서 Coordinator 또는 특정·복수 패널에 명령을 보내고, 백그라운드 작업의 상태를 관찰하며 필요할 때 패널로 승격한다. Main은 하위 작업을 병렬로 실행하고 결과를 취합해 더 나은 응답을 만든다."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 목표를 자식 에이전트에게 위임하고 결과 취합 (Priority: P1)

사용자는 Worktree Session의 Main Coordinator에게 하나의 목표를 전달한다. Main은 목표를
독립적인 하위 작업으로 나누고 서로 다른 역할의 직접 자식에게 배정한 뒤, 진행 상황과
구조화된 결과를 모아 출처가 구분된 최종 응답을 만든다.

**Why this priority**: 단순히 여러 패널을 여는 기능을 실제 협업으로 바꾸는 핵심 가치이며,
이 흐름만으로도 병렬 조사와 다각도 검토라는 독립적인 MVP를 제공한다.

**Independent Test**: Main에게 조사·검토·검증 관점이 필요한 목표를 위임하고, 세 직접
자식이 각자 작업을 수행한 뒤 Main이 세 결과를 구분해 종합한 응답을 반환하는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 활성 Main이 있고 자식이 없는 창에서, **When** 사용자가 세 역할이 필요한
   목표를 Coordinator에 위임하면, **Then** Main은 역할과 목표가 명시된 세 직접 자식
   작업을 만들고 병렬로 시작한다.
2. **Given** 세 자식 작업이 실행 중일 때, **When** 각 자식이 진행 상황과 결과를
   보고하면, **Then** Main은 보고를 올바른 작업과 자식에 귀속하고 모든 결과를
   사용자에게 구분해 보여 준다.
3. **Given** 자식 결과 사이에 불일치가 있을 때, **When** Main이 최종 응답을 만들면,
   **Then** 불일치와 각 결과의 출처를 숨기지 않고 비교·종합한다.
4. **Given** 사용자가 수동으로 만든 추가 에이전트 패널이 있을 때, **When** Main이
   해당 패널에 작업을 배정하면, **Then** 그 패널은 Main의 직접 자식으로 동일한 작업
   생명주기에 참여한다.

---

### User Story 2 - 백그라운드 작업 관찰과 패널 승격 (Priority: P1)

사용자는 자식 작업을 모두 패널로 열어 두지 않아도 한곳에서 상태와 최근 활동을 확인한다.
입력이나 관찰이 필요한 작업은 주의 대상으로 표시되고, 사용자가 원할 때 기존 실행을
중단하지 않고 탭 또는 타일 패널로 열었다가 다시 백그라운드로 내릴 수 있다.

**Why this priority**: 병렬 작업 수가 늘어도 화면을 과도하게 차지하지 않으면서 실패와
입력 요청을 놓치지 않게 하는 필수 운영 기능이다.

**Independent Test**: 세 백그라운드 작업을 실행하고, 한 작업의 입력 요청을 확인한 뒤
패널로 승격하고 다시 닫아도 작업과 실행 상태가 유지되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 패널로 열리지 않은 자식 작업이 있을 때, **When** 사용자가 작업 목록을
   확인하면, **Then** 역할, 작업 상태, 경과 시간, 최근 활동과 결과 유무를 볼 수 있다.
2. **Given** 백그라운드 작업이 사용자 입력을 요구할 때, **When** 입력 요청이
   보고되면, **Then** 작업은 주의 필요 상태가 되지만 현재 초점은 자동으로 바뀌지 않는다.
3. **Given** 실행 중인 백그라운드 작업이 있을 때, **When** 사용자가 패널로 열기를
   선택하면, **Then** 같은 실행과 대화가 탭 또는 타일에 표시되며 다시 시작되지 않는다.
4. **Given** 패널에 표시된 실행 중 작업이 있을 때, **When** 사용자가 패널을 닫으면,
   **Then** 작업은 취소되지 않고 백그라운드에서 계속되며 별도의 취소 동작을 제공한다.
5. **Given** 백그라운드 Child에 기존 run과 누적된 ACP timeline이 있을 때, **When**
   사용자가 패널로 승격하면, **Then** 패널은 해당 run에 즉시 결합하고 journal의 과거
   event를 재수화한 뒤 live event를 중복이나 누락 없이 이어서 표시한다.
6. **Given** 실행 중인 Child를 패널로 승격했을 때, **When** 패널 UI가 처음 mount되면,
   **Then** 빈 내부 초기값이 workspace의 기존 run binding과 실행 상태를 덮어쓰지 않는다.

---

### User Story 3 - 하나의 프롬프트 영역에서 대상 선택 (Priority: P1)

사용자는 워크스페이스의 하나의 프롬프트 작성 영역에서 현재 패널, 선택한 여러 패널,
전체 패널 또는 Main Coordinator를 대상으로 명령을 보낸다. 직접 전송과 목표 위임은
서로 다른 의도로 구분되고, 다중 전송의 결과는 대상별로 확인한다.

**Why this priority**: 여러 패널의 입력 영역을 오가야 하는 부담을 없애고, 단순 명령
전달과 오케스트레이션 시작을 사용자가 명확히 통제하게 한다.

**Independent Test**: 같은 작성 영역에서 네 대상 모드를 차례로 사용하고 각 명령이 선택한
대상에만 한 번 전달되며 대상별 성공·실패가 표시되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 여러 패널이 열린 창에서, **When** 사용자가 현재 패널을 대상으로 명령을
   보내면, **Then** 초점 패널만 명령을 받는다.
2. **Given** 사용자가 두 패널을 선택했을 때, **When** 같은 명령을 보내면, **Then**
   두 패널은 각각 한 번 명령을 받고 선택하지 않은 패널은 받지 않는다.
3. **Given** 사용자가 Coordinator 대상을 선택했을 때, **When** 목표를 제출하면,
   **Then** Main은 이를 단순 브로드캐스트가 아닌 위임 목표로 받아 하위 작업 계획을
   시작한다.
4. **Given** 다중 전송에서 일부 대상이 명령을 받을 수 없을 때, **When** 전송이
   끝나면, **Then** 성공한 전송은 유지되고 사용자에게 대상별 성공과 실패 이유가
   표시된다.

---

### User Story 4 - 차단·실패·Main 교체에서 안전하게 복구 (Priority: P2)

사용자는 자식이 차단되거나 실패하거나 Main의 현재 실행이 교체되어도 작업을 잃지 않고,
입력 제공, 재시도, 취소, 재배정 또는 새 Main 실행으로의 명시적 인계를 선택한다.

**Why this priority**: 장기 실행 협업은 정상 흐름뿐 아니라 중단과 재시작을 예측 가능하게
처리해야 신뢰할 수 있다.

**Independent Test**: 실행 중인 자식의 입력 요청과 실패를 발생시키고 Main을 재시작한 뒤,
각 작업이 올바른 상태로 남으며 사용자의 선택 없이는 소유권이나 문맥이 바뀌지 않는지
확인한다.

**Acceptance Scenarios**:

1. **Given** 자식이 추가 정보 없이는 진행할 수 없을 때, **When** 입력 요청을
   보고하면, **Then** 사용자는 요청 사유를 확인하고 응답하거나 취소할 수 있다.
2. **Given** 자식 작업 하나가 실패했을 때, **When** 실패가 보고되면, **Then** 다른
   자식의 성공 결과는 유지되고 실패한 작업만 재시도하거나 재배정할 수 있다.
3. **Given** 진행 중 작업이 있는 동안 Main의 실행이 종료되고 새 실행이 시작될 때,
   **When** 새 Main이 활성화되면, **Then** 진행 중 작업과 요약이 인계 대기 상태로
   표시되고 사용자의 명시적 선택 전에는 자동 귀속되지 않는다.
4. **Given** 다른 Worktree Session 창의 대상이 지정되었을 때, **When** 명령 또는
   작업 배정을 시도하면, **Then** 요청은 거부되고 현재 창의 작업 상태는 바뀌지 않는다.
5. **Given** Child가 사용자 입력을 요청했을 때, **When** 사용자가 Activity Rail에서
   응답하면, **Then** 응답은 동일한 active Child run에 전달되고 전달 성공이 확인된
   뒤에만 task가 다시 실행 중으로 전이한다.
6. **Given** Child가 progress, result, input request 또는 blocked report를 제출했을 때,
   **When** active Main Coordinator가 있으면, **Then** Main은 report가 도착했음을
   자동으로 통지받고 구조화된 원문을 조회해 후속 조치를 수행할 수 있다.
7. **Given** Main 또는 사용자가 Child에 후속 메시지를 보냈을 때, **When** runtime
   전달이 실패하면, **Then** 시스템은 성공으로 표시하거나 task 상태를 앞서 변경하지
   않고 재시도 가능한 전달 실패를 보존한다.

### Edge Cases

- Main 패널은 존재하지만 활성 실행이 없으면 새 위임을 시작하지 않고, 시작에 필요한
  조건과 보류 여부를 사용자에게 보여 준다.
- 동시 실행 한도에 도달하면 나머지 작업은 순서를 보존한 대기 상태가 되며 기존 작업을
  임의로 종료하지 않는다.
- 동일한 생성 또는 전송 요청이 재시도되면 동일 요청은 한 번만 적용되고 기존 결과를
  반환한다.
- 자식 패널이 닫히거나 실행이 교체되는 순간 도착한 명령은 오래된 실행에 전달되지 않고
  실패 이유와 안전한 재시도 선택지를 제공한다.
- 사용자가 Main 패널 삭제를 시도하면 삭제되지 않으며 Coordinator 역할이 유지된다.
- 자식이 다른 자식을 생성하거나 형제에게 직접 작업을 배정하려 하면 권한 부족으로
  거부된다.
- 결과가 보고되기 전에 실행이 비정상 종료되면 작업은 완료가 아닌 실패 또는 차단으로
  남고, 부분 결과가 있으면 별도로 보존된다.
- 백그라운드 작업이 완료된 뒤 패널로 열면 최종 대화와 결과를 확인할 수 있다.
- 백그라운드 Child에 event가 하나도 없는 경우 패널은 실제 run 연결 상태와 마지막
  확인 시각을 표시해야 하며, 단순한 빈 timeline을 `응답 없음`의 확정 증거로 표현하지
  않아야 한다.
- runtime journal에 보존 한도를 넘긴 gap이 있으면 패널은 durable task/report snapshot을
  먼저 표시하고 누락된 전체 대화를 복원한 것처럼 보이지 않아야 한다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 시스템은 각 Worktree Session 창에 삭제할 수 없는 Main Coordinator
  패널을 정확히 하나 유지해야 한다.
- **FR-002**: 시스템은 Main 이후 생성된 모든 에이전트 패널과 백그라운드 에이전트를
  Main의 직접 자식으로 관리해야 한다.
- **FR-003**: 시스템은 첫 버전에서 자식의 하위 에이전트 생성과 자식 간 직접 작업
  배정을 허용하지 않아야 한다.
- **FR-004**: 시스템은 안정적인 패널 관계, 현재 실행 관계와 작업 관계를 서로
  독립적으로 식별해야 한다.
- **FR-005**: 사용자는 활성 Main에게 하나의 목표를 위임할 수 있어야 한다.
- **FR-006**: Main은 위임 목표를 역할, 목적, 제약과 기대 결과가 포함된 복수의 독립
  작업으로 나눌 수 있어야 한다.
- **FR-007**: Main은 새 자식 또는 기존 수동 생성 자식에게 작업을 배정할 수 있어야 한다.
- **FR-008**: Main은 자식 작업을 병렬 또는 제한에 따른 대기 상태로 시작하고, 중단,
  취소, 재시도와 재배정할 수 있어야 한다.
- **FR-009**: 자식은 자기 작업의 진행, 구조화된 결과, 차단 사유와 입력 요청을 Main에게
  보고할 수 있어야 한다.
- **FR-010**: Main은 여러 자식의 결과를 작업과 출처별로 수집하고 불일치를 보존한 채
  최종 응답으로 종합할 수 있어야 한다.
- **FR-011**: 시스템은 작업 상태, 실행 상태와 표시 상태를 독립적으로 관리해야 한다.
- **FR-012**: 작업 상태는 최소한 대기, 준비, 실행, 입력 필요, 차단, 완료, 실패와 취소를
  구분해야 한다.
- **FR-013**: 사용자는 패널로 열리지 않은 모든 자식 작업의 역할, 상태, 경과 시간, 최근
  활동, 입력 요청과 결과 유무를 한곳에서 확인할 수 있어야 한다.
- **FR-014**: 시스템은 입력 필요 또는 차단된 작업을 주의 대상으로 표시하되 사용자의
  현재 초점을 자동으로 변경하지 않아야 한다.
- **FR-015**: 사용자는 백그라운드 작업을 기존 실행 그대로 탭 또는 타일 패널로 승격할
  수 있어야 한다.
- **FR-016**: 사용자는 패널을 닫아 작업을 백그라운드로 분리할 수 있어야 하며 이 동작은
  작업 취소와 구분되어야 한다.
- **FR-017**: 시스템은 워크스페이스에 하나의 공용 프롬프트 작성 영역을 제공해야 한다.
- **FR-018**: 공용 작성 영역은 현재 패널, 선택 패널, 전체 패널과 Coordinator 대상
  모드를 제공해야 한다.
- **FR-019**: 시스템은 패널 대상 직접 전송과 Coordinator에 대한 목표 위임을 명확히
  구분해야 한다.
- **FR-020**: 다중 대상 전송은 하나의 전송 묶음과 대상별 결과를 기록하고, 일부 실패가
  성공한 대상의 결과를 취소하지 않아야 한다.
- **FR-021**: 동일한 생성, 배정 또는 전송 요청의 재시도는 중복 작업이나 중복 명령을
  만들지 않아야 한다.
- **FR-022**: Main 패널에 활성 실행이 없으면 시스템은 새 오케스트레이션을 실행하지
  않고 사용자에게 실행 불가 또는 보류 상태를 알려야 한다.
- **FR-023**: Main 실행이 교체되면 시스템은 새로운 Coordinator 세대를 구분하고 진행 중
  작업을 요약과 함께 명시적으로 인계할 선택지를 제공해야 한다.
- **FR-024**: 시스템은 사용자의 확인 없이 진행 중 작업을 새 Main 실행에 자동
  귀속하지 않아야 한다.
- **FR-025**: 한 자식의 실패나 취소는 다른 자식의 작업과 이미 수집한 결과를 제거하지
  않아야 한다.
- **FR-026**: 모든 자식 관계, 작업 배정과 명령 전달은 동일한 Worktree Session 창
  내부로 제한되어야 한다.
- **FR-027**: 시스템은 동시 실행 한도를 적용하고 초과 작업을 예측 가능한 순서로
  대기시켜야 한다.
- **FR-028**: 첫 버전의 자동 생성 자식은 읽기 전용 작업을 기본으로 수행해야 하며
  동시 파일 쓰기를 허용하지 않아야 한다.
- **FR-029**: 작업 완료는 명시적인 결과 보고를 우선 기준으로 삼아야 하며, 실행 종료나
  산출물 존재만으로 성공 처리하지 않아야 한다.
- **FR-030**: 사용자는 각 작업을 취소할 수 있어야 하며, 취소 결과와 남아 있는 부분
  결과를 확인할 수 있어야 한다.
- **FR-031**: 시스템은 background와 panel 표시 여부와 무관하게 Child run별 runtime
  controller를 하나만 유지하고, 해당 controller가 run binding, timeline cursor와 live
  event 구독을 소유하게 해야 한다.
- **FR-032**: Child 패널 승격 시 시스템은 Node에 저장된 기존 run ID와 실행 상태로
  panel view를 초기화하고, 빈 panel-local 초기 상태가 기존 workspace binding을
  덮어쓰지 못하게 해야 한다.
- **FR-033**: 시스템은 승격 또는 frontend remount 시 runtime journal snapshot을 기존
  timeline reducer에 적용하고, snapshot의 마지막 sequence 이후 live event만 수신하여
  중복과 누락을 방지해야 한다.
- **FR-034**: runtime journal gap, runtime 유실 또는 event가 아직 없는 상태를 서로
  구분해 표시하고, 사용자가 task 상태와 ACP timeline의 신뢰 범위를 판단할 수 있게
  해야 한다.
- **FR-035**: 사용자 입력 응답, Main의 후속 메시지, interrupt와 cancel은 동일한
  application command delivery 경로를 통해 현재 Child run에 전달되어야 한다.
- **FR-036**: 시스템은 Child 대상 command를 request ID, task, node, run, attempt,
  payload와 delivery 상태를 포함해 내구성 있게 기록해야 한다.
- **FR-037**: 사용자 입력 응답은 active Child가 전달을 수락한 경우에만 task를
  `inputRequired`에서 `running`으로 전이해야 하며, 전달 실패 시 입력 대기 상태와
  사용자 응답을 보존해야 한다.
- **FR-038**: Child report 저장은 active Main Coordinator에 대한 내구성 있는
  notification을 생성해야 하며, Main이 busy이면 queue하고 unavailable이면 다음
  generation의 명시적 handoff 전까지 pending 상태로 보존해야 한다.
- **FR-039**: Main notification은 report 전체를 자연어로 복제하지 않고 workspace,
  task, report ID와 종류를 전달하며, Main은 권한 검증된 collect/get 경로로 원문을
  조회해야 한다.
- **FR-040**: UI command, Coordinator MCP command와 복구 재전송은 동일한 worker port와
  idempotency 규칙을 사용해야 하며 서로 다른 상태 전이를 구현해서는 안 된다.
- **FR-041**: retry와 reassign은 새 attempt의 worker를 실제로 scheduling·launch해야
  하며, cancel은 active worker cancel 결과와 task terminal 상태를 일관되게 조정해야 한다.
- **FR-042**: 시스템은 command의 `pending`, `dispatching`, `accepted`, `failed`,
  `cancelled` 상태와 Coordinator notification의 전달 상태를 Activity Rail에서 확인할
  수 있게 해야 한다.

### Key Entities *(include if feature involves data)*

- **Agent Node**: 한 창 안의 Main 또는 직접 자식을 나타낸다. 안정적인 신원, 역할,
  부모, 현재 실행과 표시 상태를 가진다.
- **Coordinator Generation**: Main의 한 실행 기간과 책임 범위를 나타낸다. Main 재시작
  전후의 작업 소유권을 구분한다.
- **Orchestration Task**: 사용자 목표 또는 Main이 분해한 작업을 나타낸다. 부모 작업,
  담당 자식, 역할, 목적, 제약, 상태와 의존성을 가진다.
- **Task Report**: 자식이 부모에게 보내는 진행, 결과, 입력 요청 또는 차단 보고를
  나타낸다. 작성자, 대상 작업, 시점과 구조화된 내용을 가진다.
- **Prompt Dispatch**: 공용 작성 영역에서 발생한 한 번의 전송을 나타낸다. 대상 모드,
  대상별 요청과 결과를 가진다.
- **Artifact Reference**: 작업이 찾거나 만든 결과물의 위치와 설명을 나타내며 해당
  작업과 결과 보고에 연결된다.
- **Promotion Policy**: 백그라운드 작업을 언제 주의 대상으로 표시하거나 패널로 열지
  결정하는 사용자 선택을 나타낸다.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: 범위는 `apps/agentic-workbench`의 Worktree Session과 에이전트
  런 도메인이다. 다른 앱과의 공유는 현재 필요하지 않으며, 공급자 독립적인 순수 모델만
  실제 재사용 필요가 확인될 때 `packages/*`로 분리한다.
- **Frontend layering**: 화면 조립과 선택 상태는 `app/pages`, 통합 프롬프트·승격·작업
  제어는 `features`, 에이전트·작업·전송 모델과 어댑터는 `entities`, 범용 UI 원시는
  `shared` 또는 `components/ui` 경계를 따른다.
- **Backend boundary**: 순수 관계와 상태 규칙은 `domain`, 목표 분해 이후의 작업 생명주기와
  권한 적용은 `application`, 화면 및 에이전트 요청 진입점은 `inbound`, 실행·저장·이벤트
  연동은 `infrastructure`에 둔다. 명령 진입점은 저장 세부사항을 직접 다루지 않는다.
- **Shared core vs UI**: 작업 그래프와 상태 전이는 UI에 의존하지 않는 순수 코어로
  정의한다. 통합 Composer와 Activity Rail은 AW의 사용 흐름에 특화되어 교차 앱 공유
  대상으로 간주하지 않는다.
- **Persistence and safety**: 작업, 결과, Coordinator 세대와 중복 방지 정보는 복구 가능한
  경계 뒤에서 관리한다. 모든 요청은 창, worktree, 부모, 현재 실행과 작업 소유권을
  검증하며 첫 버전의 자동 자식은 읽기 전용으로 제한한다.
- **Documentation and Storybook**: `docs/agent-orchestration-workspace.md`를 유지하고, 통합
  Composer, Activity Rail, 작업 상태 표시와 승격 UI를 재사용 가능한 단위로 분리해
  atomic design 범주에 맞는 Storybook 사례를 추가한다.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 사용자는 하나의 목표를 제출한 뒤 30초 이내에 세 역할의 직접 자식 작업이
  생성·배정되었음을 확인할 수 있다.
- **SC-002**: 자동 또는 수동으로 생성된 추가 에이전트의 100%가 해당 창의 Main 직접
  자식으로 귀속되고, 다른 창의 Main 또는 작업에는 귀속되지 않는다.
- **SC-003**: 세 자식이 참여하는 대표 협업 시나리오에서 Main은 세 결과의 역할과 출처를
  모두 구분하고 누락 없이 하나의 최종 응답으로 제공한다.
- **SC-004**: 작업 상태 또는 최근 활동이 바뀌면 사용자는 1초 이내에 작업 목록에서
  변경을 확인할 수 있다.
- **SC-005**: 사용자는 실행 중인 작업을 2번 이하의 동작으로 패널에 열거나
  백그라운드로 내릴 수 있고, 이 과정에서 실행과 대화가 유지된다.
- **SC-006**: 현재·선택·전체·Coordinator 대상 모드에 대한 검증 시나리오의 100%에서
  의도한 대상만 명령 또는 목표를 정확히 한 번 받는다.
- **SC-007**: 다중 대상 중 하나가 실패하는 모든 검증 시나리오에서 성공한 대상의 결과는
  유지되고, 실패 대상과 이유가 개별 표시된다.
- **SC-008**: 입력 요청, 실패, 취소와 Main 교체 검증 시나리오의 100%에서 작업과 부분
  결과가 유실되지 않고 사용 가능한 다음 동작이 표시된다.
- **SC-009**: Main 교체 검증 시나리오의 100%에서 사용자의 명시적 선택 없이 진행 중
  작업이 새 Main에 자동 귀속되지 않는다.
- **SC-010**: 기존 탭·타일 보기, 타일 진입 시 균등 비율, 패널 추가·닫기와 패널 간
  명시적 메시지 교환의 기존 검증 시나리오가 모두 통과한다.
- **SC-011**: background에서 하나 이상의 ACP event를 생성한 Child를 승격하는 검증
  시나리오의 100%에서 같은 run ID가 유지되고, 승격 전 event와 승격 후 live event가
  정확히 한 번씩 timeline에 표시된다.
- **SC-012**: event 없음, journal gap과 runtime lost 세 시나리오의 100%에서 서로 다른
  상태 안내가 표시되고 빈 timeline이 task 완료나 agent 무응답으로 오인되지 않는다.
- **SC-013**: Child 입력 요청에 대한 사용자 응답 검증 시나리오의 100%에서 동일 run이
  응답을 정확히 한 번 수신하며, 전달 실패 시 task가 잘못 `running`으로 전이하지 않는다.
- **SC-014**: progress, result, input request와 blocked report 검증 시나리오의 100%에서
  active Main이 1초 이내 notification을 받고 report ID로 원문을 조회할 수 있다.
- **SC-015**: UI와 Coordinator MCP에서 동일한 send, cancel, retry, reassign 시나리오를
  실행했을 때 task, worker와 delivery 상태 결과가 모두 동일하다.

## Assumptions

- 한 Worktree Session 창에는 기존과 같이 안정적인 Main 패널이 항상 하나 존재한다.
- 첫 버전의 자식 계층은 Main과 직접 자식의 1단계 구조로 충분하다.
- 역할 기반 병렬 조사, 검토와 검증이 첫 번째 대표 사용 사례다.
- 기본 승격 정책은 입력 필요 또는 차단 시 주의 표시 후 사용자가 여는 방식이다.
- 기존 탭·타일 레이아웃과 동일 창 내부의 메시지 교환 기능을 재사용할 수 있다.
- 첫 버전은 읽기 전용 병렬 작업에 집중하며 쓰기 작업은 단일 담당자 또는 후속 격리
  전략이 마련된 뒤 확장한다.
- 공급자 고유의 하위 에이전트 기능은 후속 호환 계층으로 고려하되 사용자에게 보이는
  관계, 작업과 상태 의미는 AW가 일관되게 유지한다.
