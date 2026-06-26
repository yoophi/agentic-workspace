import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import type { ElementType } from "react";
import { MessageSquare, Pencil, StickyNote, Trash2, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { AnnotationType } from "@/entities/annotation";
import type { MarkdownBlock } from "@/entities/markdown-block";
import { cn } from "@/lib/utils";

export type MarkdownViewerInlineAnnotation = {
  id: string;
  comment: string;
  endOffset: number;
  startOffset: number;
  type: AnnotationType;
};

export type MarkdownViewerBlockNote = {
  id: string;
  comment: string;
};

type MarkdownViewerProps = {
  blocks: MarkdownBlock[];
  annotatedBlockIds?: Set<string>;
  deletedBlockIds?: Set<string>;
  inlineAnnotationsByBlock?: ReadonlyMap<string, MarkdownViewerInlineAnnotation[]>;
  noteAnnotationsByBlock?: ReadonlyMap<string, MarkdownViewerBlockNote[]>;
  onCancelInlineAnnotation?: (annotationId: string) => void;
  onEditInlineAnnotation?: (annotationId: string) => void;
  onRequestBlockComment?: (block: MarkdownBlock) => void;
  onRequestBlockDelete?: (block: MarkdownBlock) => void;
};

const deleteAnnotationClassName =
  "text-destructive line-through decoration-destructive decoration-2 [&_*]:text-destructive";

function InlineAnnotationMark({
  annotation,
  children,
  onCancelInlineAnnotation,
  onEditInlineAnnotation,
}: {
  annotation: MarkdownViewerInlineAnnotation;
  children: string;
  onCancelInlineAnnotation?: (annotationId: string) => void;
  onEditInlineAnnotation?: (annotationId: string) => void;
}) {
  const isDelete = annotation.type === "delete";
  const isNote = annotation.type === "note";
  const handleActionMouseDown = (event: React.MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
  };
  const mark = (
    <mark
      className={cn(
        "group/inline-annotation relative inline-block px-0.5",
        isDelete && cn("bg-transparent", deleteAnnotationClassName),
        isNote &&
          "relative bg-yellow-200 text-foreground after:absolute after:right-0 after:top-0 after:size-0 after:border-l-[7px] after:border-t-[7px] after:border-l-transparent after:border-t-yellow-600",
      )}
      data-annotation-id={annotation.id}
    >
      {children}
      <span
        className="absolute -right-3 top-0 z-10 hidden translate-x-full -translate-y-1/2 items-center gap-1 rounded-lg border bg-popover p-1 shadow-sm before:absolute before:-left-3 before:top-0 before:h-full before:w-3 before:content-[''] group-hover/inline-annotation:inline-flex group-focus-within/inline-annotation:inline-flex"
        onMouseDown={handleActionMouseDown}
        onMouseUp={(event) => event.stopPropagation()}
        onClick={(event) => event.stopPropagation()}
      >
        {isNote ? (
          <Button
            aria-label="Edit note annotation"
            size="icon-xs"
            type="button"
            variant="ghost"
            onClick={() => onEditInlineAnnotation?.(annotation.id)}
          >
            <Pencil aria-hidden="true" />
          </Button>
        ) : null}
        <Button
          aria-label={isDelete ? "Cancel delete annotation" : "Delete note annotation"}
          size="icon-xs"
          type="button"
          variant={isDelete ? "destructive" : "ghost"}
          onClick={() => onCancelInlineAnnotation?.(annotation.id)}
        >
          {isDelete ? <X aria-hidden="true" /> : <Trash2 aria-hidden="true" />}
        </Button>
      </span>
    </mark>
  );

  if (isDelete) {
    return mark;
  }

  return (
    <Tooltip>
      <TooltipTrigger render={mark} />
      <TooltipContent className="max-w-sm">
        <p>{annotation.comment}</p>
      </TooltipContent>
    </Tooltip>
  );
}

function AnnotatedText({
  annotations,
  children,
  onCancelInlineAnnotation,
  onEditInlineAnnotation,
}: {
  annotations: MarkdownViewerInlineAnnotation[];
  children: string;
  onCancelInlineAnnotation?: (annotationId: string) => void;
  onEditInlineAnnotation?: (annotationId: string) => void;
}) {
  const sortedAnnotations = [...annotations]
    .filter((annotation) => annotation.endOffset > annotation.startOffset)
    .sort((a, b) => a.startOffset - b.startOffset);
  const segments: React.ReactNode[] = [];
  let cursor = 0;

  sortedAnnotations.forEach((annotation) => {
    const startOffset = Math.max(0, Math.min(annotation.startOffset, children.length));
    const endOffset = Math.max(startOffset, Math.min(annotation.endOffset, children.length));

    if (startOffset < cursor || startOffset === endOffset) {
      return;
    }

    if (cursor < startOffset) {
      segments.push(children.slice(cursor, startOffset));
    }

    segments.push(
      <InlineAnnotationMark
        annotation={annotation}
        key={annotation.id}
        onCancelInlineAnnotation={onCancelInlineAnnotation}
        onEditInlineAnnotation={onEditInlineAnnotation}
      >
        {children.slice(startOffset, endOffset)}
      </InlineAnnotationMark>,
    );
    cursor = endOffset;
  });

  if (cursor < children.length) {
    segments.push(children.slice(cursor));
  }

  return <>{segments}</>;
}

function InlineMarkdown({
  annotations = [],
  children,
  onCancelInlineAnnotation,
  onEditInlineAnnotation,
}: {
  annotations?: MarkdownViewerInlineAnnotation[];
  children: string;
  onCancelInlineAnnotation?: (annotationId: string) => void;
  onEditInlineAnnotation?: (annotationId: string) => void;
}) {
  if (annotations.length > 0) {
    return (
      <AnnotatedText
        annotations={annotations}
        onCancelInlineAnnotation={onCancelInlineAnnotation}
        onEditInlineAnnotation={onEditInlineAnnotation}
      >
        {children}
      </AnnotatedText>
    );
  }

  return (
    <ReactMarkdown
      components={{
        p: ({ children: inlineChildren }) => <>{inlineChildren}</>,
      }}
      remarkPlugins={[remarkGfm]}
    >
      {children}
    </ReactMarkdown>
  );
}

function BlockShell({
  block,
  annotated,
  deleted,
  inlineAnnotations,
  notes,
  children,
  onRequestBlockComment,
  onRequestBlockDelete,
}: {
  block: MarkdownBlock;
  annotated: boolean;
  deleted: boolean;
  inlineAnnotations: MarkdownViewerInlineAnnotation[];
  notes: MarkdownViewerBlockNote[];
  children: React.ReactNode;
  onCancelInlineAnnotation?: (annotationId: string) => void;
  onEditInlineAnnotation?: (annotationId: string) => void;
  onRequestBlockComment?: (block: MarkdownBlock) => void;
  onRequestBlockDelete?: (block: MarkdownBlock) => void;
}) {
  const hasNotes = notes.length > 0;

  const handleToolbarMouseDown = (event: React.MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
  };

  const handleToolbarInteraction = (event: React.MouseEvent) => {
    event.stopPropagation();
  };

  return (
    <div
      className={cn(
        "group/markdown-block relative border-r-4 border-transparent bg-transparent pr-12 transition-colors",
        "hover:border-border",
      )}
      data-annotated={annotated || undefined}
      data-block-id={block.id}
      data-block-type={block.type}
      data-start-line={block.startLine}
      data-end-line={block.endLine}
    >
      {hasNotes ? (
        <div
          className="absolute right-2 top-0 z-20"
          onMouseDown={handleToolbarMouseDown}
          onMouseUp={handleToolbarInteraction}
          onClick={handleToolbarInteraction}
        >
          <Tooltip>
            <TooltipTrigger
              render={
                <Button
                  aria-label="Show note annotations"
                  className="relative"
                  size="icon-sm"
                  type="button"
                  variant="secondary"
                />
              }
            >
              <StickyNote aria-hidden="true" />
              {notes.length > 1 ? (
                <span className="absolute -right-1 -top-1 flex size-4 items-center justify-center rounded-full bg-primary text-[0.625rem] font-medium text-primary-foreground">
                  {notes.length}
                </span>
              ) : null}
            </TooltipTrigger>
            <TooltipContent align="end" className="max-w-sm">
              <div className="flex flex-col gap-2">
                {notes.map((note) => (
                  <p key={note.id}>{note.comment}</p>
                ))}
              </div>
            </TooltipContent>
          </Tooltip>
        </div>
      ) : null}

      <div
        className={cn(
          "absolute right-2 top-0 z-30 hidden items-center gap-1 rounded-lg border bg-popover p-1 shadow-sm",
          "group-hover/markdown-block:flex group-focus-within/markdown-block:flex",
          hasNotes && "right-11",
        )}
        onMouseDown={handleToolbarMouseDown}
        onMouseUp={handleToolbarInteraction}
        onClick={handleToolbarInteraction}
      >
        <Tooltip>
          <TooltipTrigger
            render={
              <Button
                aria-label="Delete block"
                aria-pressed={deleted}
                size="icon-sm"
                type="button"
                variant={deleted ? "destructive" : "ghost"}
                onClick={() => onRequestBlockDelete?.(block)}
              />
            }
          >
            <Trash2 aria-hidden="true" />
          </TooltipTrigger>
          <TooltipContent>{deleted ? "Cancel delete" : "Delete block"}</TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger
            render={
              <Button
                aria-label="Comment on block"
                size="icon-sm"
                type="button"
                variant="ghost"
                onClick={() => onRequestBlockComment?.(block)}
              />
            }
          >
            <MessageSquare aria-hidden="true" />
          </TooltipTrigger>
          <TooltipContent>Comment on block</TooltipContent>
        </Tooltip>
      </div>
      <div
        className={cn(
          "relative z-0",
          deleted && deleteAnnotationClassName,
        )}
      >
        {children}
      </div>
    </div>
  );
}

function MarkdownBlockRenderer({
  block,
  annotated,
  deleted,
  inlineAnnotations,
  notes,
  onCancelInlineAnnotation,
  onEditInlineAnnotation,
  onRequestBlockComment,
  onRequestBlockDelete,
}: {
  block: MarkdownBlock;
  annotated: boolean;
  deleted: boolean;
  inlineAnnotations: MarkdownViewerInlineAnnotation[];
  notes: MarkdownViewerBlockNote[];
  onCancelInlineAnnotation?: (annotationId: string) => void;
  onEditInlineAnnotation?: (annotationId: string) => void;
  onRequestBlockComment?: (block: MarkdownBlock) => void;
  onRequestBlockDelete?: (block: MarkdownBlock) => void;
}) {
  switch (block.type) {
    case "heading": {
      const Tag = `h${block.level ?? 1}` as ElementType;
      return (
        <BlockShell
          annotated={annotated}
          block={block}
          deleted={deleted}
          inlineAnnotations={inlineAnnotations}
          notes={notes}
          onCancelInlineAnnotation={onCancelInlineAnnotation}
          onEditInlineAnnotation={onEditInlineAnnotation}
          onRequestBlockComment={onRequestBlockComment}
          onRequestBlockDelete={onRequestBlockDelete}
        >
          <Tag data-block-content>
            <InlineMarkdown
              annotations={inlineAnnotations}
              onCancelInlineAnnotation={onCancelInlineAnnotation}
              onEditInlineAnnotation={onEditInlineAnnotation}
            >
              {block.content}
            </InlineMarkdown>
          </Tag>
        </BlockShell>
      );
    }

    case "blockquote":
      return (
        <BlockShell
          annotated={annotated}
          block={block}
          deleted={deleted}
          inlineAnnotations={inlineAnnotations}
          notes={notes}
          onCancelInlineAnnotation={onCancelInlineAnnotation}
          onEditInlineAnnotation={onEditInlineAnnotation}
          onRequestBlockComment={onRequestBlockComment}
          onRequestBlockDelete={onRequestBlockDelete}
        >
          <blockquote data-block-content>
            <InlineMarkdown
              annotations={inlineAnnotations}
              onCancelInlineAnnotation={onCancelInlineAnnotation}
              onEditInlineAnnotation={onEditInlineAnnotation}
            >
              {block.content}
            </InlineMarkdown>
          </blockquote>
        </BlockShell>
      );

    case "list-item":
      return (
        <BlockShell
          annotated={annotated}
          block={block}
          deleted={deleted}
          inlineAnnotations={inlineAnnotations}
          notes={notes}
          onCancelInlineAnnotation={onCancelInlineAnnotation}
          onEditInlineAnnotation={onEditInlineAnnotation}
          onRequestBlockComment={onRequestBlockComment}
          onRequestBlockDelete={onRequestBlockDelete}
        >
          <div
            className="flex items-start gap-3"
            style={{ marginLeft: `${(block.level ?? 0) * 1.25}rem` }}
          >
            <span className="mt-0.5 text-muted-foreground">
              {block.ordered ? `${block.orderedStart ?? 1}.` : "-"}
            </span>
            <div className={cn(block.checked && "text-muted-foreground line-through")} data-block-content>
              <InlineMarkdown
                annotations={inlineAnnotations}
                onCancelInlineAnnotation={onCancelInlineAnnotation}
                onEditInlineAnnotation={onEditInlineAnnotation}
              >
                {block.content}
              </InlineMarkdown>
            </div>
          </div>
        </BlockShell>
      );

    case "code":
      return (
        <BlockShell
          annotated={annotated}
          block={block}
          deleted={deleted}
          inlineAnnotations={inlineAnnotations}
          notes={notes}
          onCancelInlineAnnotation={onCancelInlineAnnotation}
          onEditInlineAnnotation={onEditInlineAnnotation}
          onRequestBlockComment={onRequestBlockComment}
          onRequestBlockDelete={onRequestBlockDelete}
        >
          <pre>
            <code data-block-content>
              <AnnotatedText
                annotations={inlineAnnotations}
                onCancelInlineAnnotation={onCancelInlineAnnotation}
                onEditInlineAnnotation={onEditInlineAnnotation}
              >
                {block.content}
              </AnnotatedText>
            </code>
          </pre>
        </BlockShell>
      );

    case "table":
      return (
        <BlockShell
          annotated={annotated}
          block={block}
          deleted={deleted}
          inlineAnnotations={inlineAnnotations}
          notes={notes}
          onCancelInlineAnnotation={onCancelInlineAnnotation}
          onEditInlineAnnotation={onEditInlineAnnotation}
          onRequestBlockComment={onRequestBlockComment}
          onRequestBlockDelete={onRequestBlockDelete}
        >
          <div data-block-content>
            <ReactMarkdown remarkPlugins={[remarkGfm]}>{block.content}</ReactMarkdown>
          </div>
        </BlockShell>
      );

    case "hr":
      return (
        <BlockShell
          annotated={annotated}
          block={block}
          deleted={deleted}
          inlineAnnotations={inlineAnnotations}
          notes={notes}
          onCancelInlineAnnotation={onCancelInlineAnnotation}
          onEditInlineAnnotation={onEditInlineAnnotation}
          onRequestBlockComment={onRequestBlockComment}
          onRequestBlockDelete={onRequestBlockDelete}
        >
          <hr />
        </BlockShell>
      );

    case "paragraph":
    default:
      return (
        <BlockShell
          annotated={annotated}
          block={block}
          deleted={deleted}
          inlineAnnotations={inlineAnnotations}
          notes={notes}
          onCancelInlineAnnotation={onCancelInlineAnnotation}
          onEditInlineAnnotation={onEditInlineAnnotation}
          onRequestBlockComment={onRequestBlockComment}
          onRequestBlockDelete={onRequestBlockDelete}
        >
          <p data-block-content>
            <InlineMarkdown
              annotations={inlineAnnotations}
              onCancelInlineAnnotation={onCancelInlineAnnotation}
              onEditInlineAnnotation={onEditInlineAnnotation}
            >
              {block.content}
            </InlineMarkdown>
          </p>
        </BlockShell>
      );
  }
}

export function MarkdownViewer({
  blocks,
  annotatedBlockIds = new Set(),
  deletedBlockIds = new Set(),
  inlineAnnotationsByBlock = new Map(),
  noteAnnotationsByBlock = new Map(),
  onCancelInlineAnnotation,
  onEditInlineAnnotation,
  onRequestBlockComment,
  onRequestBlockDelete,
}: MarkdownViewerProps) {
  return (
    <article className="markdown-viewer flex max-w-none flex-col gap-0">
      {blocks.map((block) => (
        <MarkdownBlockRenderer
          annotated={annotatedBlockIds.has(block.id)}
          block={block}
          deleted={deletedBlockIds.has(block.id)}
          inlineAnnotations={inlineAnnotationsByBlock.get(block.id) ?? []}
          key={block.id}
          notes={noteAnnotationsByBlock.get(block.id) ?? []}
          onCancelInlineAnnotation={onCancelInlineAnnotation}
          onEditInlineAnnotation={onEditInlineAnnotation}
          onRequestBlockComment={onRequestBlockComment}
          onRequestBlockDelete={onRequestBlockDelete}
        />
      ))}
    </article>
  );
}
