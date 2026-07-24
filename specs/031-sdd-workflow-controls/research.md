# Research: SDD 워크플로 단계 표시 및 제어

## Decision 1: 활성 기능은 기존 text-file query로 읽는다

**Decision**: Speckit tab에서 기존 `readWorktreeTextFile(worktree.path, ".specify/feature.json")`와 worktree file query key를 사용한다.

**Rationale**: `entities/worktree-file`은 이미 Tauri의 root-relative text-file read를 감싸며, backend application/provider는 worktree root와 경로 검증을 수행한다. `.specify/feature.json`은 작은 텍스트 파일이므로 새 list scope나 Tauri command가 필요 없다.

**Alternatives considered**:

- 목록 조회에서 hidden `.specify`를 포함하도록 확장: 활성 기능만 필요하므로 불필요한 목록/권한 범위 확장이다.
- frontend에서 OS filesystem 직접 접근: Tauri 경계와 root validation을 우회하므로 채택하지 않는다.
- 별도 backend command: 기존 read contract로 충족되어 중복된다.

## Decision 2: `.specify/feature.json`만 활성 기능 판별에 사용한다

**Decision**: JSON의 `feature_directory`가 상대 경로이며 `specs/<one feature>`를 가리키고 현재 목록의 feature와 정확히 일치할 때만 활성으로 인정한다. 누락·파싱 실패·절대 경로·상위 경로·존재하지 않는 경로는 unavailable 상태다.

**Rationale**: 사용자의 명시 요구대로 활성 기능을 선택된 문서나 목록의 첫 기능으로 추정하지 않는다. 잘못된 설정이 다음 SDD 요청을 다른 기능에 보내는 위험을 막는다.

**Alternatives considered**:

- selected feature fallback: 편리하지만 활성 대상의 의미를 바꾸므로 거절한다.
- 목록의 최신 기능 fallback: 정렬과 파일 시간에 따라 비결정적이고 안전하지 않다.

## Decision 3: 산출물 존재와 tasks 진행을 분리해 단계를 계산한다

**Decision**: `spec.md`, `plan.md`, `tasks.md` 존재 여부로 생성 단계를 완료로 판단하고, `tasks.md`의 기존 `TaskProgressSummary`로 implement의 실행 가능성과 완료 안내를 보강한다. review-spec/review-plan은 영구 상태로 추정하지 않고 Plan/Tasks 버튼을 누를 때 명시 확인 대화상자로 처리한다.

**Rationale**: 현재 Spec Kit 산출물에는 검토 승인 저장 모델이 없고, spec과 plan 작성자는 다를 수 있다. UI 확인은 승인 게이트를 보장하면서 별도 persistence를 피한다.

**Alternatives considered**:

- markdown 본문에서 승인 문구를 파싱: 신뢰할 수 없고 문서 형식에 과도하게 결합된다.
- 승인 상태를 새 파일에 저장: 사용자 요구에 비해 상태 관리와 동기화 범위를 불필요하게 늘린다.

## Decision 4: draft 주입과 전송을 명시적으로 분리한다

**Decision**: cross-panel `AgentPromptRequest`에 delivery mode를 추가한다. SDD 단계 버튼의 확정 요청은 기존 즉시 send/queue 흐름을 사용하고, 활성 기능이 unavailable일 때의 초기 SDD 프롬프트는 draft mode로 textarea에만 채운다.

**Rationale**: 현 external prompt 처리기는 설정된 agent가 있으면 즉시 run을 시작한다. 이는 "초기 프롬프트는 자동 실행하지 않는다"는 요구와 충돌한다. delivery mode는 annotation 및 saved prompt의 현재 동작을 보존하면서 draft-only 흐름을 지원한다.

**Alternatives considered**:

- 모든 외부 프롬프트를 draft로 변경: annotation/send 등 기존 명시적 send 동작을 회귀시킨다.
- 초기 프롬프트를 별도 dialog에만 표시: 사용자가 결국 agent 입력창으로 복사해야 하며 주입 요구를 충족하지 못한다.

## Decision 5: existing watcher invalidation을 재사용한다

**Decision**: `.specify/feature.json` query는 worktree text-file query prefix 아래에 두고, 이미 동작하는 watcher invalidation과 explicit refresh에 함께 갱신한다.

**Rationale**: feature pointer 또는 spec artifacts가 다른 agent/session에서 바뀌어도 하이라이트와 controls가 오래된 대상에 남지 않아야 한다.

**Alternatives considered**:

- tab 입장 때만 조회: 외부 변경을 놓친다.
- 별도 file watcher: 중복 listener와 lifecycle 복잡도를 만든다.

## Decision 6: tasks markdown을 heading section 단위로 파싱한다

**Decision**: checkbox를 단독 정규식 목록으로만 표시하지 않고, 각 task를 가장 가까운 상위 heading section에 귀속시킨다. 작업 필요 보기는 미완료 task가 있는 section의 heading, 비작업 문맥, 미완료 task만 반환한다.

**Rationale**: 사용자가 요구한 대로 완료 task와 완료-only section 전체를 숨기면서도 남은 작업을 이해할 heading/설명을 보존한다.

**Alternatives considered**:

- 미완료 checkbox 줄만 표시: 작업 맥락이 사라진다.
- 모든 heading과 설명 유지: 완료된 영역을 표시하지 말라는 요구를 위반한다.
