# Quickstart Validation: Main Coordinator 기반 에이전트 오케스트레이션

## 목적

이 가이드는 구현 완료 후 Main Coordinator, direct Child, 단일 Composer,
background Activity Rail, 명시적 결과 수집과 안전 복구가 end-to-end로 동작하는지
검증한다. 세부 상태와 payload는 [data-model.md](./data-model.md)와
[contracts](./contracts/)를 참조한다.

## Prerequisites

```sh
corepack enable
pnpm install
```

Node.js, pnpm 9.10.0, Rust toolchain과 macOS Tauri 개발 의존성이 필요하다.

## 구현 전 회귀 기준

2026-07-27에 기존 tab/tile workspace와 agent exchange 경로를 기준으로 다음 검증을
수행했다.

- frontend: workspace, equal tile layout, tile UI, worktree area, exchange repository의
  5개 test file과 14개 test가 모두 통과했다.
- backend: `cargo test -p agentic-workbench agent_exchange`의 관련 6개 test가 모두
  통과했다.
- 구현 후에도 tab/tile 전환 시 균등 비율, 최대 panel/depth 제한, run owner scope와
  exchange 상태 전이를 같은 기준으로 회귀 검증한다.

## Deterministic smoke Worker

구현 단계에서 다음 fixture를 추가한다.

- `apps/agentic-workbench/scripts/acp-orchestration-smoke-agent.mjs`
- `apps/agentic-workbench/scripts/acp-orchestration-smoke-agents.json`

Smoke Worker 역할:

- Researcher: progress 2회 후 성공 result
- Reviewer: 사용자 input request 후 응답을 받아 성공 result
- Tester: 선택 가능한 성공 또는 실패 result
- mutation profile: 읽기 전용 위반 시도
- 각 report에 task ID와 결정적인 fixture 내용을 사용

개발 앱 실행:

```sh
ACP_AGENT_CATALOG_PATH="$PWD/apps/agentic-workbench/scripts/acp-orchestration-smoke-agents.json" \
ACP_WORKBENCH_MAX_RUNS=4 \
pnpm run tauri:dev:workbench
```

환경 limit이 4보다 작으면 Main과 세 Child가 동시에 active가 될 수 없으므로 일부 Child가
ready queue에 남는 동작을 별도 검증한다.

## Scenario 1: Bootstrap과 Main 관계

1. 프로젝트의 Worktree Session 창을 연다.
2. Main run을 시작한다.
3. Activity Rail과 Main 표시를 확인한다.
4. 추가 수동 패널을 하나 만든다.

Expected:

- Main Node는 `main-agent-run` 하나뿐이고 닫기 동작이 없다.
- Main run에 active Coordinator Generation이 하나 생긴다.
- 수동 extra는 Main의 direct Child이며 초기 task는 unassigned다.
- 같은 worktree를 두 번째 창에서 열어도 다른 workspace ID를 사용한다.

## Scenario 2: Researcher·Reviewer·Tester 병렬 협업

1. Composer target을 `Coordinator`로 선택한다.
2. “현재 구조를 조사하고 위험을 검토한 뒤 검증 시나리오를 만들어 종합해 주세요”를
   제출한다.
3. Main이 Researcher, Reviewer, Tester task를 만드는 것을 관찰한다.
4. 세 Child가 background에서 실행되는 동안 Activity Rail을 확인한다.
5. 모두 끝난 뒤 Main의 최종 응답을 확인한다.

Expected:

- 목표 submit 후 30초 이내에 역할, 목적, 제약과 예상 결과가 있는 direct Child 3개가
  생성·배정된다.
- Child의 parent는 모두 Main이며 grandchild는 없다.
- 가능한 capacity 안에서 병렬 시작되고 progress가 1초 이내 Rail에 보인다.
- Main은 세 결과의 역할/출처와 불일치를 구분해 종합한다.
- task 완료에는 각 Child의 명시적 result report가 존재한다.

## Scenario 3: 단일 Composer target modes

1. Main, Researcher, Reviewer panel을 연다.
2. `focused`로 Reviewer에 명령을 보낸다.
3. Researcher와 Reviewer를 선택하고 `selected`로 명령을 보낸다.
4. `all`로 명령을 보낸다.
5. `coordinator`로 새 목표를 위임한다.

Expected:

- 화면에 Composer textarea는 하나만 있다.
- focused는 초점 panel 하나, selected는 두 panel, all은 Main 포함 open panel 전체만
  정확히 한 번 받는다.
- coordinator는 broadcast가 아니라 durable root task와 delegate dispatch를 만든다.
- focus와 selected 집합은 서로 독립적이다.
- target별 accepted/delivered/rejected/failed 상태가 표시된다.

## Scenario 4: Partial dispatch failure와 idempotency

1. 두 target을 선택한다.
2. submit 직전에 한 target의 run을 종료하거나 panel을 closing 상태로 만든다.
3. 같은 dispatch를 보낸다.
4. 동일 dispatch ID/동일 payload와 다른 payload로 각각 재시도한다.

Expected:

- 정상 target은 delivered되고 실패 target만 typed failure를 가진다.
- 성공 결과를 rollback하지 않는다.
- 동일 ID/동일 payload는 다시 전달하지 않고 기존 결과를 반환한다.
- 동일 ID/다른 payload는 `duplicateConflict`다.
- 실패 target만 새 request ID로 재시도할 수 있다.

## Scenario 5: Background 승격과 분리

1. 실행 중인 Reviewer background task의 `패널로 열기`를 선택한다.
2. Main 오른쪽에 열고 run ID, timeline, queue와 profile을 확인한다.
3. panel에서 `백그라운드에서 계속`을 선택한다.
4. 다시 아래쪽 panel로 승격한다.

Expected:

- 승격마다 같은 Node, task와 run ID를 사용한다.
- 과거 timeline과 live output이 중복/누락 없이 이어진다.
- 분리는 worker cancel을 호출하지 않는다.
- 명시적 open 외에는 현재 focus를 빼앗지 않는다.
- background Node는 탭→타일 균등 비율 계산에 포함되지 않는다.

## Scenario 5A: Background Child panel 재수화 회귀

1. 패널을 열지 않은 background Child를 실행한다.
2. Child가 lifecycle, thought 또는 message event를 최소 2개 생성할 때까지 기다린다.
3. Activity Rail에서 run ID와 task 상태를 기록한다.
4. `패널로 열기`를 실행한다.
5. 승격 직후 과거 event를 확인하고, Child에 후속 메시지를 보내 live event를 생성한다.
6. 패널을 분리한 뒤 background에서 event를 하나 더 생성하고 다시 승격한다.

Expected:

- 최초 승격과 재승격 모두 Activity Rail에서 기록한 같은 run ID를 사용한다.
- 승격 전 event, 승격 도중 event와 승격 후 event가 각각 정확히 한 번 표시된다.
- panel mount가 workspace의 active run ID를 `null`로 바꾸지 않는다.
- 승격은 새 process를 시작하거나 초기 prompt를 다시 보내지 않는다.
- panel과 background observer가 같은 마지막 sequence와 timeline을 공유한다.

오류 상태도 별도로 검증한다.

1. event를 아직 생성하지 않은 유효한 run을 승격한다.
2. bounded journal의 보존 범위를 넘겨 gap을 만든 뒤 승격한다.
3. durable task는 남아 있지만 runtime owner가 없는 fixture를 승격한다.

Expected:

- 유효하지만 event가 없는 경우 `ACP 이벤트 대기 중`과 연결된 run 정보를 표시한다.
- gap은 일부 과거 event를 복원할 수 없음을 알리고 durable report를 표시한다.
- runtime owner가 없으면 `runtimeLost`와 복구 동작을 표시한다.
- 세 상태 모두 동일한 `ACP 응답이 아직 없습니다` 빈 화면으로 축약되지 않는다.

## Scenario 6: Input request, failure와 부분 결과

1. Reviewer가 parent input을 요청하도록 smoke scenario를 실행한다.
2. Reviewer panel을 열지 않은 상태에서 attention을 확인한다.
3. Rail에서 답변을 보낸다.
4. Tester를 partial findings가 있는 실패로 실행한다.
5. Tester만 retry 또는 다른 Child로 reassign한다.

Expected:

- input request는 `inputRequired`와 `attentionRequired`로 표시되지만 focus는 유지된다.
- 사용자 응답 후 Reviewer가 같은 task/run에서 계속한다.
- Tester 실패가 Researcher/Reviewer 성공 결과를 제거하지 않는다.
- partial findings는 retry/reassign 뒤에도 보존된다.
- cancel은 별도 destructive action이다.

## Scenario 6A: 사용자·Main·Child 양방향 통신

1. Reviewer가 `aw_request_parent_input`으로 질문을 제출한다.
2. Main 패널을 idle 상태로 두고 Activity Rail에서 input request를 확인한다.
3. Main이 받은 notification에서 task/report ID를 확인하고 report 원문을 조회한다.
4. Activity Rail에서 `inputReportId`에 답변을 한 번 제출한다.
5. 같은 request ID/같은 답변과 같은 request ID/다른 답변을 각각 재시도한다.
6. Reviewer가 답변을 받은 뒤 progress와 result를 보고하게 한다.

Expected:

- input request, UI attention과 Main notification은 동일 report ID를 가리킨다.
- 사용자 답변은 동일 active Child run에 정확히 한 번 전달된다.
- worker가 답변을 수락한 뒤에만 task가 `running`이 된다.
- 동일 request/payload 재시도는 기존 outcome을 반환하고 다시 전송하지 않는다.
- 동일 request ID의 다른 payload는 `duplicateConflict`다.
- result report는 task를 완료하고 Main은 notification 뒤 collect로 같은 report를 읽는다.

실패와 race도 검증한다.

1. Child가 질문한 뒤 worker를 종료하고 사용자 답변을 제출한다.
2. input response와 cancel을 동시에 제출한다.
3. Child result 저장과 Main generation handoff를 동시에 실행한다.
4. retry/reassign 후 이전 run에서 늦은 progress/result를 제출한다.

Expected:

- worker unavailable이면 답변은 보존되고 task는 `inputRequired`를 유지한다.
- response/cancel 중 먼저 durable하게 확정된 결과를 따르고 중복 prompt를 보내지 않는다.
- report notification은 old/new Main 중 정확히 하나에만 귀속되며 유실되지 않는다.
- 이전 attempt/run report는 이력으로 보존되지만 현재 task 상태를 변경하지 않는다.

## Scenario 6B: UI와 Coordinator command parity

1. 동일 fixture에서 Main MCP와 Activity Rail/UI를 각각 사용해 message, cancel, retry,
   reassign을 실행한다.
2. 각 동작의 `TaskCommand`, worker 호출, task 상태와 새 run binding을 비교한다.
3. command 저장 직후와 worker 수락 직후 crash fixture를 실행해 복구한다.

Expected:

- UI와 MCP가 동일한 application command service와 worker port를 호출한다.
- message/cancel은 실제 Child runtime에 도달한 경우에만 accepted다.
- retry는 attempt를 증가시키고 capacity에 따라 queue하거나 새 worker를 launch한다.
- reassign은 이전 worker를 fence한 뒤 target Child에 새 worker를 launch한다.
- crash 복구 후 pending command는 재개되며 accepted command는 중복 전송되지 않는다.

## Scenario 7: Main restart와 generation handoff

1. 세 Child 중 하나 이상이 실행 중일 때 Main run을 종료한다.
2. 새 Main run을 시작한다.
3. handoff UI에서 아무 선택도 하지 않은 상태를 확인한다.
4. 일부 task는 인계하고 하나는 미할당으로 유지한다.

Expected:

- 새 Coordinator Generation이 생긴다.
- 이전 generation task는 awaiting handoff이며 자동 귀속되지 않는다.
- old Main capability는 새 task를 제어할 수 없다.
- 새 Main은 인계 승인 뒤에만 선택 task와 summary/partial result를 관리한다.
- 미할당 task는 유실되거나 임의 취소되지 않는다.

## Scenario 8: Window/worktree 격리와 caller spoofing

1. 같은 worktree의 두 번째 Worktree Session 창을 연다.
2. 다른 worktree 창도 연다.
3. 첫 창의 task/node/run ID로 다른 창에서 command를 시도한다.
4. Child가 Main run ID를 argument에 넣어 Coordinator-only tool을 호출하도록 시도한다.
5. Child가 sibling에게 직접 메시지/assignment를 시도한다.

Expected:

- 다른 workspace/window/worktree 요청은 `scopeMismatch` 또는 unknown 오류다.
- 다른 run ID를 적어도 per-run capability source가 바뀌지 않는다.
- Child의 spawn/assign/cancel/reassign과 sibling direct action은 `forbiddenActor`다.
- 원 workspace 상태는 바뀌지 않는다.

## Scenario 9: Read-only enforcement

1. 자동 Child를 read-only capable smoke profile로 실행한다.
2. mutation permission을 요청하게 한다.
3. change-violation smoke profile을 실행한다.
4. read-only 미지원 profile로 자동 Child launch를 시도한다.

Expected:

- automatic Child는 read-only/autoAllow false로 시작한다.
- mutation permission은 거부된다.
- change가 감지되면 해당 worker가 중단되고 task는 `readOnlyViolation`이다.
- 자동 revert는 실행되지 않는다.
- 미지원 profile은 process를 시작하지 않고 `unsupportedReadOnlyProfile`을 반환한다.
- 다른 Child는 계속 실행한다.

## Scenario 10: Capacity와 종료 복구

1. `ACP_WORKBENCH_MAX_RUNS=2`로 앱을 다시 실행한다.
2. Main에서 세 Child task를 만든다.
3. 실행 중 Worktree Session 창을 닫는다.
4. 앱에서 recoverable workspace를 확인하고 명시적으로 복구한다.

Expected:

- capacity를 넘는 task는 안정적인 FIFO `ready` queue에 남는다.
- active task 종료 후 다음 task가 시작된다.
- 창 종료 시 live run은 취소되지만 durable task/report는 삭제되지 않는다.
- runtime이 없는 task는 완료가 아니라 `workspaceClosed`/`runtimeLost`로 표시된다.
- 새 창에 자동 attach되지 않고 사용자 선택 후 복구한다.

## Scenario 11: 기존 탭·타일·exchange 회귀

1. Main + Extra 1 + Extra 2를 열고 탭에서 타일로 전환한다.
2. 세 panel의 가로 비율을 확인한다.
3. 오른쪽/아래 추가, resize와 close를 실행한다.
4. tab/tile을 20회 왕복한다.
5. 기존 peer `send`, `queue`, `draft`를 실행한다.

Expected:

- 타일 진입 시 세 open panel은 `1:1:1`이다.
- panel identity, run, prompt draft, queue, permission과 timeline이 유지된다.
- split/close invariant와 panel/depth limit가 유지된다.
- 기존 peer exchange는 동일 창 exact target에 한 번 전달된다.
- orchestration 참여 Child의 sibling ACL은 별도 적용된다.

## Automated verification

Frontend:

```sh
pnpm --filter @yoophi/agentic-workbench run check-types
pnpm --filter @yoophi/agentic-workbench run test
pnpm --filter @yoophi/agentic-workbench run build
pnpm --filter @yoophi/agentic-workbench run build-storybook
```

Backend:

```sh
cargo test -p agentic-workbench
cargo check -p agentic-workbench
```

Repository-wide regression이 필요할 때:

```sh
pnpm run check-types
pnpm run test
cargo test --workspace
```

### 2026-07-27 구현 검증 기록

- frontend typecheck: 통과
- frontend unit/component test: 67 files, 298 tests 통과
- backend 전체 단위 test: 163 tests 통과
- backend integration test: 2 tests 통과. Researcher/Reviewer/Tester direct Child 3개와
  structured result 수집, 실제 Node ACP worker의 parent input request→response→result와
  Bearer MCP 호출을 검증
- frontend production build: 통과
- Storybook production build: 통과(신규 atoms/molecules/Activity Rail/Composer/workspace/page
  story 포함)

자동 검증으로 확인한 핵심 회귀:

- tab에서 tile로 진입할 때 Main + Extra 2개의 leaf width가 각각 `1/3`
- background node promote/detach 뒤 task/run identity 유지
- 화면의 panel별 composer는 숨고 workspace composer 하나만 렌더
- focused/selected/all/coordinator target 계산과 빈 selected 거부
- journal overflow gap 감지 후 durable snapshot 재조회
- cancel 뒤 늦게 도착한 result가 task를 completed로 되돌리지 않음
- 명시적 confirmation 없는 Main run 교체 거부와 generation capability 폐기
- runtime 유실 시 완료가 아닌 retry 가능한 blocked/attention 상태
- 정상 window destroy와 비정상 종료 뒤 stale window binding 해제, 명시적 복구 전
  자동 attach 금지

실제 `Agentic Workbench Dev.app` 검증:

- Main + Extra 1 + Extra 2를 탭에서 타일로 전환해 1:1:1 leaf 비율을 확인하고, 타일
  toolbar와 panel command에서 새 타일을 추가했다.
- 단일 Workspace Composer의 `Main 위임`으로 실제 Codex Main을 실행해 Researcher,
  Reviewer, Tester direct Child를 만들고 Activity Rail 및 capacity queue를 확인했다.
- background Researcher 승격 중 runtime host effect가 반복되던 빈 화면 회귀를 실제
  앱에서 발견했다. controller snapshot 이중 적용과 불안정한 effect dependency를
  제거한 뒤 같은 Child를 승격해 Main/Researcher 탭과 Activity Rail이 유지됨을 확인했다.
- 앱 프로세스를 종료한 뒤 새 Worktree Session을 열어 `이전 에이전트 작업 복구`와
  `새로 시작` 선택지가 자동 attach 전에 표시됨을 확인했다.
- 이전 작업 복구 후 세 Child가 모두 `차단됨`과 `런타임 연결 유실`로 표시되고,
  report/task 이력을 보존한 채 Researcher를 다시 panel로 승격해 화면이 안정적으로
  유지됨을 확인했다.

## Required automated assertions

- Main 정확히 하나, direct Child만 허용, cycle/grandchild 거부
- task/execution/presentation state transition과 terminal 역행 금지
- explicit result 없이 completion 금지
- generation handoff required와 stale Main 거부
- FIFO scheduling과 app/workspace capacity
- request/report/dispatch idempotency가 repository reload 뒤에도 유지
- JSON atomic write, backup recovery와 revision conflict
- per-run capability가 서로 다르고 source spoofing 불가
- canonical artifact root, size와 UTF-8 validation
- cancel/result, close/delivery, generation/reports race
- runtime journal snapshot/live sequence dedupe
- promotion/detach 뒤 같은 Node/task/run/timeline
- promoted panel이 기존 run binding으로 hydrate되고 빈 mount state로 덮어쓰지 않음
- event 없음, journal gap과 runtime lost를 서로 다른 UI 상태로 표시
- user input response가 exact Child run에 전달된 뒤에만 running 전이
- Child report가 durable Main notification을 만들고 collect 원문과 연결
- UI/MCP send·cancel·retry·reassign이 동일 worker 호출과 상태 결과를 가짐
- full-payload idempotency와 old attempt/run fencing
- partial dispatch와 partial task result 보존
- read-only profile/permission/change violation
- owner window에만 Tauri event 전달

## Documentation verification

- `docs/agent-orchestration-workspace.md`가 scope, non-scope, 부모·자식 관계, 세 상태 축,
  Composer, Activity Rail, 승격, 안전 원칙, 구현 단계와 완료 기준을 포함한다.
- Mermaid preview에서 관계, 상태, sequence와 구현 순서도가 오류 없이 렌더링된다.
- 계약과 data model의 command/tool/status 이름이 구현과 일치한다.
