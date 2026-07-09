import { detectMermaidBlock } from "@yoophi/markdown-annotation-core";

export type AgentRunCodeBlockRenderKind = "ordinary-code" | "mermaid-diagram";

function stripHtmlCommentsOutsideFences(content: string) {
  const lines = content.split(/(\n)/);
  let inFence = false;

  return lines
    .map((line) => {
      if (line === "\n") return line;

      const trimmedStart = line.trimStart();
      if (trimmedStart.startsWith("```")) {
        inFence = !inFence;
        return line;
      }

      return inFence ? line : line.replace(/<!--[\s\S]*?-->/g, "");
    })
    .join("");
}

export function normalizeStreamingMarkdown(content: string) {
  const withoutComments = stripHtmlCommentsOutsideFences(content);
  const fenceMatches = withoutComments.match(/```/g);
  if (fenceMatches && fenceMatches.length % 2 === 1) {
    const suffix = withoutComments.endsWith("\n") ? "```" : "\n```";
    return `${withoutComments}${suffix}`;
  }
  return withoutComments;
}

export function extractCodeLanguage(className?: string) {
  if (!className) return undefined;
  const match = className.match(/(?:^|\s)language-([^\s]+)/);
  return match?.[1];
}

export function getAgentRunCodeBlockRenderKind({
  language,
  source,
}: {
  language?: string;
  source: string;
}): AgentRunCodeBlockRenderKind {
  return detectMermaidBlock({ content: source, language }) ? "mermaid-diagram" : "ordinary-code";
}

export function createAgentRunMermaidBlockId({
  source,
  startLine,
}: {
  source: string;
  startLine?: number;
}) {
  let hash = 0;
  for (let index = 0; index < source.length; index += 1) {
    hash = Math.imul(31, hash) + source.charCodeAt(index);
  }

  return `agent-run-mermaid-${startLine ?? "unknown"}-${Math.abs(hash).toString(36)}`;
}
