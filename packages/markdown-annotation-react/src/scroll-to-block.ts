export type ScrollToBlockOptions = {
  behavior?: ScrollBehavior;
};

function resolveScrollBehavior(): ScrollBehavior {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
    return "smooth";
  }

  return window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth";
}

export function scrollToBlock(
  container: ParentNode | null,
  blockId: string,
  options: ScrollToBlockOptions = {},
): boolean {
  if (!container) {
    return false;
  }

  const target = container.querySelector(`[data-block-id="${blockId}"]`);
  if (!target) {
    return false;
  }

  target.scrollIntoView({
    behavior: options.behavior ?? resolveScrollBehavior(),
    block: "start",
  });
  return true;
}
