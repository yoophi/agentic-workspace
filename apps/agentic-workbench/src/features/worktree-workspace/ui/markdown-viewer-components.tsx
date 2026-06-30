import type {
  MarkdownViewerComponents,
  ViewerTooltipProps,
} from "@yoophi/markdown-annotation-react";
import { Button } from "@/components/ui/button";
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

export const markdownViewerComponents: MarkdownViewerComponents = {
  Button,
  Tooltip: ViewerTooltip,
};
