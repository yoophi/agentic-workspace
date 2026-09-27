# Feature Specification: 창 정체 분해와 run·exchange·orchestration 이관 (서버-클라이언트 전환 2b)

**Feature Branch**: `040-workbench-owners`

**Created**: 2026-09-27

**Status**: Draft

**Input**: User description: "040 workbench-owners (AW 서버-클라이언트 전환 2단계 후반, 2b): 창 label(window_label)에 묶인 소유자 식별을 분해하고, 2단계로 이연된 run 8·exchange 4·orchestration 18 command 30개를 Workbench.call/events 경로로 이관. orchestration·exchange 이벤트 스트림을 구독 가능하게 연다." — 039(2a) 머지(main `54806dc`) 뒤의 정본 2단계 후반. 정본은 [서버-클라이언트 전환 조사](../../docs/client-server-architecture-research.md) §2 "event 모델 통합"·§Invariants 7·8·12, 이연 근거는 [ADR 0001](../../docs/adr/0001-defer-event-bound-commands-to-stage-2.md), 현재 상태는 [Workbench Seam](../../docs/workbench-seam.md).

## 배경과 목적

037·038은 command 31개를, 039는 이벤트 Seam과 watcher 2개를 옮겼다. 남은 30개(run 8·exchange 4·orchestration 18)는 모두 **창 label**을 정체로 쓴다. 2026-09-27 코드 조사 결과, 창 label은 한 값으로 네 가지 역할을 동시에 한다.

| 역할 | 오늘의 동작 | 문제 |
|---|---|---|
| run 소유자 | run 시작 시 호출한 창 label을 소유자로 기록(메모리) | 창이 없는 호출자(CLI·TUI·HTTP)는 run을 시작·제어할 수 없다 |
| exchange 작업 영역 키 | 창 label별로 작업 영역 스냅샷·교환 이력을 보관(메모리) | 같은 작업 영역을 다른 클라이언트가 볼 수 없다 |
| orchestration 바인딩 | workspace가 어느 창에 묶였는지 label로 **디스크에 저장** | 창 label은 열 때마다 새로 만들어지므로 재시작 뒤 절대 일치하지 않고, 복구는 "주인 없는 workspace"를 찾는 우회로 동작한다 |
| 이벤트 전달 대상 | exchange·orchestration 이벤트를 창에 보냄 | 네이티브 창 이벤트는 모든 창에 방송되고 삽입 경로로 한 번 더 가서, 다른 창이 받거나 같은 창이 두 번 받는다(039 ADR 0004와 같은 결함) |

권한 판단도 일관되지 않다. 권한 응답·도구 후보 조회·작업 영역 동기화·orchestration은 "다른 창 소유" 검사가 있지만, 프롬프트 전송·조향·취소·권한 모드 변경은 어느 창에서 온 run id든 받는다. orchestration 런타임 이벤트 replay는 범위 검사가 없다. MCP 도구는 실행 토큰의 주체에 창 label을 담아 workspace 키로 쓴다.

정본 결정 7은 "창 label은 도메인 정체가 아니다. workspace·run·client instance를 구분한다"이다. 이 spec은 창 label을 서버가 소유하는 **소유 단위**와 데스크톱만 아는 **client instance**로 분해하고, 그 위에서 30개 command를 1단계와 같은 절차로 `Workbench`에 옮긴다. exchange·orchestration 이벤트는 039의 이벤트 hub로 발행해 구독 가능하게 연다.

| 범위 | 039(2a, 완료) | 이 spec(040, 2b) |
|---|---|---|
| 이벤트 Seam | 구현 | 사용 |
| run | 이벤트 발행 전환 | **command 8개 이관**, 소유를 창에서 분리 |
| exchange | 스키마 예약 | **command 4개 이관 + 이벤트 스트림 개방** |
| orchestration | 스키마 예약 | **command 18개 이관 + 이벤트 스트림 개방**, 저장된 창 바인딩 제거 |
| MCP 도구 | 변경 없음 | 실행 토큰 주체가 창 label 대신 소유 단위를 가리킴 |
| 운영 HTTP/WebSocket | — | 3단계(변경 없음) |

## User Scenarios & Testing *(mandatory)*

### User Story 1 - run의 소유와 제어가 창이 아니라 서버의 소유 단위로 판단된다 (Priority: P1)

데스크톱 사용자는 세션 창에서 agent run을 시작하고, 프롬프트를 보내고, 조향·취소·권한 응답을 한다. 이 동작이 창 label 없이 `Workbench` 호출로 이루어지고, 테스트 경로(창이 없는 호출자)도 같은 소유 단위를 지정해 같은 결과를 얻는다.

**Why this priority**: 30개 중 가장 많이 쓰이고, exchange·orchestration이 모두 run 소유에 기대므로(교환은 run의 소유자로 작업 영역을 찾고, orchestration 자식 worker는 workspace 소유자로 run을 연다) 이것이 먼저 성립해야 나머지를 옮길 수 있다.

**Independent Test**: 메모리 내 경로와 테스트 HTTP 경로에서 run 시작 → 프롬프트 → 권한 응답 → 취소 fixture를 실행해 결과가 같은지 비교한다. 다른 소유 단위로 같은 run을 제어하면 거절되는지, 데스크톱 화면의 run 동작이 이전과 같은지 확인한다.

**Acceptance Scenarios**:

1. **Given** 소유 단위 A가 시작한 run, **When** A가 프롬프트 전송·조향·취소·권한 모드 변경·권한 응답·도구 후보 조회를 하면, **Then** 오늘과 같은 결과가 나오고 run 이벤트는 A의 구독자에게 전달된다.
2. **Given** 소유 단위 A의 run, **When** 소유 단위 B가 같은 run을 제어하면, **Then** 권한 오류로 거절되고 run 상태는 바뀌지 않는다(오늘 검사가 없던 프롬프트 전송·조향·취소·권한 모드 변경 포함).
3. **Given** 데스크톱 세션 창, **When** 사용자가 run을 쓰는 모든 조작을 하면, **Then** 화면 동작·오류 문구가 이전과 같다.
4. **Given** 조회 전용 호출자, **When** run을 시작하거나 제어하면, **Then** 권한 오류로 거절된다.

---

### User Story 2 - 두 agent 패널 사이의 교환이 소유 단위 기준으로 동작하고 구독으로 전달된다 (Priority: P2)

사용자는 한 세션 창의 두 패널에서 agent 사이에 요청을 주고받는다(교환). 작업 영역 스냅샷과 교환 이력이 창 label이 아니라 소유 단위에 묶이고, 교환 이벤트는 그 소유 단위의 교환 스트림으로 구독한 클라이언트에게만 전달된다.

**Why this priority**: 이벤트 방송 결함(다른 창이 교환 이벤트를 받음)을 없애고, 창 label을 registry 키로 쓰는 가장 작은 도메인이라 소유 단위 모델을 검증하기 좋다.

**Independent Test**: 교환 fixture(동기화·전송·확인·목록)를 두 경로에서 실행하고, 교환 스트림 구독 fixture로 이벤트가 해당 소유 단위의 구독자에게만 가는지 확인한다. MCP 교환 도구가 실행 토큰으로 같은 작업 영역을 찾는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 소유 단위 A의 두 패널 run, **When** 한 쪽이 교환을 보내면, **Then** A의 교환 스트림 구독자만 요청·상태 이벤트를 받고 다른 세션 창은 받지 않는다.
2. **Given** 소유 단위가 끝남(창 닫힘), **When** 그 작업 영역을 조회하면, **Then** 오늘처럼 스냅샷·이력이 사라져 있다.
3. **Given** agent가 MCP 교환 도구를 부르면, **When** 실행 토큰의 run이 소유 단위 A에 속하면, **Then** A의 작업 영역에 반영된다.

---

### User Story 3 - orchestration workspace가 창 없이 식별되고 재시작 뒤 복구된다 (Priority: P3)

사용자는 세션 창에서 orchestration workspace를 시작·위임·재시도·인계하고, 앱을 다시 켜면 이전 workspace를 복구한다. workspace는 저장된 창 label이 아니라 workspace 정체와 소유 단위로 식별되고, 복구 목록과 복구 동작은 "주인 없는 workspace 찾기" 우회 없이 명시적으로 판단된다. orchestration 이벤트는 workspace 스트림으로 구독된다.

**Why this priority**: 가장 큰 묶음(18개)이며 디스크 형식이 바뀌는 유일한 도메인이다. US1의 소유 단위 위에서만 옮길 수 있다.

**Independent Test**: orchestration fixture(시작·조회·위임·자식 채택·보고 수집·입력 응답·자식 명령·취소·재시도·재할당·인계·프롬프트 배포·표현 설정·복구)를 두 경로에서 실행한다. 오늘 형식의 저장 파일을 읽어 복구 목록이 같은지, 재시작 시나리오에서 복구가 같은 결과를 내는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 소유 단위 A가 연 workspace, **When** A가 18개 동작 중 하나를 하면, **Then** 오늘과 같은 결과가 나오고 workspace 스트림 구독자가 갱신 이벤트를 받는다.
2. **Given** 소유 단위 B, **When** A의 workspace를 변경하면, **Then** 오늘의 "다른 창 소유" 오류와 같은 의미의 권한 오류로 거절된다.
3. **Given** 오늘 형식(창 label 포함)의 저장 파일, **When** 새 버전이 기동하면, **Then** 모든 workspace가 "묶이지 않음"으로 읽혀 복구 목록이 오늘과 같고, 파일은 다시 저장될 때 새 형식이 된다.
4. **Given** 소유 단위가 끝남(창 닫힘), **When** 그 workspace를 보면, **Then** 오늘처럼 진행 중이던 노드가 멈춤·런타임 유실로 표시되고 복구 가능 목록에 나타난다.
5. **Given** 조정자·자식 agent가 MCP orchestration 도구를 부르면, **When** 실행 토큰이 workspace A에 묶여 있으면, **Then** 오늘과 같은 도구 권한 규칙으로 A에 반영된다.

---

### User Story 4 - 새 operation과 이벤트가 계약 조회·생성 타입·계약 테스트에 포함된다 (Priority: P4)

클라이언트 개발자는 계약 조회로 run·exchange·orchestration operation과 교환·orchestration 이벤트 스키마를 보고, 생성된 타입으로 잘못된 입력·짝을 실행 전에 검출한다.

**Why this priority**: 3단계 HTTP/WebSocket과 4단계 데스크톱 전환이 같은 계약을 쓰게 하는 장치다. 사용자 가치는 간접적이다.

**Independent Test**: 계약 조회 결과에 새 operation 30개 분과 이벤트 스키마가 있고, 생성 타입 상관 테스트가 통과하며, 정의 하나를 바꾸면 drift 검사가 실패하는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 전체 권한 호출자, **When** 계약을 조회하면, **Then** run·exchange·orchestration operation과 교환·orchestration 이벤트 스키마가 포함된다.
2. **Given** 조회 전용 호출자, **When** 계약을 조회하면, **Then** 조회 operation과 구독 가능한 스트림만 보인다.

---

### Edge Cases

- **창이 닫힐 때**: 오늘의 동작(그 창의 run 취소, 교환 작업 영역 삭제, orchestration workspace 풀어 두기, 노드 멈춤 표시)은 사용자에게 같게 보여야 한다. 다만 이것은 "client instance가 끊김"이 아니라 데스크톱이 소유 단위를 **명시적으로 끝내는** 동작으로 표현한다(정본 결정 8: 연결 끊김은 run 취소가 아니다).
- **같은 run을 두 소유 단위가 제어하려 할 때**: 소유 단위가 아닌 쪽은 거절된다. 관찰(이벤트 구독)은 scope가 있으면 허용된다(정본 결정 12).
- **run이 이미 끝났을 때 권한 응답**: 오늘 문구("unknown or finished run")를 유지한다.
- **소유 단위가 끝난 뒤 늦게 도착한 발행**(자식 run의 마지막 이벤트 등): 발행은 버려지지 않고 스트림에 기록되되 삭제된 교환 작업 영역에는 반영하지 않는다.
- **재시작 직후**: 메모리에 있던 run 소유·교환 작업 영역은 없다. orchestration workspace는 모두 "묶이지 않음"이며 새 소유 단위가 복구로 다시 묶는다. 이전 세대 run은 실행 정보 유실로 표시된다.
- **복구 중인 workspace를 두 창이 동시에 복구하려 할 때**: 한 쪽만 성공하고 다른 쪽은 오늘의 "이 창에 묶을 수 없음"과 같은 의미로 거절된다.
- **MCP 실행 토큰의 run이 끝났거나 소유 단위가 끝났을 때**: 오늘처럼 "활성 run이 아님" 계열 오류를 돌려준다. 토큰 폐기 시점은 오늘과 같다.
- **데스크톱 창 제목 변경(MCP 제목 도구)**: 창 제목은 데스크톱 표현 상태이므로 서버는 "이 소유 단위의 제목 요청" 이벤트를 발행하고 데스크톱이 자기 창에 적용한다.
- **orchestration 런타임 이벤트 replay**: 오늘은 범위 검사가 없다. 이관 뒤에는 run 구독과 같은 `run:read` 규칙을 따른다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 시스템은 run·교환 작업 영역·orchestration 바인딩을 소유하는 서버 측 **소유 단위**를 가져야 한다. 소유 단위는 서버가 발급하는 식별자를 가지며, 호출자는 operation 입력으로 소유 단위를 지정한다. 창 label은 서버 계약·저장 형식·도메인 타입 어디에도 나타나지 않아야 한다.
- **FR-002**: 데스크톱은 세션 창 하나당 소유 단위 하나를 열고, 창 label ↔ 소유 단위 대응은 데스크톱 어댑터 안에서만 관리해야 한다. 화면 코드는 소유 단위를 알 필요가 없어야 한다(오늘도 화면은 창 label을 모른다).
- **FR-003**: 소유 단위를 끝내는 동작은 명시적 operation이어야 하며, 끝내면 오늘 창 닫힘과 같은 결과(소유 run 취소, 교환 작업 영역 삭제, orchestration workspace 풀기와 노드 멈춤 표시)가 나와야 한다. 데스크톱은 창이 닫힐 때 이 동작을 호출한다. 구독 연결이 끊기는 것만으로는 run이 취소되지 않아야 한다.
- **FR-004**: run command 8개(도구 후보 조회, 시작, 취소, 프롬프트 전송, 조향, 현재 프롬프트 취소 후 전송, 권한 모드 변경, 권한 응답)는 `Workbench.call` operation으로 이관되어야 한다. 제어 operation은 모두 run의 소유 단위를 검사해야 한다. 오늘 검사가 있던 동작의 오류 문구는 유지한다.
- **FR-005**: exchange command 4개(작업 영역 동기화, 전송, 확인, 목록)는 operation으로 이관되고, 교환 이벤트(요청·상태)는 소유 단위별 교환 스트림으로 발행되어야 한다. 데스크톱은 자기 소유 단위의 교환 이벤트만 받아야 한다.
- **FR-006**: orchestration command 18개는 operation으로 이관되어야 하며, workspace는 workspace 식별자와 현재 바인딩된 소유 단위로 식별되어야 한다. orchestration 갱신 이벤트는 workspace별 스트림으로 발행되어야 한다. 데스크톱은 자기가 연 workspace의 이벤트만 받아야 한다.
- **FR-007**: orchestration 저장 파일은 창 label을 더 이상 저장하지 않아야 한다. 오늘 형식의 파일은 읽을 수 있어야 하고, 저장된 창 바인딩은 모두 "묶이지 않음"으로 해석되어야 한다. 복구 가능 목록은 오늘과 같은 workspace를 반환해야 한다.
- **FR-008**: MCP 실행 토큰의 주체는 창 label 대신 소유 단위(orchestration 역할이면 workspace)를 가리켜야 한다. 도구 권한 규칙(역할별 허용 도구, run 일치 검사)과 오류 문구는 오늘과 같아야 한다.
- **FR-009**: 창 제목 변경처럼 데스크톱 표현 상태에 닿는 요청은 서버 상태를 바꾸지 않고, 소유 단위 대상의 알림 이벤트로 데스크톱에 전달되어야 한다.
- **FR-010**: 교환·orchestration 이벤트의 분류(상태 복원용/알림용)와 보관 규칙은 계약으로 문서화되어야 한다. orchestration 갱신은 "revision이 바뀌었으니 다시 조회하라"는 의미이므로 화면의 오늘 동작(workspace·revision 필터 후 재조회)과 호환되어야 한다.
- **FR-011**: 이관된 30개 Tauri command는 `Workbench` 호출로 변환하는 얇은 어댑터가 되어야 하며, 화면에서 보이는 동작·오류 문구가 바뀌지 않아야 한다. 이동한 도메인·서비스·어댑터 코드는 AW에 남지 않아야 한다.
- **FR-012**: 변경 operation은 1단계와 같은 멱등성·변경 기록·재시작 판정 규칙을 따라야 한다. 외부 프로세스(agent)를 띄우는 동작처럼 재시작 판정이 불가능한 효과는 판정 규칙을 문서화해야 한다.
- **FR-013**: 메모리 내 경로와 테스트 HTTP 경로는 새 operation·구독 fixture 전부에서 같은 결과·오류·이벤트를 반환해야 한다. 실제 agent 프로세스가 필요한 fixture는 테스트용 agent로 대체한다.
- **FR-014**: 새 operation과 이벤트 스키마는 계약 조회·OpenAPI·생성 타입에 포함되고 drift 검사를 받아야 한다.
- **FR-015**: 이 spec은 운영 HTTP/WebSocket 노출, 데스크톱의 HTTP 전환, daemon, 데스크톱 표현 상태 command 8개를 변경하지 않아야 한다. `crates/acp-agent-core`와 `packages/agent-client`의 변경은 다른 소비 앱(hushline·ask-code)을 깨지 않는 최소 범위여야 한다.

### Key Entities *(include if feature involves data)*

- **소유 단위(가칭)**: 서버가 발급하는 식별자를 가진, run·교환 작업 영역·orchestration 바인딩의 주인. 데스크톱에서는 세션 창 하나에 대응한다. 이름은 grill에서 확정한다(용어집은 "session"을 Provider Session과 혼동되어 피한다).
- **Client Instance**: 이벤트 구독을 여는 주체(039 정의 유지). 소유 단위와 별개이며 run을 소유하지 않는다.
- **Run 소유**: run ↔ 소유 단위. 제어 권한 판단의 근거.
- **교환 작업 영역**: 소유 단위별 패널 스냅샷과 교환 이력(메모리).
- **Orchestration workspace 바인딩**: workspace ↔ 현재 소유 단위(없으면 "묶이지 않음"). 저장 파일에는 바인딩 여부만 의미가 있다.
- **MCP 실행 주체**: 실행 토큰이 가리키는 run·역할·소유 단위(또는 workspace).
- **교환 스트림·orchestration 스트림**: 039에서 이름만 정한 `exchange:<id>`, `orchestration:<workspaceId>`를 구독 가능하게 연다.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: `crates/workbench-protocol`(operation·이벤트 계약), `crates/workbench-core`(run·exchange·orchestration 도메인·서비스·소유 단위, MCP 도구가 쓰는 application 포트), `apps/agentic-workbench/src-tauri`(Tauri command 어댑터, 창 ↔ 소유 단위 대응, 창 닫힘 시 소유 단위 종료, 이벤트 전달), `apps/agentic-workbench/src`(이벤트 수신 경로가 바뀌는 exchange·orchestration 수신 어댑터만), `packages/workbench-client`(생성 타입), `docs/`. `crates/acp-agent-core`의 run 소유 모델 변경은 다른 소비 앱을 깨지 않는 범위로 제한한다.
- **Frontend layering**: 화면 동작은 바뀌지 않는다. 바뀌면 `entities/agent-run/api`(교환 수신)·`entities/agent-orchestration/api`(orchestration 수신)의 수신 어댑터에 한정한다.
- **Backend boundary**: 도메인·서비스는 core application, 저장은 core infrastructure, Tauri 전달·창 대응은 AW inbound/infrastructure. MCP 서버의 위치(AW 유지 또는 core 이동)는 plan에서 정한다.
- **Shared core vs UI**: 순수 core만 공유한다.
- **Persistence and safety**: orchestration 저장 파일의 형식 변경은 읽기 호환을 유지하고, 다시 저장될 때만 새 형식을 쓴다. 제어 operation에 소유 검사를 일관되게 적용한다. 외부 프로세스 기동은 1단계의 intent-first 규칙과 재시작 판정 규칙을 따른다.
- **Documentation and Storybook**: `docs/workbench-seam.md`의 인벤토리(이연 0개), 소유 단위·이벤트 전달 절, 정본 진행 각주를 갱신한다. ADR 후보: 소유 단위 모델, 창 닫힘 = 명시적 소유 단위 종료, orchestration 저장 형식 변경. Storybook N/A(orchestration Storybook 샘플의 `boundWindowLabel` 정리는 필요 시).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Tauri command 71개 중 "2단계로 이연"이 0개가 되고, 이관된 30개 command는 모두 `Workbench` 호출만 한다(도메인·저장·프로세스를 직접 조립하지 않음).
- **SC-002**: 서버 계약·도메인 타입·저장 파일에 창 label이 0건 나타난다(데스크톱 어댑터 제외).
- **SC-003**: 메모리 내 경로와 테스트 HTTP 경로가 새 operation·구독 fixture 전부에서 같은 결과·오류·이벤트를 반환한다.
- **SC-004**: 다른 소유 단위의 run·workspace를 제어하는 시나리오 전부가 거절되고 상태가 바뀌지 않는다(오늘 검사가 없던 run 제어 4종 포함).
- **SC-005**: 교환·orchestration 이벤트가 다른 세션 창에 도착하는 경우가 0건이고, 같은 창에 같은 이벤트가 두 번 도착하는 경우가 0건이다.
- **SC-006**: 오늘 형식의 orchestration 저장 파일로 기동했을 때 복구 가능 목록이 이전 버전과 100% 같고, 재시작 뒤 복구 시나리오가 같은 결과를 낸다.
- **SC-007**: 기존 run·exchange·orchestration·MCP 자동 테스트가 기대값 수정 없이 통과한다(검사 추가로 거절이 바뀌는 cross-window 시나리오는 제외하고 문서화).
- **SC-008**: 화면에서 보이는 run·교환·orchestration 동작과 오류 문구가 이전과 같다(수동 확인 절차로 검증).

## Assumptions

- **범위**: 30개를 한 spec에서 다루되, 스토리 순서(run → exchange → orchestration)대로 독립 검증·커밋한다. 크기 때문에 PR을 나눌지는 grill에서 정한다.
- **창 닫힘 동작 유지**: 정본 결정 8(연결 끊김 ≠ run 취소)을 따르되 사용자에게 보이는 오늘 동작은 유지한다 — 데스크톱이 창을 닫을 때 명시적으로 소유 단위를 끝낸다. "창을 닫아도 run이 계속되는" 동작은 daemon(5단계 이후)과 함께 다룬다.
- **소유 검사 강화**: 오늘 검사가 없던 run 제어 4종에도 소유 검사를 추가한다. 화면은 자기 창의 run만 제어하므로 사용자에게 보이는 변화는 없다.
- **메모리 상태**: run 소유와 교환 작업 영역은 오늘처럼 메모리에 두며 재시작 뒤 사라진다. orchestration workspace만 저장된다.
- **orchestration 저장 형식**: 창 바인딩 필드는 읽을 때 무시("묶이지 않음")하고 쓸 때 생략한다. 되돌릴 때 구버전은 필드가 없는 파일을 "묶이지 않음"으로 읽으므로 호환된다(plan에서 확인).
- **이벤트 전달**: exchange·orchestration 데스크톱 전달은 run과 같은 방식(039 ADR 0003·0004 — 발행 결과를 창 삽입 경로 하나로)을 기본으로 한다. 데스크톱이 구독자로 바뀔지는 grill에서 정한다.
- **MCP 서버 위치**: 이 spec은 MCP 서버의 실행 위치를 바꾸지 않는다. 토큰 주체의 의미만 바꾼다.
- **권한**: run·exchange·orchestration 변경은 새 쓰기 scope를, 조회·구독은 도메인별 조회 scope를 쓴다. 이름은 plan에서 정한다.
- **4단계까지의 시리즈 범위**(2026-09-26 결정)와 단계별 PR·squash merge 규칙은 그대로다.
