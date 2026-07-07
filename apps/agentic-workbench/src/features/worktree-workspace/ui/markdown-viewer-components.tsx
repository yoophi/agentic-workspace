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
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

/**
 * radix(shadcn) Tooltip을 공유 MarkdownViewer의 통합 Tooltip 계약에 맞춘 어댑터.
 * trigger는 `asChild`로 주입하며, radix는 Provider를 요구하므로 각 Tooltip을
 * Provider로 감싼다.
 */
function ViewerTooltip({ content, align, children }: ViewerTooltipProps) {
  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>{children}</TooltipTrigger>
        <TooltipContent align={align} className="max-w-sm">
          {content}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
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
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <DialogTrigger asChild>{children}</DialogTrigger>
        </TooltipTrigger>
        <TooltipContent>{tooltip}</TooltipContent>
      </Tooltip>
    </TooltipProvider>
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
