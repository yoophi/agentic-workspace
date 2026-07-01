export const MERMAID_START_TOKENS = [
  "graph",
  "flowchart",
  "sequenceDiagram",
  "classDiagram",
  "stateDiagram",
  "stateDiagram-v2",
  "erDiagram",
  "journey",
  "gantt",
  "pie",
  "quadrantChart",
  "requirementDiagram",
  "gitGraph",
  "mindmap",
  "timeline",
  "zenuml",
  "sankey-beta",
  "xychart-beta",
  "block-beta",
  "packet-beta",
  "kanban",
  "architecture-beta",
  "radar-beta",
  "eventModel",
  "treemap-beta",
  "venn",
  "ishikawa",
  "wardley",
  "tree",
  "info",
] as const;

export type MermaidStartToken = (typeof MERMAID_START_TOKENS)[number];

export type MermaidDetectionResult = {
  detected: true;
  reason: "language-marker" | "leading-declaration";
  declaration: MermaidStartToken | "mermaid";
  source: string;
};

function normalizeLanguage(language?: string) {
  return language?.trim().toLowerCase();
}

function firstNonEmptyLine(source: string) {
  return source
    .split(/\r?\n/)
    .map((line) => line.trim())
    .find(Boolean);
}

function startsWithToken(line: string, token: MermaidStartToken) {
  if (line === token) {
    return true;
  }

  const nextCharacter = line.charAt(token.length);
  return line.startsWith(token) && (nextCharacter === "" || /\s/.test(nextCharacter));
}

export function detectMermaidBlock({
  content,
  language,
}: {
  content: string;
  language?: string;
}): MermaidDetectionResult | undefined {
  if (normalizeLanguage(language) === "mermaid") {
    return {
      detected: true,
      reason: "language-marker",
      declaration: "mermaid",
      source: content,
    };
  }

  const firstLine = firstNonEmptyLine(content);
  if (!firstLine) {
    return undefined;
  }

  const declaration = MERMAID_START_TOKENS.find((token) => startsWithToken(firstLine, token));
  if (!declaration) {
    return undefined;
  }

  return {
    detected: true,
    reason: "leading-declaration",
    declaration,
    source: content,
  };
}
