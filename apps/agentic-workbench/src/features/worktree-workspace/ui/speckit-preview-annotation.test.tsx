import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const SOURCE = readFileSync(new URL("./worktree-workspace-panel.tsx", import.meta.url), "utf8");

describe("Speckit Preview annotation and TOC integration", () => {
  it("connects path-keyed annotation state and prompt dispatch", () => {
    expect(SOURCE).toContain("useMarkdownAnnotationWorkspace({");
    expect(SOURCE).toContain("documentPath: selectedDocumentPath");
    expect(SOURCE).toContain("onSendAnnotationPrompt={onSendAnnotationPrompt}");
    expect(SOURCE).toContain("model={annotationWorkspace}");
  });

  it("derives TOC entries and scopes scrolling to the Speckit preview pane", () => {
    expect(SOURCE).toContain("const tocEntries = useMemo(() => extractTocEntries(blocks), [blocks])");
    expect(SOURCE).toContain("previewRef={previewPaneRef}");
    expect(SOURCE).toContain("tocEntries={tocEntries}");
  });
});
