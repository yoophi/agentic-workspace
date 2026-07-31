import { useCallback, useEffect, useRef, useState } from "react";
import {
  usePanelRef,
  type PanelImperativeHandle,
  type PanelSize,
} from "react-resizable-panels";

import {
  clampPanelWidth,
  shouldPersistPanelWidth,
} from "@/features/worktree-workspace/model/workspace-layout";

type SplitPersistenceOptions = {
  /// 저장된 B의 선호 폭. hydrate 전이거나 저장 값이 없으면 undefined.
  preferredWidth: number | undefined;
  /// 저장된 레코드를 읽어 적용한 뒤에만 저장을 허용한다. (research.md 결정 5)
  hydrated: boolean;
  onPersist: (widthPx: number) => void;
  /// A와 B가 모두 조작 가능하도록 유지할 최소 폭.
  minimumA: number;
  minimumB: number;
  /// 저장된 폭이 없을 때 쓸 기본 크기. `"60%"`처럼 CSS 크기 문자열.
  fallbackSize: string;
  /// 같은 hook 인스턴스가 다른 분할을 담당하게 될 때(예: 표시 패널 전환) 내부 상태를 초기화할 키.
  resetKey?: string;
};

type FrameScheduler = {
  requestFrame: (callback: FrameRequestCallback) => number;
  cancelFrame: (handle: number) => void;
};

const DYNAMIC_PANEL_LAYOUT_ERRORS = [
  /^Group .+ not found$/,
  /^Panel constraints not found for Panel /,
  /^Layout not found for Panel /,
];
const MAX_PANEL_RESIZE_ATTEMPTS = 3;

function isPendingDynamicPanelLayout(error: unknown): boolean {
  return (
    error instanceof Error &&
    DYNAMIC_PANEL_LAYOUT_ERRORS.some((pattern) => pattern.test(error.message))
  );
}

export function schedulePanelResize(
  getPanel: () => Pick<PanelImperativeHandle, "resize"> | null,
  displayWidth: number,
  scheduler: FrameScheduler = {
    // WebKit의 requestAnimationFrame/cancelAnimationFrame은 Window receiver를 검사한다.
    // 함수를 객체 속성에 그대로 담으면 scheduler가 this가 되어 번들 앱에서 예외가 난다.
    requestFrame: (callback) => window.requestAnimationFrame(callback),
    cancelFrame: (handle) => window.cancelAnimationFrame(handle),
  },
) {
  let cancelled = false;
  let frameHandle: number | undefined;
  let attempts = 0;

  const resizeAfterLayout = () => {
    if (cancelled) return;
    attempts += 1;

    try {
      getPanel()?.resize(displayWidth);
    } catch (error) {
      if (isPendingDynamicPanelLayout(error) && attempts < MAX_PANEL_RESIZE_ATTEMPTS) {
        frameHandle = scheduler.requestFrame(resizeAfterLayout);
      }
      // 저장 폭 복원은 보조 동작이다. 패널이 끝내 준비되지 않거나 예상하지 못한
      // 오류가 발생해도 기본 레이아웃을 유지하고 화면 렌더를 중단하지 않는다.
    }
  };

  frameHandle = scheduler.requestFrame(resizeAfterLayout);

  return () => {
    cancelled = true;
    if (frameHandle !== undefined) scheduler.cancelFrame(frameHandle);
  };
}

/// 분할 하나의 `A:B = *:1` 규칙을 담당한다.
///
/// - B만 저장된 픽셀 폭으로 고정하고(`preserve-pixel-size`), A는 남은 공간을 채운다.
/// - 저장된 폭이 현재 컨테이너에 맞지 않으면 표시 폭만 최소 A/B 범위로 좁힌다.
///   저장 값은 그대로 유지된다. (research.md 결정 3)
/// - 저장은 분할선을 직접 조작(포인터/키보드/더블클릭)해 레이아웃이 안정된 뒤에만 한다.
///   `Group.onLayoutChanged`는 포인터를 놓은 뒤 한 번만 호출되므로 드래그 중 파일 쓰기가 없다.
///   창 크기 변화나 표시 제한으로 생긴 폭은 사용자 조작이 아니므로 저장하지 않는다.
///   (research.md 결정 5)
export type SplitPersistence = ReturnType<typeof useSplitPersistence>;

export function useSplitPersistence({
  preferredWidth,
  hydrated,
  onPersist,
  minimumA,
  minimumB,
  fallbackSize,
  resetKey,
}: SplitPersistenceOptions) {
  const panelRef = usePanelRef();
  const [containerWidth, setContainerWidth] = useState<number | undefined>(undefined);
  const observerRef = useRef<ResizeObserver | null>(null);
  const latestWidthRef = useRef<number | undefined>(undefined);
  const userInitiatedRef = useRef(false);

  const setGroupElement = useCallback((element: HTMLDivElement | null) => {
    observerRef.current?.disconnect();
    observerRef.current = null;
    if (!element) return;
    setContainerWidth(element.clientWidth || undefined);
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width;
      if (width) setContainerWidth(width);
    });
    observer.observe(element);
    observerRef.current = observer;
  }, []);

  useEffect(() => () => observerRef.current?.disconnect(), []);

  // 담당 분할이 바뀌면 이전 분할의 폭·조작 의도를 물려받지 않는다.
  // 아래 표시 폭 보정 effect보다 먼저 선언해 같은 커밋에서 먼저 실행되게 한다.
  useEffect(() => {
    latestWidthRef.current = undefined;
    userInitiatedRef.current = false;
  }, [resetKey]);

  const displayWidth = clampPanelWidth({
    preferredWidth,
    containerWidth,
    minimumA,
    minimumB,
  });

  // 표시 폭이 선호 폭과 달라지는 경우(좁은 화면 등)에만 명령형으로 맞춘다.
  // 이 변경은 사용자 조작이 아니므로 저장되지 않는다.
  useEffect(() => {
    if (!displayWidth || !panelRef.current) return;
    if (latestWidthRef.current !== undefined && Math.abs(latestWidthRef.current - displayWidth) < 1) {
      return;
    }
    return schedulePanelResize(() => panelRef.current, displayWidth);
  }, [displayWidth, panelRef]);

  const handleResize = useCallback((size: PanelSize) => {
    latestWidthRef.current = size.inPixels;
  }, []);

  const markUserInitiated = useCallback(() => {
    userInitiatedRef.current = true;
  }, []);

  const handleLayoutChanged = useCallback(() => {
    const userInitiated = userInitiatedRef.current;
    userInitiatedRef.current = false;
    const nextWidth = latestWidthRef.current;
    if (!shouldPersistPanelWidth({ nextWidth, preferredWidth, hydrated, userInitiated })) return;
    onPersist(Math.round(nextWidth as number));
  }, [hydrated, onPersist, preferredWidth]);

  return {
    /// `Group`에 전개한다.
    groupProps: { elementRef: setGroupElement, onLayoutChanged: handleLayoutChanged },
    /// 분할선(`Separator`)에 전개한다. 사용자 조작 의도를 표시한다.
    separatorProps: {
      onPointerDown: markUserInitiated,
      onKeyDown: markUserInitiated,
      onDoubleClick: markUserInitiated,
    },
    /// B(오른쪽) `Panel`에 전개한다.
    panelProps: {
      panelRef,
      defaultSize: displayWidth ? `${displayWidth}px` : fallbackSize,
      minSize: `${minimumB}px`,
      groupResizeBehavior: "preserve-pixel-size" as const,
      onResize: handleResize,
    },
  };
}
