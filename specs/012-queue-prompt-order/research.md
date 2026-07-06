# Research: Queue Prompt Order

## Decision 1: 첫 실행 프롬프트도 queue 상태로 먼저 모델링한다

**Decision**: agent 세션이 아직 준비되지 않은 첫 프롬프트는 timeline user message로 즉시 추가하지 않고, queued prompt 상태로 먼저 보관하고 표시한다.

**Rationale**: 사용자 요구의 핵심은 보이는 순서다. 첫 프롬프트가 timeline user message로 먼저 들어가면 agent 준비 메시지보다 프롬프트 실행이 앞선 것처럼 보인다. 기존 queued prompt 표현을 재사용하면 "입력은 접수됐지만 아직 실행되지 않음"을 명확히 표현할 수 있다.

**Alternatives considered**:
- timeline에 user message를 유지하고 label만 바꾸기: 같은 항목이 실행된 메시지처럼 읽힐 위험이 남는다.
- agent 준비 메시지를 숨기기: 실제 agent 실행 상태를 감춰 디버깅과 사용자의 진행 인지가 나빠진다.

## Decision 2: queue 전환 규칙은 feature model의 순수 함수로 검증한다

**Decision**: 첫 프롬프트 등록, 실행 전환, 실패 복구, 중복 방지 같은 규칙은 `features/agent-run/model/run-panel-state.ts`에 순수 함수로 추가하거나 기존 함수와 함께 테스트한다.

**Rationale**: `agent-run-panel.tsx`는 이미 입력 모드, queue, 실행 상태, settings, goal 상태를 함께 다룬다. 순서 규칙을 UI 이벤트 핸들러에만 두면 회귀 테스트가 어렵다. 순수 함수로 분리하면 최초 실행, saved prompt, external prompt, 추가 prompt 입력의 공통 규칙을 같은 테스트로 검증할 수 있다.

**Alternatives considered**:
- 패널 컴포넌트 내부에서만 상태를 직접 조작하기: 단기 구현은 빠르지만 회귀 위험이 높고 테스트가 복잡하다.
- backend에서 모든 queue 상태를 소유하기: 이번 요구는 보이는 UI 순서 조정이 중심이며 새 persistence나 protocol 변경이 필요하지 않다.

## Decision 3: prompt 실행 기록은 실제 전송 성공 경로와 연결한다

**Decision**: prompt history와 direct prompt 상태는 사용자가 입력한 텍스트를 보존하되, "실행된 프롬프트" 표시는 agent가 prompt를 받을 준비가 된 뒤의 전송/처리 흐름에 맞춰 갱신한다.

**Rationale**: 첫 프롬프트를 queue로 보이는 동안에도 사용자의 입력 이력은 유지되어야 한다. 하지만 timeline에서 실행 완료처럼 보이는 user message는 중복 없이 한 번만 나타나야 한다.

**Alternatives considered**:
- queue 등록 시 history와 timeline을 모두 갱신하기: prompt가 실행 실패해도 실행된 것처럼 남을 수 있다.
- 응답 완료 후에만 history를 기록하기: 실패/취소 전후 사용자의 제출 이력을 복구하기 어려워진다.

## Decision 4: 계약은 UI 상태 순서 계약으로 정의한다

**Decision**: 이 기능의 계약은 외부 API가 아니라 Worktree Session Page의 agent-run timeline/queue 표시 순서로 정의한다.

**Rationale**: 기능은 사용자가 보는 app UI 흐름을 바꾸며, 새 public API나 파일 형식이 없다. 계약을 UI 상태 순서로 문서화하면 task 생성과 검증이 요구사항의 실제 위험 지점에 집중된다.

**Alternatives considered**:
- backend command contract 작성: 기존 `start_agent_run`과 `send_prompt_to_run` 호출 자체는 유지될 가능성이 높아 과도하다.
- 별도 persistence contract 작성: durable storage 변경이 없어 불필요하다.
