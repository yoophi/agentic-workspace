import { describe, expect, it } from "vitest";

import { scrollToBlock } from "./scroll-to-block";

type ScrollIntoViewCall = ScrollIntoViewOptions | undefined;

function createFakeContainer(existingBlockIds: string[]) {
  const calls: ScrollIntoViewCall[] = [];
  const queries: string[] = [];
  const container = {
    querySelector: (selector: string) => {
      queries.push(selector);
      const matched = existingBlockIds.some(
        (blockId) => selector === `[data-block-id="${blockId}"]`,
      );
      if (!matched) {
        return null;
      }
      return {
        scrollIntoView: (options?: ScrollIntoViewOptions) => {
          calls.push(options);
        },
      };
    },
  } as unknown as ParentNode;

  return { calls, container, queries };
}

describe("scrollToBlock", () => {
  it("scrolls the matching data-block-id element to start and returns true", () => {
    const { calls, container, queries } = createFakeContainer(["block-3"]);

    const result = scrollToBlock(container, "block-3");

    expect(result).toBe(true);
    expect(queries).toEqual(['[data-block-id="block-3"]']);
    expect(calls).toHaveLength(1);
    expect(calls[0]?.block).toBe("start");
  });

  it("defaults to smooth behavior when reduced motion cannot be detected", () => {
    const { calls, container } = createFakeContainer(["block-3"]);

    scrollToBlock(container, "block-3");

    expect(calls[0]?.behavior).toBe("smooth");
  });

  it("passes an explicit behavior option through", () => {
    const { calls, container } = createFakeContainer(["block-3"]);

    scrollToBlock(container, "block-3", { behavior: "auto" });

    expect(calls[0]?.behavior).toBe("auto");
  });

  it("returns false without throwing when the container is null", () => {
    expect(scrollToBlock(null, "block-3")).toBe(false);
  });

  it("returns false without throwing when the block does not exist", () => {
    const { calls, container } = createFakeContainer([]);

    expect(scrollToBlock(container, "missing")).toBe(false);
    expect(calls).toHaveLength(0);
  });
});
