# Feature Specification: 이벤트 모델 통합 — Workbench 이벤트 스트림 (서버-클라이언트 전환 2a)

**Feature Branch**: `039-workbench-events`

**Created**: 2026-09-27

**Status**: Draft

**Input**: User description: "계속 진행해주세요" — 038(1b) 머지(main `4cb6be5`) 뒤 시리즈 순서상 다음 단계인 **정본 2단계(event 모델 통합)** 의 첫 조각. 정본은 [서버-클라이언트 전환 조사](../../docs/client-server-architecture-research.md) §2 "event 모델 통합"·§Interface의 전체 계약 "Ordering", 이연 근거는 [ADR 0001](../../docs/adr/0001-defer-event-bound-commands-to-stage-2.md), 현재 상태는 [Workbench Seam](../../docs/workbench-seam.md).

## 배경과 목적

1단계(037·038)는 `Workbench.call` 한 동작으로 command 31개를 옮겼다. 정본 Seam의 나머지 절반인 `Workbench.events`는 자리만 있고 항상 "지원 안 함"을 돌려준다. 결과를 이벤트로 흘리는 command 32개(run 8·exchange 4·orchestration 18·watcher 2)는 그래서 1단계에서 옮기지 못했다.

오늘의 이벤트 전달에는 다음 문제가 있다(2026-09-27 코드 조사).

| 문제 | 오늘의 동작 | 결과 |
|---|---|---|
| live와 replay의 정체가 다름 | run journal은 run별 sequence를 매기지만 live 이벤트에는 싣지 않는다. 화면이 `마지막 sequence + 1`로 추정한다 | replay가 끝나기 전에 live 이벤트가 오면 같은 이벤트를 두 번 반영하거나 하나를 건너뛸 수 있다 |
| 구독 경계 없음 | "replay 요청"과 "live 수신"이 서로 다른 경로이고 사이에 동기화가 없다 | replay 기준점과 live 등록 사이의 이벤트가 사라질 수 있다(정본 P0) |
| 창이 수신자이자 소유자 | 이벤트 대상과 run·orchestration·exchange·watcher의 소유를 모두 창 label이 정한다 | 창이 없는 클라이언트(CLI·TUI·HTTP)는 이벤트를 받을 방법이 없다 |
| 서버 재시작 구분 없음 | journal은 메모리에만 있고 세대 표식이 없다 | 재시작 뒤 같은 sequence가 다른 이벤트를 가리킬 수 있다 |
| 전달 방식 이중화 | 이벤트 7종 중 5종이 Tauri 이벤트와 창 스크립트 삽입(fallback) 두 경로로 나간다 | 클라이언트마다 두 경로를 다 들어야 하고, 어느 쪽이 정본인지 불명확하다 |

정본 2단계는 (a) 공통 이벤트 봉투와 구독 계약, (b) 창 label을 workspace·run·client instance 정체로 분해하는 일, (c) 그 위에서 32개 command를 옮기는 일을 모두 포함한다. 037·038이 1단계를 "Seam 성립(1a) → 도메인 이관(1b)"으로 나눈 것처럼, 이 spec은 **2a: 이벤트 Seam 성립**만 다룬다. 창 정체 분해와 run·exchange·orchestration command 30개 이관은 2b(040)로 둔다.

| 범위 | 이 spec(039, 2a) | 다음(040, 2b) |
|---|---|---|
| 이벤트 봉투·구독·세대·gap 계약 | **구현** | 사용 |
| run 이벤트 | **같은 봉투로 발행**(live·replay 동일 sequence), 기존 데스크톱 전달은 새 발행자를 소비 | run command 8개 이관, run 소유를 창에서 분리 |
| worktree 변경 이벤트 | **구독으로 전환**(watcher 시작·중지 command 2개를 구독 수명에 흡수) | — |
| orchestration·exchange 이벤트 | 봉투 스키마만 정의(발행 경로는 그대로) | 발행 전환 + command 22개 이관 |
| 창 label 분해 | 이벤트 전달에서만 client instance로 대체 | domain·worker·session 소유 분해 |

목적은 세 가지다. (1) 정본 "Ordering" 계약(수신자 먼저 등록 → 기준점 → replay → 버퍼 비우기 → live)이 실제 코드에서 누락 없이 성립함을 race test로 확인한다. (2) run 이벤트의 live·replay가 같은 정체를 갖게 해 화면의 추정 로직이 필요 없게 한다. (3) 창이 없는 호출자(테스트 HTTP 경로)가 같은 이벤트를 받을 수 있음을 보여 2b·3단계의 전제를 만든다.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 이벤트를 구독하는 호출자가 재연결해도 이벤트를 잃거나 중복 반영하지 않는다 (Priority: P1)

클라이언트(데스크톱 창, 테스트 HTTP 호출자, 앞으로의 CLI)는 관심 있는 스트림과 마지막으로 반영한 위치(cursor)를 주고 구독한다. 서버는 그 위치 이후의 보관된 이벤트를 먼저 보내고 이어서 새 이벤트를 보낸다. 구독을 시작하는 순간 발행된 이벤트도 정확히 한 번 전달된다. 보관 범위를 벗어났거나 서버가 재시작되어 이어 붙일 수 없으면, 서버는 조용히 건너뛰지 않고 "다시 동기화 필요(gap)" 신호를 보낸다.

**Why this priority**: 정본이 P0로 지목한 결함(replay와 live 사이 유실)을 닫는 핵심이고, 2b의 모든 command 이관과 3단계 WebSocket이 이 계약을 그대로 쓴다. 이것이 성립하지 않으면 뒤의 모든 전환이 같은 결함을 복제한다.

**Independent Test**: 메모리 내 경로와 테스트 HTTP 경로에서 같은 구독 fixture(처음부터·중간 cursor·보관 범위 밖 cursor·다른 세대 cursor·알 수 없는 스트림)를 실행해 받은 이벤트 목록과 gap 신호가 일치하는지 비교한다. 구독 시작 직전·도중·직후에 이벤트를 주입하는 race test를 반복 실행해 누락·중복이 0건인지 확인한다.

**Acceptance Scenarios**:

1. **Given** run R의 이벤트 1–10이 보관된 상태에서, **When** 호출자가 cursor 4로 구독하면, **Then** 이벤트 5–10을 순서대로 받고 이후 발행되는 11부터 live로 받는다.
2. **Given** 구독 처리 도중 이벤트가 계속 발행될 때, **When** 1,000회 반복해 구독을 시작하면, **Then** 매번 받은 sequence가 빈틈없이 연속이고 중복이 없다.
3. **Given** 보관 한도(run당 512개)를 넘어 오래된 이벤트가 지워진 상태에서, **When** 지워진 구간의 cursor로 구독하면, **Then** 이벤트 대신 gap 신호와 현재 보관 범위를 받는다.
4. **Given** 서버가 재시작되어 세대 표식이 바뀐 뒤, **When** 이전 세대의 cursor로 구독하면, **Then** gap 신호를 받는다. 보관 한도로 지워진 run을 구독하면 cursor와 무관하게 gap 신호를 받는다(아직 시작 전인 run과 구별된다).
5. **Given** 구독자가 처리 속도를 따라가지 못해 대기열이 한도를 넘으면, **When** 서버가 이를 감지하면, **Then** 구독을 gap 신호와 함께 닫고 cursor를 임의로 전진시키지 않는다.
6. **Given** 호출자에게 허용되지 않은 스트림을 구독하면, **When** 요청을 보내면, **Then** 1단계와 같은 권한 오류로 거절된다.

---

### User Story 2 - agent run 화면이 live와 replay를 같은 번호로 받아 추정 없이 복원한다 (Priority: P2)

AW 사용자가 agent run을 실행하는 동안 화면은 이벤트를 실시간으로 표시한다. 창을 다시 열거나 화면이 재수화(hydrate)될 때는 놓친 이벤트를 이어 받는다. 이제 실시간 이벤트에도 서버가 부여한 번호가 실려 있어, 화면이 번호를 추정하지 않는다. 사용자에게 보이는 run 진행 표시·메시지·권한 요청·완료 상태는 이전과 같고, 재수화 중 도착한 이벤트가 두 번 표시되거나 빠지는 일이 없다.

**Why this priority**: 사용자가 가장 자주 보는 실시간 화면이며, 오늘 알려진 중복·누락 가능성을 없앤다. US1의 계약을 실제 데스크톱 경로에 처음 적용하는 사례다.

**Independent Test**: run 이벤트 fixture(메시지·도구 호출·권한 요청·완료·오류)를 발행하면서 화면 상태 복원 로직(reducer)에 live 경로와 replay 경로로 각각 흘려 최종 상태가 같은지 비교한다. 재수화 도중 live 이벤트를 끼워 넣는 시나리오에서 표시 항목 수가 발행 수와 같은지 확인한다. 기존 run 관련 자동 테스트는 수정 없이 통과한다.

**Acceptance Scenarios**:

1. **Given** run이 이벤트 20개를 발행하는 동안, **When** 화면이 12번째 이벤트 시점에 재수화를 시작하면, **Then** 최종 표시는 20개 이벤트를 각각 한 번씩 반영한 것과 같다.
2. **Given** run이 완료된 뒤, **When** 창을 다시 열어 재수화하면, **Then** 완료 상태와 메시지가 이전 버전과 같게 표시된다.
3. **Given** 보관 run 상한을 넘어 오래전에 끝난 run의 journal이 지워졌을 때, **When** 그 run 화면을 새로 열어 재수화하면, **Then** 빈 화면을 정상으로 보이는 대신 gap(다시 동기화 필요) 상태가 표시된다. 서버 재시작 전 run을 재수화하는 경우는 오늘과 같다(run 목록의 정본이 생기는 2b에서 "실행 정보 유실"로 확정).
4. **Given** run 이벤트를 보내는 기존 데스크톱 전달 경로가 있을 때, **When** 이 spec을 적용하면, **Then** 데스크톱은 새 발행자에서 나온 같은 이벤트를 받고, 창을 닫으면 run이 취소되는 오늘의 동작은 바뀌지 않는다(2b에서 다룸).

---

### User Story 3 - worktree 파일 변경 알림이 구독으로 동작한다 (Priority: P3)

AW 사용자가 worktree 창을 열어 두면 파일이나 Git 상태가 바뀔 때 목록이 갱신된다. 이 알림은 이제 "worktree 변경 스트림 구독"으로 동작한다. 구독이 시작되면 감시가 시작되고, 마지막 구독이 끝나면 감시가 멈춘다. 같은 worktree를 두 창이 보면 감시는 하나만 돌고 두 창이 모두 알림을 받는다. 사용자에게 보이는 갱신 시점(0.5초 묶음)·대상 필터·창을 닫으면 감시가 멈추는 동작은 이전과 같다.

**Why this priority**: 이연된 32개 중 창 정체 분해 없이 옮길 수 있는 유일한 묶음(watcher 2개)이고, "command를 구독 수명에 흡수한다"는 정본 방향의 첫 사례다. 사용 빈도는 run보다 낮다.

**Independent Test**: 임시 디렉터리를 worktree 스트림으로 구독한 뒤 파일 생성·수정·Git 변경을 일으켜, 묶음 규칙대로 알림이 오는지 메모리 내 경로와 HTTP 경로에서 비교한다. 구독자 수 0→1→2→1→0 변화에 따라 감시가 정확히 한 번 시작되고 한 번 멈추는지 확인한다.

**Acceptance Scenarios**:

1. **Given** worktree W를 구독한 상태에서, **When** 0.5초 안에 파일 세 개를 바꾸면, **Then** 알림을 한 번 받는다(오늘과 같은 묶음).
2. **Given** 두 창이 같은 W를 구독 중일 때, **When** 한 창이 구독을 끝내면, **Then** 다른 창은 계속 알림을 받고 감시는 하나만 돈다.
3. **Given** W를 구독한 창이 닫히면, **When** 그 창이 마지막 구독자였다면, **Then** 감시가 멈춘다.
4. **Given** 데스크톱이 오늘처럼 감시 시작·중지 command를 부르면, **When** 이 spec을 적용하면, **Then** 두 command는 구독 시작·종료로 변환되어 화면 코드 변경 없이 같은 결과를 낸다.

---

### User Story 4 - 이벤트 계약이 계약 조회·생성 타입에 포함되어 클라이언트가 실행 전에 검증한다 (Priority: P4)

다음 단계 개발자와 클라이언트 작성자는 계약 조회로 허용된 이벤트 스트림과 이벤트 종류(스키마)를 확인하고, 생성된 타입으로 이벤트 본문을 다룬다. run·worktree·orchestration·exchange 이벤트 종류가 모두 계약에 나열되며, 잘못된 이벤트 종류를 다루는 코드는 컴파일 단계에서 실패한다. 정의와 생성물이 어긋나면 저장소 검증이 실패한다.

**Why this priority**: 2b와 3단계(WebSocket)가 같은 계약을 쓰게 하는 장치다. 사용자 가치는 간접적이다.

**Independent Test**: 계약 조회 결과에 이벤트 스키마 목록이 있고, 생성된 TypeScript 타입에서 스키마 이름 ↔ 본문 타입이 짝지어져 잘못 짝지으면 타입 검사가 실패하는지 확인한다. 정의 하나를 바꾸면 drift 검사가 실패하는지 실증한다.

**Acceptance Scenarios**:

1. **Given** 데스크톱 호출자가 계약을 조회하면, **When** 결과를 보면, **Then** operation 목록과 함께 허용된 이벤트 스키마 목록이 있다.
2. **Given** 생성된 클라이언트 타입으로 run 메시지 이벤트를 처리하는 코드를 쓸 때, **When** worktree 변경 본문 타입으로 잘못 다루면, **Then** 타입 검사가 실패한다.

---

### Edge Cases

- **구독 시작과 발행이 겹칠 때**: 수신자를 먼저 등록하고 기준점을 잡은 뒤 replay를 보내고, 기준점보다 큰 이벤트만 버퍼에서 보낸다. 기준점과 같은 번호는 한 번만 전달된다.
- **cursor가 미래를 가리킬 때**(현재 마지막 번호보다 큼): 같은 세대라면 잘못된 입력으로 거절하고, 다른 세대라면 gap으로 처리한다.
- **알 수 없는 스트림**: run이 존재한 적 없는 스트림을 cursor 0으로 구독하면 빈 replay 뒤 live로 기다린다(아직 시작 전인 run을 미리 구독할 수 있어야 한다). cursor가 0보다 크면 gap이다.
- **완료된 run**: 완료 뒤에도 보관 한도 안의 이벤트는 replay된다. 완료된 run의 journal이 언제 사라지는지(오늘은 사라지지 않음)는 전역 한도 규칙으로 정한다.
- **구독자가 느릴 때**: 구독자마다 대기열 한도가 있고, 넘으면 gap으로 닫는다. 한 구독자가 느려도 다른 구독자와 발행자는 막히지 않는다.
- **데스크톱 창이 닫힐 때**: 그 창의 구독이 모두 끝난다. run 취소(오늘 동작)는 이 spec에서 바꾸지 않는다.
- **같은 worktree를 서로 다른 경로 표기로 구독**(`/a/b`와 `/a/b/`, 심볼릭 링크): 실제 경로 기준으로 같은 스트림으로 본다.
- **발행 실패**: 데스크톱 전달이 실패해도 journal 기록과 다른 구독자 전달은 영향받지 않는다.
- **서버 재시작 직후 진행 중이던 run**: 새 세대에는 그 run의 journal이 없다. cursor를 가진 구독자는 gap을 받고, cursor 없이 새로 재수화하는 화면은 오늘과 같다("실행 정보 유실" 확정은 2b). **보관 한도로 지워진 run**은 제거 표식으로 구별해 gap을 알린다.
- **같은 run에 여러 발행자가 동시에 발행할 때**(본 흐름과 stderr 진단 등): 데스크톱에 도착하는 순서는 부여된 순번 순서와 같아야 하고, 늦게 도착해 버려지는 이벤트가 없어야 한다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 시스템은 `Workbench`의 이벤트 구독 동작을 제공해야 한다. 호출자는 스트림별 cursor(스트림 식별자·세대·마지막 반영 번호) 목록을 주고 구독하며, 결과는 순서가 있는 이벤트 흐름이다. 권한 판단은 1단계 `call`과 같은 principal·scope 규칙을 쓴다.
- **FR-002**: 모든 이벤트는 공통 봉투를 가져야 한다: 이벤트 식별자, 스트림 식별자, 세대, 스트림 안의 순번, 이벤트 스키마 이름, 발생 시각, 관련 요청 식별자(있으면), 본문. 한 스트림 안에서 순번은 1씩 증가한다. 스트림 사이의 순서는 약속하지 않는다.
- **FR-003**: 구독은 정본 Ordering 3의 순서를 한 조정자 안에서 수행해야 한다: 권한·필터 확정 → 수신자 등록 → 기준점 capture → 기준점 이하 replay 전송 → 수신자에 쌓인 기준점 초과 이벤트를 중복 제거해 전송 → live 전환. 이 경계에서 이벤트가 빠지거나 중복되지 않아야 한다.
- **FR-004**: 이어 붙일 수 없는 경우 시스템은 이벤트를 건너뛰지 않고 gap 신호를 보내야 한다: cursor가 보관 범위보다 오래됨, 세대가 다름, 구독자 대기열 초과. gap 신호는 해당 스트림의 현재 세대와 보관 범위를 알려 주며, 서버는 cursor를 임의로 전진시키지 않는다.
- **FR-005**: 시스템은 기동마다 새 세대 표식을 가져야 하고, 계약 조회와 모든 이벤트에 현재 세대를 실어야 한다. 이전 세대 cursor로 구독하면 gap을 받아야 한다. 보관 한도로 지워진 run은 제거 표식으로 "시작 전 run"과 구별되어야 하며, 그 run의 구독·재수화는 gap을 알려야 한다. 이전 세대 run을 cursor 없이 조회하는 경우의 "실행 정보 유실" 확정은 run 목록의 정본이 생기는 2b에서 한다.
- **FR-006**: agent run 이벤트는 공통 봉투로 발행되어야 하며, 같은 이벤트는 live와 replay에서 같은 스트림·세대·순번을 가져야 한다. 순번은 오늘의 run journal이 부여한 값이다. run당 보관 한도(512)는 유지한다.
- **FR-007**: 데스크톱의 기존 run 이벤트 전달은 새 발행자를 소비하도록 바뀌어야 하며, 데스크톱에 전달되는 이벤트에 서버가 부여한 순번이 포함되어야 한다. 화면 복원 로직은 이 순번을 사용하고 더 이상 순번을 추정하지 않아야 한다. 사용자에게 보이는 run 화면의 동작은 바뀌지 않아야 한다.
- **FR-008**: worktree 변경 알림은 worktree 변경 스트림(실제 경로 기준)으로 제공되어야 한다. 감시는 첫 구독에서 시작하고 마지막 구독이 끝나면 멈춘다. 묶음 간격(0.5초), 변경 분류(파일·Git), 제외 디렉터리 규칙은 오늘과 같아야 한다. worktree 변경 이벤트는 상태 복원용이 아니라 "다시 조회하라"는 알림이므로 replay하지 않고, 구독 시작 이후 이벤트만 전달한다.
- **FR-009**: 데스크톱의 감시 시작·중지 command 2개는 worktree 변경 스트림의 구독 시작·종료로 변환되어야 하며, 화면 코드는 바뀌지 않아야 한다. 창이 닫히면 그 창의 구독이 끝나야 한다.
- **FR-010**: 구독자마다 대기열 한도가 있어야 하고, 전역으로 보관 이벤트 수·스트림 수·구독자 수에 한도가 있어야 한다. 한도 값과 초과 시 동작(gap으로 닫기, 오래된 완료 run부터 정리)은 문서화되어야 한다. 한 구독자가 느려도 발행자와 다른 구독자는 막히지 않아야 한다.
- **FR-011**: 이벤트는 "상태 복원용"(replay 대상)과 "알림용"(replay하지 않음)으로 분류되어야 한다. run 이벤트는 상태 복원용, worktree 변경은 알림용이다. orchestration·exchange 이벤트의 분류와 본문 스키마는 이 spec에서 정의만 하고 발행 경로 전환은 2b에서 한다.
- **FR-012**: 이벤트 스키마는 operation과 같은 계약 정의에서 생성되어야 한다. 계약 조회는 허용된 이벤트 스키마 목록을 반환하고, 생성된 클라이언트 타입은 스키마 이름과 본문 타입을 짝지어 불일치를 실행 전 단계에서 검출해야 한다. 정의와 생성물이 어긋나면 저장소 검증이 실패해야 한다.
- **FR-013**: 메모리 내 경로와 테스트 HTTP 경로는 같은 구독 입력에 같은 이벤트 목록·gap 신호·오류를 반환해야 한다. 테스트 HTTP 경로의 이벤트 전송 방식은 테스트 전용이며 운영 노출(3단계)이 아니다.
- **FR-014**: 이 spec은 run·exchange·orchestration command의 이관, run·orchestration·worker·session 소유의 창 label 분해, 창을 닫으면 run이 취소되는 동작, orchestration·exchange·창 제목·외관 설정 이벤트의 발행 경로, 운영 HTTP/WebSocket 노출을 변경하지 않아야 한다(2b·3단계).

### Key Entities *(include if feature involves data)*

- **이벤트 스트림**: 순서가 보장되는 이벤트의 단위. `run:<runId>`, `worktree:<실제 경로>`, 그리고 2b용으로 이름만 정해 두는 `orchestration:<workspaceId>`·`exchange:<workspaceId>`.
- **세대(Server Epoch)**: 서버 기동 한 번을 가리키는 표식. 세대가 다르면 순번을 비교하지 않는다.
- **Cursor**: 호출자가 마지막으로 반영한 (스트림, 세대, 순번).
- **이벤트 봉투**: FR-002의 필드를 가진 전달 단위.
- **이벤트 스키마**: 본문 형식의 이름과 버전(예: run 메시지 v1). 상태 복원용/알림용 분류를 가진다.
- **구독**: 한 호출자의 cursor 목록과 대기열. 끝나면 수신자가 해제된다.
- **Gap 신호**: 이어 붙일 수 없음을 알리는 제어 메시지. 현재 세대와 보관 범위를 담는다.
- **Client instance**: 구독을 여는 주체(데스크톱 창 하나, 테스트 HTTP 연결 하나). 이 spec에서는 이벤트 전달 대상 식별에만 쓰이고 run 소유와는 무관하다.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: `crates/workbench-protocol`(이벤트 봉투·구독·스키마 계약과 생성), `crates/workbench-core`(구독 조정자·스트림 journal·worktree 감시 이동), `apps/agentic-workbench/src-tauri`(run 발행 경로와 watcher command를 새 발행자·구독으로 전환), `apps/agentic-workbench/src`(run 화면의 순번 추정 제거 — 이 spec에서 유일한 프론트 변경), `packages/workbench-client`(이벤트 타입 생성), `docs/`. `crates/acp-agent-core`의 run journal 모델 변경은 필요한 최소로 제한하고 다른 소비 앱(hushline·ask-code)을 깨지 않는다.
- **Frontend layering**: `features/agent-run`(재수화·live 이벤트 처리)와 `entities/agent-run/api`(이벤트 수신 어댑터)만 바뀐다. worktree 화면은 바뀌지 않는다.
- **Backend boundary**: 구독 조정자와 journal은 core application/infrastructure, 이벤트 발행 포트는 core ports, Tauri 이벤트 전달은 AW inbound/infrastructure 어댑터로 남는다. AW는 발행자를 소비해 창에 전달만 한다.
- **Shared core vs UI**: 순수 core만 공유한다. 공유 UI 없음.
- **Persistence and safety**: journal은 메모리에 두고 세대로 구분한다(정본 결정 7). worktree 스트림은 실제 경로로 정규화하며 감시 대상은 오늘과 같은 제외 규칙을 따른다. run·permission 소유 범위는 바뀌지 않는다.
- **Documentation and Storybook**: `docs/workbench-seam.md`에 이벤트 계약·구독 순서·한도·분류를 추가하고 정본 진행 각주를 갱신한다. ADR 후보: 이벤트 전달의 창 스크립트 삽입(fallback) 경로 처리. Storybook N/A.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 구독 경계 race test(구독 시작 전·중·후에 이벤트 주입)를 1,000회 반복해 모든 회차에서 받은 순번이 연속이고 누락·중복이 0건이다.
- **SC-002**: 같은 run 이벤트 fixture를 live 경로와 replay 경로로 화면 복원 로직에 흘렸을 때 최종 상태가 100% 같고, 재수화 도중 live 이벤트를 끼워 넣는 시나리오에서 반영 횟수가 발행 수와 정확히 같다.
- **SC-003**: 보관 범위 밖 cursor·다른 세대 cursor·대기열 초과 fixture 전부에서 gap 신호가 나오고, 이벤트를 건너뛴 채 이어지는 경우가 0건이다.
- **SC-004**: 메모리 내 경로와 테스트 HTTP 경로가 구독 fixture 전부에서 같은 이벤트 목록·gap 신호·오류를 반환한다.
- **SC-005**: 같은 worktree를 N개 구독자가 동시에 구독·해지해도 감시는 정확히 한 번 시작되고 한 번 멈추며, 묶음 간격 안의 변경은 구독자마다 알림 한 번으로 전달된다.
- **SC-006**: 기존 run·worktree 관련 자동 테스트가 수정 없이 통과하고, run 이벤트 발행에서 데스크톱 수신까지의 지연 증가가 이전 대비 10ms 이내다.
- **SC-007**: 계약 조회 결과에 이벤트 스키마 목록이 있고, 생성된 타입에서 스키마 ↔ 본문을 잘못 짝지은 코드가 타입 검사에서 실패하며, drift 검사가 정의 변경을 검출한다.
- **SC-008**: 프론트엔드 변경은 run 화면의 순번 처리에 한정되며(`features/agent-run`, `entities/agent-run/api`), 다른 화면 코드 diff는 0건이다.

## Clarifications

### Session 2026-09-27 (`/grill-with-docs`)

- Q1 분할: **2a(039) 이벤트 Seam + run 발행 전환 + watcher 구독 / 2b(040) 창 정체 분해 + command 30개 이관**으로 확정. orchestration·exchange는 039에서 스키마 정의만.
- Q2 프론트: run 화면의 순번 추정 제거에 한해 **최소 프론트 수정 허용**(`features/agent-run`, `entities/agent-run/api`).
- Q3 run 데스크톱 전달: **창 스크립트 삽입 경로 하나만** 남기고 Tauri `agent-run-event` 발행 제거. Tauri 2.11.6 `WebviewWindow::emit`이 모든 창에 방송함을 확인(ADR `docs/adr/0004`).
- Q4 데스크톱 연결: 데스크톱은 구독자가 아니라 **발행 결과(봉투)를 그대로 전달**(ADR `docs/adr/0003`). journal·구독 조정자는 workbench-core.
- Q5 run 본문 타입: 스키마 **`run.event.v1` 하나** + RunEvent 전체 union의 protocol DTO 미러와 wire 동일성 테스트. acp-agent-core 불변.
- Q6 권한: **`run:read` 신설**, worktree 스트림은 `worktree:read` 재사용. 조회 전용 호출자도 구독 가능.
- Q7 테스트 HTTP 전송: 테스트 하네스에 **WebSocket**(event·gap·hello 제어 프레임)으로 미리 두어 3단계가 같은 프레임 계약을 재사용.
- Q8 journal 정리: run당 512 유지 + **보관 run 수 상한, 가장 오래전에 끝난 run부터** 제거. 진행 중 run은 제거하지 않음.
- Q9 ADR 4건: `crates/workbench-core/docs/adr/0002`(메모리 journal + 세대), `0003`(알림용 이벤트 replay 안 함), `docs/adr/0003`(데스크톱은 발행 결과 전달), `docs/adr/0004`(run 삽입 경로만 유지). 용어집 `crates/workbench-core/CONTEXT.md`에 "이벤트" 절 추가.

## Assumptions

- **2단계 분할**: 정본 2단계를 **2a(039, 이벤트 Seam)** 와 **2b(040, 창 정체 분해 + run·exchange·orchestration command 30개 이관)** 로 나눈다(Clarifications Q1 확정).
- **내구성**: 정본 결정 7을 따른다 — memory journal + 세대 + 유실 표시. durable journal은 없고, 서버 재시작 뒤 일부 transcript가 사라질 수 있다.
- **프론트 변경 허용 범위**: 1단계와 달리 run 화면의 순번 추정 제거라는 **결함 수정**에 한해 프론트 변경을 허용한다. 화면에 보이는 동작은 같다.
- **창 스크립트 삽입(fallback) 경로**: run 이벤트는 삽입 경로 하나만 남긴다(Clarifications Q3).
- **worktree 변경 알림은 replay하지 않는다**: 알림은 "다시 조회하라"는 뜻이므로 놓친 알림 대신 구독 시작 시 화면이 다시 조회하는 오늘의 동작을 유지한다.
- **테스트 HTTP 이벤트 전송**: 037의 테스트 전용 HTTP 하네스에 WebSocket 이벤트 전송을 추가해 세 번째 경로를 검증한다(Clarifications Q7). 운영 노출·ticket 인증은 3단계다.
- **권한 범위**: 이벤트 구독 권한은 도메인별 조회 scope(`run`은 신설, `worktree:read` 재사용)를 따른다. 조회 전용 테스트 호출자도 조회 가능한 스트림은 구독할 수 있다.
- **한도 기본값**: run당 512(오늘 값), 보관 run 수 상한과 구독자 대기열 크기는 plan에서 측정 근거와 함께 정한다(정리 규칙은 Clarifications Q8).
- **4단계까지의 시리즈 범위**(2026-09-26 결정)와 단계별 PR·squash merge 규칙은 그대로다.
