import { ChevronRightIcon } from "lucide-react";
import type { TocEntry } from "@yoophi/markdown-annotation-core/types";
import { MarkdownToc } from "@yoophi/markdown-annotation-react";
import { Button } from "@/components/ui/button";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";

export type MarkdownPreviewTocProps = {
  entries: TocEntry[];
  onEntrySelect?: (entry: TocEntry) => void;
  defaultOpen?: boolean;
  className?: string;
};

export function MarkdownPreviewToc({
  entries,
  onEntrySelect,
  defaultOpen = false,
  className,
}: MarkdownPreviewTocProps) {
  if (entries.length === 0) {
    return null;
  }

  return (
    <Collapsible asChild defaultOpen={defaultOpen}>
      <section className={cn("rounded-lg border bg-card", className)}>
        <CollapsibleTrigger asChild>
          <Button
            className="group w-full justify-start gap-2"
            size="sm"
            type="button"
            variant="ghost"
          >
            <ChevronRightIcon
              aria-hidden="true"
              className="size-4 transition-transform group-data-[state=open]:rotate-90"
            />
            Contents
          </Button>
        </CollapsibleTrigger>
        <CollapsibleContent>
          <div className="max-h-64 overflow-y-auto border-t p-2">
            <MarkdownToc entries={entries} onEntrySelect={onEntrySelect} />
          </div>
        </CollapsibleContent>
      </section>
    </Collapsible>
  );
}
