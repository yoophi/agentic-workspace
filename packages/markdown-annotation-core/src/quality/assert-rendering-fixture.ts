import { parseMarkdownToBlocks } from "../parse/parse-markdown-to-blocks";
import type { MarkdownRenderingFixture } from "./types";

export function assertRenderingFixture(fixture: MarkdownRenderingFixture) {
  const blocks = parseMarkdownToBlocks(fixture.markdown);
  const actualTypes = blocks.map((block) => block.type);
  for (const type of fixture.expectedBlockTypes) {
    if (!actualTypes.includes(type)) throw new Error(`${fixture.id}: expected block type ${type}, got ${actualTypes.join(", ")}`);
  }
  const text = blocks.map((block) => block.content).join("\n");
  for (const expected of fixture.expectedText) {
    if (!text.includes(expected)) throw new Error(`${fixture.id}: expected text ${JSON.stringify(expected)}`);
  }
}
