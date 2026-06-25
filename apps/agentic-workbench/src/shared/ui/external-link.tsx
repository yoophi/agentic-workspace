import type { AnchorHTMLAttributes, MouseEvent } from "react";

import { openExternalUrl } from "@/shared/api/external-url";
import { isOpenableExternalUrl } from "@/shared/lib/external-url";
import { cn } from "@/lib/utils";

type ExternalLinkProps = Omit<AnchorHTMLAttributes<HTMLAnchorElement>, "href"> & {
  href?: string;
};

export function ExternalLink({
  children,
  className,
  href,
  onClick,
  ...props
}: ExternalLinkProps) {
  const canOpen = isOpenableExternalUrl(href);

  function handleClick(event: MouseEvent<HTMLAnchorElement>) {
    onClick?.(event);
    if (event.defaultPrevented) return;

    event.preventDefault();
    if (!canOpen) return;

    void openExternalUrl(href).catch((error) => {
      console.error("Failed to open external URL", error);
    });
  }

  return (
    <a
      className={cn(
        "font-medium text-primary underline underline-offset-4 hover:text-primary/80",
        !canOpen && "cursor-not-allowed opacity-70",
        className
      )}
      href={href}
      onClick={handleClick}
      rel="noreferrer"
      target="_blank"
      title={canOpen ? "Open in browser" : undefined}
      {...props}
    >
      {children}
    </a>
  );
}
