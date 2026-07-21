# Feature Specification: Hushline 모노레포 편입 및 Agent Run 기능 추가

**Feature Branch**: `030-hushline-monorepo-integration`

**Created**: 2026-07-21

**Status**: Draft

**Input**: User description: "설계한 내용을 바탕으로 hushline 을 모노레포로 가져오고, agent run 기능을 추가한다"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Hushline을 모노레포에서 그대로 실행 (Priority: P1)

Hushline(YouTube 자막 생성 앱)을 `agentic-workspace` 모노레포의 앱으로 편입해,
기존 자막 생성·queue 관리 기능이 회귀 없이 동일하게 동작하고, 모노레포의 공용 빌드·
테스트·타입체크 파이프라인으로 함께 검증된다.

**Why this priority**: 모든 후속 agent 기능의 토대. 편입이 안정적으로 끝나기 전에는
어떤 agent 기능도 얹을 수 없다. 이 단계만으로도 "여러 앱을 한 저장소에서 원자적으로
빌드·검증"이라는 독립 가치를 제공한다.

**Independent Test**: 모노레포에서 hushline을 실행해 YouTube URL로 자막을 생성하고,
queue 추가·삭제·진행 표시가 편입 전과 동일하게 동작함을 확인한다. 공용 빌드/테스트
명령이 hushline을 포함해 통과한다.

**Acceptance Scenarios**:

1. **Given** hushline이 모노레포 앱으로 편입된 상태, **When** 공용 빌드·타입체크·테스트를 실행하면, **Then** hushline이 다른 앱들과 함께 성공적으로 검증된다.
2. **Given** 편입된 hushline 실행, **When** YouTube URL을 입력해 자막을 생성하면, **Then** 편입 전과 동일한 자막 결과와 진행 표시를 얻는다.
3. **Given** 여러 URL을 queue에 넣은 상태, **When** 항목을 추가·삭제하면, **Then** 입력 순서 처리와 항목별 상태 표시가 회귀 없이 동작한다.

---

### User Story 2 - 자막을 원하는 방식으로 정리해 새 문서로 저장 (Priority: P2)

사용자가 생성된 자막(transcript)에 대해 "이런 방식으로 정리해줘"라고 지시하면,
앱이 agent run을 실행해 자막을 사용자가 원하는 형식(요약, 회의록, 불릿 정리 등)으로
재구성하고, 그 결과를 새 문서로 저장한다. 진행 중 에이전트의 출력은 실시간으로 표시된다.

**Why this priority**: 이 기능이 "agent run 추가"의 핵심 사용자 가치다. P1 위에서
가장 작은 end-to-end agent 흐름(실행 → 스트리밍 표시 → 저장)을 제공한다.

**Independent Test**: 자막이 있는 결과 카드에서 "정리하기"를 실행하고 정리 방식을 지정하면,
에이전트 출력이 스트리밍으로 표시되고, 완료 후 새 정리 문서가 저장되어 다시 열 수 있다.

**Acceptance Scenarios**:

1. **Given** 자막이 생성된 결과 항목, **When** 정리 방식을 지정해 정리를 요청하면, **Then** agent run이 시작되고 진행 상태와 생성되는 텍스트가 실시간으로 표시된다.
2. **Given** 정리 run이 완료됨, **When** 결과를 저장하면, **Then** 원본 자막과 연결된 새 정리 문서가 영구 저장되고 목록에서 확인·재열람할 수 있다.
3. **Given** 진행 중인 정리 run, **When** 사용자가 취소하면, **Then** run이 중단되고 부분 결과 처리 방침이 사용자에게 명확히 안내된다.
4. **Given** agent 실행 실패(도구 미설치·네트워크 등), **When** 오류가 발생하면, **Then** 사용자에게 원인이 표시되고 앱은 안정 상태를 유지한다.

---

### User Story 3 - 정리 문서 기반 지식 대화 (Priority: P3)

사용자가 저장된 정리 문서(또는 자막)를 대상으로 다회에 걸쳐 질문하고 답을 받는
대화를 진행한다. 대화는 하나의 세션으로 유지되어 앞선 질문의 맥락이 이어진다.

**Why this priority**: 정리·저장(P2) 위에 얹히는 확장 가치. 단발 실행을 넘어 다회
대화 흐름을 검증한다. P2 없이는 대상 문서가 없으므로 후순위.

**Independent Test**: 저장된 문서에서 대화를 시작해 두 개 이상의 후속 질문을 던지고,
이전 질문의 맥락을 반영한 답변이 오는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 저장된 정리 문서, **When** 대화를 시작해 질문하면, **Then** 문서 내용을 근거로 한 답변이 스트리밍으로 표시된다.
2. **Given** 진행 중인 대화 세션, **When** 후속 질문을 이어서 보내면, **Then** 이전 대화 맥락을 유지한 채 답변한다.
3. **Given** 대화 세션 진행 중, **When** 창이나 앱을 정상 종료하면, **Then** 진행 중이던 run이 안전하게 정리되고 자원이 누수되지 않는다.

---

### Edge Cases

- 자막이 매우 긴 경우(대용량 문서) 정리·대화 요청이 어떻게 처리되는가(길이 제한/분할/경고)?
- 선택한 agent 실행 도구가 설치·구성되어 있지 않을 때 사용자에게 무엇이 안내되는가?
- 동일 자막에 대해 여러 정리 run을 동시에 요청하면 어떻게 되는가(동시 실행 상한/직렬화)?
- agent가 파일 쓰기 등 부작용 도구를 사용하려 할 때 사용자 승인 흐름은 어떻게 되는가?
- run 도중 앱이 강제 종료되면 부분 결과·세션 상태는 어떻게 처리되는가?
- 오프라인이거나 네트워크가 끊긴 상태에서 정리/대화를 요청하면?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Hushline은 모노레포의 앱으로 편입되어 공용 빌드·타입체크·테스트 파이프라인에 포함되어야 한다.
- **FR-002**: 편입 후 hushline의 기존 기능(자막 생성, queue 관리, 결과 저장·재사용, 진행 표시)은 회귀 없이 동작해야 한다.
- **FR-003**: 사용자는 생성된 자막에 대해 정리 방식을 지정해 agent run 기반 문서 정리를 요청할 수 있어야 한다.
- **FR-004**: 시스템은 정리 run의 진행 상태와 생성되는 출력을 실시간(스트리밍)으로 사용자에게 표시해야 한다.
- **FR-005**: 시스템은 정리 결과를 원본 자막과 연결된 새 문서로 영구 저장하고, 이후 목록에서 재열람할 수 있어야 한다.
- **FR-006**: 사용자는 진행 중인 agent run을 취소할 수 있어야 하며, 취소 시 부분 결과 처리 방침이 명확해야 한다.
- **FR-007**: 시스템은 agent 실행 실패·오류를 사용자에게 이해 가능한 메시지로 표시하고, 앱을 안정 상태로 유지해야 한다.
- **FR-008**: 사용자는 저장된 정리 문서(또는 자막)를 대상으로 다회 질의응답 대화를 진행할 수 있어야 하며, 세션 내에서 이전 맥락이 유지되어야 한다.
- **FR-009**: 시스템은 창/앱 종료 시 진행 중인 run과 세션을 안전하게 정리하고 소유 범위(세션/창)를 검증해야 한다.
- **FR-010**: agent가 부작용(예: 파일 쓰기)을 유발하는 동작을 수행하려 할 때, 시스템은 정책에 따라 사용자 승인 또는 안전한 기본 제한을 적용해야 한다.
- **FR-011**: 시스템은 동시에 진행되는 agent run 수를 안전 상한으로 제한해야 한다.
- **FR-012**: 정리·대화에 사용할 자막/문서 데이터는 앱이 관리하는 저장 위치 안에서만 접근·기록되어야 한다(경로·크기·인코딩 검증 포함).

### Key Entities *(include if feature involves data)*

- **Transcript(자막 결과)**: 기존 hushline 산출물. URL, 제목, 인식 텍스트, 언어, 모델, 저장 경로 등. agent 정리·대화의 입력 원본.
- **OrganizedDocument(정리 문서)**: 자막을 사용자가 지정한 방식으로 재구성한 새 문서. 원본 자막 참조, 정리 방식/스타일, 내용, 생성 시각, 저장 위치를 가진다.
- **AgentRun(실행)**: 하나의 agent 실행 단위. 실행 목표(goal), 상태(진행/완료/취소/오류), 스트리밍 이벤트, 소유 범위(어느 세션/창이 소유)를 가진다.
- **ChatSession(대화 세션)**: 특정 문서를 대상으로 한 다회 대화. 대상 문서 참조, 연결된 실행, 주고받은 메시지 목록을 가진다.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: 대상은 신규 편입 앱 `apps/hushline`. agent 통신 코어는 재사용을 위해
  `apps/agentic-workbench`에서 순수 코어를 공유 단위(`crates/*`)로 먼저 추출하는 것을 전제로 한다.
  앱 간 직접 import는 금지하며, 재사용은 workspace crate/package를 통해서만 이뤄진다.
  (상세 설계: `docs/20260721-acp-agent-core-reuse-strategy.md`)
- **Frontend layering**: hushline `apps/hushline/src`는 Feature-Sliced Design을 유지한다.
  정리/대화 액션은 `features/*`, 자막·정리문서 모델과 어댑터는 `entities/*`,
  agent run 호출 래퍼 소비는 `shared/api` 또는 `entities`, 화면 구성은 `pages/*`·`widgets/*`.
- **Backend boundary**: hushline `src-tauri`는 hexagonal 유지. 통신 유스케이스는 공유 코어의
  application/ports를 소비하고, Tauri command(inbound)는 입력 검증·위임만 담당하며,
  이벤트 emit·저장 경로 등 어댑터는 `infrastructure`에 둔다. `domain`은 Tauri·FS 비의존.
- **Shared core vs UI**: 공유는 순수 코어(agent run domain·ports·application·ACP 어댑터) 우선.
  UI는 공유하지 않고 hushline 고유로 둔다(요구가 아직 수렴하지 않음).
- **Persistence and safety**: 자막·정리 문서 접근은 앱 관리 경로 안에서만 이뤄지며 경로·크기·
  UTF-8 검증을 적용한다. agent run/세션은 소유 범위(세션/창)를 검증하고, 부작용 도구 사용에는
  권한 정책을 적용한다.
- **Documentation and Storybook**: 편입·통합 설계 문서는 `docs/*.md`(영문 파일명·한국어 본문)로
  유지·갱신한다. 재사용 가능한 신규 UI 컴포넌트는 Storybook에 atomic 분류로 등록한다.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 편입 후 공용 빌드·타입체크·테스트가 hushline을 포함해 성공하며, 기존 자막/queue 기능에서 회귀 결함이 0건이다.
- **SC-002**: 사용자가 자막 결과에서 정리 방식을 지정해 새 정리 문서를 저장 완료하기까지 별도 안내 없이 3분 이내에 수행할 수 있다.
- **SC-003**: 정리·대화 요청 시 첫 스트리밍 출력이 나타나기까지(정상 환경 기준) 사용자가 "반응 없음"으로 느끼지 않는 수준으로 진행 표시가 즉시 갱신된다.
- **SC-004**: 저장된 정리 문서 기반 대화에서, 두 번째 이후 후속 질문의 답변이 직전 질문의 맥락을 반영하는 비율이 검증 시나리오에서 100%다.
- **SC-005**: 진행 중 run을 취소하거나 창을 종료했을 때 남는 실행/세션 자원이 없다(누수 0건).
- **SC-006**: agent 실행 실패 시 100%의 경우에 사용자에게 원인 메시지가 표시되고 앱은 계속 사용 가능한 상태를 유지한다.

## Assumptions

- 이번 기능의 범위는 (1) hushline 모노레포 편입과 (2) agent run 기반 문서 정리·대화이며,
  "hushline 기능을 MCP로 노출해 에이전트가 앱을 구동하는 오케스트레이션"은 후속 단계(별도 스펙)로 둔다.
  (근거: `docs/20260721-acp-agent-core-reuse-strategy.md`의 Phase 4~5는 후속으로 분리)
- 재사용할 agent 통신 코어는 `agentic-workbench`의 ACP 기반 실행 코어이며, 이를 공유 crate로
  추출해 hushline이 소비한다. 실행에 쓰는 agent 종류는 초기엔 단일 기본값으로 고정한다.
- 세션 지속성(앱 재시작 후 대화 이어가기)은 1차 범위 밖이며, 대화는 세션 내 유지로 한정한다.
  정리 문서와 대화 로그 자체는 앱이 영구 저장한다.
- 정리 문서 저장 위치·형식은 기존 자막 결과 저장 방식(앱 관리 출력 폴더의 구조화 데이터)을 따른다.
- 정리 결과는 에이전트 출력 스트림을 앱이 수집해 저장하는 방식을 기본으로 한다(에이전트 직접 파일 쓰기는 옵션).
- 자막 생성에 필요한 외부 도구(다운로더/추출/인식 엔진)와 agent 실행 도구는 사용자 환경에 설치·구성되어 있다고 가정한다.
