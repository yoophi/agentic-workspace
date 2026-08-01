// @vitest-environment happy-dom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { FileBrowserRow } from "@yoophi/file-browser-core";
import { FileBrowserTree } from "./FileBrowserTree";

const rows: FileBrowserRow[] = Array.from({ length: 100 }, (_, index) => ({
  id: `file:file-${index}.md`, path: `file-${index}.md`, label: `file-${index}.md`,
  kind: "file", depth: 0, expanded: false, hasChildren: false, matchRanges: [], chainPaths: [`file-${index}.md`],
}));

afterEach(cleanup);

describe("FileBrowserTree", () => {
  it("exposes tree semantics and roving focus", () => {
    render(<FileBrowserTree rows={rows.slice(0, 3)} selectedPath="file-1.md" ariaLabel="문서" onSelect={vi.fn()} onToggle={vi.fn()} renderRow={(row) => row.label} />);
    expect(screen.getByRole("tree", { name: "문서" })).toBeTruthy();
    expect(screen.getByRole("treeitem", { name: "file-1.md" }).getAttribute("aria-selected")).toBe("true");
    expect(screen.getAllByRole("treeitem").filter((item) => item.tabIndex === 0)).toHaveLength(1);
  });

  it("moves with Arrow and End and selects with Enter", () => {
    const onSelect = vi.fn();
    render(<FileBrowserTree rows={rows.slice(0, 3)} ariaLabel="문서" onSelect={onSelect} onToggle={vi.fn()} renderRow={(row) => row.label} />);
    const first = screen.getByRole("treeitem", { name: "file-0.md" });
    fireEvent.keyDown(first, { key: "End" });
    const last = screen.getByRole("treeitem", { name: "file-2.md" });
    fireEvent.keyDown(last, { key: "Enter" });
    expect(onSelect).toHaveBeenCalledWith(rows[2]);
  });

  it("keeps the DOM window proportional to viewport size", () => {
    render(<FileBrowserTree rows={rows} height={96} rowHeight={32} overscan={1} ariaLabel="문서" onSelect={vi.fn()} onToggle={vi.fn()} renderRow={(row) => row.label} />);
    expect(screen.getAllByRole("treeitem").length).toBeLessThan(10);
  });
});
