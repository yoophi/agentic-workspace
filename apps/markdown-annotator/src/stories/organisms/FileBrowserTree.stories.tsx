import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { createFileBrowserRows, type FileBrowserEntry } from "@yoophi/file-browser-core";

import { FileBrowserPanelView } from "@/features/file-browser/ui/FileBrowserPanel";

const entries: FileBrowserEntry[] = [
  { path: "a/file.md", kind: "file" },
  { path: "b/b1/file2.md", kind: "file" },
  { path: "문서/릴리즈10.md", kind: "file" },
  { path: "문서/릴리즈2.md", kind: "file" },
];
const rows = createFileBrowserRows(entries, { expandedPaths: new Set(["a", "b/b1", "문서"]) });

const meta = {
  title: "Organisms/File Browser Tree",
  component: FileBrowserPanelView,
  args: { rows, scanning: false, visitedEntries: 12, matchedDocuments: 4, warnings: [], searchQuery: "", sortBy: "name", selectedPath: "a/file.md", onSearchChange: fn(), onSortChange: fn(), onToggle: fn(), onSelect: fn() },
  parameters: { layout: "fullscreen" },
} satisfies Meta<typeof FileBrowserPanelView>;
export default meta;
type Story = StoryObj<typeof meta>;

export const CompressedAndUnicode: Story = {};
export const Loading: Story = { args: { scanning: true, visitedEntries: 842, matchedDocuments: 31 } };
export const PermissionWarning: Story = { args: { warnings: [{ relativePath: "private", code: "permission_denied" }] } };
export const Empty: Story = { args: { rows: [], matchedDocuments: 0 } };
export const LargeVirtualized: Story = {
  args: { rows: createFileBrowserRows(Array.from({ length: 1_000 }, (_, index) => ({ path: `docs/file-${index}.md`, kind: "file" as const })), { expandedPaths: new Set(["docs"]) }), matchedDocuments: 1_000 },
};
