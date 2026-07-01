import type { MarkdownDocument } from "@/entities/document";

export function shouldSwapDocument(current: MarkdownDocument, reloaded: MarkdownDocument) {
  return current.markdownText !== reloaded.markdownText;
}
