import { toString } from "mdast-util-to-string";
import remarkGfm from "remark-gfm";
import remarkParse from "remark-parse";
import { unified } from "unified";
import type { MarkdownBlock, MarkdownBlockType } from "../types";
import { detectMermaidBlock } from "../mermaid/detect-mermaid-block";
import { stripHtmlComments } from "./inline-markdown";

type Position = {
  start: { line: number; column: number; offset?: number };
  end: { line: number; column: number; offset?: number };
};

type AstNode = {
  type: string;
  children?: AstNode[];
  position?: Position;
  depth?: number;
  lang?: string | null;
  ordered?: boolean;
  start?: number | null;
  checked?: boolean | null;
};

type FrontmatterResult = { content: string; contentStartLine: number };

function extractFrontmatter(markdown: string): FrontmatterResult {
  const trimmed = markdown.trimStart();
  if (!trimmed.startsWith("---")) return { content: markdown, contentStartLine: 1 };
  const endIndex = trimmed.indexOf("\n---", 3);
  if (endIndex === -1) return { content: markdown, contentStartLine: 1 };
  const rawAfterFrontmatter = trimmed.slice(endIndex + 4);
  const afterFrontmatter = rawAfterFrontmatter.trimStart();
  const leadingChars = markdown.length - trimmed.length;
  const consumed = leadingChars + endIndex + 4 + (rawAfterFrontmatter.length - afterFrontmatter.length);
  return { content: afterFrontmatter, contentStartLine: (markdown.slice(0, consumed).match(/\n/g) ?? []).length + 1 };
}

function blockType(node: AstNode): MarkdownBlockType | undefined {
  switch (node.type) {
    case "heading": return "heading";
    case "paragraph": return "paragraph";
    case "blockquote": return "blockquote";
    case "listItem": return "list-item";
    case "code": return "code";
    case "table": return "table";
    case "thematicBreak": return "hr";
    default: return undefined;
  }
}

function sourceOf(node: AstNode, markdown: string) {
  if (!node.position) return "";
  return markdown.slice(node.position.start.offset ?? 0, node.position.end.offset ?? markdown.length);
}

function inlineMarkdownSource(node: AstNode, type: MarkdownBlockType, markdown: string) {
  const raw = sourceOf(node, markdown);
  if (type === "paragraph" || type === "table" || type === "code" || type === "hr") return raw;
  if (type === "heading") return raw.replace(/^ {0,3}#{1,6}[ \t]+/, "").replace(/[ \t]+#+[ \t]*$/, "");
  if (type === "blockquote") return raw.replace(/^ {0,3}>[ \t]?/gm, "");
  if (type === "list-item") {
    return (node.children ?? [])
      .filter((child) => child.type !== "list")
      .map((child) => sourceOf(child, markdown))
      .join("\n\n");
  }
  return raw;
}

/**
 * CommonMark/GFM AST에서 annotation·TOC가 소비할 의미 블록을 만든다.
 * 목록은 parser 문맥을 보존한 채 item을 추출하고 parentId로 중첩 관계를 남긴다.
 */
export function parseMarkdownToBlocks(markdown: string): MarkdownBlock[] {
  const { content, contentStartLine } = extractFrontmatter(stripHtmlComments(markdown));
  const root = unified().use(remarkParse).use(remarkGfm).parse(content) as unknown as AstNode;
  const blocks: MarkdownBlock[] = [];
  let nextId = 0;

  const push = (node: AstNode, parentId?: string, list?: AstNode, level = 0) => {
    const type = blockType(node);
    if (!type || !node.position) return undefined;
    const position = node.position;
    const id = `block-${nextId++}`;
    const rawContent = content.slice(position.start.offset ?? 0, position.end.offset ?? content.length);
    const text = type === "hr"
      ? ""
      : type === "code"
        ? toString(node as never)
        : inlineMarkdownSource(node, type, content);
    const language = type === "code" ? node.lang ?? undefined : undefined;
    blocks.push({
      id,
      type,
      content: text,
      rawContent,
      order: blocks.length,
      startLine: contentStartLine + position.start.line - 1,
      endLine: contentStartLine + position.end.line - 1,
      sourceRange: {
        startOffset: position.start.offset ?? 0,
        endOffset: position.end.offset ?? content.length,
        startColumn: position.start.column,
        endColumn: position.end.column,
      },
      parentId,
      level: type === "heading" ? node.depth : type === "list-item" ? level : undefined,
      language,
      mermaid: type === "code" ? detectMermaidBlock({ content: text, language }) : undefined,
      ordered: type === "list-item" ? list?.ordered : undefined,
      orderedStart: type === "list-item" && list?.ordered ? list.start ?? 1 : undefined,
      checked: type === "list-item" && node.checked !== null ? node.checked ?? undefined : undefined,
    });
    return id;
  };

  const visit = (node: AstNode, parentId?: string, level = 0) => {
    if (node.type === "list") {
      node.children?.forEach((item) => {
        const itemId = push(item, parentId, node, level);
        item.children?.forEach((child) => {
          if (child.type === "list") visit(child, itemId, level + 1);
        });
      });
      return;
    }
    push(node, parentId, undefined, level);
  };

  root.children?.forEach((node) => visit(node));
  return blocks;
}
