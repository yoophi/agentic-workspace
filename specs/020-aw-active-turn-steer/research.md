# Research: AW Active-Turn Steer

## Decision: steer와 queue를 별도 입력 class로 모델링한다

**Rationale**: issue #143의 핵심 문제는 "현재 작업에 넣을 입력"과 "다음 작업으로 실행할 입력"이 UI와 상태에서 섞이는 것이다. `queuedPrompts`만으로 두 의미를 표현하면 자동 전송, 즉시 전송, 재시작 fallback, 거절된 steer의 처리 순서를 테스트하기 어렵다. `pendingSteers`, `rejectedSteers`, `queuedPrompts`를 분리하면 각 입력의 대상과 수명주기를 독립적으로 검증할 수 있다.

**Alternatives considered**:

- 기존 `queuedPrompts`에 `kind` 필드만 추가: 작은 변경이지만 자동 dispatch 조건과 steer 수락/거절 처리가 계속 한 배열에 묶여 race가 남는다.
- 모든 입력을 timeline event로만 표현: 표시에는 유리하지만 아직 provider가 수락하지 않은 입력의 복구/재시도 상태를 관리하기 어렵다.

## Decision: provider capability가 없는 경우 cancel-free steer를 강제하지 않는다

**Rationale**: 현재 backend의 ACP `send_prompt`는 `in_flight` guard가 있으며, prompt 응답 중 동시 dispatch를 거절한다. ACP adapter가 Codex app-server의 active-turn steer 같은 기능을 보장하지 않으므로, 모든 provider에 같은 의미의 cancel-free steer를 강제하면 실패를 숨기거나 session 안정성을 해칠 수 있다. 따라서 capability가 명시적으로 가능할 때만 active-turn steer를 수행하고, 불가능하면 입력을 보존한 뒤 queue 이동 또는 명시적 restart-with-steering을 사용자 선택으로 처리한다.

**Alternatives considered**:

- 기존처럼 항상 cancel 후 restart: 현재 문제를 해결하지 못하고 사용자가 원하지 않은 세션 종료를 계속 만든다.
- `send_prompt_to_run`을 실행 중에도 호출: 현재 `in_flight` 정책과 충돌하며 provider가 해당 입력을 현재 turn에 반영한다는 보장이 없다.
- Codex app-server adapter를 즉시 도입: 장기적으로 적합하지만 이번 v1 범위를 넘어서는 provider integration spike가 필요하다.

## Decision: restart-with-steering은 명시적 fallback으로 유지한다

**Rationale**: 일부 provider나 상태는 steer를 받을 수 없다. 이때 기존 cancel/restart 동작을 완전히 제거하면 사용자는 긴 작업을 새 지시로 다시 시작할 방법을 잃는다. 다만 이 동작은 steer 버튼의 암묵적 동작이 아니라 사용자가 선택한 fallback이어야 하며, UI와 상태 이름에서 restart로 드러나야 한다.

**Alternatives considered**:

- 미지원 상태에서 입력을 항상 queue로 이동: 사용자가 "현재 작업 방향 조정"을 기대한 경우 의도와 다를 수 있다.
- 미지원 상태에서 오류만 표시: 입력 보존과 복구 요구를 만족하지 못한다.

## Decision: 늦은 lifecycle event는 run identity와 transition token으로 격리한다

**Rationale**: 현재 `applyRunEvent`는 `runId !== activeRunId` 이벤트를 무시하지만, steer transition 중 active run 교체와 queue 보존이 겹치면 old run의 terminal event가 새 상태를 지울 수 있다. prompt dispatch state에 target run id와 transition token을 두면 terminal event가 어떤 입력 묶음에 영향을 줄 수 있는지 명확해진다.

**Alternatives considered**:

- UI effect 순서만 조정: 비동기 event ordering 문제를 안정적으로 막지 못한다.
- terminal event에서 queue를 절대 지우지 않기: 정상 완료 후 queue cleanup이 흐려지고 기존 동작과 충돌한다.

## Decision: backend는 `steer_prompt_to_run` command와 `SteerPromptUseCase`를 추가한다

**Rationale**: send, steer, cancel, restart는 사용자 의미가 다르다. inbound command와 application use case를 분리하면 UI가 명시적으로 steer를 요청할 수 있고, backend가 unsupported capability를 structured error로 반환할 수 있다. 실제 ACP provider 동작은 `SessionHandle` port 뒤에 숨긴다.

**Alternatives considered**:

- `send_prompt_to_run`에 옵션 추가: 기존 prompt dispatch와 active-turn steer의 의미가 섞인다.
- frontend-only 상태 변경: 실제 provider 수락/거절 결과를 알 수 없고, cancel-free steer 성공 기준을 만족하지 못한다.

## Decision: app-local documentation과 Storybook 상태 검증을 포함한다

**Rationale**: queue/steer/restart는 사용자가 헷갈리기 쉬운 실행 모델이다. 문서에는 Korean design note와 Mermaid state diagram을 두고, UI가 분리되면 Storybook에서 pending, queued, rejected, restart fallback, long-content 상태를 검증한다.

**Alternatives considered**:

- code tests만 작성: state machine은 검증되지만 사용자가 보는 상태 표현의 회귀를 잡기 어렵다.
- docs 생략: constitution의 documentation 요구와 issue의 조사 성격에 맞지 않는다.
