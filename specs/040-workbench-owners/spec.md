# Feature Specification: 작업대(Bench) 도입과 run·교환 command 이관 (서버-클라이언트 전환 2b-1)

**Feature Branch**: `040-workbench-owners`

**Created**: 2026-09-27

**Status**: Draft

**Input**: User description: "040 workbench-owners (AW 서버-클라이언트 전환 2단계 후반, 2b): 창 label(window_label)에 묶인 소유자 식별을 분해하고, 2단계로 이연된 run 8·exchange 4·orchestration 18 command 30개를 Workbench.call/events 경로로 이관. orchestration·exchange 이벤트 스트림을 구독 가능하게 연다." — 039(2a) 머지(main `54806dc`) 뒤의 정본 2단계 후반. grill(Q1)에서 **040(2b-1) = 작업대 + run 8 + 교환 4**, **041(2b-2) = orchestration 18**로 나눴다. 정본은 [서버-클라이언트 전환 조사](../../docs/client-server-architecture-research.md) §2 "event 모델 통합"·§Invariants 7·8·12, 이연 근거는 [ADR 0001](../../docs/adr/0001-defer-event-bound-commands-to-stage-2.md), 현재 상태는 [Workbench Seam](../../docs/workbench-seam.md).

## 배경과 목적

037·038은 command 31개를, 039는 이벤트 Seam과 watcher 2개를 옮겼다. 남은 30개(run 8·교환 4·orchestration 18)는 모두 **창 label**을 정체로 쓴다(30개 중 28개가 읽는다, 2026-09-27 코드 조사). 창 label은 한 값으로 네 가지 역할을 동시에 한다.

| 역할 | 오늘의 동작 | 문제 |
|---|---|---|
| run 소유자 | run 시작 시 호출한 창 label을 소유자로 기록(메모리) | 창이 없는 호출자(CLI·TUI·HTTP)는 run을 시작·제어할 수 없다 |
| 교환 작업 영역 키 | 창 label별로 패널 스냅샷·교환 이력을 보관(메모리) | 같은 작업 영역을 다른 클라이언트가 다룰 수 없다 |
| orchestration 바인딩 | workspace가 묶인 창 label을 **디스크에 저장** | 창 label은 열 때마다 새로 만들어져 재시작 뒤 절대 일치하지 않는다(041) |
| 이벤트 전달 대상 | 교환·제목 이벤트를 네이티브 창 이벤트와 창 삽입 두 경로로 보냄 | 네이티브 이벤트는 모든 창에 방송되어, **다른 세션 창이 교환 요청을 받아 "rejected"로 확인할 수 있고** 같은 창은 요청을 두 번 받는다 |

권한 판단도 일관되지 않다. 권한 응답·도구 후보 조회·작업 영역 동기화는 "다른 창 소유" 검사가 있지만, 프롬프트 전송·조향·현재 프롬프트 취소 후 전송·권한 모드 변경·run 취소는 어느 창에서 온 run id든 받는다. MCP 교환·제목 도구는 run → 창 label을 찾아 서비스를 직접 부른다.

정본 결정 7은 "창 label은 도메인 정체가 아니다"이다. 이 spec은 창 label을 서버가 소유하는 **작업대(Bench)** 와 데스크톱만 아는 "창 → 작업대" 대응으로 분해하고, 그 위에서 run 8개·교환 4개 command와 MCP 교환·제목 도구를 `Workbench`로 옮긴다.

| 범위 | 039(2a, 완료) | 이 spec(040, 2b-1) | 다음(041, 2b-2) |
|---|---|---|---|
| 작업대 | — | **도입**(열기·닫기, 소유 검사) | orchestration 바인딩에 사용 |
| run | 이벤트 발행 전환 | **command 8개 이관**, 소유 = 작업대 | 자식 worker run 소유 |
| 교환 | 스키마 예약 | **command 4개 이관 + 교환 스트림 개방** | — |
| MCP 도구 | — | 교환·제목 도구가 **agent principal로 `Workbench.call`** | orchestration 도구 |
| 표현 요청(창 제목) | — | **작업대 알림 스트림** | — |
| orchestration | 스키마 예약 | 변경 없음(창 → 작업대 대응표를 통해 run을 엶) | **command 18개 이관 + 저장 형식 변경 + 스트림 개방** |

## User Scenarios & Testing *(mandatory)*

### User Story 1 - run의 소유와 제어가 창이 아니라 작업대로 판단된다 (Priority: P1)

데스크톱 사용자는 세션 창에서 agent run을 시작하고, 프롬프트를 보내고, 조향·취소·권한 응답을 한다. 이 동작이 창 label 없이 작업대를 지정한 `Workbench` 호출로 이루어지고, 테스트 경로(창이 없는 호출자)도 작업대를 열어 같은 결과를 얻는다.

**Why this priority**: 가장 많이 쓰이고, 교환(run의 작업대로 작업 영역을 찾음)과 041 orchestration(자식 run 소유)이 모두 기대는 기반이다.

**Independent Test**: 메모리 내 경로와 테스트 HTTP 경로에서 작업대 열기 → run 시작 → 프롬프트 → 권한 응답 → 취소 → 작업대 닫기 fixture를 실행해 결과가 같은지 비교한다(agent는 스크립트형 가짜 launcher). 다른 작업대·다른 principal로 같은 run을 제어하면 거절되는지, 데스크톱 화면의 run 동작이 이전과 같은지 확인한다.

**Acceptance Scenarios**:

1. **Given** 작업대 A에서 시작한 run, **When** A로 프롬프트 전송·조향·현재 프롬프트 취소 후 전송·권한 모드 변경·권한 응답·도구 후보 조회·취소를 하면, **Then** 오늘과 같은 결과가 나오고 run 이벤트는 A의 창에 전달된다.
2. **Given** 작업대 A의 run, **When** 작업대 B로 같은 run을 제어하면, **Then** 권한 오류로 거절되고 run 상태는 바뀌지 않는다(오늘 검사가 없던 5종 포함).
3. **Given** principal P가 연 작업대, **When** 다른 principal이 그 작업대 식별자로 호출하면, **Then** 거절된다.
4. **Given** 작업대 A에 진행 중인 run, **When** A를 닫으면, **Then** 오늘 창 닫힘처럼 run이 취소되고, 다시 닫아도 성공한다.
5. **Given** 작업대 A의 구독 연결이 끊김, **When** 잠시 뒤 다시 구독하면, **Then** run은 계속 진행 중이다.
6. **Given** 데스크톱 세션 창, **When** 사용자가 run을 쓰는 모든 조작을 하고 창을 닫으면, **Then** 화면 동작·오류 문구가 이전과 같다.
7. **Given** 조회 전용 호출자, **When** 작업대를 열거나 run을 시작·제어하면, **Then** 권한 오류로 거절된다.

---

### User Story 2 - 두 agent 패널 사이의 교환이 작업대 기준으로 동작하고 그 작업대에만 전달된다 (Priority: P2)

사용자는 한 세션 창의 두 패널에서 agent 사이에 요청을 주고받는다(교환). 패널 스냅샷과 교환 이력이 작업대에 묶이고, 교환 요청·상태는 그 작업대의 교환 스트림으로만 전달된다. agent가 MCP 교환 도구를 불러도 같은 경로를 탄다.

**Why this priority**: 여러 창을 열었을 때 다른 창이 교환 요청을 가로채 거절하는 실제 결함을 없앤다. 창 label을 registry 키로 쓰는 가장 작은 도메인이라 작업대 모델을 검증하기 좋다.

**Independent Test**: 교환 fixture(동기화·전송·확인·목록)를 두 경로에서 실행하고, 교환 스트림 구독 fixture로 요청·상태 이벤트가 해당 작업대에만 가는지 확인한다. agent principal로 교환 operation을 불러 같은 작업대에 반영되는지, 같은 요청의 확인이 두 번 와도 한 번만 적용되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 세션 창 두 개(작업대 A·B), **When** A의 패널이 교환을 보내면, **Then** A의 창만 요청을 받고 한 번만 라우팅·확인하며, B의 창은 아무것도 받지 않는다.
2. **Given** 같은 교환 요청의 확인이 두 번 도착, **When** 처리하면, **Then** 첫 확인만 적용되고 상태 이벤트는 한 번 나간다.
3. **Given** 작업대가 닫힘, **When** 그 작업 영역을 조회하면, **Then** 오늘처럼 스냅샷·이력이 사라져 있다.
4. **Given** agent가 MCP 교환 도구를 부름, **When** 실행 토큰의 run이 작업대 A 소유이면, **Then** A의 작업 영역에 반영되고 도구 오류 문구는 오늘과 같다.

---

### User Story 3 - agent의 창 제목 요청이 그 작업대의 창에만 적용된다 (Priority: P3)

agent가 MCP 제목 도구로 창 제목을 바꾸면, 그 run이 속한 작업대의 창 제목만 바뀐다. 서버는 제목을 저장하지 않고 "제목 요청이 있었다"는 알림만 보낸다.

**Why this priority**: 사용 빈도는 낮지만 같은 방송 결함을 가진 마지막 경로이고, 정본 2단계의 "표현 요청 이벤트"의 첫 사례다.

**Independent Test**: agent principal로 제목 요청 operation을 부르고, 작업대 알림 스트림 구독 fixture로 요청 이벤트가 해당 작업대에만 오는지 확인한다. 데스크톱에서 창 두 개를 열어 한 쪽 agent가 제목을 바꿀 때 다른 창 제목이 그대로인지 확인한다.

**Acceptance Scenarios**:

1. **Given** 작업대 A의 run, **When** 그 agent가 제목 도구를 부르면, **Then** A의 창 제목만 바뀐다.
2. **Given** run이 활성 상태가 아니거나 작업대가 닫힘, **When** 제목 도구를 부르면, **Then** 오늘과 같은 "활성 run이 아님" 계열 오류가 난다.

---

### User Story 4 - 새 operation·이벤트·principal이 계약 조회·생성 타입·계약 테스트에 포함된다 (Priority: P4)

클라이언트 개발자는 계약 조회로 작업대·run·교환 operation과 교환·작업대 이벤트 스키마를 보고, 생성된 타입으로 잘못된 입력·짝을 실행 전에 검출한다.

**Why this priority**: 3단계 HTTP/WebSocket과 4단계 데스크톱 전환이 같은 계약을 쓰게 하는 장치다. 사용자 가치는 간접적이다.

**Independent Test**: 계약 조회 결과에 새 operation과 이벤트 스키마가 있고, 조회 전용·agent principal에게는 허용된 것만 보이며, 생성 타입 상관 테스트가 통과하고, 정의 하나를 바꾸면 drift 검사가 실패하는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 데스크톱 principal, **When** 계약을 조회하면, **Then** 작업대·run·교환 operation과 교환·작업대 이벤트 스키마가 포함되고 각 operation의 멱등 규칙 종류(영속·세대 범위)가 드러난다.
2. **Given** 조회 전용 또는 agent principal, **When** 계약을 조회하면, **Then** 그 principal에 허용된 operation·스트림만 보인다.

---

### Edge Cases

- **창이 두 번 닫힘 이벤트를 받을 때**: 작업대 닫기는 멱등이라 두 번째도 성공한다.
- **작업대를 처음 쓰기 전에 창이 닫힐 때**: 연 작업대가 없으므로 닫을 것도 없다.
- **같은 Worktree에 세션 창 두 개**: 작업대 두 개가 생기고 run·교환은 서로 독립이다.
- **run이 이미 끝났을 때 권한 응답**: 오늘 문구("unknown or finished run")를 유지한다.
- **작업대가 닫힌 뒤 늦게 도착한 run 이벤트**: run 스트림에는 기록되지만(039 규칙), 삭제된 교환 작업 영역에는 반영하지 않는다.
- **같은 멱등성 키로 재시도**: 영속 operation(`run.start`)은 재시작 뒤에도 같은 결과, 세대 범위 operation은 같은 세대 안에서만 같은 결과를 돌려준다.
- **재시작 직후**: 작업대·run 소유·교환 작업 영역은 모두 없다. `pending`으로 남은 `run.start`는 `unknown`으로 판정하고 그 run은 실행 정보 유실로 본다. 이전 작업대 식별자로 호출하면 `notFound`다.
- **MCP 실행 토큰의 run이 끝났거나 작업대가 닫혔을 때**: 오늘처럼 "활성 run이 아님" 계열 오류. 토큰 폐기 시점은 오늘과 같다.
- **orchestration이 자식 run을 열 때(041 전)**: orchestration은 AW에 남아 있고, 데스크톱 어댑터의 "창 → 작업대" 대응으로 그 창의 작업대에서 run을 연다. 화면 동작은 바뀌지 않는다.
- **교환 요청이 replay로 다시 전달될 때**(나중에 구독한 클라이언트): 확인은 요청 식별자 기준으로 한 번만 적용된다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 시스템은 run과 교환 작업 영역을 소유하는 **작업대**를 제공해야 한다. 작업대 열기는 Worktree를 받아 서버가 발급한 작업대 식별자를 돌려주고, 작업대 닫기는 소유 run을 취소하고 교환 작업 영역을 삭제한다. 닫기는 멱등이다. 작업대는 메모리에만 있으며 재시작하면 사라진다.
- **FR-002**: 작업대는 연 principal에 묶여야 하며, 다른 principal이 그 식별자로 호출하면 거절되어야 한다.
- **FR-003**: run·교환의 모든 제어 operation은 작업대 식별자를 받고 "대상 run이 이 작업대 소유인가"를 검사해야 한다. 오늘 검사가 있던 동작(권한 응답, 도구 후보 조회, 작업 영역 동기화)은 오류 문구를 유지하고, 새로 검사하는 동작(프롬프트 전송, 조향, 현재 프롬프트 취소 후 전송, 권한 모드 변경, run 취소)은 새 문구를 쓴다.
- **FR-004**: 창 label은 서버 계약·도메인 타입·core 저장 어디에도 나타나지 않아야 한다. 데스크톱은 세션 창마다 작업대를 **처음 쓸 때** 열고, "창 → 작업대" 대응은 데스크톱 어댑터 안에서만 관리한다. 화면 코드는 작업대를 알 필요가 없다.
- **FR-005**: 데스크톱은 세션 창이 닫힐 때 그 창의 작업대를 닫아야 한다. 이벤트 구독 연결이 끊기는 것만으로는 작업대도 run도 끝나지 않아야 한다.
- **FR-006**: run command 8개(도구 후보 조회, 시작, 취소, 프롬프트 전송, 조향, 현재 프롬프트 취소 후 전송, 권한 모드 변경, 권한 응답)는 `Workbench.call` operation으로 이관되어야 한다. run 이벤트는 039의 run 스트림으로 발행되고, 데스크톱에는 run이 속한 작업대의 창으로 전달되어야 한다.
- **FR-007**: 교환 command 4개(작업 영역 동기화, 전송, 확인, 목록)는 operation으로 이관되어야 한다. 교환 요청·상태 이벤트는 작업대별 교환 스트림(상태 복원용, 보관 한도 있음) 하나로 발행되고, 데스크톱에는 그 작업대의 창에 창 삽입 경로 **하나로만** 전달되어야 한다. 교환 확인은 요청 식별자 기준으로 멱등이어야 한다. 작업대를 닫으면 교환 스트림도 정리된다.
- **FR-008**: MCP 실행 토큰은 run에 묶인 **agent principal**로 인증되어야 하며, MCP 교환·제목 도구는 `Workbench.call`을 거쳐야 한다. 작업대는 호출한 run의 소유로 서버가 찾는다. 도구 권한 규칙(run 일치 검사)과 오류 문구는 오늘과 같아야 한다. agent principal은 교환 쓰기와 표현 요청에 필요한 scope만 가진다.
- **FR-009**: 창 제목 요청은 서버 상태를 바꾸지 않고 작업대 알림 스트림(알림용)에 제목 요청 이벤트로 발행되어야 하며, 데스크톱은 그 작업대의 창에 창 삽입 경로 하나로 전달해 적용해야 한다.
- **FR-010**: 권한은 도메인별 scope를 따른다: 작업대 열기·닫기, run 시작·제어, 교환 조회, 교환 변경, 표현 요청에 각각 scope를 둔다. run 구독은 039의 run 조회 scope, 교환·작업대 스트림 구독은 각 조회 scope를 쓴다.
- **FR-011**: `run.start`는 1단계 변경 기록(intent-first)을 통과하고 멱등성 키가 필수여야 한다. 기동 시 `pending`으로 남은 `run.start`는 `unknown`으로 판정하고 그 run은 실행 정보 유실로 본다. 나머지 run 제어와 교환·작업대 operation은 **세대 범위 멱등성**(같은 세대 안에서 같은 키의 재시도는 같은 결과)을 쓰며 변경 기록에 쓰지 않는다. 계약 조회는 operation마다 멱등 규칙 종류를 드러내야 한다.
- **FR-012**: 이관된 12개 Tauri command는 `Workbench` 호출로 변환하는 얇은 어댑터가 되어야 하며, 화면에서 보이는 동작·오류 문구가 바뀌지 않아야 한다(FR-003의 새 거절은 화면이 만들지 않는 호출에서만 생긴다). 이동한 run·교환 도메인·서비스·어댑터 코드는 AW에 남지 않아야 한다. 교환·제목의 네이티브 창 이벤트 발행은 제거한다.
- **FR-013**: 메모리 내 경로와 테스트 HTTP 경로는 새 operation·구독 fixture 전부에서 같은 결과·오류·이벤트를 반환해야 한다. run fixture의 agent는 주입 가능한 스크립트형 가짜 launcher로 대체한다.
- **FR-014**: 새 operation·이벤트 스키마·principal 종류는 계약 조회·OpenAPI·생성 타입에 포함되고 drift 검사를 받아야 한다.
- **FR-015**: 이 spec은 orchestration command 18개와 저장 형식, MCP orchestration 도구(041), 운영 HTTP/WebSocket 노출(3단계), 데스크톱의 HTTP 전환(4단계), 데스크톱 표현 상태 command 8개를 변경하지 않아야 한다. `crates/acp-agent-core`와 `packages/agent-client`는 변경하지 않는 것을 목표로 한다(run 소유자는 원래 불투명한 문자열이다).

### Key Entities *(include if feature involves data)*

- **작업대 (Bench)**: Worktree 하나를 대상으로 열린 작업 단위. 식별자, 대상 Worktree, 연 principal, 소유 run 목록, 교환 작업 영역을 가진다(용어집 정의).
- **Run 소유**: run ↔ 작업대. 제어 권한 판단의 근거.
- **교환 작업 영역**: 작업대별 패널 스냅샷과 교환 이력(메모리).
- **Agent principal**: MCP 실행 토큰으로 인증된 호출자. run 하나에 묶이고 교환 쓰기·표현 요청 scope만 가진다.
- **교환 스트림** `exchange:<benchId>`: 요청·상태 이벤트(상태 복원용).
- **작업대 알림 스트림** `bench:<benchId>`: 제목 요청 같은 표현 요청(알림용).
- **Client Instance**: 이벤트를 받는 주체(039 정의 유지). 작업대와 별개이며 아무것도 소유하지 않는다.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: `crates/workbench-protocol`(작업대·run·교환 operation, 이벤트 스키마, agent principal, scope), `crates/workbench-core`(작업대 registry, run·교환 서비스와 포트, 가짜 launcher 주입 지점), `apps/agentic-workbench/src-tauri`(Tauri command 어댑터, "창 → 작업대" 대응, 창 닫힘 시 작업대 닫기, 이벤트 전달, MCP 서버의 principal·호출 경로), `packages/workbench-client`(생성 타입), `docs/`. 화면 코드(`apps/agentic-workbench/src`)는 바뀌지 않는 것을 목표로 한다. `crates/acp-agent-core`·`packages/agent-client`는 불변을 목표로 한다.
- **Frontend layering**: 변경 없음 목표. 교환·제목 수신 어댑터는 오늘도 창 삽입 경로를 듣고 있어 네이티브 이벤트를 없애도 동작한다.
- **Backend boundary**: 작업대·run·교환 도메인과 서비스는 core application, 이벤트 전달·창 대응·MCP transport는 AW inbound/infrastructure. MCP 서버는 AW에 남되 도구 구현은 `Workbench.call`만 부른다.
- **Shared core vs UI**: 순수 core만 공유한다.
- **Persistence and safety**: 영속 상태 변경은 `run.start`의 변경 기록뿐이다. 작업대·run 소유·교환은 메모리. 모든 제어 operation에 작업대 소유 검사, 작업대는 principal에 묶임. agent principal은 최소 scope.
- **Documentation and Storybook**: `docs/workbench-seam.md`의 인벤토리(이연 30 → 18), 작업대·이벤트 전달·표현 요청 절, 정본 진행 각주를 갱신한다. ADR 5건(grill Q10)은 작성 완료. Storybook N/A.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Tauri command 인벤토리의 "2단계로 이연"이 30개에서 18개(orchestration)로 줄고, 이관된 12개는 모두 `Workbench` 호출만 한다.
- **SC-002**: 서버 계약·core 도메인 타입에 창 label이 0건 나타난다(데스크톱 어댑터 제외).
- **SC-003**: 메모리 내 경로와 테스트 HTTP 경로가 새 operation·구독 fixture 전부에서 같은 결과·오류·이벤트를 반환한다.
- **SC-004**: 다른 작업대·다른 principal로 run·교환을 제어하는 시나리오 전부가 거절되고 상태가 바뀌지 않는다(오늘 검사가 없던 5종 포함).
- **SC-005**: 세션 창 두 개를 연 시나리오에서 교환 요청·상태와 제목 요청이 다른 창에 도착하는 경우가 0건이고, 같은 창에 같은 이벤트가 두 번 도착하는 경우가 0건이다.
- **SC-006**: 작업대 구독 연결이 끊겼다 다시 이어져도 run이 취소되는 경우가 0건이고, 작업대를 닫으면 소유 run이 모두 취소된다.
- **SC-007**: 기존 run·교환·MCP 자동 테스트가 기대값 수정 없이 통과한다(검사 추가로 결과가 바뀌는 교차 작업대 시나리오는 문서화).
- **SC-008**: 화면 코드 diff가 0건이고, 화면에서 보이는 run·교환·제목 동작과 오류 문구가 이전과 같다(수동 확인 절차로 검증).

## Clarifications

### Session 2026-09-27 (`/grill-with-docs`)

- Q1 범위: **040(2b-1) = 작업대 + run 8 + 교환 4 + MCP 교환·제목 도구**, **041(2b-2) = orchestration 18 + 저장 형식 변경 + MCP orchestration 도구**. 040 동안 orchestration은 데스크톱 "창 → 작업대" 대응으로 run을 연다.
- Q2 이름: **작업대(Bench)** — "호출자 하나가 Worktree 하나를 대상으로 연 작업 단위. run과 교환 작업 영역을 소유한다". 용어집 `crates/workbench-core/CONTEXT.md`에 추가(Avoid: window, session, workspace).
- Q3 수명: **명시적 `bench.open`/`bench.close`(멱등)**. 데스크톱은 처음 쓸 때 열고 창 `Destroyed`에서 닫는다. 구독 끊김은 작업대·run을 끝내지 않는다. 메모리 전용(ADR `docs/adr/0005`).
- Q4 권한: scope `run:write`·`bench:write`·`exchange:read`·`exchange:write` 신설, **모든 제어 operation에서 작업대 소유 검사**, 작업대는 연 principal에 묶임. 오늘 검사가 있던 동작은 문구 유지, 새로 검사하는 동작은 새 문구(ADR core `0004`).
- Q5 교환 이벤트: `exchange:<benchId>` **상태 복원용 스트림 하나**, 데스크톱은 발행 결과를 **창 삽입 경로 하나로** 전달하고 네이티브 `emit` 제거(039 ADR 0003·0004 적용, ADR 0003에 040 갱신 기록). 확인은 요청 식별자로 멱등.
- Q6 MCP 교환 도구: 실행 토큰 → **agent principal(`PrincipalKind::Agent`)**, `Workbench.call` 경유, 작업대는 run의 소유로 서버가 해석(ADR `docs/adr/0006`).
- Q7 MCP 제목 도구: **`bench.requestTitle`(scope `presentation:write`)** → 알림용 스트림 `bench:<benchId>`의 `bench.titleRequested.v1`, 데스크톱 삽입 경로 하나로 전달(ADR `docs/adr/0007`).
- Q8 멱등성: **`run.start`만 변경 기록**(재시작 뒤 `pending` → `unknown` + 실행 정보 유실), 나머지 run 제어·교환·작업대는 **세대 범위 멱등성**(ADR core `0005`). 확인한 사실: `acp-agent-core`의 run 소유자는 불투명한 문자열이라 `benchId`를 그대로 넣는다(변경 없음).
- Q9 테스트 agent: **가짜 `SessionLauncher`를 `RuntimeAdapters`로 주입**(스크립트형, 프로세스 없음). 실제 spawn 경로는 기존 AW 테스트와 수동 확인.
- Q10 ADR 5건: core `0004`(작업대 소유), core `0005`(`run.start`만 변경 기록), `docs/adr/0005`(창 닫힘 = 명시적 작업대 닫기), `0006`(MCP는 agent principal로 `Workbench.call`), `0007`(표현 요청은 작업대 알림 스트림).

## Assumptions

- **2b 분할**: 040은 run·교환, 041은 orchestration(Clarifications Q1). 041은 작업대 위에서 workspace 바인딩을 옮기고 저장 파일의 창 label을 제거한다.
- **화면 불변**: 교환·제목 수신 어댑터는 오늘도 창 삽입 경로를 들으므로 화면 코드를 바꾸지 않는다. 바꿔야 한다면 수신 어댑터(`entities/*/api`)에 한정한다.
- **메모리 상태**: 작업대·run 소유·교환 작업 영역은 메모리이며 재시작 뒤 사라진다. 서버 재시작 = 데스크톱 재시작인 현재 구조에서는 사용자에게 보이는 차이가 없다.
- **보관 한도**: 교환 스트림의 작업대당 보관 한도와 작업대 수 한도는 plan에서 039 한도 체계에 맞춰 정한다.
- **MCP 서버 위치**: AW에 남는다. 토큰 → agent principal 변환과 도구 → `Workbench.call` 호출만 바뀐다.
- **세대 범위 멱등성의 저장**: 메모리 표(키 → 결과)이며 크기 상한과 만료는 plan에서 정한다.
- **4단계까지의 시리즈 범위**(2026-09-26 결정)와 단계별 PR·squash merge 규칙은 그대로다.
