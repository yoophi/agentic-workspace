import { describe, expect, it } from "vitest";
import { getSelectionAnchors } from "./use-selection-anchors";

describe("getSelectionAnchors", () => {
  it("returns no anchor when no browser selection is available", () => {
    expect(getSelectionAnchors(null)).toEqual([]);
  });
});
