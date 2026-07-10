# 공유 패키지와 크레이트

모노레포 전체에서 재사용되는 공유 코드. JavaScript/TypeScript는 `packages/`, Rust는 `crates/`에 위치합니다. 공유 승격 기준은 "최소 2개 소비자"입니다.

## crates/git-core

Rust 공유 크레이트. Git 히스토리/그래프/상세/diff의 도메인 타입과 CLI 구현의 단일 진실 공급원.

소스 경로: `crates/git-core/src/`

| 파일 | 역할 |
|------|------|
| `domain.rs` | 15개 도메인 구조체: `GitCommitSummary`, `GitCommitHistory`, `GitCommitPage` (`cursor_invalidated` 필드로 리베이스 감지), `GitCommitGraph`, `GitGraphCommit`, `GitGraphPage`, `GitGraphLayoutHints`, `GitGraphRef`, `GitGraphRefKind` (LocalBranch/RemoteBranch/Tag), `GitCommitFileChange`, `GitFileDiff` (200KB에서 `is_truncated`), `GitCommitDetail`, `GitChangedFileGroup` (Staged/Unstaged/Untracked/Conflicted), `GitChangedFile`, `GitWorktreeChanges`, `GitWorktreeFileDiff` |
| `ports.rs` | `GitHistoryReader` 트레이트 (`list_history`, `get_commit_graph`, `get_commit_detail`, `get_file_diff`) + `GitWorktreeStatusReader` 트레이트 (`status`, `diff`) |
| `git_cli.rs` | `GitCliHistoryReader` — `std::process::Command`로 git CLI 호출 (git2 crate 미사용). 첫 페이지: `std::thread::scope`로 카운트+head+log+refs 병렬 실행. 이후 페이지: 커서 기반 log만 실행. `GitCliWorktreeStatusReader` — `git status --porcelain`과 `git diff` 파싱. `git_error_message` — stderr 메시지 추출 |
| `lib.rs` | 공개 API 재내보내기 |

**소비자**: `apps/agentic-workbench` (worktree git 히스토리/그래프), `apps/git-explorer` (저장소 히스토리/그래프/상세/diff/워킹트리 상태)

## packages/git-graph

`git-core`의 Rust 도메인 타입을 미러링하는 TypeScript 타입 + 커밋 DAG → 레인/세그먼트 그래프 레이아웃 알고리즘.

소스 경로: `packages/git-graph/src/`

| 파일 | 주요 내보내기 |
|------|-------------|
| `types.ts` | 15개 타입: `GitCommitSummary`, `GitCommitPage` (`cursorInvalidated`), `GitCommitHistory`, `GitGraphCommit`, `GitGraphRef`, `GitGraphLayoutHints`, `GitCommitGraph`, `GitCommitFileChange`, `GitCommitDetail`, `GitFileDiff`, `GitCommitQueryOptions`, `GitChangedFileGroup`, `GitChangedFile`, `GitWorktreeChanges`, `GitWorktreeFileDiff` |
| `graph-layout.ts` | `computeGitGraphRows(commits)` — 커밋 리스트를 `GitGraphRow[]`로 변환. 레인 할당, first-parent 메인라인 추적 (lane 0), 10색 팔레트, 세그먼트 연결 (vertical/branch-in/branch-out/merge-in/merge-out). `getMaxGraphLane()` |
| `index.ts` | 배럴 재내보내기 |

## packages/git-ui

프레임워크 독립 React Git UI 컴포넌트. `git-explorer`와 `agentic-workbench`에서 소비.

소스 경로: `packages/git-ui/src/`

### 모델 계층 (`src/model/`)

| 파일 | 역할 |
|------|------|
| `commit-graph.ts` | 커서 기반 페이지네이션 (`GitPageParam`, `getNextGitPageParam`), 페이지 병합 (중복 제거 + 첫 페이지 `totalCount` 보존) |
| `diff.ts` | `parseDiffLines(content)` — unified diff → `DiffLine[]` (이전/새 줄 번호 포함) |
| `file-tree.ts` | `buildFileTree(files)` — 평면 파일 리스트 → 폴더/파일 트리. 단일 자식 폴더 압축 |
| `graph-render.ts` | `laneX(lane)` (레인 → SVG x 좌표, 20px 간격), `graphSegmentPath()` (세그먼트 → SVG 경로), `refsByTarget()` |
| `virtual-rows.ts` | `computeVirtualRowRange()` — 순수 가상화 수학 (startIndex, endExclusive, totalHeight 반환) |

### UI 컴포넌트 (`src/ui/`)

| 컴포넌트 | 역할 |
|---------|------|
| `HistoryGraphView` | 가상화 4열 테이블 (그래프/커밋/작성자/날짜), 크기 조절 열 (localStorage 영속), SVG 그래프 셀, 무한 스크롤 센티넬 |
| `CommitDetailView` | 커밋 메타데이터 + 파일 변경 목록 (트리/리스트 토글) + diff 뷰어 |
| `DiffViewer` | unified diff 렌더링 (이전/새 줄 번호, 색상 코딩) |
| `WorktreeChangesView` | 워킹트리 변경사항을 상태별 그룹화 (conflicted/staged/unstaged/untracked) |
| `GraphCell` | 단일 커밋 그래프 행 SVG 렌더링 |
| `InfiniteLoadSentinel` | IntersectionObserver 기반 무한 스크롤 트리거 |
| `useVirtualRows` | `computeVirtualRowRange`를 감싼 React 훅 |

## packages/markdown-annotation-core

프레임워크 무관 (React 의존성 없음) TypeScript 라이브러리. Markdown 파싱, Mermaid 감지, TOC 추출, 주석 → 에이전트 프롬프트 포맷.

소스 경로: `packages/markdown-annotation-core/src/`

| 파일 | 주요 내보내기 |
|------|-------------|
| `types/annotation.ts` | `AnnotationType` (delete/question/change-request/note/approve), `AnnotationAnchor`, `AnnotationDraft` |
| `types/markdown-block.ts` | `MarkdownBlockType` (heading/paragraph/blockquote/list-item/code/table/hr), `MarkdownBlock` (`mermaid?` 메타데이터 포함) |
| `types/toc.ts` | `TocLevel` (1\|2\|3), `TocEntry` |
| `parse/parse-markdown-to-blocks.ts` | `parseMarkdownToBlocks(markdown)` — 줄 기반 파서: frontmatter, 제목, 펜스드 코드, 테이블, 인용구, 리스트, HR 처리. 안정적인 `block-N` ID와 줄 범위 할당 |
| `mermaid/detect-mermaid-block.ts` | `detectMermaidBlock()` — 언어가 `mermaid`이거나 30+ 선언 토큰(`graph`, `flowchart`, `sequenceDiagram` 등)으로 감지 |
| `format/format-annotations-for-agent.ts` | `formatAnnotationsForAgent()` — 주석을 타입별 지시문으로 포맷 (delete/change-request/question/note/approve), 원본 Markdown 컨텍스트와 줄 범위 포함. `AgentPromptGoal` (edit-document/review-reference/custom) |
| `toc/extract-toc-entries.ts` | `extractTocEntries(blocks)` — 제목 블록 필터링 (레벨 1-3) |
| `toc/strip-inline-markdown.ts` | `stripInlineMarkdown(text)` — `**bold**`, `[link](url)`, `` `code` ``, `~~strike~~`, 이미지 등 제거 |

## packages/markdown-annotation-react

React 컴포넌트 라이브러리. Markdown 뷰잉 + 주석 오버레이 + Mermaid 렌더 + TOC 네비게이션. **킷 독립적**: UI 프리미티브(Button, Tooltip, Dialog)를 컴포넌트 컨트랙트로 주입받아 base-ui(markdown-annotator)와 radix(agentic-workbench) 양쪽에서 사용 가능.

소스 경로: `packages/markdown-annotation-react/src/`

| 컴포넌트 | 역할 |
|---------|------|
| `MarkdownViewer` | 블록을 `react-markdown` + `remark-gfm`으로 렌더링. 인라인 `<mark>` 주석, 블록 레벨 노트 아이콘, Mermaid 확대 표시. 주입형 `components` prop |
| `MarkdownToc` | 목차 네비게이션 (들여쓰기 제목 버튼) |
| `AnnotationInputDialog` | 주석 생성/편집 모달 (타입 선택 + 코멘트). 주입형 `DialogShell` + `TypeSelect` |
| `MermaidDiagram` | 동적 `import("mermaid")`로 Mermaid 렌더. 렌더 상태 관리 (loading/rendered/failed), 오류 분류 |
| `MermaidExpandedView` | Mermaid 확대 모달 뷰 |
| `scroll-to-block.ts` | 블록 ID → 스크롤 이동 헬퍼 |

## packages/ui

최소 shadcn/ui 스타일 컴포넌트 라이브러리 (Tailwind + radix-ui).

소스 경로: `packages/ui/src/`

| 컴포넌트 | 내보내기 |
|---------|---------|
| `lib/utils.ts` | `cn(...inputs)` — `twMerge(clsx(inputs))` |
| `button.tsx` | `Button` (cva 변형: default/outline/secondary/ghost; 크기: default/sm/lg/icon/icon-sm) + `buttonVariants` |
| `input.tsx` | `Input` |
| `resizable.tsx` | `ResizablePanelGroup`, `ResizablePanel`, `ResizableHandle` (`react-resizable-panels` 래핑) |
| `table.tsx` | `Table`, `TableHeader`, `TableBody`, `TableRow`, `TableCell`, `TableHead` |

## packages/workspace-auto-refresh

파일시스템 감시 기반 자동 새로고침 React 훅.

## 다른 앱

### apps/git-explorer

Git 저장소 탐색 데스크톱 앱. 저장소 등록, 커밋 히스토리 시각화, 커밋 상세/diff, 워킹트리 변경사항 모니터링.

**프론트엔드** (`src/`): 단일 `RepositoryPage` 라우트. `widgets/changes-panel/ui/ChangesPanel.tsx`(~39KB)가 메인 패널 — 무한 스크롤 히스토리 그래프, 커밋 상세, 워킹트리 상태, diff 뷰어. `@yoophi/git-ui` 컴포넌트 사용.

**백엔드** (`src-tauri/src/`): 헥사고날 아키텍처. 14개 Tauri 명령. `git-core`의 `GitHistoryReader`/`GitWorktreeStatusReader`에 위임. `notify` crate 기반 파일시스템 감시자(`FsRepositoryWatcher`)로 `.git/` 경로 모니터링, 500ms 디바운스.

### apps/markdown-annotator

Markdown 주석 에디터 데스크톱 앱. Markdown 문서를 열어 블록 단위로 파싱하고, 인라인/블록 주석(delete, change-request, question, note, approve)을 생성하여 AI 에이전트 프롬프트로 내보냅니다.

**프론트엔드** (`src/`): 단일 `AnnotatorPage` (~42KB). 문서 로딩, 블록 파싱, 주석 CRUD, 선택 영역 → 앵커 매핑, 에이전트 프롬프트 내보내기, CLI 설치, TOC, Mermaid 확대, 문서 변경 감지. `@yoophi/markdown-annotation-core`와 `-react` 사용.

**UI 어댑터 패턴**: 앱별 shadcn/ui(base-ui 기반) 컴포넌트를 `MarkdownViewerComponents`/`AnnotationDialogComponents` 컨트랙트로 변환하는 어댑터를 `shared/ui/`에 둡니다. 이로 인해 같은 `-react` 패키지가 서로 다른 UI 킷을 사용하는 앱에서 재사용됩니다.
