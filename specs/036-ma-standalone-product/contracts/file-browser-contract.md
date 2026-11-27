# Contract: Shared File Browser

## Package boundary

- `@yoophi/file-browser-core`: React/Tauri/filesystem 비의존 순수 TypeScript.
- `@yoophi/file-browser-react`: React tree interaction과 virtualization. UI kit와 앱 store 비의존.
- AW/MA adapter: scanning/lazy query, scope filter, loading/error copy, icons, menu, persistence.

앱끼리 source를 직접 import하지 않는다. public API는 package `src/index.ts`에서만 노출한다.

## Core API

```ts
type FileBrowserEntry = {
  path: string;
  kind: "file" | "directory";
  modifiedAt?: string;
  size?: number;
  childState?: "unknown" | "loading" | "loaded";
};

type FileBrowserOptions = {
  expandedPaths: ReadonlySet<string>;
  searchQuery?: string;
  sort?: { by: "name" | "path" | "modifiedAt"; direction: "asc" | "desc" };
  compressSingleDirectoryChains?: boolean;
};

declare function createFileBrowserRows(
  entries: readonly FileBrowserEntry[],
  options: FileBrowserOptions,
): readonly FileBrowserRow[];
```

## Observable rules

1. path separator는 `/`로 정규화하며 invalid/absolute/traversal entry는 명시적 validation 오류로 반환한다.
2. file-only input에서도 조상 directory row를 합성한다.
3. 기본 정렬은 directory-first, case-insensitive natural name ascending이며 tie는 normalized path로 결정한다.
4. 검색은 filename과 전체 상대 경로의 case-insensitive substring이다. 일치 row와 조상만 보이고 match range를 제공한다.
5. 파일 없이 단일 child directory만 잇는 chain은 압축한다. 표시 label은 join한 path, action identity는 마지막 directory path다.
6. expanded state는 canonical path로만 판단하며 active file의 조상 확장은 caller가 options에 반영한다.
7. 같은 entries/options는 referential side effect 없이 의미상 동일한 rows를 생성한다.

## React API responsibilities

공통 component는 `role=tree/treeitem`, `aria-level`, `aria-expanded`, `aria-selected`, roving tabindex, Arrow Up/Down/Left/Right, Home/End, Enter/Space selection을 제공한다. focus 대상이 virtual window 밖이면 먼저 scroll한다.

앱은 row render slot, empty/loading/error/progress UI, search/sort controls와 selection callback을 제공한다. package는 Radix/base-ui primitive나 React Query/Zustand를 import하지 않는다.

## Contract fixtures

- file-only ancestor synthesis
- `a/file.md`, `b/b1/file2.md`, `d/file.md`에서 `b/b1` 압축, 빈 `c` 미표시
- Unicode/case/numeric natural sort
- filename/path substring search와 조상 유지
- duplicated progressive batches
- unknown/loading/loaded lazy directory
- 10,000 entries의 deterministic output
- keyboard/VoiceOver semantics와 virtualized focus

shared package 변경은 package test와 AW·MA adapter test/build를 같은 변경 단위에서 통과해야 한다.
