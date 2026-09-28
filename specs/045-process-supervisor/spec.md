# Feature Specification: 서버 자식 프로세스 감독

**Feature Branch**: `045-process-supervisor`

**Created**: 2026-09-28

**Status**: Draft

**Input**: User description: "독립 Workbench 서버가 만드는 모든 자식 프로세스를 공통 감독 경계로 옮기고, 실행 공개 순서·프로세스 트리 종료·출력 제한·회수를 보장한다."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 시작했다고 거짓 보고하지 않기 (Priority: P1)

사용자가 agent run이나 terminal을 시작하면, 시스템은 실제 자식 프로세스를 소유하고 관리할 준비가 끝난 뒤에만 시작됨을 알린다. 실행 파일이 없거나 시작에 실패하면 사용자는 명확한 실패를 받고, 시작됐다는 이벤트나 고아 프로세스는 남지 않는다.

**Why this priority**: 시작 이벤트는 화면, scheduler, 취소와 복구 판단의 기준이다. 실제 프로세스보다 먼저 공개되면 사용자는 존재하지 않는 실행을 기다리고 후속 동작은 잘못된 상태를 기준으로 움직인다.

**Independent Test**: 프로세스 시작을 성공·실패·중단 지점별로 제어해, 소유 예약과 감독 등록이 끝나기 전에는 외부에서 시작 상태를 한 번도 관측하지 못하고 성공 때만 정확히 한 번 관측하는지 확인한다.

**Acceptance Scenarios**:

1. **Given** durable run owner가 예약된 상태, **When** 자식 시작과 감독 등록이 성공하면, **Then** 등록 뒤에만 accepted/started가 공개되고 PID와 시작 identity가 그 owner에 연결된다.
2. **Given** durable owner 예약은 성공했지만 자식 시작이 실패한 상태, **When** 호출이 끝나면, **Then** started는 공개되지 않고 예약은 실패 상태로 정리되며 실행 중 프로세스가 없다.
3. **Given** 자식은 시작됐지만 외부 공개 전 요청 future가 취소된 상태, **When** 정리 절차가 끝나면, **Then** 자식과 descendant가 모두 종료·회수되고 started는 공개되지 않는다.

---

### User Story 2 - 취소와 서버 종료가 자식 트리를 남기지 않기 (Priority: P1)

사용자가 run이나 terminal을 취소하거나 서버를 종료하면, 해당 프로세스가 만든 descendant까지 같은 수명 단위로 정리된다. 정상 종료에 응답하지 않는 프로세스도 제한 시간 뒤 강제 종료되고, 완료 뒤에는 좀비나 살아 있는 descendant가 남지 않는다.

**Why this priority**: 독립 서버는 데스크톱보다 오래 살 수 있으므로 누적된 고아 agent·shell·Git 프로세스가 작업 공간, 자격 증명, scheduler 용량과 업데이트를 계속 점유할 수 있다.

**Independent Test**: 자식이 손자 프로세스를 만들고 종료 신호를 무시하는 fixture를 실행한 뒤 cancel, wait-stop, force-stop, 서버 정상 종료, 서버 강제 종료를 각각 수행해 전체 트리 생존과 회수 상태를 확인한다.

**Acceptance Scenarios**:

1. **Given** 정상 종료에 응답하는 자식 트리, **When** 사용자가 취소하면, **Then** graceful 종료 뒤 모든 direct child가 wait되고 descendant가 남지 않는다.
2. **Given** 정상 종료를 무시하는 자식 트리, **When** 제한 시간이 지나면, **Then** 전체 트리가 강제 종료되고 direct child가 회수된다.
3. **Given** 여러 run·terminal·helper가 동시에 실행 중, **When** 서버가 force-stop되면, **Then** 각 owner의 정책에 따라 모두 정리되고 서버 종료 뒤 감독 대상 프로세스가 0개다.
4. **Given** 서버가 비정상 종료된 상태, **When** 운영체제별 containment 또는 다음 시작의 recovery가 수행되면, **Then** 서버가 발급한 identity로 확인된 잔여 트리만 정리되고 PID만 같은 다른 프로세스는 건드리지 않는다.

---

### User Story 3 - 과도한 출력에도 서버가 응답하기 (Priority: P2)

agent나 helper가 stdout 또는 stderr를 빠르게 내보내거나 줄바꿈 없이 계속 써도, 다른 run과 서버 제어 요청은 계속 처리된다. 사용자는 제한된 출력과 잘림·제한 상태를 알 수 있고, 비밀 환경 값은 로그에 노출되지 않는다.

**Why this priority**: pipe를 동시에 소비하지 않거나 무제한 보관하면 자식과 서버가 교착되거나 메모리가 고갈된다. 독립 daemon 전체의 가용성 문제로 번진다.

**Independent Test**: stdout/stderr 각각과 동시 출력, 줄바꿈 없는 대용량 출력, 느린 소비자를 주입해 메모리·이벤트 한도와 서버 제어 응답, 최종 종료를 검증한다.

**Acceptance Scenarios**:

1. **Given** stdout과 stderr를 동시에 빠르게 쓰는 자식, **When** 한 스트림의 소비자가 느려져도, **Then** 두 스트림은 독립적으로 drain되고 자식 완료와 취소가 교착되지 않는다.
2. **Given** 사용자 표시 로그가 줄바꿈 없이 한도를 넘은 상태, **When** 제한이 적용되면, **Then** 메모리와 이벤트 수가 정해진 상한 안에 있고 잘림 사실이 관측된다.
3. **Given** JSON-RPC 같은 protocol frame이 허용 크기를 넘은 상태, **When** reader가 이를 발견하면, **Then** frame 일부를 버리고 계속하지 않으며 typed protocol failure를 기록하고 해당 process를 종료한다.
4. **Given** 실행 환경에 credential이 있는 상태, **When** 시작·실패·종료가 기록되면, **Then** executable과 안전한 메타데이터만 남고 환경 값과 credential은 남지 않는다.

---

### User Story 4 - 모든 서버 소유 실행을 같은 정책으로 다루기 (Priority: P2)

운영자는 agent, terminal, Git 조회, watcher 보조 작업, agent catalog 조회와 로그인 셸 probe가 서로 다른 종료 규칙을 갖지 않고 같은 감독 정책 아래 있음을 확인할 수 있다. 반대로 데스크톱이 서버를 띄우는 bootstrap과 데스크톱 전용 OS launcher는 역할이 섞이지 않는다.

**Why this priority**: 일부 짧은 helper가 감독 밖에 남으면 shutdown·timeout·출력·비밀정보 규칙에 예외가 생기고, 누락된 경로가 이후 고아 프로세스의 원인이 된다.

**Independent Test**: production source의 프로세스 실행 inventory를 정본 분류표와 대조하고, 서버 소유 항목은 모두 공통 감독 진입점을 통과하며 제외 항목은 이유와 별도 수명 owner가 있는지 검사한다.

**Acceptance Scenarios**:

1. **Given** ACP agent, terminal, Git, watcher, catalog 조회, PATH probe가 실행되는 상태, **When** 실행 추적을 검사하면, **Then** 모두 owner·purpose·timeout을 가진 감독 기록에 나타난다.
2. **Given** desktop daemon bootstrap, desktop native URL/file opener, build script, test fixture, 다른 앱의 실행 경로, **When** inventory를 검사하면, **Then** 서버 workload와 분리된 범주와 수명 책임이 기록된다.
3. **Given** remote client가 서버에 연결된 상태, **When** process cwd와 path를 사용하는 작업을 시작하면, **Then** 그 경로와 프로세스는 서버 host 기준으로 해석됨이 인터페이스와 오류에 일관되게 드러난다.

### Edge Cases

- spawn 성공 직후 감독 등록이나 durable 상태 갱신이 실패한다.
- spawn/adopt와 cancel, 자연 종료, timeout, 서버 drain이 같은 순간 경쟁한다.
- 자식이 매우 빨리 끝나 PID를 등록하기 전에 wait 가능 상태가 된다.
- 이전 실행의 늦은 release/terminate가 재사용된 owner나 새 attempt를 종료하려 한다.
- 자식이 새로운 process group/session을 만들거나 이중 fork로 containment를 벗어나려 한다.
- stdout은 닫혔지만 stderr가 계속 열려 있거나, 반대 순서로 닫힌다.
- 출력이 유효하지 않은 UTF-8이거나 한 이벤트보다 긴 단일 바이트 스트림이다.
- helper timeout과 서버 shutdown이 겹치고 양쪽이 동시에 kill/wait를 시도한다.
- 서버 crash 뒤 PID가 재사용되거나 process identity 자료가 불완전하다.
- supervisor 자체의 등록·정리 future가 취소된다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 시스템은 서버가 만드는 모든 workload 및 보조 child를 하나의 감독 계약으로 관리해야 한다.
- **FR-002**: 감독 범위는 ACP agent runner, ACP terminal, Git 작업, worktree watcher 보조 작업, agent catalog의 외부 모델 조회, 로그인 셸 PATH probe를 포함해야 한다.
- **FR-003**: 시스템은 desktop daemon bootstrap, desktop 전용 native launcher, build-time command, test fixture, 다른 앱의 child를 서버 workload와 분리해 정본 inventory에 기록해야 한다.
- **FR-004**: 각 실행 요청은 executable, 인자, 환경, 작업 디렉터리, owner, purpose, timeout, 출력 정책, 종료 정책을 구조화해 표현해야 하며 shell 문자열 결합에 의존하지 않아야 한다.
- **FR-005**: 환경 값과 credential은 진단·오류·이벤트·로그에 노출하지 않아야 한다.
- **FR-006**: durable execution과 run/terminal/workspace business owner는 child 생성 전에 domain store에 예약되어야 한다. read-only helper의 domain 실행 의미는 transient로 유지하되, 모든 서버 소유 child는 별도의 durable containment recovery anchor를 spawn 전에 가져야 한다.
- **FR-007**: child 생성 뒤 PID/handle과 재사용을 구분할 시작 identity를 감독자가 소유한 다음에만 호출 성공과 accepted/started 상태를 외부에 공개해야 한다.
- **FR-008**: child 생성, 감독 등록, durable 상태 갱신 중 어느 단계가 실패하거나 취소돼도 started를 거짓으로 공개하지 않고 예약과 실제 child를 정리해야 한다.
- **FR-009**: 동일 owner/attempt의 accepted/started는 정확히 한 번 공개되어야 하며, 이전 attempt의 늦은 완료가 새 attempt 상태를 바꾸지 않아야 한다.
- **FR-009a**: child adoption 뒤 durable publication acknowledgement 전 caller가 중단돼도, durable published attempt는 계속 실행되고 unpublished attempt는 정리되어야 한다. 단순 response 유실만으로 적용 여부를 추정해서는 안 된다.
- **FR-009b**: publication과 cleanup은 `Adopted`에서 하나의 atomic conditional transition으로 경쟁해야 하며, 승리한 상태를 다른 경로가 뒤집거나 publication과 process 종료가 동시에 성공해서는 안 된다.
- **FR-009c**: accepted/started publication과 idempotent response는 attempt identity에 연결된 durable logical event로 commit되어야 하며, crash/reconnect 재전달이 사용자 projection에 중복 적용되거나 committed event가 유실되어서는 안 된다.
- **FR-010**: 감독 단위는 direct child 하나가 아니라 그 child가 만든 descendant tree 전체여야 한다.
- **FR-011**: 취소·timeout·shutdown은 graceful tree terminate, 제한 시간, force tree kill, direct child wait 순서를 따라야 한다.
- **FR-012**: 서로 경쟁하는 cancel, 자연 종료, force-stop, future 취소 중 정확히 하나의 종료 결과가 owner에 반영되고 나머지는 멱등이어야 한다.
- **FR-013**: 서버의 정상 종료가 완료됐을 때 감독 중인 child와 회수되지 않은 direct child는 0개여야 한다.
- **FR-014**: 서버 비정상 종료 뒤 각 지원 플랫폼에서 descendant가 남지 않게 하거나, 남을 수 있는 경우 다음 시작이 서버가 발급한 nonce와 시작 identity를 검증해 해당 tree만 정리해야 한다. leader 조기 종료, 새 process group/session, double-fork와 reparent 뒤에도 같은 보장을 유지해야 하며 PID만으로 종료해서는 안 된다.
- **FR-014a**: keeper가 죽고 서버가 계속 실행되는 경우 서버는 이를 즉시 감지해 cleanup ownership을 인계해야 하며, 서버와 keeper가 함께 죽은 경우 다음 시작은 readiness 전에 모든 durable containment anchor를 reconcile해야 한다.
- **FR-015**: stdout과 stderr는 동시에 drain되어야 하고, 어느 한쪽의 backpressure가 다른 쪽과 child 종료를 막지 않아야 한다.
- **FR-016**: 각 stream은 protocol transport와 사용자 표시 로그 중 어느 계약인지 명시해야 하며, 두 종류에 같은 truncation/drop 정책을 적용해서는 안 된다.
- **FR-017**: 사용자 표시 로그는 stream별 byte·event·rate 상한을 가지며 줄바꿈 없는 출력과 유효하지 않은 UTF-8에도 같은 상한이 적용되어야 한다. 잘리거나 버려진 양은 관측 가능한 상태로 보고되어야 한다.
- **FR-018**: protocol stream은 frame별 최대 크기, incomplete-frame 진행 deadline/최소 진행률, owner 전체 runtime 상한을 가져야 한다. frame 일부를 truncate/drop한 뒤 성공한 stream처럼 parsing을 계속해서는 안 되며, 한도 위반·진행 정지·malformed frame·EOF는 typed protocol failure와 해당 process 종료로 귀결되어야 한다.
- **FR-019**: protocol stream이 느리거나 한도를 위반해도 stderr/log drain과 status·cancel·shutdown은 교착 없이 계속 진행되어야 한다.
- **FR-020**: 짧은 helper도 timeout, cancel, output, wait/reap 규칙을 따라야 하며 동기 대기로 서버 전체 executor를 막지 않아야 한다.
- **FR-021**: 기존 run, terminal, Git, watcher와 catalog의 사용자 관측 결과는 감독 도입 뒤에도 호환되어야 한다. 단, 현재의 잘못된 started-before-spawn/adopt 순서는 바로잡는다.
- **FR-022**: remote 사용에서 cwd, executable과 filesystem path가 server host에 속한다는 계약을 공개 인터페이스와 오류에 명시해야 한다.
- **FR-023**: 서버 상태와 진단은 민감 정보를 제외한 owner, purpose, lifecycle, 종료 이유와 현재 감독 수를 제공해야 한다.
- **FR-024**: production process 실행 inventory는 자동 검사 가능해야 하며, 새 직접 spawn 경로가 정본 분류와 감독 경계를 우회하면 검증이 실패해야 한다.
- **FR-025**: 플랫폼별 containment 동작은 macOS, Linux, Windows의 지원 방식과 한계를 각각 기록하고 실제 target 검증을 가져야 한다.
- **FR-026**: desktop이 standalone server를 시작하는 launcher는 시작 대상 서버 자신의 supervisor에 종속되지 않아야 하며, 기존 단일 daemon·readiness·stop 계약을 유지해야 한다.

### Key Entities

- **Process specification**: child를 시작하는 데 필요한 실행 파일, 인자, 환경, cwd, owner, 목적, timeout, 출력·종료 정책의 구조화된 요청.
- **Process owner**: run, terminal, workspace helper 또는 server lifecycle처럼 child의 수명과 결과를 책임지는 stable identity.
- **Process attempt**: owner 아래 한 번의 실행 시도. 새 시도와 이전 시도의 늦은 결과를 구분한다.
- **Supervised process**: PID/handle, 시작 identity, containment identity, 상태, output counters, 종료 이유를 가진 감독 기록.
- **Containment unit**: direct child와 모든 descendant를 함께 종료하는 플랫폼별 프로세스 트리 단위.
- **Output stream state**: stdout/stderr별 protocol/log 분류, 수집량, 전달량, frame 또는 drop/truncation과 rate-limit 상태.
- **Process inventory entry**: production spawn site의 범주, owner, 감독 포함 여부와 제외 근거.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: 공통 감독 추상화와 운영체제 adapter는 재사용 Rust crate에 둔다. AW server가 이를 소비하며 앱 간 직접 import는 만들지 않는다.
- **Frontend layering**: 새 사용자 화면은 필수가 아니다. 기존 run/terminal 상태와 오류 표시는 현재 FSD 경계를 유지한다.
- **Backend boundary**: lifecycle 규칙과 상태 전이는 application/domain, 실행·신호·pipe·운영체제 containment는 infrastructure, 호출자는 port를 통해 사용한다.
- **Shared core vs UI**: 공유 UI는 없다. 공통 process lifecycle core와 platform adapter만 공유한다.
- **Persistence and safety**: durable owner를 먼저 예약하고, process nonce/start identity로 PID 재사용을 방어한다. 환경 값·credential을 저장하거나 출력하지 않는다.
- **Documentation and Storybook**: ProcessSupervisor 계약, inventory와 플랫폼별 보장을 문서화한다. 새 UI가 없으므로 Storybook 변경은 필요하지 않다.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: spawn 실패·adopt 실패·요청 취소를 각 100회 반복해도 외부에서 started를 관측한 횟수와 남은 child 수가 모두 0이다.
- **SC-002**: 정상 시작을 100회 반복하면 각 attempt마다 감독 등록 뒤 accepted/started가 정확히 한 번 공개되고, 공개 전 등록 누락이 0건이다.
- **SC-002a**: publication CAS와 cleanup CAS의 양쪽 승리 interleaving, commit 전후 crash, send/ack 전후 reconnect를 반복해 process keep/kill과 durable logical event가 같은 승자를 따르고 사용자 projection 적용 수가 attempt마다 정확히 1이다.
- **SC-003**: 정상 종료 응답, 종료 무시, 손자 생성 fixture 각각에 cancel·wait-stop·force-stop을 적용한 모든 지원 target에서 종료 후 descendant와 unreaped direct child가 0개다.
- **SC-004**: 서버 강제 종료 뒤 platform containment 또는 startup recovery를 거치면 감독 대상 descendant가 0개이고, PID 재사용 대조 프로세스는 100% 생존한다.
- **SC-004a**: keeper-only hard kill과 server+keeper hard kill 뒤 durable/transient domain owner 모두에서 recovery anchor로 확인된 descendant는 0개가 되고 unrelated 대조 process는 모두 생존한다.
- **SC-005**: stdout/stderr 동시 폭주와 줄바꿈 없는 각 100 MiB 표시 로그 fixture에서 정해진 메모리·event 상한을 넘지 않고 status·cancel 요청이 2초 안에 처리된다.
- **SC-005a**: protocol 최대 frame의 경계값은 정상 처리되고, 1 byte 초과·slow-loris 진행 정지·malformed·중간 EOF는 각각 typed failure와 process 종료가 되며 손상된 frame 이후의 메시지가 성공 처리되는 경우는 0건이다.
- **SC-006**: production spawn inventory의 서버 소유 항목 100%가 공통 감독 진입점을 통과하며, 미분류 직접 spawn은 0개다.
- **SC-007**: 기존 ACP run, terminal, Git/history/status, watcher refresh, catalog 조회, server stop 통합 시나리오가 기대 결과 변경 없이 통과한다.
- **SC-008**: 로그·오류·이벤트 fixture에 주입한 credential sentinel이 모든 산출물에서 0회 나타난다.
- **SC-009**: macOS, Linux, Windows 검증표에 각 target의 containment·graceful/force·reap·crash 결과와 미지원 한계가 빈칸 없이 기록된다.

## Assumptions

- 044에서 만든 standalone server와 thin desktop 경계, 단일 writer, drain/stop 계약을 기반으로 한다.
- desktop daemon bootstrap은 서버 workload child가 아니며 server readiness까지 책임지는 별도 lifecycle owner로 남는다.
- desktop의 `open`/`rundll32`/`xdg-open` 실행은 native shell 기능이며 이 증분의 server supervisor에 넣지 않는다.
- build script와 test fixture의 직접 spawn은 production inventory에서 제외하되, fixture는 supervisor의 실제 process-tree 동작을 검증하는 데 사용한다.
- process supervision은 storage migration/backup, signed sidecar packaging/update, CLI/TUI/MCP 교체를 완료하지 않는다. 이 항목들은 5단계 이후 로드맵에 남는다.
- remote transport 자체를 여는 일은 범위 밖이지만 process와 path가 server host 소유라는 계약은 지금 고정한다.
