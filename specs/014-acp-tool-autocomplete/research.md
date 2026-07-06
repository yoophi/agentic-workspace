# Research: ACP Tool List 기반 Prompt Command 자동완성

## Decision: `$`와 `/` prefix는 v1에서 같은 후보 목록을 연다

**Rationale**: 이슈 #106은 두 prefix 모두 command/tool 호출 힌트를 찾는 진입점으로 요구한다. prefix별 의미를 지금 분리하면 사용자는 같은 후보를 두 경로에서 다르게 이해해야 하고, 구현도 후보 분류 정책에 먼저 묶인다. v1은 동일 후보 목록을 제공하되 사용자가 입력한 prefix를 삽입 token에 유지해 후속 확장 여지를 둔다.

**Alternatives considered**:

- `$`는 skill/tool, `/`는 앱 command로 분리: 의미가 명확해질 수 있지만 현재 spec에는 분류 기준이 없다.
- `/`만 지원: issue 요구사항의 `$` prefix를 충족하지 못한다.

## Decision: 후보 선택은 prompt draft만 수정한다

**Rationale**: spec FR-012는 선택만으로 agent 실행, tool 호출, permission 승인 같은 실행성 동작을 시작하지 말라고 요구한다. 사용자가 최종 prompt 제출을 하기 전까지 자동완성은 텍스트 편집 기능이어야 한다.

**Alternatives considered**:

- 선택 즉시 tool 호출 형태로 변환 또는 실행: 빠르지만 오작동 위험이 크고 permission 흐름과 충돌한다.
- 선택 후 별도 confirmation 표시: v1 범위에는 불필요하며 prompt 입력 흐름을 무겁게 만든다.

## Decision: 후보 데이터는 session-scoped runtime data로 취급하고 저장하지 않는다

**Rationale**: tool/command 후보는 agent, provider session, MCP server 구성, run scope에 따라 달라질 수 있다. 저장하면 stale 후보가 표시될 수 있고 session owner scope 안전성을 흐린다. 후보가 없거나 아직 준비되지 않은 상태는 UI의 loading/empty fallback으로 처리한다.

**Alternatives considered**:

- worktree별 후보 캐시 저장: 빠른 표시가 가능하지만 session별 tool set 차이를 잘못 보여줄 수 있다.
- app-wide 정적 command catalog: `/goal` 같은 앱 command에는 유용할 수 있지만 ACP tool list 기반 요구사항을 충분히 만족하지 않는다.

## Decision: token parsing, filtering, insertion은 headless helper로 둔다

**Rationale**: 자동완성의 위험 지점은 현재 커서 주변 token 감지, 후보 필터링, 선택 삽입, Enter/Tab 충돌이다. 이 로직은 UI와 분리해야 단위 테스트로 빠르게 검증할 수 있고, `agent-run-panel.tsx`의 복잡도를 줄일 수 있다.

**Alternatives considered**:

- 모든 로직을 prompt panel component 내부에 둠: 빠르게 만들 수 있지만 테스트와 유지보수가 어렵다.
- cross-app package로 분리: 아직 두 번째 소비자가 없으므로 constitution의 공유 조건을 충족하지 않는다.

## Decision: 후보 조회는 frontend abstraction을 먼저 정의하고 backend 연결은 session metadata availability에 맞춘다

**Rationale**: 현재 frontend `entities/agent-run`에는 tool candidate 조회 API가 없고, backend ACP runner는 initialize/session 흐름과 MCP server 주입은 다루지만 프론트로 available tool list를 전달하는 계약은 없다. 계획은 후보 provider contract를 먼저 정의하고, 구현 단계에서 ACP runner가 노출할 수 있는 metadata가 확인되면 Tauri command를 추가한다. metadata가 없는 agent/session에서는 empty/loading fallback으로 prompt 작성이 계속 가능해야 한다.

**Alternatives considered**:

- 초기 구현에서 backend 없이 hard-coded 후보 사용: Storybook에는 가능하지만 실제 기능 요구사항을 만족하지 못한다.
- ACP raw protocol에 즉시 강결합: 빠르게 후보를 얻을 수 있어도 provider별 capability 차이와 session owner scope 테스트 없이 위험하다.

## Decision: keyboard conflict 우선순위는 autocomplete open state가 가장 높다

**Rationale**: 후보 목록이 열린 상태에서 Enter/Tab은 선택 확정으로 해석되어야 하며, prompt 제출 또는 queue shortcut을 유발하면 안 된다. 목록이 닫힌 상태에서는 기존 prompt history navigation, queue, submit 동작이 유지되어야 한다.

**Alternatives considered**:

- Enter는 항상 prompt 제출, Tab만 선택: 사용자가 command palette류 UI에서 기대하는 기본 선택 패턴과 어긋난다.
- Tab은 queue shortcut 유지, Enter만 선택: running 상태에서 기존 queue shortcut과 자동완성 선택이 충돌한다.
