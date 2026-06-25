import { useState } from "react";
import type { ReactNode } from "react";

import {
  Popover,
  PopoverAnchor,
  PopoverContent,
} from "@/components/ui/popover";
import { cn } from "@/lib/utils";

type EllipsisPopoverTextProps = {
  value: ReactNode;
  popoverValue?: ReactNode;
  className?: string;
  contentClassName?: string;
  focusable?: boolean;
};

export function EllipsisPopoverText({
  value,
  popoverValue = value,
  className,
  contentClassName,
  focusable = true,
}: EllipsisPopoverTextProps) {
  const [open, setOpen] = useState(false);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverAnchor asChild>
        <span
          tabIndex={focusable ? 0 : undefined}
          className={cn(
            "block min-w-0 max-w-full cursor-default truncate outline-none focus-visible:ring-2 focus-visible:ring-ring/50",
            className,
          )}
          onMouseEnter={() => setOpen(true)}
          onMouseLeave={() => setOpen(false)}
          onFocus={focusable ? () => setOpen(true) : undefined}
          onBlur={focusable ? () => setOpen(false) : undefined}
        >
          {value}
        </span>
      </PopoverAnchor>
      <PopoverContent
        side="top"
        align="start"
        className={cn(
          "max-w-[min(32rem,var(--radix-popover-content-available-width))] break-words font-normal",
          contentClassName,
        )}
        onOpenAutoFocus={(event) => event.preventDefault()}
      >
        {popoverValue}
      </PopoverContent>
    </Popover>
  );
}
