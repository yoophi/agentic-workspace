import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const SOURCE = readFileSync(new URL("./markdown-annotation-workspace.tsx", import.meta.url), "utf8");

describe("MarkdownAnnotationWorkspace contract", () => {
  it("wires block, inline, selection, annotation list, prompt, dialog, and TOC interactions", () => {
    expect(SOURCE).toContain("onRequestBlockComment={model.requestBlockComment}");
    expect(SOURCE).toContain("onRequestBlockDelete={model.toggleBlockDelete}");
    expect(SOURCE).toContain("onCancelInlineAnnotation={model.removeAnnotation}");
    expect(SOURCE).toContain("onEditInlineAnnotation={model.editAnnotation}");
    expect(SOURCE).toContain('aria-label="선택 영역에 노트 추가"');
    expect(SOURCE).toContain('aria-label="선택 영역 삭제"');
    expect(SOURCE).toContain("onSendAnnotationPrompt?.(model.annotationPrompt)");
    expect(SOURCE).toContain("annotationsVisible = true");
    expect(SOURCE).toContain('aria-label={annotationsVisible ? "주석 영역 숨기기" : "주석 영역 보이기"}');
    expect(SOURCE).toContain("onAnnotationsVisibleChange?.(!annotationsVisible)");
    expect(SOURCE).toContain("{annotationsVisible ? <aside");
    expect(SOURCE).toContain("model.annotations.length");
    expect(SOURCE).toContain("model.draftTarget");
    expect(SOURCE).toContain("model.annotationPrompt");
    expect(SOURCE).toContain("<AnnotationInputDialog");
    expect(SOURCE).toContain("<MarkdownPreviewToc");
  });
});
