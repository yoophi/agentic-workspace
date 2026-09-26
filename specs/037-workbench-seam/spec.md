# Feature Specification: Workbench Seam 도입 (서버-클라이언트 전환 1a)

**Feature Branch**: `037-workbench-seam`

**Created**: 2026-09-26

**Status**: Draft

**Input**: User description: "AW Workbench Seam 도입 (서버-클라이언트 전환 1a 단계). 정본 docs/client-server-architecture-research.md 의 1단계 중 첫 세로 slice: workbench-protocol(wire DTO, WorkbenchFault, CallRequest/CallReply, EventEnvelope, operation descriptor)과 workbench-core(Workbench `call`/`events`, typed operation registry, in-memory Adapter) 신설; StorageCoordinator(aggregate lock + revision CAS) 도입; SQLite WAL operation ledger/outbox(pending/applied/failed/unknown, idempotency key) 도입; `project.list`·`system.describe` read operation과 `project.create` mutation 1개를 ledger 경유로 통과; 기존 Tauri command list_projects/create_project를 같은 Workbench.call을 쓰는 얇은 compat Adapter로 교체; test-only HTTP `POST /v1/calls`로 동일 fixture 실행; Tauri·in-memory·HTTP 세 Adapter가 같은 success/error/authorization contract test 통과; OpenAPI 3.1 oneOf request schema + TS OperationMap 생성 spike와 project.list input/output 상관 타입 compile test. 범위 밖: frontend transport 변경, 나머지 command 이관(038), 이벤트 envelope(2단계), production HTTP 노출(3단계), daemon."

## 배경과 목적

AW는 지금 Tauri 데스크톱 앱 하나가 UI·비즈니스 로직·네이티브 프로세스·저장을 모두 소유한다. 정본 설계([서버-클라이언트 전환 조사](../../docs/client-server-architecture-research.md))는 이를 **독립 서버 + 얇은 데스크톱 클라이언트** 구조로 옮기되, 71개 Tauri command를 한 번에 바꾸지 말고 먼저 작은 `Workbench` 인터페이스 뒤로 기존 구현을 모은 뒤 기능별로 전환하라고 권고한다.

이 spec은 그 첫 세로 slice다. 기능을 새로 만들지 않고, **가장 단순한 조회 하나(`project.list`)와 변경 하나(`project.create`)를 새 인터페이스로 통과시켜** 인터페이스·오류·멱등성·동시성·계약 생성의 규칙이 실제 코드에서 성립하는지 검증한다. 뒤이어 038(나머지 command 이관), 2단계(이벤트 통합), 3단계(HTTP/WS), 4단계(Desktop 전환)가 같은 규칙 위에 쌓인다.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 프로젝트 목록·생성이 그대로 동작하고, 데스크톱 밖에서도 같은 결과를 낸다 (Priority: P1)

AW 사용자는 프로젝트 목록을 보고 새 프로젝트를 등록하는 일을 오늘과 똑같이 한다. 화면, 응답 속도, 저장 파일 위치와 형식은 바뀌지 않는다. 동시에 같은 두 기능이 데스크톱 화면을 거치지 않는 두 번째·세 번째 호출 경로(메모리 내 호출, 테스트용 로컬 HTTP 호출)에서도 **같은 입력에 같은 결과와 같은 오류**를 낸다.

**Why this priority**: 이 slice의 존재 이유다. 인터페이스가 세 경로에서 같은 계약을 지키지 못하면 이후 단계(HTTP 전환, CLI)는 전부 다시 포장하는 일이 된다. 반대로 기존 사용자 동작이 조금이라도 달라지면 전환 자체가 신뢰를 잃는다.

**Independent Test**: 기존 프로젝트 목록·생성 UI 테스트가 변경 없이 통과하고, 같은 fixture(성공·검증 실패·권한 없음·없는 operation)를 세 호출 경로에 보내 응답과 오류 코드를 비교한다. 이것만으로도 "Seam이 실제로 깊은가"를 판정할 수 있다.

**Acceptance Scenarios**:

1. **Given** 프로젝트가 N개 등록된 상태에서, **When** 사용자가 AW에서 프로젝트 목록을 열면, **Then** 이전과 동일한 N개 항목이 동일한 순서·내용으로 표시된다.
2. **Given** 같은 상태에서, **When** 테스트 호출자가 메모리 내 경로와 로컬 HTTP 경로로 각각 프로젝트 목록을 요청하면, **Then** 두 응답이 데스크톱이 받은 것과 항목 단위로 일치한다.
3. **Given** 사용자가 AW에서 새 프로젝트를 등록하면, **When** 등록이 끝난 뒤 저장 파일을 보면, **Then** 파일 위치와 형식이 이전 버전이 만든 것과 호환되며 목록에 새 항목이 나타난다.
4. **Given** 존재하지 않는 operation 이름 또는 계약에 어긋난 입력으로 호출하면, **When** 세 경로 어디로 보내도, **Then** 사람이 읽는 메시지가 아닌 **같은 안정적 오류 코드**가 돌아온다.

---

### User Story 2 - 변경 요청은 재시도·중단·동시 실행에도 정확히 한 번만 적용된다 (Priority: P2)

프로젝트 생성처럼 상태를 바꾸는 요청은 네트워크 재시도, 호출자 중단, 앱 강제 종료, 두 호출자의 동시 요청 같은 상황에서도 **한 번만 적용되거나, 적용 여부를 명확히 알 수 있어야** 한다. 사용자는 "생성 버튼을 두 번 눌렀는데 프로젝트가 둘 생겼다"거나 "생성됐는지 알 수 없어 다시 만들었다가 중복됐다"를 겪지 않는다.

**Why this priority**: 정본 재점검이 P0로 지목한 누락이다. JSON 파일 여러 개로는 상태·결과·이벤트를 원자적으로 기록할 수 없어, 이 규칙 없이 변경 요청을 네트워크로 열면 중복 실행이 생긴다. P1 뒤에 두는 이유는 P1이 성립해야 이 규칙을 검증할 경로가 있기 때문이다.

**Independent Test**: 같은 멱등성 키로 생성 요청을 반복 전송하고, 요청 처리 중 세 지점(기록 직후·저장 직후·응답 직전)에서 프로세스를 중단한 뒤 재시작해, 프로젝트 수와 재조회 결과가 계약과 일치하는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 프로젝트 생성 요청이 멱등성 키 K와 함께 성공한 뒤, **When** 같은 키 K와 같은 내용으로 다시 요청하면, **Then** 새 프로젝트가 생기지 않고 첫 요청과 같은 결과가 돌아온다.
2. **Given** 키 K로 요청이 성공한 뒤, **When** 같은 키 K에 **다른 내용**으로 요청하면, **Then** 적용되지 않고 충돌 오류가 돌아온다.
3. **Given** 생성 요청이 접수되어 기록은 남았지만 완료 전에 앱이 강제 종료됐을 때, **When** 재시작 뒤 같은 키로 재조회하면, **Then** 시스템은 자동으로 다시 실행하지 않고 적용 여부를 `적용됨`·`적용 안 됨`·`불명` 중 하나로 정확히 알려 준다.
4. **Given** 두 호출자가 같은 저장 단위를 동시에 바꾸려 할 때, **When** 요청이 겹치면, **Then** 한 번에 하나만 적용되고 나중 요청은 최신 revision 기준으로 처리되거나 stale revision 오류로 거절된다. 어느 변경도 조용히 사라지지 않는다.
5. **Given** 성공 응답을 받은 뒤, **When** 곧바로 목록을 다시 조회하면, **Then** 방금 만든 프로젝트가 반드시 보인다(응답이 저장보다 먼저 나가지 않는다).

---

### User Story 3 - 새 클라이언트 개발자가 계약을 조회하고 타입 안전하게 호출한다 (Priority: P3)

앞으로 CLI·TUI·서버 클라이언트를 만들 개발자는 "어떤 operation이 있고 입력·출력이 무엇인지"를 코드에서 뒤지지 않고 **시스템에 물어서** 알 수 있다. 생성된 타입으로 `project.list`를 호출하면 입력과 출력의 짝이 맞지 않는 실수가 실행 전 컴파일 단계에서 잡힌다. 자기에게 허용되지 않은 operation은 목록에 나오지도, 호출되지도 않는다.

**Why this priority**: 4단계 Desktop 전환과 6단계 CLI가 이 계약 생성물 위에 서지만, 이 slice에서는 `project.list` 하나로 "생성 파이프라인이 원하는 상관 타입을 만드는가"만 확인하면 된다. P1·P2가 성립한 뒤 붙여도 가치가 훼손되지 않는다.

**Independent Test**: 계약 조회 결과에 `project.list`·`project.create`·계약 조회 자체가 입력·출력 스키마와 함께 나오는지, 생성된 타입으로 잘못된 출력 타입을 기대하는 코드가 컴파일에 실패하는지, 권한이 좁은 호출자에게 `project.create`가 목록에서 빠지고 호출도 거절되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 개발자가 계약 조회 operation을 호출하면, **When** 응답을 보면, **Then** 자신에게 허용된 operation 각각의 이름, 종류(조회/변경), 입력·출력 스키마, 멱등성 필요 여부가 담겨 있다.
2. **Given** 계약 정의에서 생성한 타입을 쓰는 코드가 있을 때, **When** `project.list`의 출력을 다른 operation의 출력 타입으로 다루면, **Then** 컴파일 단계에서 오류가 난다.
3. **Given** 계약 정의를 바꾸고 생성물을 갱신하지 않았을 때, **When** 저장소 검증을 실행하면, **Then** 생성물 불일치로 실패한다.
4. **Given** 변경 권한이 없는 호출자가, **When** 계약을 조회하고 `project.create`를 호출하면, **Then** 목록에 그 operation이 없고 호출은 권한 오류로 거절된다.

---

### Edge Cases

- **멱등성 키 없이 변경 요청**: 변경(mutation) operation은 키를 필수로 요구하고, 없으면 입력 검증 오류로 거절한다. 조회 operation은 키를 요구하지 않는다.
- **요청 ID와 멱등성 키를 같은 값으로 보낸 경우**: 둘은 다른 수명의 식별자다. 요청 ID는 시도마다 새로 만들고 멱등성 키만 재시도에 재사용한다. 계약 조회 결과와 오류 응답이 둘을 구분해 돌려준다.
- **멱등성 결과의 보존 기간이 지난 뒤 재시도**: 보존 기간(TTL)이 지나면 새 요청으로 처리된다. 기간은 계약에 기록되고 응답에 만료 시각이 포함된다. 진행 중인 기록은 만료되지 않는다.
- **변경 기록 저장소가 잠겨 있거나 열 수 없는 경우**: 변경 요청은 일시 불가 오류로 거절되고 조회는 계속 동작한다. 조용히 JSON에만 쓰지 않는다. 단, 상태 변경이 **이미 저장된 뒤** 기록 확정만 실패하면 `적용 안 됨`이 아니라 `불명`으로 알리고 진행 기록을 남겨 재시작 시 판정한다(FR-009).
- **기존 프로젝트 저장 파일이 손상된 경우**: 목록 조회는 기존과 같은 방식으로 실패를 알린다. 이 slice가 새 실패 모드를 만들지 않는다.
- **호출자 식별 정보가 없는 호출**: 인증되지 않은 호출로 거절한다. 데스크톱 경로는 앱이 만든 데스크톱 호출자 정체로 호출하므로 사용자에게는 보이지 않는다.
- **알 수 없는 operation 버전이나 입력에 계약에 없는 필드**: 알 수 없는 operation은 거절한다. 출력의 추가 필드는 호출자가 무시할 수 있어야 하지만 입력은 계약대로 엄격히 검증한다.
- **응답 출력이 계약 스키마를 어긋나는 경우**(구현 결함): 개발·CI 검증에서 출력도 스키마로 검사해 잡는다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 시스템은 프로젝트 목록 조회, 프로젝트 생성, 계약 조회를 **하나의 호출 인터페이스**(operation 이름 + 구조화된 입력 → 결과 또는 오류)로 제공해야 한다.
- **FR-002**: 데스크톱의 기존 프로젝트 목록·생성 기능은 이 인터페이스만을 통해 동작해야 하며, 사용자에게 보이는 동작·저장 파일 위치·저장 형식은 변하지 않아야 한다.
- **FR-003**: 데스크톱 경로, 메모리 내 경로, 테스트용 로컬 HTTP 경로 세 호출 경로는 같은 입력에 대해 같은 결과와 같은 오류 코드를 반환해야 한다.
- **FR-004**: 시스템은 호출자에게 허용된 operation 목록과 각 operation의 종류, 입력·출력 계약, 멱등성 요구 여부를 조회하는 operation을 제공해야 한다. 목록에 없는 operation 호출은 거절해야 한다.
- **FR-005**: 모든 입력은 operation 계약 검증을 통과한 뒤에만 처리되어야 하며, 위반 시 안정적인 오류 코드로 거절해야 한다.
- **FR-006**: 오류 응답은 최소한 안정적 오류 코드, 재시도 가능 여부, 적용 여부(`적용 안 됨`·`적용됨`·`불명`), 요청 ID를 포함해야 한다. 오류 코드 집합은 정본 문서의 코드 표를 따른다.
- **FR-007**: 변경 operation은 멱등성 키를 필수로 요구해야 한다. 같은 키와 같은 내용의 재요청은 새 부작용 없이 같은 결과를 반환하고, 같은 키와 다른 내용은 충돌 오류로 거절해야 한다.
- **FR-008**: 변경 operation은 **부작용 전에 의도(`대기` 기록)를 내구성 있게 남기고**, 상태 변경과 새 revision을 적용한 뒤, 멱등성 결과를 `적용됨`으로 기록한 다음에만 성공 응답을 반환해야 한다. 이 세 기록은 순서가 보장되어야 하며, 어느 단계에서 중단되어도 재시작 시 FR-009의 판정이 가능해야 한다.
- **FR-009**: 변경 진행 기록은 `대기`·`적용됨`·`실패`·`불명` 상태를 가져야 한다. 중단 뒤 재시작 시 미확정 기록은 자동 재실행하지 않고 `불명`으로 판정하여 재조회 시 그 사실을 반환해야 한다.
- **FR-010**: 같은 저장 단위를 바꾸는 동시 요청은 한 번에 하나만 적용되어야 한다(lost update 없음). 호출자가 기대 revision을 보냈고 불일치하면 거절해야 한다.
- **FR-011**: 요청 ID(시도별 추적)와 멱등성 키(부작용 중복 제거)는 별도 필드로 받고 별도 수명으로 다뤄야 한다.
- **FR-012**: 계약 정의 한 곳에서 전송 계약 문서와 클라이언트용 타입 정의가 생성되어야 하며, `project.list`의 입력과 출력이 짝지어진 타입으로 표현되어 불일치가 실행 전(컴파일) 단계에서 검출되어야 한다. 정의와 생성물이 어긋나면 저장소 검증이 실패해야 한다.
- **FR-013**: 호출자 정체와 권한은 인증 계층만 부여하며, 호출 입력으로 지정할 수 없어야 한다. 계약 조회·호출 모두 같은 권한 판단을 사용해야 한다.
- **FR-014**: 이 slice는 데스크톱 화면, 프론트엔드 통신 방식, 프로젝트 외 다른 기능, 이벤트 전달 방식, 운영 환경의 HTTP 노출을 변경하지 않아야 한다.

### Key Entities

- **Operation**: 호출 가능한 기능 하나. 안정적 이름, 종류(조회/변경), 입력·출력 계약, 필요한 권한, 멱등성 요구 여부, 외부 노출 여부를 가진다. 이 slice에는 `project.list`, `project.create`, `system.describe` 세 개가 있다.
- **호출 요청과 응답**: operation 이름, 요청 ID, 입력, 선택적 멱등성 키·기대 revision·시간 제한으로 이루어진 요청과, 결과(완료 또는 접수됨)·새 revision 또는 오류로 이루어진 응답.
- **오류(Fault)**: 안정적 코드, 안전한 메시지, 재시도 가능 여부, 적용 여부, 요청 ID, 선택적 세부 정보.
- **변경 진행 기록(Operation ledger 항목)**: 호출자·operation·멱등성 키·정규화된 입력 지문·상태(`대기`/`적용됨`/`실패`/`불명`)·실행 ID·결과·만료 시각.
- **저장 단위 revision**: 프로젝트 저장소처럼 하나로 묶여 바뀌는 저장 단위의 단조 증가 버전. 동시성 제어와 기대 revision 검사에 쓴다.
- **호출자(Principal)**: 호출 주체의 종류와 권한 범위. 이 slice에서는 데스크톱 호출자와 테스트용 호출자(전체 권한·조회 전용)만 존재한다.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: 새 Rust crate 두 개를 `crates/`에 둔다. 하나는 wire 계약(요청·응답·오류·operation 정의)만, 하나는 `Workbench` 인터페이스와 구현(레지스트리, 저장 조정, 변경 기록)을 담는다. 소비자는 지금 `apps/agentic-workbench/src-tauri` 하나지만, 정본 설계상 서버·CLI·TUI가 같은 crate를 소비할 예정이고 이 slice의 **세 경로 공통 contract test가 재사용성을 검증하는 공통 fixture** 역할을 한다(Principle I의 예외 조건 충족). TypeScript 쪽은 `packages/workbench-client`에 생성 타입 골격만 만든다. 기존 `packages/agent-client`는 hushline·ask-code가 공유하므로 건드리지 않는다.
- **Frontend layering**: UI 변경 없음. `entities/project/api/project-repository.ts`를 포함한 프론트 코드는 그대로다. 생성 타입 패키지는 이 slice에서 어떤 앱도 import하지 않는다.
- **Backend boundary**: 프로젝트 도메인(`Project`·`ProjectDraft`), 비즈니스 규칙(`project_service`), `ProjectRepository` port, JSON 저장 어댑터를 AW에서 core crate로 **옮긴다**. 저장 어댑터는 앱 핸들 대신 데이터 경로를 생성자로 주입받는다. 이 이동이 038에서 나머지 도메인을 옮길 때의 템플릿이다. `inbound/tauri_commands.rs`의 `list_projects`·`create_project`는 호출 요청을 만들고 응답을 되돌리는 얇은 호환 어댑터가 되며, 오류는 기존과 같은 문자열 형태와 문구를 유지한다. 저장 조정과 변경 기록 저장소는 core crate의 infrastructure 모듈에 둔다. 새 crate의 domain·port 층은 Tauri·파일시스템·저장 기술에 의존하지 않는다.
- **Shared core vs UI**: 공유 UI 없음. 공유는 순수 계약과 core만이다.
- **Persistence and safety**: 프로젝트 JSON 저장 파일은 형식과 위치를 유지하고, 서버 런타임이 유일한 쓰기 주체가 된다. 읽기 경로는 저장 파일에 쓰지 않으며, 손상 파일의 백업 복구는 변경과 같은 잠금 아래에서만 수행한다(조회가 진행 중인 변경을 덮어쓰지 않도록). 변경 기록 저장소는 앱 데이터 디렉터리 아래 서버 소유 파일로 두고 사용자 프로젝트 디렉터리에는 쓰지 않는다. 모든 호출은 인증 계층이 만든 호출자 정체를 요구하며, 입력으로 정체나 권한을 지정할 수 없다. 테스트용 HTTP 경로는 테스트 실행 중에만 임의 loopback 포트에 열리고 운영 빌드에는 노출되지 않는다.
- **Documentation and Storybook**: Storybook 대상 없음. `docs/`에 Workbench Seam의 계약(요청·응답·오류 코드·멱등성 규칙)과 038 이후 이관 절차를 한국어로 기록하고, 정본 문서에 1a 완료 상태를 반영한다. Mermaid로 세 호출 경로와 Seam의 관계를 그린다.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 프로젝트 목록·생성 관련 기존 자동 테스트가 수정 없이 100% 통과하고, 사용자가 두 기능을 쓸 때 이전 버전과 차이를 느끼지 못한다(목록 표시 지연 증가 50ms 이내).
- **SC-002**: 성공·입력 검증 실패·권한 거절·없는 operation·멱등성 충돌·stale revision을 포함한 공통 fixture 전부에 대해 세 호출 경로의 결과와 오류 코드가 100% 일치한다.
- **SC-003**: 같은 멱등성 키로 재시도하는 fixture에서 프로젝트 중복 생성이 0건이고, 처리 중 세 지점에서 중단·재시작하는 fixture 전부에서 적용 여부 판정이 계약과 100% 일치한다.
- **SC-004**: 동시에 20건의 생성 요청을 보내면 정확히 20개가 생기고 저장 단위 revision이 단조 증가하며 사라진 변경이 0건이다.
- **SC-005**: 계약 정의를 의도적으로 바꾼 뒤 생성물을 갱신하지 않으면 저장소 검증이 실패하고, 생성된 타입으로 `project.list` 출력을 잘못 다루는 코드는 컴파일에 실패한다.
- **SC-006**: 변경 권한이 없는 호출자의 계약 조회 결과에 변경 operation이 0개 포함되고 호출 시 100% 거절된다.

## Assumptions

- **범위는 정본 1단계의 첫 세로 slice(1a)로 한정한다.** 나머지 Tauri command 이관은 038, 이벤트 통합은 2단계, 운영 HTTP/WS 노출은 3단계, Desktop 통신 전환은 4단계에서 다룬다. 이 시리즈는 4단계까지이며 daemon·CLI·TUI·MCP 교체는 별도 시리즈다(2026-09-26 확정).
- **변경 기록 저장소는 원자적 커밋이 가능한 로컬 임베디드 데이터베이스(SQLite, WAL 모드)에 둔다.** 정본 재점검이 JSON 파일 여러 개로는 상태·결과를 원자 기록할 수 없다고 P0로 지목했기 때문이다. 도메인 JSON 저장소 10개는 읽기 형식을 유지한다. 구체 라이브러리는 plan에서 확정한다.
- **호출자 정체는 이 slice에서 두 종류만 있다.** 데스크톱 앱이 만든 전체 권한 호출자와, 테스트에서 쓰는 전체 권한·조회 전용 호출자. 실제 토큰 발급·검증은 3단계에서 도입하며, 여기서는 인증 계층이 정체를 부여하고 입력으로 위조할 수 없다는 규칙만 성립시킨다.
- **테스트용 HTTP 경로는 계약 직렬화를 검증하기 위한 것이다.** 운영 빌드에서 포트를 열지 않고, 인증 헤더·Host·Origin 검사 같은 운영 보안은 3단계에서 붙인다.
- **계약 생성 도구는 정본 문서의 선택(OpenAPI 3.1 + TypeScript 타입 생성)을 따른다.** 이 slice는 `project.list` 하나로 생성 파이프라인이 입력·출력 상관 타입을 만드는지 확인하는 spike이며, 결과가 기대와 다르면 plan에서 대안을 기록한다.
- **기존 저장 동시성은 이 slice에서 프로젝트 저장소에만 적용한다.** 다른 JSON 저장소의 lock·revision은 038에서 각 도메인을 이관하면서 같은 조정 계층에 태운다.
- **정본 문서의 오류 코드 표와 멱등성 상태 표를 그대로 채택한다.** 코드 이름·HTTP 대응·상태 전이를 이 spec에서 재정의하지 않는다.
- **기존 의존성의 major 업그레이드는 하지 않는다.** 현재 버전 선에서 동작 동등성을 먼저 만들고, 업그레이드는 별도 변경으로 분리한다.

### 2026-09-26 확정 결정 (plan이 그대로 따를 것)

- **crate 배치**: 계약 crate(`workbench-protocol`: 요청·응답·오류·operation 정의·계약 문서 생성)와 core crate(`workbench-core`: `Workbench` 인터페이스·레지스트리·저장 조정·변경 기록·메모리 내 어댑터) 두 개. 테스트용 HTTP 경로는 core의 테스트 전용 harness로 두고 3단계에서 별도 서버 crate로 승격한다.
- **도메인 이동**: 프로젝트 도메인·서비스·port·JSON 어댑터를 core로 옮기고 core가 `project.*` handler를 등록한다. 메모리 내 경로의 계약 테스트는 AW 없이 core만으로 실행된다.
- **`project.create` 내구성 패턴(intent-first)**: 변경 기록에 `대기`(멱등성 키·정규화 입력 지문·미리 생성한 프로젝트 ID) 기록 → 저장 조정 lock 안에서 JSON 읽기·추가·원자 저장 → 변경 기록을 `적용됨`(결과·새 revision)으로 갱신 → 응답. 앱 시작 시 reconciler가 `대기` 항목의 프로젝트 ID가 JSON에 있는지 보고 `적용됨` 또는 `불명`으로 확정한다. 이 패턴은 038의 worktree 생성·프로세스 시작처럼 외부 부작용이 있는 변경에도 그대로 쓴다.
- **변경 기록 저장소 스키마**: `schema_version`(정수 migration 버전), `operation_ledger`, 그리고 만료·정리 대상이 아닌 `aggregate_revision`(저장 단위별 현재 revision의 정본)을 만든다. revision을 변경 기록에서 유도하면 보존 기간이 지나 정리된 뒤 revision이 되돌아가 FR-010이 깨지므로 별도로 둔다(2026-09-26 설계 리뷰 반영). 이벤트 outbox는 2단계에서 migration v2로 추가한다.
- **계약 생성물 관리**: 계약 문서(`openapi.json`)와 생성된 클라이언트 타입(`packages/workbench-client/src/generated/`)을 저장소에 커밋하고, CI에 "재생성 후 diff가 있으면 실패" 단계를 추가한다.
- **오류 형태 유지**: 호환 어댑터가 된 Tauri command는 문자열 오류를 유지하고 오류 객체의 message만 돌려준다. 입력 검증 오류의 message에는 기존 문구("Project name is required." 등)를 그대로 실어 화면 문구가 바뀌지 않게 한다. 오류 코드·적용 여부 같은 구조화 정보는 4단계 HTTP 경로에서만 노출한다.
- **기본값**: 변경 기록 파일은 앱 데이터 디렉터리 아래 `workbench/ledger.sqlite`(WAL), 앱 시작 시 migration과 reconciler 실행. 멱등성 결과 TTL 24시간, `대기`·`불명` 항목은 만료·정리 대상에서 제외. 호출자는 데스크톱(프로젝트 읽기·쓰기 전체)과 테스트용 조회 전용 하나. 프로젝트 ID는 `대기` 기록 전에 생성해 지문과 함께 저장하므로 재시도에 같은 ID를 돌려준다.
