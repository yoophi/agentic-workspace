# Research: Main Coordinator 기반 에이전트 오케스트레이션

## 1. 오케스트레이션의 기준 단위

**Decision**: 실행 세션이 아니라 `OrchestrationTask`를 제어 평면의 기준 단위로 삼고,
Agent Node와 실제 run을 작업에 연결한다.

**Rationale**: run은 재시작되거나 교체될 수 있지만 사용자가 맡긴 작업, 진행 보고와
부분 결과는 유지되어야 한다. OpenAI의 Symphony도 세션과 결과물을 분리하고 작업 상태를
state machine 및 DAG로 운용한다. 현재 AW도 안정적인 `panelId`와 일시적인 `runId`가
이미 분리되어 있으므로 작업 중심 모델을 추가하기에 적합하다.

**Alternatives considered**:

- run을 작업으로 간주: 구현은 단순하지만 재시작, 재배정과 Main 교체 때 결과가 끊긴다.
- 패널을 작업으로 간주: 한 패널에서 여러 작업을 순차 실행하거나 백그라운드 작업을
  표시하지 않는 경우를 표현하지 못한다.
- 파일 생성 여부를 작업으로 간주: 조사·검토처럼 파일을 만들지 않는 작업과 부분 실패를
  표현하지 못한다.

Reference: [OpenAI Symphony orchestration](https://openai.com/index/open-source-codex-orchestration-symphony/)

## 2. 기존 exchange와 오케스트레이션 저장소의 경계

**Decision**: `InMemoryAgentWorkspaceRegistry`와 `AgentExchangeService`는 동일 창의 저수준
메시지 전달에 그대로 사용한다. 작업, Agent Node, Coordinator Generation, 보고,
중복 방지는 별도의 AW app-local `OrchestrationRepository`와 application service가
소유한다.

**Rationale**: 기존 exchange의 `delivered`는 대상 패널이 prompt를 받았다는 뜻이지
작업 성공이 아니다. 또한 window destroy 때 기록이 제거되므로 Main 재시작과 결과 보존의
source of truth가 될 수 없다. 두 모델을 분리하면 기존 탭·타일 메시지 계약을 깨지 않고
작업 생명주기를 엄격하게 정의할 수 있다.

**Alternatives considered**:

- exchange 상태를 task 상태로 확장: 전달과 실행 의미가 섞이고 기존 상태 전이의
  호환성이 깨진다.
- 기존 in-memory registry에 모든 작업을 추가: Main 재시작과 앱 오류 뒤 복구가
  불가능하다.
- `acp-agent-core`에 AW task를 추가: Hushline 등 공유 crate 소비자에게 AW UI 개념이
  유입된다.

## 3. 영속성 모델

**Decision**: app data의 `orchestration-sessions.json`에 session aggregate snapshot을
원자적으로 저장하고, 실행 중 조회와 wait notification은 메모리 projection으로
제공한다. 기존 `json_store`의 임시 파일, backup, rename 복구 방식을 재사용한다.

**Rationale**: 현재 저장소는 데이터베이스 없이 JSON repository 패턴을 사용하며,
작업 규모는 창당 최대 8개로 작다. aggregate 단위 revision과 atomic write는
Main 재시작, 중복 요청과 부분 결과를 보존하기에 충분하며 새 데이터베이스 의존성을
도입하지 않는다.

각 Worktree Session은 canonical worktree path와 별도의 안정적인
`orchestrationSessionId`를 가진다. 같은 worktree의 두 창은 서로 다른 session ID를
사용하고 현재 window label binding을 별도로 검증한다. 앱 또는 창 종료로 live run이
사라진 비종료 작업은 자동 완료하지 않고 다음 조회에서 차단 상태와 복구 선택지를
제공한다.

**Alternatives considered**:

- canonical path만 저장 key로 사용: 같은 worktree의 두 창이 같은 작업 집합을 공유해
  격리가 깨진다.
- SQLite/event store 도입: 현재 규모와 저장소 관례에 비해 과도하다.
- 영구 저장 없음: 명시된 Main 교체와 부분 결과 보존 요구를 충족하지 못한다.

## 4. 부모·자식 토폴로지와 신원

**Decision**: 기존 `main-agent-run`을 안정적인 Main Agent Node로 사용하고 모든 추가
Node의 `parentNodeId`를 Main으로 강제한다. 첫 버전은 손자를 금지한다. 패널 관계,
run binding과 task 관계는 독립적으로 저장한다.

**Rationale**: 별 모양 구조는 권한, 결과 수집과 실패 격리를 단순화한다. 사용자가 만든
기존 extra 패널도 같은 직접 자식으로 승격할 수 있고, Main run이 교체되어도 안정적인
부모 Node는 유지된다.

**Alternatives considered**:

- 재귀 트리: 유연하지만 권한 전파, 순환 방지와 UI 표현 범위가 크게 늘어난다.
- 형제 직접 통신: 빠르지만 결과 출처와 책임이 분산되고 자동 대화 루프 위험이 있다.
- Main run ID를 부모 ID로 사용: Main 재시작 때 모든 자식 관계가 끊긴다.

## 5. 첫 Worker 실행 어댑터

**Decision**: backend application service가 사용하는 AW app-local `AgentWorkerPort`를
정의하고, 첫 구현은 기존 `StartAgentRunUseCase`, `SendPromptUseCase`,
`CancelAgentRunUseCase`를 감싸는 `AcpAgentWorkerAdapter`로 한다. task와 planned run
binding을 먼저 저장한 뒤 repository lock을 놓고 backend에서 process를 시작한다.

`OrchestrationAwareRunEventSink`는 기존 Tauri event 전달과 함께 bounded runtime event
journal에 run별 sequence를 기록한다. frontend는 `AgentRunPanel`의 prompt/run/timeline
controller를 표시 UI와 분리하고, 백그라운드 Node도 controller를 유지한다. 늦게 mount된
controller는 journal snapshot 이후 live event를 이어 받아 같은 대화를 복원한다.
승격은 기존 controller를 tile leaf에 투영하고, 분리는 leaf만 제거한다.

**Rationale**: process 시작과 취소를 application layer가 소유해야 scheduler,
동시 실행 제한, 창 종료와 launch failure를 일관되게 처리할 수 있다. 동시에 현재
`AgentRunPanel`의 풍부한 timeline reducer를 controller로 재사용하고 짧은 event journal을
두면 headless Worker를 패널로 승격할 때 대화를 잃지 않는다. 구조화된 task report는
durable JSON에 남고, 전체 runtime journal은 active window 수명에만 유지하면 된다.

**Alternatives considered**:

- frontend event로 launch를 요청하고 panel이 process를 시작: 기존 UI 재사용은 쉽지만
  scheduler와 process 시작의 성공 여부가 두 경계에 분산된다.
- 패널을 닫을 때 컴포넌트 제거 후 run만 유지: 재승격 시 대화와 제어 상태가 유실된다.
- 공급자 CLI process를 별도 감시: 기존 ACP lifecycle, permission과 session resume를
  우회한다.

## 5.1 Runtime controller 소유권과 패널 재수화

**Decision**: Child의 runtime controller는 panel component가 아니라 Worktree Session
workspace가 Node/run별로 하나씩 소유한다. panel은 controller의 view projection이며,
승격 시 `Node.currentRunId`로 기존 controller에 결합한다. controller가 메모리에 없으면
동일 run ID로 만들고 runtime journal의 sequence 0부터 snapshot을 적용한 뒤,
`lastSequence` 이후 live event를 구독한다.

panel mount에서 생성되는 빈 `activeRunId`, `isRunning` 또는 timeline은 authoritative
workspace binding을 갱신할 수 없다. panel에서 상위로 보내는 run state change는
controller hydration이 끝난 뒤의 사용자 실행 변경이나 terminal event에만 사용한다.
background observer도 journal cursor만 소비하지 않고 같은 controller에 snapshot을
적용해야 한다.

**Rationale**: 현재 vertical slice는 promotion slot에 기존 run ID를 넣지만
`AgentRunPanel`이 이를 입력으로 받지 않고 `activeRunId=null`로 시작한다. 따라서 live
event listener가 기존 Child event를 모두 거부하고 빈 panel state가 상위 binding을
덮어쓸 수 있다. 또한 background runtime host는 replay snapshot의 cursor만 저장하고
event payload를 폐기하므로 panel 승격 뒤 과거 timeline을 재구성할 수 없다. 안정적인
controller ownership과 snapshot/live handoff를 하나의 경계에서 처리해야 동일 run과
대화를 보장할 수 있다.

**Alternatives considered**:

- panel에 initial run ID만 전달: 새 live event는 받을 수 있지만 승격 전 timeline과
  controller 상태가 복원되지 않는다.
- 승격 때 run을 다시 시작: task identity, 비용과 대화 연속성이 깨지고 중복 실행된다.
- panel과 background host가 각각 journal을 소비: cursor와 reducer가 분리되어 event
  중복·누락 및 서로 다른 timeline을 만들 수 있다.

## 6. 완료 판정과 결과 수집

**Decision**: 자식의 `report_task_result`를 작업 완료의 primary signal로 사용한다.
구조화된 결과는 summary, findings, artifact references, unresolved items와 confidence를
포함한다. ACP `PromptCompleted`는 결과 보고 누락을 감지하는 보조 신호이며, process
exit나 파일 watcher는 실패·중단 감지에만 사용한다.

**Rationale**: 명시적 보고만이 작업 ID, 결과 출처와 완료 의미를 확실히 연결한다.
Prompt 완료는 대화 turn 완료일 뿐 작업 목표 완료와 같지 않을 수 있고, 파일 존재는
성공 품질을 증명하지 않는다.

**Alternatives considered**:

- `PromptCompleted` 즉시 완료: 결과가 없거나 추가 질의가 필요한 작업을 오판한다.
- process exit code 0을 완료로 사용: 장기 세션과 resume 모델에 맞지 않는다.
- 특정 파일 생성 감시: 비파일 작업을 지원하지 못하고 stale artifact를 오인할 수 있다.

## 6.1 양방향 Child command와 Main notification

**Decision**: 사용자와 Main이 Child에 보내는 message, input response, interrupt와 cancel을
하나의 backend application use case로 통합한다. use case는 current task attempt와
`WorkerBinding`을 서버에서 해석하고 durable `TaskCommand`를 먼저 저장한 뒤 repository
lock 밖에서 `AgentWorkerPort`를 호출한다. 입력 응답은 worker가 수락한 뒤에만
`inputRequired → running`으로 전이한다.

Child report는 기존 durable 저장과 UI event 외에 `CoordinatorNotification`을 같은
transaction에서 생성한다. notification dispatcher는 active generation의 Main run에
report ID와 종류만 queue하고, Main은 collect/get 도구로 원문을 조회한다. Main이
unavailable이면 notification은 pending으로 남으며 generation handoff 없이 새 Main에
자동 전달하지 않는다.

**Rationale**: 현재 Main MCP의 `aw_send_child_message`만 실제 worker port를 호출한다.
Activity Rail의 `respond_to_input`은 응답을 검증한 뒤 `let _ = response`로 버리고 task
상태만 running으로 바꾼다. Child report도 repository와 UI event만 갱신하고 active Main
run을 깨우지 않는다. UI와 MCP가 서로 다른 path를 가지면 delivery 성공과 task 상태가
계속 어긋난다.

**Alternatives considered**:

- panel-local prompt routing: detached/background Child와 remount에서 동작하지 않는다.
- React가 `send_prompt_to_run` 직접 호출: task/node/run/generation authorization과
  durable idempotency를 우회한다.
- 모든 progress를 즉시 Main prompt로 전송: token과 turn interruption이 과도하다.
- Main polling만 유지: input request와 completion을 적시에 알 수 없고 사용자 경험이
  runtime polling 빈도에 의존한다.

## 6.2 Command outbox, idempotency와 attempt fencing

**Decision**: `TaskCommand`와 `CoordinatorNotification`을 aggregate의 durable outbox로
사용한다. idempotency는 revision 검사보다 먼저 actor, operation, request ID와 정규화된
전체 payload fingerprint를 비교한다. command/report는 task attempt와 run ID에 묶고,
retry/reassign 뒤 이전 run의 늦은 command/report는 현재 상태를 변경하지 못한다.

retry/reassign/cancel/respond는 UI와 MCP 모두 같은 application orchestration command
service를 사용한다. retry는 attempt 증가 후 scheduler와 실제 worker launch까지,
reassign은 이전 worker cancel/capability fence와 ownership 저장 후 새 worker launch까지
완료해야 accepted다.

**Rationale**: 현재 task action idempotency와 report 비교는 message 전체, findings,
artifact, unresolved, confidence 등 일부 payload를 포함하지 않아 같은 request ID의 다른
요청을 기존 결과로 오인할 수 있다. 또한 UI action은 상태만 바꾸고 MCP action 일부만
worker adapter를 호출하며 retry/reassign은 새 실행을 시작하지 않는다.

**Alternatives considered**:

- state 저장 후 즉시 best-effort send: crash와 전송 실패 뒤 false running/accepted가 남는다.
- runtime send 후에만 저장: send 성공 직후 crash하면 중복 전송을 막을 기록이 없다.
- workspace revision만으로 concurrency 제어: 무관한 report/UI 변경 때문에 안전한 네트워크
  retry가 revision conflict로 실패한다.

## 7. MCP 호출자 인증과 역할 권한

**Decision**: 앱 공용 bearer token 대신 run별 opaque capability token을 발급한다.
MCP adapter는 token에서 source run을 서버 측으로 해석하고, 현재 workspace snapshot과
orchestration repository에서 window, Node, actor kind, task와 Coordinator Generation을
도출한다. 새 도구는 호출자가 넘긴 actor ID를 신뢰하지 않는다.

기존 exchange 도구의 `runId` 인자는 호환성을 위해 유지할 수 있지만 capability가
가리키는 run과 일치해야 한다. run 종료 시 capability는 더 이상 active owner를
통과하지 못하고 제거된다.

**Rationale**: 현재 토큰은 앱 전체에서 공유되고 도구 인자의 `runId`로 source를
결정한다. 자식이 알려진 Main run ID를 제출하면 Coordinator로 가장할 수 있으므로
Coordinator-only 생성·취소 권한을 안전하게 강제할 수 없다. 실행별 capability는
transport 인증과 application 권한 검사를 연결한다.

**Alternatives considered**:

- 기존 공용 token + caller `runId`: 역할 위조를 막지 못한다.
- 자연어 지침으로 자식 spawn 금지: 악성 또는 오류 호출에 대한 기술적 경계가 아니다.
- 도구별 고정 token: 역할 변경과 Main generation 교체를 유연하게 검증하기 어렵다.

안전 원칙은 low-risk 작업을 bounded environment에서 실행하고 고위험 동작을 명시적
검토로 올리는 방향과 일치한다.

Reference: [Running Codex safely at OpenAI](https://openai.com/index/running-codex-safely/)

## 8. MCP 도구 표면

**Decision**: Coordinator와 Child 도구를 역할별 allow-list로 분리한다.

Coordinator-only:

- `create_child_task`
- `assign_child_task`
- `list_child_tasks`
- `send_child_message`
- `wait_for_child_tasks`
- `collect_child_results`
- `interrupt_child_task`
- `cancel_child_task`

Child-only:

- `get_assigned_task`
- `report_task_progress`
- `report_task_result`
- `request_parent_input`
- `send_parent_message`

`wait_for_child_tasks`는 bounded timeout 후 현재 snapshot을 반환하며 timeout 자체는 task
상태를 바꾸지 않는다. 패널 승격은 초점을 바꾸는 사용자 UI 동작이므로 agent가 직접
실행하지 않고 attention request만 만들 수 있다.

**Rationale**: 역할별 최소 권한은 자식의 recursive spawn과 형제 직접 제어를 기술적으로
차단한다. wait/collect를 분리하면 Main은 장기 요청을 polling하면서도 이미 끝난 결과를
구조화해 받을 수 있다.

**Alternatives considered**:

- 하나의 범용 `send` 도구: task lifecycle과 결과 형식이 자연어에 묻힌다.
- 모든 도구를 모든 run에 노출: 자식이 생성·취소 권한을 행사할 수 있다.
- 무기한 blocking wait: MCP 연결과 Main turn이 복구 불가능하게 대기할 수 있다.

## 9. 통합 Composer와 dispatch

**Decision**: workspace에 표시되는 Composer는 하나만 두고 `focused`, `selected`, `all`,
`coordinator` target mode를 제공한다. `coordinator`는 `intent=delegate`, 나머지는
`intent=direct`다. dispatch는 하나의 `dispatchId`와 대상별 request/result를 가진다.

패널별 draft, queue와 실행 프로필은 runtime controller가 유지하고, Composer는 선택한
대상 controller에 명령을 보낸다. 비초점 패널의 incoming draft는 panel별 draft slot에
보관하고 해당 패널이 초점이 될 때 같은 단일 Composer에 표시한다.

**Rationale**: 단일 작성 surface 요구를 만족하면서도 기존 panel별 queue와 설정을
잃지 않는다. batch envelope와 target result를 분리하면 partial failure를 rollback 없이
정확히 표시할 수 있다.

**Alternatives considered**:

- 각 Panel Composer를 숨기고 새 Composer를 별도로 구현: 상태가 이중화된다.
- `all`과 `coordinator`를 같은 broadcast로 처리: 목표 분해와 단순 명령 전송의 의미가
  섞인다.
- 첫 실패에서 전체 dispatch 취소: 이미 수신한 target을 되돌릴 수 없고 결과가 불명확하다.

## 10. 작업·실행·표시 상태 분리

**Decision**: task, execution, presentation 세 상태 축을 독립적인 state machine으로
유지한다. layout에는 `presentation=panel` Node만 나타나고 Activity Rail에는 모든 task가
나타난다. `detach`는 presentation만 바꾸며 `cancel`은 task/application 동작이다.

**Rationale**: 현재 running extra close는 run을 취소하고 slot을 제거하므로 새 요구와
충돌한다. 세 상태를 분리하면 실행 중 background, 완료된 panel, 입력 대기 background
같은 조합을 명시적으로 표현할 수 있다.

**Alternatives considered**:

- panel open/closed를 task 상태에 포함: 패널을 닫는 순간 작업 의미가 바뀐다.
- running 여부 하나만 사용: 입력 대기, 차단, 재시도와 승격 중을 표현하지 못한다.
- attention 시 자동 focus: 사용자의 현재 작업을 빼앗는다.

## 11. 읽기 전용 안전 정책

**Decision**: 자동 생성 Worker는 `WorkerAccessPolicy::ReadOnly`만 허용한다. adapter는
읽기 전용 capability를 명시적으로 지원하는 agent profile만 선택하고,
`permissionMode=readOnly`, `autoAllow=false`로 시작한다. mutation permission은
거부하고 worktree change 관측 시 worker를 중단해 `readOnlyViolation`으로 표시한다.
자동 되돌리기는 하지 않는다.

**Rationale**: 현재 provider별 read-only 설정은 가장 중요한 1차 경계지만 모든 provider의
강제력이 같다고 가정할 수 없다. 지원 가능 profile 제한과 사후 change detector를 함께
사용해야 동일 worktree 병렬 실행에서 쓰기를 감지하고 확산을 막을 수 있다.

**Alternatives considered**:

- 경고 문구만 표시: 동시 쓰기를 기술적으로 막지 못한다.
- 변경을 자동 revert: 사용자 또는 다른 run의 기존 변경을 파괴할 수 있다.
- 첫 버전부터 worktree 복제: 가장 강한 격리지만 조사·검토 MVP에 비해 수명주기와
  merge 정책 범위가 크다.

## 12. 동시 실행과 스케줄링

**Decision**: task는 dependency가 끝나고 coordinator generation이 유효하며 worker
capacity가 있을 때 `ready → running`으로 전이한다. 초과 작업은 생성 순서가 안정적인
FIFO ready queue에서 대기한다. 기존 `ACP_MAX_RUNS`/`ACP_WORKBENCH_MAX_RUNS` limit을
최종 hard limit으로 존중한다.

대표 시나리오는 Main + Researcher + Reviewer + Tester의 active run 4개다. 환경 limit이
4보다 작으면 일부 자식은 queued 상태로 남지만 생성·배정 결과는 즉시 보인다.

**Rationale**: 기존 SessionRegistry 제한과 경쟁하지 않고 application scheduler가
사용자에게 대기 이유를 설명할 수 있다. dependency 기반 ready 조건은 후속 DAG 확장에도
적합하다.

**Alternatives considered**:

- limit 오류를 그대로 사용자에게 반환: retry 순서와 작업 상태가 유실된다.
- 무제한 launch: process와 token 사용량을 제어할 수 없다.
- child가 임의로 새 child를 생성: v1의 직접 자식 제한과 capacity 정책을 우회한다.

## 13. Main 재시작과 명시적 인계

**Decision**: Main run마다 `CoordinatorGeneration`을 만들고, 종료 시 그 generation의
비종료 task를 `awaitingHandoff`로 표시한다. 새 Main은 task summary와 부분 결과를 본 뒤
`accept`, `cancelOutstanding`, `leaveUnassigned` 중 하나를 revision-checked command로
선택한다.

**Rationale**: 모든 과거 대화를 새 Main prompt에 자동 주입하면 context가 커지고 잘못된
책임 승계가 숨겨진다. generation과 explicit handoff는 사용자가 소유권 변경을 확인하고
stale Main의 후속 명령을 차단하게 한다.

**Alternatives considered**:

- panel identity가 같으므로 자동 승계: stale run과 새 run의 권한을 구분하지 못한다.
- Main 종료 시 모든 child 취소: 성공한 부분 결과와 장기 작업을 불필요하게 잃는다.
- 전체 transcript 자동 복사: 비용이 크고 task 중심 요약보다 관련성이 낮다.

## 14. 공급자 고유 오케스트레이션과 후속 확장

**Decision**: v1의 관계·작업·상태는 AW가 소유하고 기존 ACP 실행으로 공급자 중립적으로
구현한다. provider-native subagent/team/App Server 연동은 동일
`OrchestrationWorkerAdapter` 뒤의 후속 adapter로 둔다.

**Rationale**: Claude Code는 subagent, background task, agent view/team과 worktree
격리를 서로 다른 협업 수준으로 나누고 있으며, Codex App Server는 장기 process,
thread lifecycle/persistence와 bidirectional JSON-RPC를 분리한다. 공통점은 task
supervision과 실행 transport를 분리한다는 것이다. AW의 UI와 저장 모델을 특정 공급자
버전에 결합하지 않고 이 패턴을 활용할 수 있다.

**Alternatives considered**:

- Claude/Codex native 기능을 곧바로 UI source of truth로 사용: 공급자마다 상태와 권한
  의미가 달라 동일 워크스페이스 경험을 제공하기 어렵다.
- 외부 CLI process/file watcher만 사용: 구조화된 진행, 결과와 입력 요청이 약하다.
- 모든 provider 기능의 최소 공통분모만 지원: native 장점을 후속 adapter로 확장하기
  어렵다.

References:

- [Claude Code parallel agents](https://code.claude.com/docs/en/agents)
- [Claude Code subagents](https://code.claude.com/docs/en/sub-agents)
- [Codex App Server architecture](https://openai.com/index/unlocking-the-codex-harness/)
