# Research: 긴 Permission 다이얼로그 레이아웃 개선

## R1. Permission 다이얼로그 분리 위치

**Decision**: 기존 `agent-run-panel.tsx` 내부 `PermissionRequestDialog`를 `apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.tsx`로 분리한다.

**Rationale**: 현재 다이얼로그는 agent run panel 내부에 inline으로 정의되어 있어 긴 콘텐츠 상태를 독립 테스트하거나 Storybook에 등록하기 어렵다. 승인/거절 interaction은 agent run 기능의 사용자 action이므로 `features/agent-run/ui`가 적절하다.

**Alternatives considered**:

- `agent-run-panel.tsx` 안에서 class만 수정: 변경량은 작지만 long-content regression test와 Storybook 등록이 어렵다.
- `shared/ui`로 이동: 아직 AW 외부 재사용 요구가 없고 permission 의미가 agent-run에 묶여 있어 공유 UI로 승격하기 이르다.

## R2. 표시 모델과 option label 요약

**Decision**: `entities/agent-run/model/permission-display.ts`에 permission input 문자열화, approval option summary, fallback label, full detail metadata를 만드는 순수 helper를 둔다.

**Rationale**: 긴 승인 prefix와 option name을 button에 그대로 넣으면 레이아웃이 깨진다. 요약 규칙은 UI class보다 비즈니스 의미에 가깝고, 순수 함수로 두면 긴 단일 문자열, reject/approve kind, 빈 name fallback을 안정적으로 테스트할 수 있다.

**Alternatives considered**:

- JSX 내부에서 inline 조건 처리: 빠르게 고칠 수 있지만 option 표시 규칙이 테스트하기 어렵고 panel 컴포넌트가 더 커진다.
- backend에서 summary 생성: permission 표시만을 위한 derived text이므로 backend protocol이나 승인 처리에 넣을 이유가 없다.

## R3. 긴 input/detail 표시 방식

**Decision**: permission title/summary와 full detail 영역을 분리하고, full detail은 viewport-bounded scroll region 안에 표시한다. 단일 긴 문자열은 wrapping/breaking이 가능해야 하며, multi-line JSON/markdown은 line break를 유지한다.

**Rationale**: 이슈의 핵심은 메시지와 버튼 영역이 서로 밀리거나 겹치는 문제다. 전체 dialog를 스크롤시키기보다 상세 영역을 제한하면 action controls를 항상 예측 가능한 위치에 둘 수 있다.

**Alternatives considered**:

- Full detail을 접어서 기본 숨김: 사용자가 승인 전 원문을 놓칠 위험이 있다.
- 전체 dialog body만 스크롤: 버튼이 화면 밖으로 밀릴 수 있어 완료 조건을 직접 충족하지 못한다.

## R4. 버튼 영역 레이아웃

**Decision**: action 영역은 dialog 하단에서 content와 분리하고, 좁은 폭에서는 버튼이 안정적으로 wrap/stack되도록 한다. 각 버튼은 summarized label을 사용하고 pending 상태에서도 같은 layout footprint를 유지한다.

**Rationale**: 긴 button text는 조작성과 가독성을 동시에 해친다. 버튼 자체는 decision action만 보여주고, 실제 command/prefix 원문은 상세 영역에서 확인하게 하면 승인 판단 정보와 조작 UI를 분리할 수 있다.

**Alternatives considered**:

- Button에 전체 option name 유지: 원문 접근성은 좋지만 레이아웃 파손 원인을 유지한다.
- Icon-only action: 승인/거절 의미가 모호해져 permission decision에 부적합하다.

## R5. 좁은 창 검증 기준

**Decision**: 360px 너비에 준하는 narrow viewport 상태를 acceptance 기준으로 삼고, Storybook과 DOM test에서 button/detail 영역이 존재하고 활성화 가능한지 검증한다.

**Rationale**: Tauri desktop window는 resize 가능하며 spec의 성공 기준도 360px를 명시한다. 실제 픽셀 screenshot 검증은 구현 단계에서 선택할 수 있지만, planning 기준으로는 narrow story와 automated render test가 최소 안전망이다.

**Alternatives considered**:

- 데스크톱 기본 폭만 검증: 좁은 창에서의 레이아웃 파손 재발을 막지 못한다.
- 모든 모바일 breakpoint 지원: 별도 모바일 앱 UX는 scope 밖이다.

## R6. Backend와 permission protocol 변경 여부

**Decision**: backend permission response API와 `RunEvent.permission` contract는 변경하지 않는다.

**Rationale**: 이 feature는 사용자가 긴 요청을 안전하게 읽고 선택하게 하는 표시/interaction 문제다. 승인 정책이나 protocol을 바꾸면 scope가 커지고 기존 agent integration regression 위험이 생긴다.

**Alternatives considered**:

- Permission event에 별도 `summary` 필드 추가: 현재 요구는 frontend derived summary로 충분하다.
- 승인 option id/kind를 새 구조로 변경: 기존 `respond_agent_permission` 흐름과 호환성에 불필요한 영향을 준다.
