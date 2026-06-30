import type {
  MarkdownViewerComponents,
  ViewerTooltipProps,
} from "@yoophi/markdown-annotation-react";
import { Button } from "@/components/ui/button";
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

export const markdownViewerComponents: MarkdownViewerComponents = {
  Button,
  Tooltip: ViewerTooltip,
};
