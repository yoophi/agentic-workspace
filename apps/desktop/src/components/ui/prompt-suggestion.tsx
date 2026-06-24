import * as React from "react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

type PromptSuggestionProps = React.ComponentProps<typeof Button> & {
  highlight?: string;
};

function PromptSuggestion({
  children,
  className,
  highlight,
  size = highlight ? "sm" : "lg",
  variant = highlight ? "ghost" : "outline",
  ...props
}: PromptSuggestionProps) {
  return (
    <Button
      type="button"
      data-slot="prompt-suggestion"
      size={size}
      variant={variant}
      className={cn("max-w-full rounded-full", className)}
      {...props}
    >
      {highlight && typeof children === "string"
        ? renderHighlightedText(children, highlight)
        : children}
    </Button>
  );
}

function renderHighlightedText(text: string, highlight: string) {
  const query = highlight.trim();

  if (!query) {
    return text;
  }

  const index = text.toLocaleLowerCase().indexOf(query.toLocaleLowerCase());

  if (index === -1) {
    return text;
  }

  return (
    <>
      {text.slice(0, index)}
      <mark className="rounded bg-primary/15 px-0.5 text-primary">
        {text.slice(index, index + query.length)}
      </mark>
      {text.slice(index + query.length)}
    </>
  );
}

export { PromptSuggestion };
