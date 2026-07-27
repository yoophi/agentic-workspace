import type { MarkdownBlockType } from "../types/markdown-block";

export type MarkdownRenderingFixtureCategory =
  | "commonmark-list"
  | "block"
  | "inline"
  | "gfm"
  | "recovery"
  | "safety"
  | "annotation";

export type MarkdownRenderingFixture = {
  id: string;
  category: MarkdownRenderingFixtureCategory;
  markdown: string;
  expectedBlockTypes: MarkdownBlockType[];
  expectedText: string[];
};
