# Contract: Quick Prompt UI and Delivery

## Purpose

Markdown viewer와 소비 앱 사이의 quick prompt 통합 계약을 정의한다. 이 계약은 public network API가 아니라 shared package와 app UI 사이의 user-interface contract이다.

## Context Scope Contract

Quick prompt context builder는 다음 scope를 받아야 한다.

| Scope | Required Input | Output Label | Empty State |
|-------|----------------|--------------|-------------|
| `selection` | current selection anchors and selected text | 선택 영역 | 선택이 없거나 공백뿐이면 disabled |
| `block` | target `MarkdownBlock` | Markdown 블럭 | block raw content가 공백뿐이면 disabled |
| `document` | loaded document source and metadata | 전체 문서 | document source가 공백뿐이면 disabled |

## Prompt Formatting Contract

Formatted prompt는 다음 정보를 포함해야 한다.

- 사용자 prompt 본문
- source label 또는 path
- scope label
- line range 또는 document-wide 표시
- context length state: complete 또는 reduced
- fenced Markdown context body

전송용 문자열은 사용자 prompt와 context를 분리해서 읽을 수 있어야 하며, reduced context이면 reduced 상태를 명시해야 한다.

## Viewer Action Contract

`MarkdownViewer` 소비 앱은 quick prompt action을 optional로 제공할 수 있다.

- Block toolbar에 block quick prompt action을 표시할 수 있어야 한다.
- Selection toolbar에는 non-empty text region selection 후 quick annotate lightning icon button을 표시해야 한다.
- Selection toolbar quick annotate button은 현재 selection context를 사용해 prompt composition dialog를 열어야 하며, 클릭만으로 agent-run에 전송하면 안 된다.
- Selection toolbar는 app page가 소유하되, same context builder를 사용해야 한다.
- Document-level quick prompt action은 page/header/prompt panel 같은 app-local 위치가 소유한다.
- quick prompt action이 없으면 기존 annotation/delete/comment UI는 현재처럼 동작해야 한다.

## Prompt Composition Contract

Prompt composition UI는 다음 상태를 지원해야 한다.

- `editing`: context preview와 prompt textarea가 표시된다.
- `ready`: prompt text, context, target이 유효하고 send가 가능하다.
- `blocked`: target 없음, empty context, empty prompt 등으로 send가 불가하며 reason이 표시된다.
- `sending`: 중복 전송 방지를 위해 send action이 잠긴다.
- `failed`: target reject 또는 delivery failure를 표시하고 재시도/수정이 가능하다.
- `sent`: 요청이 target에 접수되었음을 표시한다.

Selection toolbar에서 열린 prompt composition UI는 `selection` scope preview를 유지해야 하며, 사용자가 send를 누를 때 dialog textarea의 현재 prompt text를 전송 payload에 포함해야 한다.

## Agent Target Contract

소비 앱은 다음 정보를 prompt composition UI에 제공해야 한다.

- target label
- availability
- unavailable reason
- send function or callback

Standalone MA처럼 target이 없는 실행 모드는 unavailable target으로 표현한다. AW Markdown workspace처럼 agent run 전송 경로가 있는 실행 모드는 기존 prompt send callback을 target으로 제공한다.

## Accessibility Contract

- Lightning icon button은 accessible name을 가져야 한다.
- Tooltip은 hover와 focus 모두에서 같은 의미를 제공해야 한다.
- Disabled state는 사유를 표시해야 한다.
- Prompt composition UI는 keyboard-only flow로 open, edit, send, close가 가능해야 한다.

## Regression Contract

다음 기존 흐름은 quick prompt 추가 후에도 유지되어야 한다.

- text selection capture
- selection highlight
- inline annotation add/edit/delete
- block comment/delete
- Mermaid diagram success/fallback rendering
- ordinary code block rendering
- auto reload stale state 표시
