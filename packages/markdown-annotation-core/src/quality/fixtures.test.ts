import { describe, expect, it } from "vitest";
import { markdownRenderingFixtures } from "./fixtures";

describe("markdown rendering quality fixtures", () => {
  it("has unique ids and the required coverage counts", () => {
    expect(new Set(markdownRenderingFixtures.map((fixture) => fixture.id)).size).toBe(markdownRenderingFixtures.length);
    expect(markdownRenderingFixtures.filter((fixture) => fixture.category === "commonmark-list")).toHaveLength(20);
    expect(markdownRenderingFixtures.filter((fixture) => fixture.category === "annotation")).toHaveLength(10);
    expect(markdownRenderingFixtures.filter((fixture) => fixture.category === "recovery" || fixture.category === "safety")).toHaveLength(10);
  });
});
