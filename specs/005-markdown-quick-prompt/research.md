# Research: Markdown Quick Prompt

## Decision: Agent target is an app-provided delivery contract

Quick prompt 생성과 포맷팅은 Markdown viewer/annotator 흐름에서 처리하고, 실제 전송은 소비 앱이 제공하는 `Agent Target` 계약으로 분리한다. 대상이 없으면 작성 UI는 전송 불가 상태와 사유를 표시한다.

**Rationale**: `apps/markdown-annotator`는 현재 annotation prompt를 복사하는 독립 앱이고, `apps/agentic-workbench`는 활성 agent run으로 prompt를 보내는 경로를 이미 갖고 있다. 전송 경계를 공유 core나 viewer에 넣으면 app-to-app coupling 또는 Tauri/ACP 의존이 shared UI로 새어 나온다.

**Alternatives considered**:

- MA에서 직접 ACP/agent run 전송을 구현: 기능 범위가 커지고 Tauri backend/session-owner 검증을 새로 설계해야 하므로 v1에 부적합.
- quick prompt를 copy-only로 유지: issue의 "바로 agent에게 전달" 목표를 만족하지 못한다.
- AW 전용 기능으로 구현: issue의 Markdown annotator 진입점 목표를 놓친다.

## Decision: Context extraction uses Markdown source plus stable scope metadata

선택 영역은 현재 selection anchor의 selected text와 block range를 사용하고, 블럭/문서 범위는 `MarkdownBlock.rawContent` 및 document source를 우선 사용한다. 모든 context에는 scope, source label, line range, content length, document revision label을 포함한다.

**Rationale**: agent 작업은 렌더링된 텍스트보다 수정 가능한 Markdown 원문이 유용하다. 동시에 사용자는 전송 전 어떤 범위가 포함되는지 확인해야 하므로 line range와 label이 필요하다.

**Alternatives considered**:

- 렌더링 텍스트만 첨부: Markdown 문법, code fence, table source가 손실될 수 있다.
- DOM selection HTML 첨부: agent가 처리하기 어렵고 안전/표시 일관성이 낮다.
- 파일을 다시 읽어 context 구성: 이미 열린 문서와 reload 상태가 어긋날 수 있다.

## Decision: Long context is bounded and visibly marked before send

기본 정책은 content length를 계산하고, 제한을 넘으면 context preview에 reduced/truncated 상태를 표시한다. 실제 제한값과 축약 방식은 구현 task에서 상수로 정하되, 사용자는 전송 전에 complete/reduced 상태를 반드시 볼 수 있어야 한다.

**Rationale**: spec은 긴 문서 첨부 시 토큰/길이 제한을 요구한다. agent별 context window가 다를 수 있으므로 core contract는 "완전/축약 여부와 원래 길이"를 표현하고, 앱은 대상별 제한을 적용할 수 있어야 한다.

**Alternatives considered**:

- 항상 전체 문서 전송: 대형 문서에서 실패하거나 UI가 멈출 수 있다.
- 항상 자동 요약: 사용자가 실제 첨부 범위를 검증하기 어렵고 요약 품질이 agent에 의존한다.
- 긴 문서 전송 금지: 전체 문서 리뷰 흐름의 유용성이 낮아진다.

## Decision: Shared core first, viewer action slot second, compose UI app-local

`packages/markdown-annotation-core`는 context model/build/format을 제공한다. `packages/markdown-annotation-react`는 block toolbar와 selection toolbar에서 quick prompt action을 연결할 수 있는 optional contract만 제공한다. Prompt composition layout과 target unavailable messaging은 앱에서 구현한다.

**Rationale**: 헌장은 Markdown annotation capability에 대해 pure core before shared UI를 요구한다. 현재 MA와 AW는 서로 다른 화면 밀도와 target availability를 가지므로 compose UI를 무리하게 공유하면 app shell 의존이 생긴다.

**Alternatives considered**:

- 모든 quick prompt UI를 shared React package에 구현: target state, dialog shell, send behavior가 앱마다 달라 공유 UI가 과도하게 커진다.
- MA에만 local 구현: AW Markdown workspace와 공유 viewer 변경에서 중복이 생기고 atomic verification이 약해진다.

## Decision: Selection quick annotate lives in the selection toolbar surface

선택 영역 quick prompt는 별도 page/header 버튼이 아니라, 텍스트 선택 후 표시되는 selection toolbar 안의 번개 아이콘 quick annotate 버튼으로 제공한다. 버튼을 누르면 현재 선택 context가 고정된 prompt 작성 다이얼로그가 열리고, send는 사용자가 입력한 prompt text와 context를 함께 active agent-run target에 전달한다.

**Rationale**: 선택 영역 작업은 사용자가 방금 만든 선택 범위와 공간적으로 가까운 action에서 실행되어야 선택 변경, 복사/붙여넣기, 잘못된 scope 전송 위험을 줄일 수 있다. 또한 spec은 quick annotate 버튼이 selection toolbar에 표시되어야 한다고 요구하므로 이 surface가 selection scope의 canonical entry point이다.

**Alternatives considered**:

- Page/header의 selection quick prompt 버튼만 제공: 사용자가 현재 선택과 action의 관계를 확인하기 어렵고 toolbar 표시 요구사항을 만족하지 못한다.
- 선택 즉시 다이얼로그 자동 표시: 사용자의 일반 텍스트 선택/복사 흐름을 방해한다.
- 버튼 클릭 시 기본 prompt를 즉시 전송: 사용자가 prompt를 입력하고 전송해야 한다는 요구사항을 위반한다.

## Decision: Quick prompt response remains in receiving agent conversation

quick prompt 요청 결과는 전송 대상 agent conversation에 남긴다. 문서 annotation으로 자동 연결하거나 응답 영역을 MA에 별도로 만드는 것은 v1 범위 밖으로 둔다.

**Rationale**: spec의 사용자 흐름은 context가 첨부된 prompt 전송까지이다. 응답을 문서에 다시 연결하려면 annotation persistence, response ownership, edit confirmation이 필요해 별도 기능이 된다.

**Alternatives considered**:

- 응답을 자동 annotation으로 저장: 사용자가 검증하지 않은 agent 출력을 문서 상태에 섞을 위험이 있다.
- MA 내부 별도 response 영역 추가: standalone MA에 agent session을 직접 통합해야 하므로 v1 범위를 초과한다.

## Decision: Accessibility is part of the UI contract

번개 아이콘 버튼은 accessible name, tooltip on hover/focus, keyboard focus, disabled reason을 제공해야 한다. Storybook은 keyboard-visible and disabled/unavailable states를 포함한다.

**Rationale**: quick prompt 진입점은 작은 icon-only control이므로 접근성 계약이 없으면 발견성과 조작성 문제가 생긴다.

**Alternatives considered**:

- tooltip만 제공: 보조 기술과 키보드 사용자를 충분히 지원하지 못한다.
- text button 사용: 블럭 toolbar 밀도가 높아져 기존 annotation action과 충돌할 수 있다.
