import { useState, type ReactNode } from "react";
import ReactMarkdown from "react-markdown";
import type { Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { MermaidDiagram } from "@yoophi/markdown-annotation-react";
import { Maximize2Icon } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  createAgentRunMermaidBlockId,
  extractCodeLanguage,
  getAgentRunCodeBlockRenderKind,
  normalizeStreamingMarkdown,
} from "@/features/agent-run/model/agent-run-markdown";
import { cn } from "@/lib/utils";
import { ExternalLink } from "@/shared/ui/external-link";

function codeSource(children: ReactNode) {
  return Array.isArray(children) ? children.join("") : String(children ?? "");
}

type AgentRunMermaidDiagramProps = {
  blockId: string;
  defaultExpanded?: boolean;
  source: string;
};

export function AgentRunMermaidDiagram({
  blockId,
  defaultExpanded = false,
  source,
}: AgentRunMermaidDiagramProps) {
  const [expandedOpen, setExpandedOpen] = useState(defaultExpanded);

  return (
    <Dialog open={expandedOpen} onOpenChange={setExpandedOpen}>
      <div className="relative min-w-0 max-w-full overflow-hidden">
        <MermaidDiagram
          blockId={blockId}
          source={source}
          renderActions={
            <div className="mb-2 flex justify-end">
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <DialogTrigger asChild>
                      <Button
                        type="button"
                        variant="outline"
                        size="icon-sm"
                        aria-label="Open Mermaid diagram in full screen"
                        data-agent-run-mermaid-expanded-trigger="true"
                      >
                        <Maximize2Icon />
                      </Button>
                    </DialogTrigger>
                  </TooltipTrigger>
                  <TooltipContent>Open full screen</TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
          }
        />
      </div>
      <DialogContent className="grid h-[calc(100dvh-2rem)] max-h-[calc(100dvh-2rem)] w-[calc(100vw-2rem)] max-w-[calc(100vw-2rem)] grid-rows-[auto_minmax(0,1fr)] gap-3 overflow-hidden p-4 sm:max-w-[calc(100vw-2rem)]">
        <DialogHeader className="min-w-0 pr-8">
          <DialogTitle>Mermaid diagram</DialogTitle>
          <DialogDescription className="sr-only">Expanded agent-run diagram view</DialogDescription>
        </DialogHeader>
        <div className="min-h-0 min-w-0 overflow-hidden rounded-md border bg-background p-1 [&_.markdown-viewer__mermaid]:my-0 [&_.markdown-viewer__mermaid]:flex [&_.markdown-viewer__mermaid]:h-full [&_.markdown-viewer__mermaid]:max-h-none [&_.markdown-viewer__mermaid]:w-full [&_.markdown-viewer__mermaid]:items-center [&_.markdown-viewer__mermaid]:justify-center [&_.markdown-viewer__mermaid]:overflow-hidden [&_.markdown-viewer__mermaid]:border-0 [&_.markdown-viewer__mermaid]:p-0 [&_.markdown-viewer__mermaid>div]:flex [&_.markdown-viewer__mermaid>div]:h-full [&_.markdown-viewer__mermaid>div]:w-full [&_.markdown-viewer__mermaid>div]:items-center [&_.markdown-viewer__mermaid>div]:justify-center [&_.markdown-viewer__mermaid_svg]:h-full [&_.markdown-viewer__mermaid_svg]:max-h-full [&_.markdown-viewer__mermaid_svg]:max-w-full [&_.markdown-viewer__mermaid_svg]:w-full">
          <MermaidDiagram blockId={`${blockId}-expanded`} source={source} />
        </div>
      </DialogContent>
    </Dialog>
  );
}

export function StreamingMarkdown({ content }: { content: string }) {
  return (
    <div className="min-w-0 break-words text-sm leading-6">
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
        {normalizeStreamingMarkdown(content)}
      </ReactMarkdown>
    </div>
  );
}

const components: Components = {
  h1: ({ children }) => (
    <h1 className="mb-2 mt-4 text-2xl font-semibold tracking-tight first:mt-0">{children}</h1>
  ),
  h2: ({ children }) => (
    <h2 className="mb-2 mt-4 text-xl font-semibold tracking-tight first:mt-0">{children}</h2>
  ),
  h3: ({ children }) => (
    <h3 className="mb-2 mt-3 text-lg font-semibold tracking-tight first:mt-0">{children}</h3>
  ),
  h4: ({ children }) => (
    <h4 className="mb-2 mt-3 text-base font-semibold tracking-tight first:mt-0">{children}</h4>
  ),
  p: ({ children }) => <p className="my-2 first:mt-0 last:mb-0">{children}</p>,
  ul: ({ children }) => <ul className="my-2 list-disc pl-5">{children}</ul>,
  ol: ({ children }) => <ol className="my-2 list-decimal pl-5">{children}</ol>,
  li: ({ children }) => <li className="mt-1">{children}</li>,
  blockquote: ({ children }) => (
    <blockquote className="my-3 border-l-4 pl-4 text-muted-foreground">{children}</blockquote>
  ),
  code: ({ children, className, node }) => {
    const source = codeSource(children).replace(/\n$/, "");
    const language = extractCodeLanguage(className);
    const isInline =
      !node?.position?.start.line || node.position.start.line === node.position.end.line;

    if (isInline) {
      return (
        <code
          className={cn("rounded bg-muted px-1.5 py-0.5 font-mono text-[0.92em]", className)}
        >
          {children}
        </code>
      );
    }

    if (getAgentRunCodeBlockRenderKind({ language, source }) === "mermaid-diagram") {
      return (
        <div className="my-3 min-w-0 max-w-full overflow-hidden">
          <AgentRunMermaidDiagram
            blockId={createAgentRunMermaidBlockId({
              source,
              startLine: node.position?.start.line,
            })}
            source={source}
          />
        </div>
      );
    }

    return (
      <pre className="my-3 overflow-x-auto rounded-md border bg-muted p-3">
        <code className={cn("font-mono text-sm", className)}>{children}</code>
      </pre>
    );
  },
  pre: ({ children }) => <>{children}</>,
  a: ({ children, href }) => <ExternalLink href={href}>{children}</ExternalLink>,
  hr: () => <hr className="my-4 border-border" />,
  table: ({ children }) => (
    <div className="my-3 overflow-x-auto">
      <table className="w-full border-collapse text-sm">{children}</table>
    </div>
  ),
  th: ({ children }) => (
    <th className="border bg-muted px-2 py-1 text-left font-semibold align-top">{children}</th>
  ),
  td: ({ children }) => <td className="border px-2 py-1 align-top">{children}</td>,
  img: ({ alt, src }) => (
    <img className="h-auto max-w-full rounded-md" alt={alt ?? ""} src={src} />
  ),
};
