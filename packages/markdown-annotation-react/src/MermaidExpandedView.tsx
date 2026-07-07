import { useState, type CSSProperties, type ComponentType } from "react";
import { Maximize2, Scan, ZoomIn, ZoomOut } from "lucide-react";

import { MermaidDiagram } from "./MermaidDiagram";
import type {
  MarkdownViewerComponents,
  MermaidExpandedDialogTriggerProps,
} from "./types";

export const MERMAID_EXPANDED_FIT_ZOOM = 1;
export const MERMAID_EXPANDED_MIN_ZOOM = 0.5;
export const MERMAID_EXPANDED_MAX_ZOOM = 2.5;
export const MERMAID_EXPANDED_ZOOM_STEP = 0.25;

export function clampMermaidExpandedZoom(zoom: number) {
  return Math.min(MERMAID_EXPANDED_MAX_ZOOM, Math.max(MERMAID_EXPANDED_MIN_ZOOM, zoom));
}

export type MermaidExpandedBodyProps = {
  blockId: string;
  source: string;
  zoomPercent: number;
};

export function MermaidExpandedBody({ blockId, source, zoomPercent }: MermaidExpandedBodyProps) {
  const zoomStyle = {
    height: `${zoomPercent}%`,
    width: `${zoomPercent}%`,
  } satisfies CSSProperties;

  return (
    <div className="markdown-viewer__mermaid-expanded-body">
      <div className="markdown-viewer__mermaid-expanded-zoom-frame" style={zoomStyle}>
        <MermaidDiagram fit blockId={`${blockId}-expanded`} source={source} />
      </div>
    </div>
  );
}

export type MermaidExpandedTriggerProps = {
  components: Pick<MarkdownViewerComponents, "Button">;
  DialogTrigger: ComponentType<MermaidExpandedDialogTriggerProps>;
  triggerDataAttribute?: string;
};

export function MermaidExpandedTrigger({
  components,
  DialogTrigger,
  triggerDataAttribute,
}: MermaidExpandedTriggerProps) {
  const { Button } = components;
  const dataAttributes = triggerDataAttribute ? { [triggerDataAttribute]: "true" } : {};

  return (
    <DialogTrigger tooltip="Open full screen">
      <Button
        aria-label="Open Mermaid diagram in full screen"
        data-mermaid-expanded-trigger="true"
        size="icon-sm"
        type="button"
        variant="outline"
        {...dataAttributes}
      >
        <Maximize2 aria-hidden="true" />
      </Button>
    </DialogTrigger>
  );
}

export type MermaidExpandedViewProps = {
  blockId: string;
  components: MarkdownViewerComponents;
  defaultExpanded?: boolean;
  description?: string;
  source: string;
  title?: string;
  triggerDataAttribute?: string;
};

export function MermaidExpandedView({
  blockId,
  components,
  defaultExpanded = false,
  description = "Expanded Mermaid diagram view",
  source,
  title = "Mermaid diagram",
  triggerDataAttribute,
}: MermaidExpandedViewProps) {
  const dialog = components.MermaidExpandedDialog;
  const [expandedOpen, setExpandedOpen] = useState(defaultExpanded);
  const [expandedZoom, setExpandedZoom] = useState(MERMAID_EXPANDED_FIT_ZOOM);
  const expandedZoomPercent = Math.round(expandedZoom * 100);

  if (!dialog) {
    return <MermaidDiagram blockId={blockId} source={source} />;
  }

  const { Content, Root, Trigger } = dialog;
  const { Button, Tooltip } = components;

  function handleExpandedOpenChange(open: boolean) {
    setExpandedOpen(open);

    if (open) {
      setExpandedZoom(MERMAID_EXPANDED_FIT_ZOOM);
    }
  }

  return (
    <Root open={expandedOpen} onOpenChange={handleExpandedOpenChange}>
      <div className="markdown-viewer__mermaid-expanded-view" data-mermaid-expanded-view>
        <MermaidDiagram
          blockId={blockId}
          source={source}
          renderActions={
            <div className="markdown-viewer__mermaid-actions">
              <MermaidExpandedTrigger
                components={components}
                DialogTrigger={Trigger}
                triggerDataAttribute={triggerDataAttribute}
              />
            </div>
          }
        />
      </div>
      {expandedOpen ? (
        <Content title={title} description={description}>
          <div className="markdown-viewer__mermaid-expanded-toolbar">
            <Tooltip content="Zoom out">
              <Button
                aria-label="Zoom out Mermaid diagram"
                onClick={() =>
                  setExpandedZoom((zoom) =>
                    clampMermaidExpandedZoom(zoom - MERMAID_EXPANDED_ZOOM_STEP),
                  )
                }
                size="icon-sm"
                type="button"
                variant="ghost"
              >
                <ZoomOut aria-hidden="true" />
              </Button>
            </Tooltip>
            <span className="markdown-viewer__mermaid-expanded-zoom-label">
              {expandedZoomPercent}%
            </span>
            <Tooltip content="Zoom in">
              <Button
                aria-label="Zoom in Mermaid diagram"
                onClick={() =>
                  setExpandedZoom((zoom) =>
                    clampMermaidExpandedZoom(zoom + MERMAID_EXPANDED_ZOOM_STEP),
                  )
                }
                size="icon-sm"
                type="button"
                variant="ghost"
              >
                <ZoomIn aria-hidden="true" />
              </Button>
            </Tooltip>
            <Tooltip content="Fit to view">
              <Button
                aria-label="Fit Mermaid diagram to view"
                onClick={() => setExpandedZoom(MERMAID_EXPANDED_FIT_ZOOM)}
                size="icon-sm"
                type="button"
                variant="ghost"
              >
                <Scan aria-hidden="true" />
              </Button>
            </Tooltip>
          </div>
          <MermaidExpandedBody
            blockId={blockId}
            source={source}
            zoomPercent={expandedZoomPercent}
          />
        </Content>
      ) : null}
    </Root>
  );
}
