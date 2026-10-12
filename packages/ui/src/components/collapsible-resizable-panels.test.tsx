// @vitest-environment jsdom
import { act, createRef, useEffect, useState, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PanelImperativeHandle } from "react-resizable-panels";
import { CollapsibleResizablePanels, type CollapsibleResizablePanelsHandle, type CollapsiblePanelDefinition } from "./collapsible-resizable-panels";

// Unit tests exercise the real component/content. Layout is modeled here;
// browser tests use the actual primitive, measured widths and pointer/keyboard input.
const layout = vi.hoisted(() => ({
  sizes: { left: 70, right: 30 } as Record<string, number>,
  collapsed: new Set<string>(),
  emit: (_sizes: Record<string, number>) => {},
}));
vi.mock("@yoophi/ui/components/resizable", async () => {
  const React = await import("react");
  return {
    ResizablePanelGroup: ({ children, onLayoutChanged }: { children: ReactNode; onLayoutChanged: typeof layout.emit }) => {
      layout.emit = onLayoutChanged;
      return <div>{children}</div>;
    },
    ResizablePanel: ({ children, id, panelRef }: { children: ReactNode; id: string; panelRef: React.Ref<PanelImperativeHandle> }) => {
      React.useImperativeHandle(panelRef, () => ({
        getSize: () => ({ asPercentage: layout.sizes[id], inPixels: layout.sizes[id] * 10 }),
        isCollapsed: () => layout.collapsed.has(id),
        collapse: () => { layout.collapsed.add(id); layout.sizes[id] = 4; layout.emit({ ...layout.sizes }); },
        expand: () => { layout.collapsed.delete(id); },
        resize: (size) => { layout.sizes[id] = Number.parseFloat(String(size)); layout.emit({ ...layout.sizes }); },
      }));
      return <div>{children}</div>;
    },
    ResizableHandle: ({ disabled }: { disabled: boolean }) => <div role="separator" aria-disabled={disabled} />,
  };
});

let host: HTMLDivElement;
let root: Root;
const mounts = { count: 0, unmounts: 0 };
function Draft() {
  const [value, setValue] = useState("");
  useEffect(() => { mounts.count++; return () => { mounts.unmounts++; }; }, []);
  return <input aria-label="draft" value={value} onChange={(e) => setValue(e.target.value)} />;
}
const left: CollapsiblePanelDefinition = { id: "left", title: "Document", content: <textarea defaultValue="unsaved document" /> };
const right: CollapsiblePanelDefinition = { id: "right", title: "Chat", content: <Draft />, keepMounted: true, contentClassName: "flex" };
function mount(options: { left?: CollapsiblePanelDefinition; right?: CollapsiblePanelDefinition; initial?: "left" | "right" } = {}) {
  const ref = createRef<CollapsibleResizablePanelsHandle>();
  act(() => root.render(<CollapsibleResizablePanels leftPanel={options.left ?? left} rightPanel={options.right ?? right} controllerRef={ref} defaultCollapsedPanel={options.initial} />));
  act(() => layout.emit({ ...layout.sizes }));
  return ref;
}
function clickTitle(title: string) {
  const button = [...host.querySelectorAll("button")].find((b) => b.textContent === title)!;
  act(() => button.click());
}
beforeEach(() => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  layout.sizes = { left: 70, right: 30 }; layout.collapsed.clear();
  mounts.count = 0; mounts.unmounts = 0;
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
});
afterEach(() => { act(() => root.unmount()); host.remove(); });

describe("shared panels controller and mounted content", () => {
  it("preserves component and DOM identity, both drafts and hidden/inert content across title and external toggles", () => {
    const ref = mount();
    const input = host.querySelector("input")!;
    const content = input.parentElement!;
    const document = host.querySelector("textarea")!;
    // Native input text/selection represents browser-owned state independent of parent rerenders.
    input.value = "draft in progress"; input.setSelectionRange(3, 7);
    document.value = "unsaved edit";
    clickTitle("Chat");
    expect(ref.current!.getState().collapsedPanel).toBe("right");
    expect(host.querySelector("input")).toBe(input);
    expect(content.hidden).toBe(true); expect(content.hasAttribute("inert")).toBe(true);
    expect(content.style.display).toBe("none");
    act(() => ref.current!.toggle("right"));
    expect(host.querySelector("input")).toBe(input); expect(input.parentElement).toBe(content);
    expect(input.value).toBe("draft in progress"); expect(input.selectionStart).toBe(3);
    expect(host.querySelector("textarea")).toBe(document); expect(document.value).toBe("unsaved edit");
    expect(content.hidden).toBe(false); expect(content.hasAttribute("inert")).toBe(false);
    expect(mounts).toEqual({ count: 1, unmounts: 0 });
  });
  it("retains old unmount-on-collapse behavior by default", () => {
    const ref = mount({ right: { ...right, keepMounted: undefined } });
    act(() => ref.current!.collapse("right"));
    expect(host.querySelector("input")).toBeNull(); expect(mounts.unmounts).toBe(1);
    act(() => ref.current!.expand("right")); expect(mounts.count).toBe(2);
  });
  it.each([ { collapsible: false }, { showTitle: false } ])("guards both external and initial collapse for %j", (restriction) => {
    const ref = mount({ right: { ...right, ...restriction }, initial: "right" });
    act(() => { ref.current!.collapse("right"); ref.current!.toggle("right"); });
    expect(ref.current!.getState().collapsedPanel).toBeNull();
    expect(host.querySelector("input")!.parentElement!.hidden).toBe(false);
  });
  it("prevents collapsing the last open panel and treats repeated operations as no-ops", () => {
    const ref = mount();
    act(() => { ref.current!.collapse("right"); ref.current!.collapse("left"); ref.current!.collapse("right"); ref.current!.expand("left"); });
    expect(ref.current!.getState()).toEqual({ collapsedPanel: "right", leftSize: 70, rightSize: 30 });
    expect([...host.querySelectorAll("button")].find((b) => b.textContent === "Document")!.disabled).toBe(true);
    act(() => { ref.current!.expand("right"); ref.current!.expand("right"); });
    expect(ref.current!.getState().collapsedPanel).toBeNull();
  });
  it("initially collapses only after measurement and restores the measured width, ignoring collapsed layout callbacks", () => {
    layout.sizes = { left: 68, right: 32 };
    const ref = mount({ initial: "right", right: { ...right, defaultSize: 320 } });
    expect(ref.current!.getState().rightSize).toBe(32);
    expect(ref.current!.getState().collapsedPanel).toBe("right");
    act(() => layout.emit({ left: 96, right: 4 }));
    expect(ref.current!.getState().rightSize).toBe(32);
    act(() => ref.current!.expand("right")); expect(layout.sizes.right).toBe(32);
    act(() => { layout.sizes = { left: 60, right: 40 }; layout.emit({ ...layout.sizes }); });
    act(() => { ref.current!.collapse("right"); ref.current!.expand("right"); });
    expect(layout.sizes.right).toBe(40);
  });
  it("supports initial left collapse through the same controller path", () => {
    const ref = mount({ initial: "left", left: { ...left, keepMounted: true } });
    expect(ref.current!.getState()).toEqual({ collapsedPanel: "left", leftSize: 70, rightSize: 30 });
    act(() => ref.current!.expand("left"));
    expect(ref.current!.getState().collapsedPanel).toBeNull();
    expect(layout.sizes.left).toBe(70);
  });
  it("does not consume the initial collapse in a zero-width group", () => {
    layout.sizes = { left: 0, right: 0 };
    const ref = mount({ initial: "right" });
    expect(ref.current!.getState().collapsedPanel).toBeNull();
    act(() => { layout.sizes = { left: 80, right: 20 }; layout.emit({ ...layout.sizes }); });
    expect(ref.current!.getState()).toEqual({ collapsedPanel: "right", leftSize: 80, rightSize: 20 });
  });
  it("recognizes resize-to-rail and guards the last open panel", () => {
    const ref = mount();
    act(() => { layout.collapsed.add("right"); layout.sizes.right = 4; layout.emit({ ...layout.sizes }); });
    act(() => ref.current!.collapse("left"));
    expect(ref.current!.getState()).toEqual({ collapsedPanel: "right", leftSize: 70, rightSize: 30 });
    act(() => ref.current!.expand("right")); expect(layout.sizes.right).toBe(30);
  });
});
