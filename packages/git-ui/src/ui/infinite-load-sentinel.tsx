import { useEffect, useRef } from "react";

/**
 * 뷰포트에 들어오면 onLoadMore를 호출하는 무한 스크롤 감지용 요소.
 * rootMargin으로 바닥에 닿기 전 선행 로딩 거리를 둔다(기본 240px).
 */
export function InfiniteLoadSentinel({
  hasNextPage,
  isFetchingNextPage,
  onLoadMore,
  rootMargin = "240px 0px",
}: {
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  onLoadMore: () => void;
  rootMargin?: string;
}) {
  const sentinelRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const node = sentinelRef.current;
    if (!node || !hasNextPage || isFetchingNextPage) {
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          onLoadMore();
        }
      },
      { rootMargin },
    );
    observer.observe(node);

    return () => observer.disconnect();
  }, [hasNextPage, isFetchingNextPage, onLoadMore, rootMargin]);

  return <div ref={sentinelRef} className="h-px w-px" aria-hidden />;
}
