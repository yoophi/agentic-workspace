# 033 Markdown 렌더링 품질 개선 — 조사

## 결정 1: 행 기반 블록 파서를 CommonMark AST 기반 구조 추출로 교체한다

- **결정**: `markdown-annotation-core`는 CommonMark 호환 remark 파서와
  `remark-gfm`을 사용해 문서 전체 AST를 만든 뒤, 위치(position)와 의미론적 노드
  경계에서 주석용 블록을 추출한다. 목록/항목의 모든 하위 구조를 독립 블록으로
  평탄화하여 렌더 의미를 재구성하지 않는다.
- **근거**: CommonMark는 먼저 문서 블록 구조를 확정하고 이후 inline을 해석한다.
  목록 항목은 lazy continuation, 중첩 블록, 여러 문단을 포함할 수 있으므로 정규식
  행 단위 분해만으로는 부모 관계를 재현할 수 없다. 현재
  `parse-markdown-to-blocks.ts`는 목록 항목을 한 줄로 확정해 이 정보를 잃는다.
- **검토한 대안**: 기존 정규식 parser에 continuation·들여쓰기 규칙을 추가하는 방법은
  CommonMark의 탭, marker 폭, loose/tight list, 인용문·코드 결합 규칙을 중복 구현해야
  하므로 채택하지 않는다.

## 결정 2: 문서를 한 번만 렌더링하고 의미론적 HTML 요소에 주석 정보를 부여한다

- **결정**: `MarkdownViewer`는 블록별로 `ReactMarkdown`을 호출하지 않고 원문 전체를
  한 번 렌더링한다. 컴포넌트/AST 변환 adapter가 `li`, `p`, `h1`~`h6`, `blockquote`,
  `pre`, `table` 등 실제 의미론적 요소에 안정 block id와 주석 제어를 연결한다.
  목록의 직접 자식은 항상 `li`이고, toolbar/mark는 해당 요소 안에서 HTML 규칙을
  지키도록 배치한다.
- **근거**: `react-markdown`은 CommonMark를 렌더링하고 `remark-gfm`은 표, 작업 목록,
  취소선, 자동 링크 등 GFM 확장을 제공한다. 전체 문서 트리를 유지해야 `ul > li`,
  `ol[start] > li`, 중첩 `ul/ol`, loose list의 문단이 올바른 계층으로 나온다.
- **검토한 대안**: 각 `MarkdownBlock.content`를 독립 렌더링하고 wrapper를 보정하는
  방법은 목록의 부모와 후속 문단이 이미 분리됐기 때문에 유효한 DOM을 복원할 수 없어
  제외한다.

## 결정 3: 주석 앵커는 AST source position과 렌더 텍스트 오프셋을 모두 검증한다

- **결정**: 새 블록 모델은 source 범위(start/end line·column·offset)와 사람이 선택한
  렌더 텍스트 범위를 명시한다. 선택 영역은 가장 가까운 의미론적 주석 블록에만
  연결하고, 문서 갱신 뒤 source/text 검증이 실패한 앵커는 stale 상태로 남긴다.
  다른 블록으로 자동 이동하지 않는다.
- **근거**: 현행 `getSelectionAnchors`와 `segmentTextByAnnotations`는 렌더 텍스트
  오프셋을 공통 기준으로 사용한다. 이를 유지하면서 AST 위치를 추가하면 목록 안의
  여러 문단·중첩 요소에서도 선택과 구조를 함께 추적할 수 있다.
- **검토한 대안**: 화면의 DOM 순번만 저장하는 방식은 renderer 변경과 HTML 정규화에
  취약하고, 유사 텍스트로 자동 재부착하는 방식은 FR-007의 임의 이동 금지에 어긋난다.

## 결정 4: fixture 코퍼스는 독자 입력과 정규화된 의미 구조를 함께 검증한다

- **결정**: core에 fixture id, 분류, Markdown 입력, 기대 의미 트리/필수 HTML 관계,
  주석 기대를 포함한 코퍼스를 둔다. React 테스트는 fixture별 렌더 HTML에서 구조
  selector와 금지 관계를 검사해 실패 fixture id·기대 구조를 표시한다.
- **근거**: 표준 예제를 그대로 대량 복제하지 않고 최소 재현 입력과 독자 기대값을
  관리하면 라이선스·유지보수 부담을 낮추면서 회귀 원인을 식별할 수 있다.
- **검토한 대안**: 스냅샷 전용 검증은 실패 차이가 크고 의도가 보이지 않아, 구조
  assertion과 제한적 스냅샷을 함께 쓰는 방식보다 진단성이 낮아 제외한다.

## 결정 5: 기존 안전 정책과 특수 기능은 renderer extension으로 명시적으로 보존한다

- **결정**: 원시 HTML은 그대로 활성화하지 않으며, 링크는 기존 내부 wikilink 활성화
  정책을 유지하고 위험 스킴을 허용하지 않는다. Mermaid fenced code는 코드 AST
  노드에서만 감지해 기존 `MermaidExpandedView`로 위임한다. TOC와 task summary는
  AST 유래 block metadata를 소비한다.
- **근거**: `react-markdown`은 raw HTML을 기본적으로 실행하지 않으며, 현행 구현도
  `stripHtmlComments` 및 별도 링크 handler로 안전 경계를 둔다. 특수 기능을
  일반 Markdown 문자열 재렌더링에 의존시키면 문법 문맥을 다시 잃는다.
- **검토한 대안**: raw HTML 허용 또는 `rehype-raw` 도입은 활성 콘텐츠 정책을 넓히므로
  제외한다.

## 결정 6: AW 주석 영역 전환은 workspace-local UI 상태로 관리한다

- **결정**: AW Markdown workspace의 주석 목록·agent prompt·TOC가 있는 보조 열을
  하나의 표시 상태로 묶어 전환한다. 기본값은 표시이며, 숨김 상태에서는 본문 열이
  사용 가능한 가로 공간 전체를 차지한다. 상태 전환은 열린 문서, annotation 배열,
  선택 highlight, draft target 및 편집 id를 다시 만들거나 초기화하지 않는다.
- **근거**: 이 상태는 AW의 패널 배치와 읽기 밀도에만 관계되고, MA 또는 공유
  `MarkdownViewer`의 문법·주석 계약에는 관계되지 않는다. 현재 보조 열은 이미
  `features/worktree-workspace` 내부에서 구성되므로 화면 가까이에 상태를 두면
  불필요한 shared API와 저장 모델을 피할 수 있다.
- **검토한 대안**: 표시 상태를 문서별 annotation 데이터에 저장하는 방법은 사용자
  데이터 모델을 불필요하게 변경하고, 공유 viewer prop으로 옮기는 방법은 AW에만
  필요한 layout 책임을 모든 소비자에게 노출하므로 채택하지 않는다.

## 참고 자료

- [CommonMark Specification](https://spec.commonmark.org/spec): 블록 우선 해석,
  목록 continuation/들여쓰기 규칙.
- [GitHub Flavored Markdown Spec](https://github.github.com/gfm/): task list를
  포함한 GFM 확장.
- [remark-gfm](https://github.com/remarkjs/remark-gfm): GFM extension 지원 범위.
- [react-markdown](https://github.com/remarkjs/react-markdown): React의 안전한
  Markdown 렌더링 및 remark plugin 결합 방식.
