# Feature Specification: Rust client와 public CLI

**Feature Branch**: `047-rust-client-cli`  
**Created**: 2026-09-29  
**Status**: 구현·controlled/actual wire 검증 완료 checkpoint; 최종 workspace 검증·순차 구현 리뷰·PR/CI/merge·인계 진행 중
**Input**: macOS standalone HTTP/WS server+thin desktop 전체 로드맵에서 Rust client/CLI를 독립 진행한다. 045/046 미완료·production gate를 그대로 보존하고 다른 작업 파일을 포함하지 않는다.

## User Scenarios & Testing

### User Story 1 - 같은 서버를 안전하게 조회·호출 (Priority: P1)

사용자는 desktop을 열지 않고 기존 서버의 프로젝트와 run을 조회하고 명시적 mutation을 호출한다. 잘못된 endpoint나 권한으로 조용히 재시도하지 않는다.

**Why this priority**: desktop과 server를 분리한 효과를 headless caller로 사용할 수 있어야 한다.
**Independent Test**: 격리된 기존 wire 계약의 가짜 peer와 caller fixture로 신원·권한·호출 결과를 검증한다. 실제 agent/child를 실행하지 않는다.

**Acceptance Scenarios**:
1. **Given** 확인 가능한 기존 서버, **When** 프로젝트를 조회, **Then** 같은 저장된 결과와 request identity를 반환한다.
2. **Given** stale endpoint/잘못된 신원/호환 불일치, **When** 호출, **Then** 자격 증명 유출·새 서버 시작·자동 store migration 없이 분명히 거절한다.
3. **Given** 서버 적용 뒤 응답 유실, **When** 사용자가 재시도, **Then** 같은 key/payload/instance에 한정해 재조회하거나 replay하고 적용 여부를 추측하지 않는다.

### User Story 2 - 자동화에 안정된 machine command (Priority: P1)

사용자와 승인된 caller는 `aw`의 generic operation 및 명시 명령을 script로 사용한다.

**Why this priority**: 호출 결과를 문자열 추측으로 처리하거나 비밀 prompt를 argv에 넣지 않아야 한다.
**Independent Test**: golden stdout/stderr/exit 및 stdin fixtures, request/key 분리와 원 fault outcome 보존을 검사한다.

**Acceptance Scenarios**:
1. **Given** 성공 또는 실패, **When** machine mode 실행, **Then** 성공 stdout 결과 하나 또는 실패 stderr 오류 하나와 stable exit를 반환한다.
2. **Given** prompt/goal/큰 input, **When** stdin으로 제출, **Then** argv/log에 raw 내용이나 token을 남기지 않는다.
3. **Given** timeout/SIGINT, **When** local wait 종료, **Then** 명시 취소 없이는 서버 작업을 취소하거나 NotApplied라고 단정하지 않는다.

### User Story 3 - run/작업 상태를 끊김 뒤 계속 관찰 (Priority: P2)

사용자는 event stream을 관찰하고 연결이 끊기거나 보관 범위가 바뀌어도 적용 완료 위치에서 복구한다.

**Why this priority**: CLI와 이후 TUI가 desktop과 같은 결과를 관찰해야 한다.
**Independent Test**: event/ticket/snapshot peer fixture에 지연 소비·gap·epoch 교체·중단을 주입한다.

**Acceptance Scenarios**:
1. **Given** 지연 소비, **When** 재연결, **Then** 적용/출력 완료 cursor를 쓰고 받은 것만으로 전진하지 않는다.
2. **Given** retention gap, **When** 복구, **Then** live 확보→snapshot→buffer filtering 순서로 유실/중복을 막는다.
3. **Given** 새 server epoch 또는 old listener의 늦은 완료, **When** 이벤트 도착, **Then** 이전 작업이 새 상태/cursor를 바꾸지 않는다.

### Edge Cases

identity proof 전 redirect/proxy, owner descriptor symlink/권한/교체/oversize, identity와 handshake 사이 교체, empty/malformed response, HTTP/body fault 불일치, unknown outcome, partial stdout/broken pipe, oversized input/frame/queue, same key 다른 payload, authentication refresh 뒤 epoch 변경, dropped event consumer, live/snapshot 경합, stale server에 new daemon ensure, token·ticket URL의 Debug/panic 출력, agent profile에서 owner fallback, destructive operation의 무단 confirmation을 포함한다.

## Requirements

### Functional Requirements

- **FR-001**: shared operation/event 계약으로 Rust caller와 CLI가 같은 기존 서버를 사용해야 한다. desktop/frontend 또는 server runtime을 caller 안에 조립하지 않는다.
- **FR-002**: loopback에서 신원 확인 뒤에만 credential을 보내고 instance/protocol/storage 호환성을 확인한다. proxy/redirect/외부 주소를 거절한다.
- **FR-003**: 현재 독립 client 구현은 existing instance에 연결하며 자동 spawn/ensure, updater, process ownership, 사용자 데이터 bootstrap/migration을 수행하지 않는다. 실제 호환 검증 harness는 merged044 서버를 격리 경로에서 명시적으로 시작할 수 있으며 client 자동 spawn과 구분한다. 향후 server start/ensure 기능은 045·046·배포 readiness 뒤에 연결하고 feature 잔여로 명시한다.
- **FR-004**: full reply의 requestId/revision/replayed와 fault code/outcome/retryable/details를 의미 그대로 보존한다. transport loss는 Unknown과 명시적 NotApplied 거절을 구분한다.
- **FR-005**: request identity와 idempotency key를 구분한다. mutation 재시도는 명시적 operation identity와 동일 key/payload/instance를 유지하며 epoch 변경 후 자동 전송하지 않는다.
- **FR-006**: catalog/descriptor가 정의하는 operation·input·scope를 검사하고 generic `aw call`과 명시 명령의 의미를 같게 유지한다. unsupported operation은 silent fallback하지 않는다.
- **FR-007**: `aw events watch --input -`, `aw operations`, `aw project list`, `aw run start/watch/cancel`, `aw server status`, generic `aw call`을 제공한다. readiness가 필요한 launch/stop/ensure·agent profile 배포는 실제 prerequisite가 없으면 활성화하지 않는다.
- **FR-008**: machine finite 성공은 stdout `{ok:true,data,requestId,...}` 하나, 실패는 stdout0·stderr `{ok:false,error,requestId,...}` 하나와 stable nonzero exit다. outcome과 uncertain retry 정보를 오류에 보존한다. 출력 채널 자체가 broken/blocked이면 JSON 전달 성공을 주장하지 않고 bounded 종료·outputUnavailable을 반환한다.
- **FR-009**: human progress/warning은 stderr, machine mode는 color/spinner/interaction/log 혼합0이다. library panic/dependency log도 stdout을 오염시키지 않는다.
- **FR-010**: input은 bounded stdin/file descriptor 기본이며 token·prompt·goal을 argv/log/Debug/error에 노출하지 않는다. raw private input echo를 하지 않는다.
- **FR-011**: timeout/SIGINT는 local wait만 종료한다. 명시적 cancel-on-timeout 또는 cancel operation은 별도 요청/권한/멱등 계약으로 처리한다.
- **FR-012**: streaming mode는 `stream.open`→완전한 JSONL event/control→`stream.end`이며 open 전 실패는 finite stderr, 이후 실패는 final stream.end+nonzero exit다. gap/epoch 교체를 숨기지 않는다.
- **FR-013**: event cursor는 수신이 아니라 실제 projection/출력 완료 뒤 전진한다. reconnect/gap/snapshot/live buffer 및 listener operation generation은 원 계약에 맞게 검증한다.
- **FR-014**: HTTP/input/frame/message/queue/time 한도를 명시하고 위반 시 typed failure로 중단한다. protocol payload를 truncate/drop한 뒤 성공처럼 계속 처리하지 않는다.
- **FR-015**: owner credential와 agent-scoped profile을 구분한다. agent mode는 descriptor owner token을 fallback으로 읽거나 authority를 확대하지 않는다. unsupported issuance/proof는 거절한다.
- **FR-016**: destructive operation은 원 catalog/revision/권한·human grant 의미를 유지한다. `--yes`로 agent에게 grant를 만들어주지 않는다.
- **FR-017**: Rust client는 CLI 외 독립 fixture consumer에서 재사용 가능하고 이후 TUI가 직접 사용할 port를 제공한다. TUI UI·MCP stdio 어댑터는 전체 로드맵 후속이며 완료로 계산하지 않는다.
- **FR-018**: macOS Apple Silicon이 이번 대상이다. 계획된14+ 지원과 현재15 검증 근거를 구분하며 Linux/Windows 구현·검증을 새로 수행하지 않는다.
- **FR-019**: 실제 desktop/CLI/TUI concurrent use, desktop 종료 후 run 관찰/취소, TUI/MCP, CALVER signed/notarized package·CLI discover/update 및 desktop business fallback 제거는 이번 047 후속 이연 범위다. 미완료·재개 조건을 인계하고 fake peer 통과로 전체 서버 전환 완료를 선언하지 않는다. 후속 구현은 시작하지 않는다.
- **FR-020**: 045 T010/T016 containment/migration 및 046 backup/freeze/restore gates를 우회하지 않는다. source branch/base/미병합 의존성과 fixture 대 production 증거, 미완료 전제와 재개 gate를 인계한다. 045/046 구현 완료 자체는 최신 범위의 047 종료 전제가 아니다.

- **FR-021**: fake peer 외에도 merged044 실제 server binary와 격리 데이터 경로에서 신원→호환→조회·허용된 비실행 mutation·실제event 수신을 검증한다. agent/child launch 없이 수행하며 binary source/commit과 cleanup 증거를 보존한다. fake peer만 통과한 adapter를 최종 client 완료로 간주하지 않는다.

### Key Entities

VerifiedEndpoint(instance/epoch/compatibility, 비밀은 redacted), CallerProfile(owner 또는 제한된 agent 권한), CallAttempt(request/key/payload identity/outcome), AppliedCursor(stream/epoch/consumer generation), MachineResult/StreamRecord, ConnectionLimits.

## Constitution Alignment

- Monorepo boundary: reusable Rust client는 crates, `aw` CLI composition은 apps. 원 protocol 공유, app-to-app import 없음.
- Frontend layering: UI 변경 없음.
- Backend boundary: pure caller policy/ports와 locator/HTTP/WS/stdio adapters 분리; Tauri 및 server storage/runtime 재조립 없음.
- Shared core vs UI: headless model/fixture consumer 먼저. TUI/MCP UI는 후속.
- Persistence and safety: descriptor readonly/owner/size/identity 검사, run/bench scope·key/outcome 보존. 원 데이터 root/store 쓰기 없음.
- Documentation and Storybook: 한국어 docs와 명령 계약, Mermaid 흐름. UI 없으므로 Storybook 해당 없음.

## 최신 사용자 종료 기준 (2026-09-29)

원 전체 전환 목표를 supersede하여 이번 작업은 047 Rust client/CLI 자체 구현·가능한 모든 계약/actual exact merged044 통합 검증·OCR delegate → Codex adversarial `--wait` 순차 리뷰/수정·PR/CI·squash merge·main checkout/pull과 `docs/047-completion-handoff.md` 기록 후 종료한다. 045/046 보완·TUI/MCP·배포 등 후속 구현은 시작하지 않는다. FR019/020 및 SC006은 미완료 사실·production gate 유지·인계 정확성을 요구하며, 해당 후속 구현 완료를 047 merge 전제로 요구하지 않는다. T016의 외부 prerequisite 종속 exit6은 명시적 이연으로 리뷰하고 fake 활성화하지 않는다. SC001–005/007 및 가능한 047 계약·actual server 시험은 그대로 필수다.

## Success Criteria

### Measurable Outcomes

- **SC-001**: 잘못된 identity/호환/redirect/proxy/permission fixture 전체에서 비밀값 전달0·새 서버 시작0.
- **SC-002**: 모든 finite command golden에서 결과JSON 수1·성공 stderr diagnostics0·실패 stdout0·stable exit 일치100%.
- **SC-003**: 응답 유실/재시도/epoch 교체 fixtures에서 같은 작업 효과 최대1, unknown을 NotApplied로 바꾼 경우0.
- **SC-004**: 지연 소비/gap/epoch/live snapshot fixtures에서 적용 cursor 오진전0·필요 이벤트 유실0·새 generation에 old completion mutation0.
- **SC-005**: timeout/SIGINT/broken pipe/quota fixtures에서 bounded 종료와 원 서버 작업의 암묵 취소0.
- **SC-007**: 실제 기존 서버와 Rust client·CLI에서 조회·동일key 재시도·비실행 mutation·event 소비 결과가 일치하며 actual agent/child launch0·사용자root 접근0·test server 잔존0.
- **SC-006**: 실제 검증한 macOS host만 근거로 기재한다. 미실행 macOS14+/signed package·desktop/CLI/TUI matrix·TUI/MCP·update·fallback 제거와 045/046은 미완료로 인계하고 production gate를 유지한다. 해당 후속 완료는 047 종료 gate에서 제외하며 independent fixtures를 전체 배포 완료로 합치지 않는다.

## Assumptions

기존044 merged protocol v1와 public descriptor를 기준으로 한다. missing server는 현재 unavailable이며 자동 재기동하지 않는다. 원 로드맵6단계의 client/CLI 계약을 구현하되 process/data/packaging prerequisite를 없애지 않는다. 명령 편의보다 authority/outcome/cursor 의미 보존을 우선한다. numbering045/046은 미병합 branch에 예약되어047을 사용한다.
