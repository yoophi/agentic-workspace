// @vitest-environment happy-dom

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import {
  Group as ResizablePanelGroup,
  Panel as ResizablePanel,
  Separator as ResizableHandle,
} from "react-resizable-panels";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useSplitPersistence } from "./use-split-persistence";

const GROUP_WIDTH = 1_100;
const SAVED_WORKSPACE_WIDTH = 573;

function SavedWidthSplit({ hydrated }: { hydrated: boolean }) {
  const split = useSplitPersistence({
    preferredWidth: hydrated ? SAVED_WORKSPACE_WIDTH : undefined,
    hydrated,
    onPersist: vi.fn(),
    minimumA: 360,
    minimumB: 480,
    fallbackSize: "60%",
  });

  return (
    <ResizablePanelGroup orientation="horizontal" {...split.groupProps}>
      <ResizablePanel id="agent" minSize="360px">
        <div>Agent run</div>
      </ResizablePanel>
      <ResizableHandle aria-label="Workspace 영역 크기 조정" {...split.separatorProps} />
      {hydrated ? (
        <ResizablePanel id="workspace" {...split.panelProps}>
          <div>Workspace</div>
        </ResizablePanel>
      ) : null}
    </ResizablePanelGroup>
  );
}

describe("useSplitPersistence saved-width render", () => {
  let container: HTMLDivElement;
  let root: Root;
  let frames: FrameRequestCallback[];

  beforeEach(() => {
    frames = [];
    vi.stubGlobal("requestAnimationFrame", function (this: Window, callback: FrameRequestCallback) {
      if (this !== window) {
        throw new TypeError("Can only call Window.requestAnimationFrame on instances of Window");
      }
      frames.push(callback);
      return frames.length;
    });
    vi.stubGlobal("cancelAnimationFrame", vi.fn());
    vi.stubGlobal(
      "ResizeObserver",
      class {
        observe() {}
        disconnect() {}
      },
    );

    vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockImplementation(function (
      this: HTMLElement,
    ) {
      return this.hasAttribute("data-group") ? GROUP_WIDTH : 0;
    });
    vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockImplementation(function (
      this: HTMLElement,
    ) {
      const panelId = this.getAttribute("data-panel");
      if (panelId === "workspace") return SAVED_WORKSPACE_WIDTH;
      if (panelId === "agent") return GROUP_WIDTH - SAVED_WORKSPACE_WIDTH;
      return 0;
    });
    vi.spyOn(HTMLElement.prototype, "offsetLeft", "get").mockImplementation(function (
      this: HTMLElement,
    ) {
      return this.getAttribute("data-panel") === "workspace"
        ? GROUP_WIDTH - SAVED_WORKSPACE_WIDTH
        : 0;
    });

    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("keeps the actual split content mounted while applying a persisted width", async () => {
    await act(async () => {
      root.render(<SavedWidthSplit hydrated={false} />);
    });
    await act(async () => {
      root.render(<SavedWidthSplit hydrated />);
    });

    expect(container.textContent).toContain("Agent run");
    expect(container.textContent).toContain("Workspace");

    expect(() => {
      while (frames.length > 0) frames.shift()?.(performance.now());
    }).not.toThrow();

    expect(container.textContent).toContain("Agent run");
    expect(container.textContent).toContain("Workspace");
  });
});
