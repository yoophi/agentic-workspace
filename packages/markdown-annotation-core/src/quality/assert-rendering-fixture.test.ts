import { describe, expect, it } from "vitest";
import { assertRenderingFixture } from "./assert-rendering-fixture";
import { markdownRenderingFixtures } from "./fixtures";

describe("rendering quality corpus", () => {
  it.each(markdownRenderingFixtures)("validates $id", (fixture) => {
    expect(() => assertRenderingFixture(fixture)).not.toThrow();
  });
});
