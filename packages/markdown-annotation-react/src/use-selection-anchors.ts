import type { AnnotationAnchor } from "@yoophi/markdown-annotation-core/types";

function containsNode(parent: HTMLElement, node: Node) {
  return parent === node || parent.contains(node);
}

function optionalNumber(value: string | undefined) {
  if (value === undefined || value === "") return undefined;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : undefined;
}

/**
 * 현재 브라우저 선택 영역을 블록별 AnnotationAnchor 배열로 변환한다.
 *
 * offset 기준은 **렌더 텍스트**다: prefixRange(`Range.toString().length`)로 계산하므로
 * MarkdownViewer의 inline `<mark>` 분할 기준(segmentTextByAnnotations)과 일치한다.
 * 선택이 여러 블록에 걸치면 블록마다 하나의 anchor를 만든다.
 */
export function getSelectionAnchors(root: HTMLElement | null): AnnotationAnchor[] {
  const selection = typeof window !== "undefined" ? window.getSelection() : null;
  const selectedText = selection?.toString().trim();
  if (!root || !selection || !selectedText || selection.rangeCount === 0) {
    return [];
  }

  const range = selection.getRangeAt(0);
  const blockContentElements = Array.from(
    root.querySelectorAll<HTMLElement>("[data-block-content]"),
  );

  return blockContentElements.flatMap((blockContentElement) => {
    const annotatedElement = blockContentElement.closest<HTMLElement>("[data-block-id]");
    if (!annotatedElement || !range.intersectsNode(blockContentElement)) {
      return [];
    }

    const segmentRange = range.cloneRange();
    segmentRange.selectNodeContents(blockContentElement);

    if (containsNode(blockContentElement, range.startContainer)) {
      segmentRange.setStart(range.startContainer, range.startOffset);
    }

    if (containsNode(blockContentElement, range.endContainer)) {
      segmentRange.setEnd(range.endContainer, range.endOffset);
    }

    const segmentText = segmentRange.toString();
    if (!segmentText.trim()) {
      return [];
    }

    const prefixRange = document.createRange();
    prefixRange.selectNodeContents(blockContentElement);
    prefixRange.setEnd(segmentRange.startContainer, segmentRange.startOffset);
    const startOffset = prefixRange.toString().length;

    return [
      {
        blockId: annotatedElement.dataset.blockId ?? "unknown-block",
        startOffset,
        endOffset: startOffset + segmentText.length,
        selectedText: segmentText,
        startLine: Number(annotatedElement.dataset.startLine) || undefined,
        endLine: Number(annotatedElement.dataset.endLine) || undefined,
        sourceStartOffset: optionalNumber(annotatedElement.dataset.sourceStart),
        sourceEndOffset: optionalNumber(annotatedElement.dataset.sourceEnd),
      },
    ];
  });
}

export type SelectionRect = { top: number; left: number; width: number; height: number };

/**
 * 현재 선택 영역의 사각형들을 container 기준 상대좌표로 반환한다.
 *
 * mouseup 직후 React 리렌더 등으로 브라우저 선택이 풀려도, 이 좌표로 오버레이를
 * 그리면 선택 영역을 시각적으로 유지할 수 있다(MA·AW 공통 선택 하이라이트).
 * container는 position:relative여야 하며, 오버레이는 이 좌표를 absolute로 쓴다.
 */
export function getSelectionRects(container: HTMLElement | null): SelectionRect[] {
  const selection = typeof window !== "undefined" ? window.getSelection() : null;
  if (!container || !selection || selection.rangeCount === 0) {
    return [];
  }

  const range = selection.getRangeAt(0);
  const containerRect = container.getBoundingClientRect();

  return Array.from(range.getClientRects())
    .filter((rect) => rect.width > 0 && rect.height > 0)
    .map((rect) => ({
      top: rect.top - containerRect.top,
      left: rect.left - containerRect.left,
      width: rect.width,
      height: rect.height,
    }));
}
