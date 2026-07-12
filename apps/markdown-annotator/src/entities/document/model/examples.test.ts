import { describe, expect, it } from "vitest";
import { exampleMarkdownDocuments } from "./examples";

describe("exampleMarkdownDocuments", () => {
  it("contains unique, complete metadata", () => {
    expect(new Set(exampleMarkdownDocuments.map(({ id }) => id)).size).toBe(
      exampleMarkdownDocuments.length,
    );
    expect(new Set(exampleMarkdownDocuments.map(({ fileName }) => fileName)).size).toBe(
      exampleMarkdownDocuments.length,
    );
    expect(
      exampleMarkdownDocuments.every(
        ({ description, fileName, markdownText, title }) =>
          fileName.endsWith(".md") && description.trim() && markdownText.trim() && title.trim(),
      ),
    ).toBe(true);
  });

  it("provides all required SpecKit artifact examples", () => {
    expect(exampleMarkdownDocuments.map(({ id }) => id)).toEqual(
      expect.arrayContaining([
        "speckit-spec",
        "speckit-plan",
        "speckit-data-model",
        "speckit-tasks",
        "speckit-checklist",
      ]),
    );
  });
});
