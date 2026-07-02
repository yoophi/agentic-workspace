/**
 * 고정 높이 row virtualization의 순수 계산부(AW specs/007 research R11).
 * scroll 좌표 → 렌더할 row 범위. hook(use-virtual-rows)이 이 함수를 소비한다.
 * `endExclusive`는 배타 경계라 소비자는 `rows.slice(startIndex, endExclusive)`
 * 한 줄로 쓰고, 빈 목록도 별도 guard 없이 자연히 빈 배열이 된다.
 */
export type VirtualRowRange = {
  startIndex: number;
  endExclusive: number;
  totalHeight: number;
};

export function computeVirtualRowRange({
  rowCount,
  rowHeight,
  scrollTop,
  viewportHeight,
  containerOffsetTop,
  overscan = 10,
}: {
  rowCount: number;
  rowHeight: number;
  /** scroll 컨테이너의 scrollTop */
  scrollTop: number;
  /** scroll 컨테이너의 보이는 높이 */
  viewportHeight: number;
  /** scroll 컨테이너 content 기준 row 컨테이너의 시작 offset */
  containerOffsetTop: number;
  overscan?: number;
}): VirtualRowRange {
  const totalHeight = Math.max(0, rowCount * rowHeight);

  if (rowCount === 0 || rowHeight <= 0) {
    return { startIndex: 0, endExclusive: 0, totalHeight };
  }

  const visibleTop = Math.max(0, scrollTop - containerOffsetTop);
  const visibleBottom = Math.max(0, scrollTop + viewportHeight - containerOffsetTop);
  const startIndex = Math.min(
    rowCount - 1,
    Math.max(0, Math.floor(visibleTop / rowHeight) - overscan),
  );
  const endExclusive = Math.min(
    rowCount,
    Math.ceil(visibleBottom / rowHeight) + overscan + 1,
  );

  return { startIndex, endExclusive, totalHeight };
}
