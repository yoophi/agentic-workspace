# Feature Specification: orchestration을 작업대 기준으로 이관 (서버-클라이언트 전환 2b-2)

**Feature Branch**: `041-workbench-orchestration`

**Created**: 2026-09-27

**Status**: Draft

**Input**: User description: "041 workbench-orchestration (AW 서버-클라이언트 전환 2b-2): orchestration Tauri command 18개와 MCP orchestration 도구를 작업대(Bench) 기반 Workbench.call 경로로 이관하고 orchestration:<id> 이벤트 스트림을 구독 가능하게 연다. 창 label(boundWindowLabel) 기반 orchestration 소유를 작업대로 바꾸고, 040이 남긴 AW 과도기 대응(작업대→창 label 조회, RunTerminalHook의 orchestration 실패 처리, 창 Destroyed의 release_window)을 core로 옮긴다." — 040(2b-1) 머지(main `c8b41a4`) 뒤의 정본 2단계 마지막 조각. 정본은 [서버-클라이언트 전환 조사](../../docs/client-server-architecture-research.md) §2 "event 모델 통합", 현재 상태는 [Workbench Seam](../../docs/workbench-seam.md) "작업대 (040)"·"2단계 이관 안내 (040 → 041)". 이 작업은 독립 HTTP/WS 서버 + thin desktop client 전환의 한 단계이며, 뒤이어 3단계(HTTP/WS 어댑터)·4단계(화면 전환)·5단계(독립 서버 생명주기)·8단계(구 호환 경로 제거)가 온다.

## 배경과 목적

040까지 run·교환·창 제목은 서버가 발급한 **작업대**가 소유하게 되었지만, **agent orchestration**(Main coordinator가 최대 7개 자식 agent에게 과제를 나눠 주고 보고를 모으는 기능)은 여전히 창에 묶여 있다.

- 영속 orchestration 작업 영역을 창 label(`boundWindowLabel`)로 찾고, 창을 닫으면 label을 지워 "복구 가능" 상태로 만든다. 멱등 기록의 행위자, MCP 권한 토큰, 자식 run의 worktree 감시, 이벤트 대상도 모두 창 label이다.
- 그래서 서버가 창 없이 orchestration을 진행할 수 없고, 창이 없는 클라이언트(CLI·TUI·원격)는 orchestration에 참여할 수 없다.
- 040은 이를 위해 core에 과도기 통로(작업대 → 창 label 역조회, 런타임 내부 접근자 4개, AW에 남은 run 종료 후처리와 창 닫힘 해제)를 열어 두었다. 이 통로가 남아 있는 한 3단계 HTTP 서버는 orchestration을 노출할 수 없다.
- 부수 결함: orchestration 저장소에는 동시 쓰기 보호가 없어, 화면 명령·MCP 도구·백그라운드 전달 작업이 같은 파일을 동시에 읽고-고치고-쓰면 한쪽 변경이 사라질 수 있다. orchestration 갱신 알림은 전체 창에 방송되고 같은 창에 두 번 전달된다.

041은 orchestration 소유를 작업대로 옮기고, orchestration 18개 동작과 MCP orchestration 도구 16개를 서버 계약으로 이관하며, orchestration 이벤트 스트림을 열고, 040의 과도기 통로를 모두 닫는다. 이로써 2단계(이벤트 모델 통합·창 정체 분해)가 끝나고, 서버 계약만으로 AW의 모든 서버 소유 동작을 부를 수 있게 된다.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - orchestration 작업 영역이 창이 아니라 작업대에 묶인다 (Priority: P1)

사용자는 세션 창에서 orchestration을 시작하고, Main coordinator에게 목표를 위임하고, 자식 과제를 취소·재시도·재배정하고, 프롬프트를 여러 agent에 보낸다. 이 모든 동작은 이전과 똑같이 보이지만, 서버는 창이 아니라 그 창의 작업대로 작업 영역을 찾는다. 창을 닫으면 작업대가 닫히며 작업 영역이 "복구 가능"이 되고, 같은 worktree를 다시 열면 복구할 수 있다.

**Why this priority**: orchestration의 모든 동작·MCP 도구·이벤트가 이 소유 모델에 기댄다. 창 label 없이 작업 영역을 찾을 수 있어야 3단계 서버가 orchestration을 노출할 수 있다.

**Independent Test**: 메모리 내 경로와 테스트 HTTP 경로에서 작업대 열기 → orchestration 시작 → coordinator run 연결 → 목표 위임 → 자식 과제 생성·취소·재시도 → 작업대 닫기 → 새 작업대에서 복구 fixture를 실행해 결과가 같은지 비교한다(agent는 스크립트형 가짜 엔진). 다른 작업대·다른 주체로 같은 작업 영역을 조작하면 거절되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 작업대 A가 열려 있고 그 worktree에 orchestration 작업 영역이 없다, **When** A로 orchestration을 시작하면, **Then** 작업 영역이 만들어져 A에 묶이고, 영속 기록에는 창이나 작업대 식별자가 남지 않는다.
2. **Given** 작업 영역 W가 작업대 A에 묶여 있다, **When** 작업대 B(같은 주체)나 다른 주체가 W를 조작하면, **Then** 거절되고 W는 바뀌지 않는다.
3. **Given** W가 A에 묶여 있고 자식 run이 진행 중이다, **When** A가 닫히면, **Then** A 소유 run이 모두 취소되고 W는 "복구 가능"이 되며 노드는 주의 필요 상태로 표시된다(오늘 창 닫힘과 같음).
4. **Given** W가 복구 가능하다, **When** 새 작업대 C로 같은 worktree에서 복구를 요청하면, **Then** W가 C에 묶이고 대기 중이던 명령·알림이 다시 전달된다.
5. **Given** 서버가 재시작되었다, **When** 작업 영역 목록을 조회하면, **Then** 재시작 전 묶여 있던 작업 영역은 모두 복구 가능으로 보인다(작업대는 메모리 전용).
6. **Given** 같은 작업 영역에 화면 명령과 agent 보고가 동시에 들어온다, **When** 둘 다 처리되면, **Then** 두 변경이 모두 남는다(한쪽이 사라지지 않는다).

---

### User Story 2 - agent가 orchestration 도구를 서버 계약으로 부른다 (Priority: P2)

Main coordinator agent는 자식 과제를 만들고, 배정하고, 기다리고, 결과를 모으고, 중단·취소·재시도·재배정한다. 자식 agent는 자기 과제를 조회하고, 진행·결과·막힘을 보고하고, 부모에게 입력을 요청하거나 메시지를 보낸다. 도구의 이름·입력·결과는 이전과 같지만, 서버는 도구를 부른 agent의 run이 그 작업 영역에서 어떤 역할(현재 세대의 coordinator, 특정 과제의 자식)인지를 자기 상태에서 판단한다.

**Why this priority**: orchestration의 실제 흐름은 agent 도구 호출로 진행된다. 도구가 창 label을 요구하는 한 창 없는 서버에서 orchestration이 돌지 않는다.

**Independent Test**: agent principal로 coordinator·자식 도구에 해당하는 operation을 불러 작업 영역에 반영되는지, 이전 세대 coordinator·다른 과제의 자식·다른 작업 영역의 agent가 부르면 거절되는지 fixture로 확인한다. MCP 도구 결과 형태가 이전과 같은지 확인한다.

**Acceptance Scenarios**:

1. **Given** run R이 작업 영역 W의 현재 세대 coordinator다, **When** R이 자식 과제 생성·배정을 요청하면, **Then** 과제가 만들어지고 자식 run이 W의 작업대 아래에서 시작된다.
2. **Given** coordinator가 교대되어 R이 이전 세대가 되었다, **When** R이 coordinator 도구를 부르면, **Then** 거절된다(오늘 권한 토큰 폐기와 같은 결과).
3. **Given** run C가 과제 T의 자식이다, **When** C가 결과를 보고하면, **Then** T가 갱신되고 coordinator에게 알림이 전달된다. **When** C가 다른 과제 T2를 보고하려 하면, **Then** 거절된다.
4. **Given** 같은 요청 id의 보고가 두 번 온다, **When** 처리되면, **Then** 한 번만 적용된다.
5. **Given** coordinator가 자식 완료를 기다린다, **When** 대기 시간 안에 자식이 끝나면, **Then** 결과가 돌아오고, 끝나지 않으면 이전과 같은 시간 초과 결과가 돌아온다.

---

### User Story 3 - orchestration 갱신이 그 작업대에만 한 번 전달된다 (Priority: P3)

orchestration 작업 영역이 바뀔 때(과제 상태, 명령 전달, coordinator 알림) 그 작업 영역을 보고 있는 클라이언트만 갱신 알림을 받는다. 데스크톱에서는 그 작업대의 창에만 한 번 도착하고, 다른 창은 아무것도 받지 않는다. 연결이 끊겼다 이어지면 놓친 갱신을 순번으로 이어 받거나 끊김(gap)을 안다.

**Why this priority**: 여러 창을 열었을 때의 방송·중복 전달을 없애고, 3단계 이후 원격 클라이언트가 같은 스트림을 구독하게 한다.

**Independent Test**: orchestration 스트림 구독 fixture로 작업 영역 변경 이벤트가 순번대로 오고, 다른 작업대·다른 주체의 구독은 거절되며, 작업대가 닫히면 gap으로 끝나는지 확인한다. 데스크톱에서 창 두 개를 열어 한 쪽 orchestration 갱신이 다른 창에 도착하지 않는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 작업 영역 W가 작업대 A에 묶여 있고 A의 연 주체가 W 스트림을 구독한다, **When** W의 과제가 바뀌면, **Then** 구독자는 순번이 1씩 증가하는 갱신 이벤트를 받는다.
2. **Given** 다른 주체나 agent가 W 스트림을 구독하려 한다, **When** 구독을 요청하면, **Then** 거절된다.
3. **Given** 데스크톱 창 두 개가 서로 다른 작업대를 보고 있다, **When** 한 쪽 작업 영역이 바뀌면, **Then** 그 창에만 한 번 도착하고 다른 창은 받지 않는다.
4. **Given** 구독자가 순번 n까지 받았다, **When** 다시 연결해 n 이후를 요청하면, **Then** 보관 한도 안이면 이어 받고 넘어서면 gap을 받는다.

---

### User Story 4 - 창 없는 서버에서 orchestration 후처리가 돈다 (Priority: P4)

자식 run이 끝났을 때 worktree가 과제 중에 바뀌었으면 과제를 실패 처리하고, 대기 중인 명령·coordinator 알림을 전달하고, 동시 실행 한도에 따라 대기 과제를 진행하는 일이 창 없이 서버 안에서 일어난다. 데스크톱은 이 일에 관여하지 않는다.

**Why this priority**: 5단계에서 데스크톱이 꺼져도 서버가 orchestration을 계속하려면, 후처리가 데스크톱 어댑터가 아니라 서버에 있어야 한다. 040이 남긴 과도기 통로를 닫는 조건이다.

**Independent Test**: 가짜 엔진으로 자식 run을 끝내며 worktree 변경 여부를 바꿔 과제 상태가 이전 규칙대로 바뀌는지, 동시 실행 한도를 넘는 과제가 대기했다가 자리가 나면 진행되는지, 서버 계약 밖의 창 역조회 없이 동작하는지 테스트로 확인한다.

**Acceptance Scenarios**:

1. **Given** 자식 run이 과제 도중 worktree를 바꿨다, **When** run이 끝나면, **Then** 과제가 이전과 같은 사유로 실패 처리된다.
2. **Given** 동시 실행 한도가 가득 찼다, **When** 새 과제가 배정되면, **Then** 대기하다가 다른 과제가 끝나면 진행된다.
3. **Given** 서버 core·계약에서 창 label을 찾으면, **Then** 0건이다(데스크톱 어댑터의 "창 ↔ 작업대" 표 제외).

---

### User Story 5 - 새 operation·이벤트가 계약 조회·생성 타입·계약 테스트에 포함된다 (Priority: P5)

3단계 HTTP/WebSocket 어댑터와 4단계 화면 전환이 같은 계약을 쓰도록, orchestration operation·이벤트·권한이 계약 조회, 생성 타입, 계약 테스트에 들어간다.

**Why this priority**: 사용자 가치는 간접적이지만 다음 단계의 전제다.

**Independent Test**: 계약 조회 결과에 새 operation과 이벤트 스키마가 있고, 조회 전용·agent principal에게는 허용된 것만 보이며, 생성 타입 상관 테스트가 통과하고, 정의 하나를 바꾸면 drift 검사가 실패하는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 계약이 생성되었다, **When** 데스크톱 principal이 계약을 조회하면, **Then** orchestration operation과 이벤트 스키마가 모두 보인다.
2. **Given** agent principal, **When** 계약을 조회하면, **Then** agent가 부를 수 있는 orchestration operation만 보인다.

### Edge Cases

- 복구 가능한 작업 영역을 두 작업대가 동시에 복구하려 하면 하나만 성공하고 다른 쪽은 "이미 묶임"으로 거절된다.
- 서버 재시작 전 진행 중이던 자식 run은 재시작 뒤 사라진다. 복구 시 그 노드·과제는 이전처럼 "중단됨/주의 필요"로 표시된다(run은 메모리 전용).
- 자식 run 시작 도중 작업대가 닫히면 run은 남지 않고 과제는 실패 또는 대기로 돌아간다(작업대 입장 경계, 040 규칙).
- coordinator 교대 직후 이전 세대의 대기 중 도구 호출(예: 결과 대기)은 거절되거나 이전과 같은 결과로 끝나며, 새 세대의 상태를 바꾸지 않는다.
- 기존 영속 파일에 남은 창 label 값은 읽을 때 무시되어 모든 작업 영역이 복구 가능으로 보이고, 다음 저장부터 그 필드가 사라진다. 파일 형식은 이전 빌드가 읽을 수 있게 유지한다.
- 이전 빌드의 멱등 기록(행위자 = 창 label)은 새 행위자와 겹치지 않아 재사용되지 않는다. 같은 요청 id의 재전송은 새 기록 기준으로 판단한다.
- 작업 영역이 없거나 이미 다른 작업대에 묶인 상태에서 화면 명령이 오면 오늘과 같은 오류 문구가 돌아간다.
- run 이벤트 재생 요청은 그 run을 소유한 작업대의 주체에게만 허용된다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: orchestration 작업 영역은 실행 중에 정확히 하나의 작업대에 묶이거나(활성) 어디에도 묶이지 않는다(복구 가능). 묶임은 메모리 상태이며 영속 기록에 창·작업대 식별자를 남기지 않는다.
- **FR-002**: 작업대가 닫히면 그 작업대에 묶인 작업 영역은 오늘 창 닫힘과 같은 규칙으로 복구 가능 상태가 된다. 이 처리는 서버 안에서 작업대 닫기의 일부로 일어난다.
- **FR-003**: orchestration 화면 동작 18개(시작/재개, 조회, 복구 가능 목록, coordinator 연결, 목표 위임, 수동 자식 채택, 과제 목록, 보고 수집, 표시 설정, 자식 명령 전달, 입력 응답, 과제 취소·재시도·재배정, coordinator 교대, run 이벤트 재생, 프롬프트 분배, 복구)는 서버 계약 operation으로 제공되고, 데스크톱의 기존 command는 그 operation을 부르는 호환 어댑터가 된다. command 이름·인자·결과·오류 문구는 이전과 같다(창 label 필드 제거는 예외로 문서화).
- **FR-004**: 작업대를 받는 모든 orchestration operation은 작업대 소유(연 주체)와 작업 영역 묶임을 검사한다. 다른 작업대·다른 주체의 작업 영역 조작은 거절되고 상태를 바꾸지 않는다.
- **FR-005**: MCP orchestration 도구 16개(coordinator 10, 자식 6)는 run에 묶인 agent principal로 서버 계약을 부른다. 서버는 그 run이 어느 작업 영역의 현재 세대 coordinator인지, 어느 과제의 자식인지를 자기 상태로 판단해 허용한다. 도구 이름·입력·결과 형태는 이전과 같다.
- **FR-006**: 이전 세대 coordinator, 다른 과제의 자식, 다른 작업 영역의 run이 부르는 agent orchestration operation은 거절된다.
- **FR-007**: orchestration 변경은 동시 요청에서도 잃어버리지 않는다. 같은 작업 영역에 대한 변경은 직렬화되고, 작업 영역 revision은 변경마다 1 증가한다.
- **FR-008**: orchestration의 기존 멱등 규칙(요청 id + payload 지문, 같은 요청은 한 번만 적용)은 유지하되, 행위자 식별은 창 label이 아니라 호출 주체와 작업 영역 기준이다.
- **FR-009**: `orchestration:<작업영역 id>` 스트림을 구독할 수 있다. 작업 영역 갱신·명령 상태·coordinator 알림 변경이 순번 이벤트로 발행되고, 구독은 그 작업 영역이 묶인 작업대의 연 주체만 가능하다. 작업대가 닫히면 구독자는 gap을 받는다.
- **FR-010**: 데스크톱은 orchestration 이벤트를 그 작업대의 창에만 한 번 전달한다(전체 창 방송·중복 전달 제거).
- **FR-011**: 자식 run 종료 후처리(worktree 변경 시 과제 실패), 대기 명령·coordinator 알림 전달, 동시 실행 한도에 따른 과제 대기·진행, 작업 영역 복구 시 재조정은 서버 안에서 일어나며 데스크톱 어댑터에 의존하지 않는다.
- **FR-012**: 040이 남긴 과도기 통로(작업대 → 창 label 역조회를 쓰는 서버 동작, 런타임 내부 접근자, 데스크톱의 run 종료 orchestration 후처리, 창 닫힘 시 별도 해제)는 제거된다. 서버 core·계약에 창 label이 없다.
- **FR-013**: run 이벤트 재생(과거 이벤트 조회)은 그 run을 소유한 작업대의 주체만 할 수 있다.
- **FR-014**: 새 operation·이벤트 스키마·권한은 계약 조회, 생성 OpenAPI·TypeScript 타입, 메모리/HTTP 두 경로 계약 테스트에 포함된다. 조회 전용 principal에게는 조회 operation만, agent principal에게는 agent orchestration operation만 보인다.
- **FR-015**: 기존 영속 orchestration 파일은 그대로 열린다. 남아 있는 창 label 값은 무시되고 다음 저장에서 사라지며, 이전 빌드가 새 파일을 여전히 읽을 수 있다.

### Key Entities *(include if feature involves data)*

- **orchestration 작업 영역**: worktree 하나에서 Main coordinator와 최대 7개 자식 노드, 과제, 보고, 명령, coordinator 알림, 프롬프트 분배, 멱등 기록, 세대를 가진 영속 집합. 활성일 때 작업대 하나에 묶인다.
- **작업대 묶임**: 작업 영역 ↔ 작업대의 메모리 대응. 작업대 닫힘·서버 재시작에 사라진다.
- **coordinator 세대**: 작업 영역의 coordinator run과 그 교대 이력. 현재 세대의 run만 coordinator 권한을 가진다.
- **과제와 자식 run**: 과제는 자식 노드에 배정되고 시도(attempt)마다 자식 run을 가진다. 자식 run은 작업 영역의 작업대가 소유한다.
- **orchestration 이벤트**: 작업 영역 id, revision, 변경 사유, 관련 과제·노드를 담은 순번 이벤트.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: `crates/workbench-protocol`(orchestration operation·DTO·이벤트 스키마·권한), `crates/workbench-core`(orchestration 도메인·서비스·저장소·스케줄러·알림 전달·run 종료 후처리의 이전 대상, 작업대 닫기 연동), `apps/agentic-workbench/src-tauri`(호환 command 어댑터, 이벤트 전달, MCP 서버의 principal·호출 경로, 과도기 통로 제거), `packages/workbench-client`(생성 타입), `docs/`. 화면 코드(`apps/agentic-workbench/src`)는 바뀌지 않는 것을 목표로 한다. `crates/acp-agent-core`·`packages/agent-client`는 불변을 목표로 한다.
- **Frontend layering**: 변경 없음 목표. orchestration 수신 어댑터는 오늘도 창 삽입 경로를 듣고 있어 네이티브 방송을 없애도 동작한다.
- **Backend boundary**: orchestration 도메인·서비스·포트는 core domain/application, 영속 저장·스케줄러 상태는 core infrastructure, Tauri command·이벤트 전달·"창 ↔ 작업대" 표·MCP transport는 AW inbound/infrastructure. MCP 도구 구현은 서버 계약만 부른다.
- **Shared core vs UI**: 순수 core만 공유한다.
- **Persistence and safety**: 영속 orchestration 파일 형식은 유지(창 label 필드만 제거, 이전 빌드 호환). 모든 orchestration 변경은 작업 영역 단위로 직렬화. 작업대 소유·작업 영역 묶임·agent 역할 검사. agent principal은 최소 권한.
- **Documentation and Storybook**: `docs/workbench-seam.md`의 인벤토리(이연 18 → 0), orchestration 절, 정본 진행 각주를 갱신한다. 되돌리기 어려운 결정은 ADR로 남긴다. Storybook N/A.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Tauri command 인벤토리의 "2단계로 이연"이 18개에서 0개가 되고, orchestration command 18개는 모두 서버 계약 호출만 한다.
- **SC-002**: 서버 core·계약에 창 label이 0건 나타나고, 040이 표시한 과도기 통로가 0개 남는다.
- **SC-003**: 메모리 내 경로와 테스트 HTTP 경로가 새 orchestration operation·구독 fixture 전부에서 같은 결과·오류·이벤트를 반환한다.
- **SC-004**: 다른 작업대·다른 주체·이전 세대 coordinator·다른 과제의 자식이 작업 영역을 조작하는 시나리오 전부가 거절되고 상태가 바뀌지 않는다.
- **SC-005**: 같은 작업 영역에 대한 동시 변경 시험(화면 명령과 agent 보고 동시 100회 이상)에서 잃어버린 변경이 0건이다.
- **SC-006**: 세션 창 두 개를 연 시나리오에서 orchestration 갱신이 다른 창에 도착하는 경우와 같은 창에 두 번 도착하는 경우가 0건이다.
- **SC-007**: 기존 orchestration 자동 테스트가 기대값 수정 없이 통과하거나(위치 이동 포함), 바뀐 기대값은 행동 변경으로 문서화된다.
- **SC-008**: 화면 코드 diff가 0건이고, 화면에서 보이는 orchestration 동작과 오류 문구가 이전과 같다(수동 확인 절차로 검증).

## Assumptions

- 작업대 모델·agent principal·세대 범위 멱등성·이벤트 hub는 040 그대로 쓴다. 작업대는 메모리 전용이므로 서버 재시작 뒤 모든 작업 영역은 복구 가능 상태가 된다(오늘 앱 재시작과 같은 사용자 결과).
- orchestration의 영속 저장은 기존 JSON 파일과 작업 영역 내부 멱등 기록·revision을 유지한다. 파일 형식을 SQLite로 바꾸는 일은 이 단계의 범위가 아니다(필요하면 별도 단계).
- agent 권한은 권한 토큰의 주장(작업 영역·노드·과제·세대)이 아니라 서버 상태에서 도출한다. MCP transport의 토큰은 run 식별에만 쓴다.
- 동시 실행 한도는 오늘 값(전체 run 한도 − 1, 최소 1)과 환경 설정을 유지한다.
- 자식 agent 프로필 선택은 오늘과 같은 설정을 따른다.
- 결과 대기 도구의 최대 대기 시간(30초)과 반환 형태는 유지한다.
- 화면 코드가 쓰지 않는 orchestration 결과 필드(창 label)는 제거해도 화면 동작에 영향이 없다고 가정하며, plan에서 화면 타입과 대조해 확인한다.
- 3단계 이후의 실제 HTTP/WS 서버·화면 전환·독립 서버 생명주기·호환 경로 제거는 이 spec의 범위 밖이며 후속 feature에서 같은 절차로 진행한다.
