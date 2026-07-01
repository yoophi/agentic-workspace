import { useEffect, useId, useMemo, useState } from "react";

export type MermaidFailureCategory =
  | "empty-source"
  | "syntax-or-parse-error"
  | "renderer-runtime-error";

export type MermaidFailure = {
  category: MermaidFailureCategory;
  reason: string;
};

type MermaidRenderState =
  | { status: "loading" }
  | { status: "rendered"; svg: string }
  | { status: "failed"; failure: MermaidFailure };

export type MermaidDiagramProps = {
  blockId: string;
  source: string;
};

export function createMermaidSourceHash(source: string) {
  let hash = 0;
  for (let index = 0; index < source.length; index += 1) {
    hash = Math.imul(31, hash) + source.charCodeAt(index);
  }

  return Math.abs(hash).toString(36);
}

export function createMermaidRenderId(id: string, blockId: string, source: string) {
  const normalizedId = id.replace(/[^a-zA-Z0-9_-]/g, "");
  const normalizedBlockId = blockId.replace(/[^a-zA-Z0-9_-]/g, "-");
  return `mermaid-${normalizedId}-${normalizedBlockId}-${createMermaidSourceHash(source)}`;
}

export function toMermaidFailure(error: unknown): MermaidFailure {
  if (error instanceof Error) {
    const message = error.message || "Mermaid diagram rendering failed.";
    const category = /parse|syntax|lexical/i.test(message)
      ? "syntax-or-parse-error"
      : "renderer-runtime-error";

    return {
      category,
      reason: message,
    };
  }

  if (typeof error === "string" && error.trim()) {
    return {
      category: /parse|syntax|lexical/i.test(error)
        ? "syntax-or-parse-error"
        : "renderer-runtime-error",
      reason: error,
    };
  }

  return {
    category: "renderer-runtime-error",
    reason: "Mermaid diagram rendering failed.",
  };
}

export function emptyMermaidFailure(): MermaidFailure {
  return {
    category: "empty-source",
    reason: "Mermaid diagram source is empty.",
  };
}

function MermaidFallback({ failure, source }: { failure: MermaidFailure; source: string }) {
  return (
    <div className="markdown-viewer__mermaid-fallback" data-mermaid-status="failed">
      <div className="markdown-viewer__mermaid-fallback-header">
        <p className="markdown-viewer__mermaid-fallback-title">Mermaid diagram failed to render</p>
        <p className="markdown-viewer__mermaid-fallback-reason">{failure.reason}</p>
        <p className="markdown-viewer__mermaid-fallback-category">{failure.category}</p>
      </div>
      <pre className="markdown-viewer__mermaid-source">
        <code data-block-content>{source}</code>
      </pre>
    </div>
  );
}

export function MermaidDiagram({ blockId, source }: MermaidDiagramProps) {
  const reactId = useId();
  const renderId = useMemo(() => createMermaidRenderId(reactId, blockId, source), [blockId, reactId, source]);
  const [state, setState] = useState<MermaidRenderState>(() =>
    source.trim() ? { status: "loading" } : { status: "failed", failure: emptyMermaidFailure() },
  );

  useEffect(() => {
    let cancelled = false;
    const trimmedSource = source.trim();

    if (!trimmedSource) {
      setState({ status: "failed", failure: emptyMermaidFailure() });
      return;
    }

    setState({ status: "loading" });

    async function renderDiagram() {
      try {
        const { default: mermaid } = await import("mermaid");
        mermaid.initialize({
          securityLevel: "strict",
          startOnLoad: false,
          theme: "default",
        });
        const result = await mermaid.render(renderId, source);

        if (!cancelled) {
          setState({ status: "rendered", svg: result.svg });
        }
      } catch (error) {
        if (!cancelled) {
          setState({ status: "failed", failure: toMermaidFailure(error) });
        }
      }
    }

    void renderDiagram();

    return () => {
      cancelled = true;
    };
  }, [renderId, source]);

  if (state.status === "failed") {
    return <MermaidFallback failure={state.failure} source={source} />;
  }

  if (state.status === "loading") {
    return (
      <div className="markdown-viewer__mermaid-loading" data-mermaid-status="loading">
        Rendering Mermaid diagram...
      </div>
    );
  }

  return (
    <div
      className="markdown-viewer__mermaid"
      data-block-content
      data-mermaid-status="rendered"
      dangerouslySetInnerHTML={{ __html: state.svg }}
    />
  );
}
