import { useEffect, useRef, useState } from "react";

import { computeVirtualRowRange, type VirtualRowRange } from "../model/virtual-rows";

const DEFAULT_OVERSCAN = 10;

/**
 * 고정 높이 row 목록의 virtualization hook(AW specs/007 research R11).
 * 반환된 `containerRef`를 row들을 감싸는 요소에 연결하면, 가장 가까운 스크롤
 * 조상을 자동으로 찾아 viewport 근처의 row 범위만 계산한다. 외부 의존성 없이
 * `VirtualizedRunTimeline`(agentic-workbench) 패턴을 고정 높이용으로 단순화했다.
 */
export function useVirtualRows({
  rowCount,
  rowHeight,
  overscan = DEFAULT_OVERSCAN,
}: {
  rowCount: number;
  rowHeight: number;
  overscan?: number;
}): { containerRef: React.RefObject<HTMLDivElement | null> } & VirtualRowRange {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [range, setRange] = useState<VirtualRowRange>(() =>
    computeVirtualRowRange({
      rowCount,
      rowHeight,
      scrollTop: 0,
      viewportHeight: typeof window === "undefined" ? 0 : window.innerHeight,
      containerOffsetTop: 0,
      overscan,
    }),
  );

  useEffect(() => {
    const container = containerRef.current;
    if (!container) {
      return;
    }

    const scrollParent = findScrollParent(container);
    // containerOffsetTop은 스크롤 중에는 불변이다. scroll 핸들러에서
    // getBoundingClientRect(강제 layout)를 반복하지 않도록 캐시하고,
    // 레이아웃이 실제로 바뀌는 ResizeObserver에서만 다시 잰다.
    let containerOffsetTop = 0;

    const measureOffset = () => {
      containerOffsetTop = scrollParent
        ? container.getBoundingClientRect().top -
          scrollParent.getBoundingClientRect().top +
          scrollParent.scrollTop
        : container.getBoundingClientRect().top + window.scrollY;
    };

    const updateRange = () => {
      const viewportHeight = scrollParent
        ? scrollParent.clientHeight
        : window.innerHeight;
      const scrollTop = scrollParent ? scrollParent.scrollTop : window.scrollY;

      setRange((current) => {
        const next = computeVirtualRowRange({
          rowCount,
          rowHeight,
          scrollTop,
          viewportHeight,
          containerOffsetTop,
          overscan,
        });

        return current.startIndex === next.startIndex &&
          current.endExclusive === next.endExclusive &&
          current.totalHeight === next.totalHeight
          ? current
          : next;
      });
    };

    const remeasure = () => {
      measureOffset();
      updateRange();
    };

    const scrollTarget: HTMLElement | Window = scrollParent ?? window;
    scrollTarget.addEventListener("scroll", updateRange, { passive: true });
    const resizeObserver =
      typeof ResizeObserver === "undefined" ? null : new ResizeObserver(remeasure);
    if (resizeObserver) {
      if (scrollParent) {
        resizeObserver.observe(scrollParent);
      }
      resizeObserver.observe(container);
    }
    remeasure();

    return () => {
      scrollTarget.removeEventListener("scroll", updateRange);
      resizeObserver?.disconnect();
    };
  }, [overscan, rowCount, rowHeight]);

  return { containerRef, ...range };
}

function findScrollParent(element: HTMLElement): HTMLElement | null {
  let node = element.parentElement;

  while (node) {
    const { overflowY } = getComputedStyle(node);
    if (overflowY === "auto" || overflowY === "scroll" || overflowY === "overlay") {
      return node;
    }
    node = node.parentElement;
  }

  return null;
}
