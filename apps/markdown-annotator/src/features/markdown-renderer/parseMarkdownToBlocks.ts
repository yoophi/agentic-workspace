import type { MarkdownBlock } from "@/entities/markdown-block";

type FrontmatterResult = {
  content: string;
  contentStartLine: number;
};

function extractFrontmatter(markdown: string): FrontmatterResult {
  const trimmed = markdown.trimStart();
  if (!trimmed.startsWith("---")) {
    return { content: markdown, contentStartLine: 1 };
  }

  const endIndex = trimmed.indexOf("\n---", 3);
  if (endIndex === -1) {
    return { content: markdown, contentStartLine: 1 };
  }

  const rawAfterFrontmatter = trimmed.slice(endIndex + 4);
  const afterFrontmatter = rawAfterFrontmatter.trimStart();
  const leadingChars = markdown.length - trimmed.length;
  const consumedInTrimmed = endIndex + 4 + (rawAfterFrontmatter.length - afterFrontmatter.length);
  const consumedTotal = leadingChars + consumedInTrimmed;
  const contentStartLine = (markdown.slice(0, consumedTotal).match(/\n/g) ?? []).length + 1;

  return { content: afterFrontmatter, contentStartLine };
}

function isTableStart(lines: string[], index: number) {
  const current = lines[index]?.trim() ?? "";
  const next = lines[index + 1]?.trim() ?? "";
  return current.includes("|") && /^\|?\s*:?-{3,}:?\s*(\|\s*:?-{3,}:?\s*)+\|?$/.test(next);
}

export function parseMarkdownToBlocks(markdown: string): MarkdownBlock[] {
  const { content, contentStartLine } = extractFrontmatter(markdown);
  const lines = content.split("\n");
  const blocks: MarkdownBlock[] = [];
  let nextId = 0;
  let paragraphBuffer: string[] = [];
  let paragraphStartLine = contentStartLine;

  const pushBlock = (block: Omit<MarkdownBlock, "id" | "order">) => {
    const order = blocks.length;
    blocks.push({
      ...block,
      id: `block-${nextId++}`,
      order,
    });
  };

  const flushParagraph = (endLine?: number) => {
    if (paragraphBuffer.length === 0) {
      return;
    }

    pushBlock({
      type: "paragraph",
      content: paragraphBuffer.join("\n"),
      rawContent: paragraphBuffer.join("\n"),
      startLine: paragraphStartLine,
      endLine: endLine ?? paragraphStartLine + paragraphBuffer.length - 1,
    });
    paragraphBuffer = [];
  };

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    const trimmed = line.trim();
    const lineNumber = contentStartLine + index;

    if (!trimmed) {
      flushParagraph(lineNumber - 1);
      continue;
    }

    const headingMatch = trimmed.match(/^(#{1,6})\s+(.+)$/);
    if (headingMatch) {
      flushParagraph(lineNumber - 1);
      pushBlock({
        type: "heading",
        level: headingMatch[1].length,
        content: headingMatch[2],
        rawContent: line,
        startLine: lineNumber,
        endLine: lineNumber,
      });
      continue;
    }

    if (/^(```|~~~)/.test(trimmed)) {
      flushParagraph(lineNumber - 1);
      const fence = trimmed.slice(0, 3);
      const language = trimmed.slice(3).trim() || undefined;
      const codeLines: string[] = [];
      const rawCodeLines: string[] = [line];
      let endLine = lineNumber;

      for (index += 1; index < lines.length; index += 1) {
        const codeLine = lines[index];
        endLine = contentStartLine + index;
        rawCodeLines.push(codeLine);
        if (codeLine.trim().startsWith(fence)) {
          break;
        }
        codeLines.push(codeLine);
      }

      pushBlock({
        type: "code",
        content: codeLines.join("\n"),
        rawContent: rawCodeLines.join("\n"),
        language,
        startLine: lineNumber,
        endLine,
      });
      continue;
    }

    if (isTableStart(lines, index)) {
      flushParagraph(lineNumber - 1);
      const tableLines: string[] = [];
      const startLine = lineNumber;
      let endLine = lineNumber;

      while (index < lines.length && lines[index].includes("|") && lines[index].trim()) {
        tableLines.push(lines[index]);
        endLine = contentStartLine + index;
        index += 1;
      }
      index -= 1;

      pushBlock({
        type: "table",
        content: tableLines.join("\n"),
        rawContent: tableLines.join("\n"),
        startLine,
        endLine,
      });
      continue;
    }

    if (/^(---|\*\*\*)$/.test(trimmed)) {
      flushParagraph(lineNumber - 1);
      pushBlock({
        type: "hr",
        content: "",
        rawContent: line,
        startLine: lineNumber,
        endLine: lineNumber,
      });
      continue;
    }

    const listMatch = trimmed.match(/^(\*|-|(\d+)\.)\s+(.+)$/);
    if (listMatch) {
      flushParagraph(lineNumber - 1);
      const leadingWhitespace = line.match(/^(\s*)/)?.[1] ?? "";
      const level = Math.floor(leadingWhitespace.replace(/\t/g, "  ").length / 2);
      const ordered = listMatch[2] !== undefined;
      let itemContent = listMatch[3];
      let checked: boolean | undefined;
      const checkboxMatch = itemContent.match(/^\[([ xX])]\s+/);

      if (checkboxMatch) {
        checked = checkboxMatch[1].toLowerCase() === "x";
        itemContent = itemContent.replace(/^\[([ xX])]\s+/, "");
      }

      pushBlock({
        type: "list-item",
        content: itemContent,
        rawContent: line,
        level,
        ordered,
        orderedStart: ordered ? Number.parseInt(listMatch[2], 10) : undefined,
        checked,
        startLine: lineNumber,
        endLine: lineNumber,
      });
      continue;
    }

    if (trimmed.startsWith(">")) {
      flushParagraph(lineNumber - 1);
      const quoteLines: string[] = [];
      const rawQuoteLines: string[] = [];
      const startLine = lineNumber;
      let endLine = lineNumber;

      while (index < lines.length && lines[index].trim().startsWith(">")) {
        quoteLines.push(lines[index].trim().replace(/^>\s?/, ""));
        rawQuoteLines.push(lines[index]);
        endLine = contentStartLine + index;
        index += 1;
      }
      index -= 1;

      pushBlock({
        type: "blockquote",
        content: quoteLines.join("\n"),
        rawContent: rawQuoteLines.join("\n"),
        startLine,
        endLine,
      });
      continue;
    }

    if (paragraphBuffer.length === 0) {
      paragraphStartLine = lineNumber;
    }
    paragraphBuffer.push(line);
  }

  flushParagraph(contentStartLine + lines.length - 1);
  return blocks;
}
