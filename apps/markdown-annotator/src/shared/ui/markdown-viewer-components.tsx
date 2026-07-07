import type {
  MarkdownViewerComponents,
  MermaidExpandedDialogContentProps,
  MermaidExpandedDialogRootProps,
  MermaidExpandedDialogTriggerProps,
  ViewerTooltipProps,
} from "@yoophi/markdown-annotation-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

/**
 * base-ui(shadcn) Tooltip을 공유 MarkdownViewer의 통합 Tooltip 계약에 맞춘 어댑터.
 * trigger는 `render` prop으로 주입한다. TooltipProvider는 앱 루트에 이미 존재한다.
 */
function ViewerTooltip({ content, align, children }: ViewerTooltipProps) {
  return (
    <Tooltip>
      <TooltipTrigger render={children} />
      <TooltipContent align={align} className="max-w-sm">
        {content}
      </TooltipContent>
    </Tooltip>
  );
}

function MermaidExpandedDialogRoot({
  children,
  onOpenChange,
  open,
}: MermaidExpandedDialogRootProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      {children}
    </Dialog>
  );
}

function MermaidExpandedDialogTrigger({ children, tooltip }: MermaidExpandedDialogTriggerProps) {
  return (
    <Tooltip>
      <TooltipTrigger render={<DialogTrigger render={children} />} />
      <TooltipContent>{tooltip}</TooltipContent>
    </Tooltip>
  );
}

function MermaidExpandedDialogContent({
  children,
  description,
  title,
}: MermaidExpandedDialogContentProps) {
  return (
    <DialogContent className="grid h-[calc(100dvh-2rem)] max-h-[calc(100dvh-2rem)] w-[calc(100vw-2rem)] max-w-[calc(100vw-2rem)] grid-rows-[auto_minmax(0,1fr)] gap-3 overflow-hidden p-4 sm:max-w-[calc(100vw-2rem)]">
      <DialogHeader className="min-w-0 pr-40">
        <DialogTitle>{title}</DialogTitle>
        <DialogDescription className="sr-only">{description}</DialogDescription>
      </DialogHeader>
      {children}
    </DialogContent>
  );
}

export const markdownViewerComponents: MarkdownViewerComponents = {
  Button,
  MermaidExpandedDialog: {
    Content: MermaidExpandedDialogContent,
    Root: MermaidExpandedDialogRoot,
    Trigger: MermaidExpandedDialogTrigger,
  },
  Tooltip: ViewerTooltip,
};
