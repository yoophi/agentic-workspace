// @vitest-environment happy-dom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { FileBrowserRow } from "@yoophi/file-browser-core";

import { FileBrowserPanelView } from "./FileBrowserPanel";

afterEach(cleanup);
const rows: FileBrowserRow[] = [
  { id: "directory:b/b1", path: "b/b1", label: "b/b1", kind: "directory", depth: 0, expanded: false, hasChildren: true, matchRanges: [], chainPaths: ["b", "b/b1"] },
  { id: "file:file.md", path: "file.md", label: "file.md", kind: "file", depth: 0, expanded: false, hasChildren: false, matchRanges: [], chainPaths: ["file.md"] },
];

function view(overrides = {}) {
  const props = { rows, scanning: false, visitedEntries: 10, matchedDocuments: 1, warnings: [], searchQuery: "", sortBy: "name" as const, selectedPath: null, onSearchChange: vi.fn(), onSortChange: vi.fn(), onToggle: vi.fn(), onSelect: vi.fn(), ...overrides };
  render(<FileBrowserPanelView {...props} />);
  return props;
}

describe("FileBrowserPanelView", () => {
  it("renders compressed rows and selects a file by keyboard", () => {
    const props = view();
    expect(screen.getByRole("treeitem", { name: "b/b1" })).toBeTruthy();
    fireEvent.keyDown(screen.getByRole("treeitem", { name: "file.md" }), { key: "Enter" });
    expect(props.onSelect).toHaveBeenCalledWith(rows[1]);
  });
  it("reports partial warnings and empty state", () => {
    view({ rows: [], warnings: [{ relativePath: "locked", code: "permission_denied" }] });
    expect(screen.getByRole("status").textContent).toContain("1개");
    expect(screen.getByText("표시할 Markdown 문서가 없습니다.")).toBeTruthy();
  });
});
