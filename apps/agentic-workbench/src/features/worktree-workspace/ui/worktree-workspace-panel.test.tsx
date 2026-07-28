import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

import type { WorktreeFileEntry } from "@/entities/worktree-file/model/types";
import {
  buildFileTreeRows,
  isParentDirectoryLoaded,
  mergeWorktreeFileEntries,
} from "@/features/worktree-workspace/model/file-tree";

const WORKTREE_WORKSPACE_PANEL_SOURCE = readFileSync(
  new URL("./worktree-workspace-panel.tsx", import.meta.url),
  "utf8",
);

function file(relativePath: string, size = 10): WorktreeFileEntry {
  const segments = relativePath.split("/");
  return {
    name: segments[segments.length - 1] ?? relativePath,
    path: `/repo/${relativePath}`,
    relativePath,
    isDir: false,
    size,
    modifiedMs: 1,
  };
}

function dir(relativePath: string): WorktreeFileEntry {
  const segments = relativePath.split("/");
  return {
    name: segments[segments.length - 1] ?? relativePath,
    path: `/repo/${relativePath}`,
    relativePath,
    isDir: true,
    size: 0,
    modifiedMs: 1,
  };
}

describe("worktree workspace file tree", () => {
  it("merges lazy-loaded entries by root-relative path", () => {
    const entries = mergeWorktreeFileEntries([
      [file("README.md"), dir("src")],
      [file("src/app.ts"), dir("src/deep")],
      [file("src/deep/inner.ts"), file("src/app.ts", 20)],
    ]);

    expect(entries.map((entry) => entry.relativePath)).toEqual([
      "README.md",
      "src",
      "src/app.ts",
      "src/deep",
      "src/deep/inner.ts",
    ]);
    expect(entries.find((entry) => entry.relativePath === "src/app.ts")?.size).toBe(10);
  });

  it("shows nested rows only when every ancestor is expanded", () => {
    const entries = [
      file("README.md"),
      dir("src"),
      file("src/app.ts"),
      dir("src/deep"),
      file("src/deep/inner.ts"),
    ];

    expect(buildFileTreeRows(entries, new Set()).map((row) => row.relativePath)).toEqual([
      "README.md",
      "src",
    ]);

    expect(buildFileTreeRows(entries, new Set(["src"])).map((row) => row.relativePath)).toEqual([
      "README.md",
      "src",
      "src/app.ts",
      "src/deep",
    ]);

    expect(
      buildFileTreeRows(entries, new Set(["src", "src/deep"])).map((row) => [
        row.relativePath,
        row.depth,
        row.isExpanded,
      ]),
    ).toEqual([
      ["README.md", 0, false],
      ["src", 0, true],
      ["src/app.ts", 1, false],
      ["src/deep", 1, true],
      ["src/deep/inner.ts", 2, false],
    ]);
  });

  it("keeps duplicate basenames distinct by full relative path", () => {
    const entries = mergeWorktreeFileEntries([
      [dir("src"), dir("docs")],
      [file("src/app.ts"), file("docs/app.ts")],
    ]);

    const rows = buildFileTreeRows(entries, new Set(["src", "docs"]));

    expect(rows.filter((row) => row.name === "app.ts").map((row) => row.relativePath)).toEqual([
      "docs/app.ts",
      "src/app.ts",
    ]);
  });

  it("preserves Korean and space-containing relative paths", () => {
    const entries = mergeWorktreeFileEntries([
      [dir("docs")],
      [file("docs/한글 파일.md")],
    ]);

    const rows = buildFileTreeRows(entries, new Set(["docs"]));

    expect(rows.map((row) => row.relativePath)).toEqual(["docs", "docs/한글 파일.md"]);
  });

  it("treats a selection as stale only after its parent directory is loaded", () => {
    expect(isParentDirectoryLoaded("README.md", [])).toBe(true);
    expect(isParentDirectoryLoaded("src/app.ts", [])).toBe(false);
    expect(isParentDirectoryLoaded("src/app.ts", ["src"])).toBe(true);
    expect(isParentDirectoryLoaded("src/deep/inner.ts", ["src"])).toBe(false);
    expect(isParentDirectoryLoaded("src/deep/inner.ts", ["src", "src/deep"])).toBe(true);
  });

  it("connects Speckit document selection to markdown preview state", () => {
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain(
      'type WorkspaceTabId = "git" | "files" | "markdown" | "speckit"',
    );
    // prop이 추가되어도 깨지지 않게 전달 여부만 확인한다.
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toMatch(
      /<SpeckitWorkspaceTab[^>]*onSendAnnotationPrompt=\{onSendAnnotationPrompt\}/,
    );
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toMatch(
      /<SpeckitWorkspaceTab[^>]*onSendSddPrompt=\{onSendSddPrompt\}/,
    );
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain("onSelectDocument={setSelectedDocumentPath}");
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain('".specify/feature.json"');
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain("activeFeaturePath={activePointer.featurePath}");
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain(
      'readWorktreeTextFile(worktree.path, selectedDocumentPath ?? "")',
    );
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain("<MarkdownAnnotationWorkspace");
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain("tocEntries={tocEntries}");
  });

  it("keeps stale Speckit document selection separate from successful preview content", () => {
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain("staleDocumentSelection");
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain("선택한 Speckit 문서가 현재 목록에 없습니다");
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain("Speckit preview를 표시할 수 없습니다.");
  });

  it("allows both Markdown preview paths to hide and restore the annotation area", () => {
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain("const [annotationsVisible, setAnnotationsVisible] = useState(true);");
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain('aria-label={annotationsVisible ? "주석 영역 숨기기" : "주석 영역 보이기"}');
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain("annotationsVisible={annotationsVisible}");
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain("onAnnotationsVisibleChange={setAnnotationsVisible}");
  });
});

describe("WorktreeWorkspacePanel 패널 선택", () => {
  it("선택된 패널 종류의 콘텐츠만 렌더링한다", () => {
    // 네 분기가 모두 배타적이어야 선택 없음 → 표시 전환에서 잔여 콘텐츠가 남지 않는다. (FR-007)
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain('selectedTab === "git" ? (');
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain('selectedTab === "files" ? (');
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain('selectedTab === "markdown" ? (');
  });

  it("외부에서 전달한 선택 패널을 표시 대상으로 따른다", () => {
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain("selectedPanel ?? initialTab");
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain("if (selectedPanel) setSelectedTab(selectedPanel)");
  });

  it("탭 목록 UI와 고아 tabpanel role을 남기지 않는다", () => {
    // 제어는 화면 오른쪽 selector가 담당한다. (research.md 결정 4)
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).not.toContain('role="tablist"');
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).not.toContain('role="tabpanel"');
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).not.toContain('role="tab"');
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).not.toContain("workspaceTabs");
  });
});

describe("WorktreeWorkspacePanel 내부 B 폭", () => {
  const innerPanels = [
    { panel: "git", a: "git-workspace-nav", b: "git-workspace-detail" },
    { panel: "files", a: "file-workspace-tree", b: "file-workspace-preview" },
    { panel: "markdown", a: "markdown-workspace-tree", b: "markdown-workspace-preview" },
    { panel: "speckit", a: "speckit-workspace-list", b: "speckit-workspace-preview" },
  ] as const;

  it("네 패널 종류의 최소 폭과 기본 B 크기를 모두 정의한다", () => {
    for (const { panel } of innerPanels) {
      expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toMatch(
        new RegExp(`${panel}: \\{ minimumA: \\d+, minimumB: \\d+, fallbackSize: "\\d+%" \\}`),
      );
    }
  });

  it("오른쪽 B에만 저장 폭을 적용하고 왼쪽 A는 남은 공간을 채운다", () => {
    for (const { a, b } of innerPanels) {
      // A: 저장 크기(defaultSize) 없음
      expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain(`<ResizablePanel id="${a}" minSize=`);
      // B: 분할 저장 props 적용
      expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain(
        `<ResizablePanel id="${b}" {...split.panelProps}>`,
      );
    }
  });

  it("표시 중인 패널 종류를 저장 키로 써서 다른 종류를 덮어쓰지 않는다", () => {
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain("onPersistPanelWidth?.(selectedTab, widthPx)");
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain("preferredWidth: panelWidthsPx?.[selectedTab]");
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain("resetKey: selectedTab");
  });

  it("네 패널 모두 분할선 조작 의도와 안정 시점 저장을 연결한다", () => {
    expect(
      WORKTREE_WORKSPACE_PANEL_SOURCE.match(/\{\.\.\.split\.groupProps\}/g) ?? [],
    ).toHaveLength(4);
    expect(
      WORKTREE_WORKSPACE_PANEL_SOURCE.match(/\{\.\.\.split\.separatorProps\}/g) ?? [],
    ).toHaveLength(4);
    expect(
      WORKTREE_WORKSPACE_PANEL_SOURCE.match(/\{\.\.\.split\.panelProps\}/g) ?? [],
    ).toHaveLength(4);
  });
});
