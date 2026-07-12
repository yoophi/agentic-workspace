# Research: MA Spec Markdown Preview

## GFM 기반 Markdown block 처리

**Decision**: 기존 block parser를 문서 구조의 단일 기준으로 유지하고, block 내부의 GFM 표현은 `react-markdown`과 `remark-gfm`으로 렌더링한다.

**Rationale**: annotation anchor와 TOC가 이미 block id/line 범위를 공유하므로 별도 AST pipeline을 추가하면 위치 정보가 분기된다. 기존 parser를 확장하면 annotation, Preview, TOC가 동일한 문서 순서를 유지한다.

**Alternatives considered**: 전체 remark AST를 새로운 domain model로 채택하는 방법은 변환 비용과 anchor migration 범위가 크다. 정규식만으로 모든 Markdown을 렌더링하는 방법은 nested list/table/code 경계를 안전하게 처리하지 못한다.

## Wikilink 문법과 안전한 이동

**Decision**: inline 단계에서 `[[target]]`, `[[target | label]]`만 상대 Markdown link로 변환하고, activation 시 현재 문서 디렉터리를 기준으로 정규화한 뒤 허용 root 내부의 `.md` 파일만 연다.

**Rationale**: 표시와 이동 책임을 분리하면 공유 renderer는 app filesystem을 알 필요가 없고, MA boundary에서 traversal·누락·권한 오류를 처리할 수 있다. link는 사용자 click/keyboard activation 때만 이동한다.

**Alternatives considered**: 렌더 시 자동 preload는 명시적 동작 원칙과 파일 접근 최소화를 위반한다. 절대 경로 wikilink 지원은 portable SpecKit 문서와 root safety를 약화한다.

## Task 상태와 chapter 집계

**Decision**: parser가 제공하는 `checked` 값을 task 판별 기준으로 사용한다. H1부터 다음 H1 직전까지를 chapter로 집계하고 H2/H3 task는 선행 H1에 포함한다. 첫 H1 이전 task는 preamble summary로만 표시하며 H1 TOC 항목에는 귀속하지 않는다.

**Rationale**: GFM parser 결과를 사용하면 일반 bullet과 code fence 안의 유사 문구를 제외할 수 있다. H1 기준은 spec의 chapter 의미와 TOC 요약을 일치시킨다.

**Alternatives considered**: 모든 heading level마다 별도 집계하면 부모/자식 중복과 복잡한 TOC 표현이 발생한다. 문자열 검색은 code block 오탐을 만든다.

## 공유 React renderer와 접근성

**Decision**: 완료/미완료는 서로 다른 lucide icon, 텍스트/accessible label, semantic color token을 함께 사용한다. Preview의 task checkbox는 상호작용 control이 아니라 읽기 전용 상태로 렌더링한다.

**Rationale**: 색상 외 신호가 있어 light/dark와 보조기술에서 상태를 구분할 수 있다. renderer를 app-shell 독립으로 유지하면 MA와 AW가 같은 표현을 사용한다.

**Alternatives considered**: native checkbox disabled 표현은 브라우저별 대비가 낮고 Preview에서 변경 가능한 control로 오인될 수 있다. MA에만 UI를 구현하면 AW 표현과 집계가 분기된다.

## SpecKit 예제 구성

**Decision**: feature specification, implementation plan, data model, tasks, requirements checklist를 각각 독립 Markdown fixture로 제공하고 상대 wikilink로 연결한다.

**Rationale**: 다섯 유형은 narrative, table, code, Mermaid, task라는 Preview 핵심 표면을 함께 대표한다. 실제 파일 fixture는 Vite raw import, Storybook/테스트 입력, wikilink 경로 검증에 재사용할 수 있다.

**Alternatives considered**: 하나의 거대 showcase 문서는 산출물별 구조와 문서 이동을 검증하지 못한다. TypeScript 문자열 상수는 Markdown 편집성과 실제 파일 경로 검증을 떨어뜨린다.

## 변경 감지와 오류 격리

**Decision**: 로컬 파일은 기존 Tauri reader/watcher를 재사용하고, 번들 예제는 watcher 대상에서 제외한다. Mermaid와 지원 요소 오류는 해당 block의 대체 UI로 제한한다.

**Rationale**: 기존 filesystem boundary를 유지하면서 예제의 가상 경로를 실제 파일로 오인하지 않는다. block 단위 실패는 긴 spec 전체 검토를 보존한다.

**Alternatives considered**: 예제까지 filesystem watcher로 처리하면 browser surface와 bundle에서 동작이 달라진다. 전체 Preview error boundary만 사용하면 단일 잘못된 diagram이 문서 전체를 숨긴다.

## HTML5 주석 처리

**Decision**: block parsing 전에 code 문맥 밖의 완성된 HTML5 주석만 제거하고 주석 내부 개행은 유지한다. 닫히지 않은 주석은 원문으로 복구한다.

**Rationale**: Preview에서 주석을 숨기면서 기존 source line과 annotation anchor를 유지하고, code 예시 및 손상된 문서의 후속 본문을 보존한다.

**Alternatives considered**: React 렌더 단계에서만 숨기면 빈 annotation block과 TOC 입력이 남을 수 있다. 전체 문자열 정규식 제거는 fenced/inline code와 닫히지 않은 주석을 잘못 제거한다.

## AW Speckit Preview annotation과 TOC 재사용

**Decision**: AW 일반 Markdown workspace의 문서별 annotation map, selection capture, block action, annotation dialog, agent prompt와 TOC 조합을 `features/worktree-workspace` 내부 재사용 단위로 추출하여 Speckit Preview panel에도 연결한다.

**Rationale**: 일반 Markdown panel에 이미 검증된 interaction이 있으므로 Speckit 전용으로 복제하면 편집·삭제·group annotation과 selection offset 동작이 분기된다. 반면 이 상태는 AW의 worktree와 agent prompt callback에 의존하므로 공용 React package가 아니라 AW feature 내부에 남아야 한다.

**Alternatives considered**: Speckit panel에서 별도 annotation 구현은 중복과 회귀 위험이 크다. 전체 workspace 상태를 `markdown-annotation-react`로 이동하는 방법은 공유 UI의 app-shell 독립 원칙을 위반한다. Speckit panel을 일반 Markdown tab으로 강제 이동시키는 방법은 Speckit 목록과 Preview를 함께 보는 작업 흐름을 깨뜨린다.

## Speckit 문서별 상태 범위

**Decision**: annotation은 Speckit 상대 경로를 key로 분리하고, 문서 전환 시 selection, dialog editing target과 highlight는 초기화한다. TOC와 agent prompt는 현재 선택 문서의 blocks/annotations에서 매번 파생한다.

**Rationale**: 문서 간 annotation 혼합을 방지하면서 같은 문서로 돌아왔을 때 작업을 보존한다. 파생 데이터는 별도 저장하지 않아 stale TOC/prompt를 방지한다.

**Alternatives considered**: 문서 전환 시 모든 annotation을 삭제하면 검토 작업이 유실된다. 단일 전역 annotation 배열은 동일 block id가 여러 문서에 반복될 때 충돌한다.
