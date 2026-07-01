import type { ElementType, MouseEvent, ReactNode } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { MessageSquare, Pencil, StickyNote, Trash2, X } from "lucide-react";
import type { MarkdownBlock } from "@yoophi/markdown-annotation-core/types";
import { cn } from "./cn";
import { MermaidDiagram } from "./MermaidDiagram";
import { segmentTextByAnnotations } from "./segment-text";
import type {
  MarkdownViewerBlockNote,
  MarkdownViewerComponents,
  MarkdownViewerInlineAnnotation,
} from "./types";

export type MarkdownViewerProps = {
  blocks: MarkdownBlock[];
  components: MarkdownViewerComponents;
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
  components,
  onCancelInlineAnnotation,
  onEditInlineAnnotation,
}: {
  annotation: MarkdownViewerInlineAnnotation;
  children: string;
  components: MarkdownViewerComponents;
  onCancelInlineAnnotation?: (annotationId: string) => void;
  onEditInlineAnnotation?: (annotationId: string) => void;
}) {
  const { Button, Tooltip } = components;
  const isDelete = annotation.type === "delete";
  const isNote = annotation.type === "note";
  const handleActionMouseDown = (event: MouseEvent) => {
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
    <Tooltip content={<p>{annotation.comment}</p>}>
      {mark}
    </Tooltip>
  );
}

function AnnotatedText({
  annotations,
  children,
  components,
  onCancelInlineAnnotation,
  onEditInlineAnnotation,
}: {
  annotations: MarkdownViewerInlineAnnotation[];
  children: string;
  components: MarkdownViewerComponents;
  onCancelInlineAnnotation?: (annotationId: string) => void;
  onEditInlineAnnotation?: (annotationId: string) => void;
}) {
  const segments = segmentTextByAnnotations(children, annotations);

  return (
    <>
      {segments.map((segment, index) =>
        segment.kind === "text" ? (
          // eslint-disable-next-line react/no-array-index-key
          <span key={`text-${index}`}>{segment.text}</span>
        ) : (
          <InlineAnnotationMark
            annotation={segment.annotation}
            components={components}
            key={segment.annotation.id}
            onCancelInlineAnnotation={onCancelInlineAnnotation}
            onEditInlineAnnotation={onEditInlineAnnotation}
          >
            {segment.text}
          </InlineAnnotationMark>
        ),
      )}
    </>
  );
}

function InlineMarkdown({
  annotations = [],
  children,
  components,
  onCancelInlineAnnotation,
  onEditInlineAnnotation,
}: {
  annotations?: MarkdownViewerInlineAnnotation[];
  children: string;
  components: MarkdownViewerComponents;
  onCancelInlineAnnotation?: (annotationId: string) => void;
  onEditInlineAnnotation?: (annotationId: string) => void;
}) {
  if (annotations.length > 0) {
    return (
      <AnnotatedText
        annotations={annotations}
        components={components}
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
  notes,
  children,
  components,
  onRequestBlockComment,
  onRequestBlockDelete,
}: {
  block: MarkdownBlock;
  annotated: boolean;
  deleted: boolean;
  notes: MarkdownViewerBlockNote[];
  children: ReactNode;
  components: MarkdownViewerComponents;
  onRequestBlockComment?: (block: MarkdownBlock) => void;
  onRequestBlockDelete?: (block: MarkdownBlock) => void;
}) {
  const { Button, Tooltip } = components;
  const hasNotes = notes.length > 0;

  const handleToolbarMouseDown = (event: MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
  };

  const handleToolbarInteraction = (event: MouseEvent) => {
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
          <Tooltip
            align="end"
            content={
              <div className="flex flex-col gap-2">
                {notes.map((note) => (
                  <p key={note.id}>{note.comment}</p>
                ))}
              </div>
            }
          >
            <Button
              aria-label="Show note annotations"
              className="relative"
              size="icon-sm"
              type="button"
              variant="secondary"
            >
              <StickyNote aria-hidden="true" />
              {notes.length > 1 ? (
                <span className="absolute -right-1 -top-1 flex size-4 items-center justify-center rounded-full bg-primary text-[0.625rem] font-medium text-primary-foreground">
                  {notes.length}
                </span>
              ) : null}
            </Button>
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
        <Tooltip content={deleted ? "Cancel delete" : "Delete block"}>
          <Button
            aria-label="Delete block"
            aria-pressed={deleted}
            size="icon-sm"
            type="button"
            variant={deleted ? "destructive" : "ghost"}
            onClick={() => onRequestBlockDelete?.(block)}
          >
            <Trash2 aria-hidden="true" />
          </Button>
        </Tooltip>
        <Tooltip content="Comment on block">
          <Button
            aria-label="Comment on block"
            size="icon-sm"
            type="button"
            variant="ghost"
            onClick={() => onRequestBlockComment?.(block)}
          >
            <MessageSquare aria-hidden="true" />
          </Button>
        </Tooltip>
      </div>
      <div className={cn("relative z-0", deleted && deleteAnnotationClassName)}>{children}</div>
    </div>
  );
}

function MarkdownBlockRenderer({
  block,
  annotated,
  deleted,
  inlineAnnotations,
  notes,
  components,
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
  components: MarkdownViewerComponents;
  onCancelInlineAnnotation?: (annotationId: string) => void;
  onEditInlineAnnotation?: (annotationId: string) => void;
  onRequestBlockComment?: (block: MarkdownBlock) => void;
  onRequestBlockDelete?: (block: MarkdownBlock) => void;
}) {
  const shellProps = {
    annotated,
    block,
    components,
    deleted,
    notes,
    onRequestBlockComment,
    onRequestBlockDelete,
  };

  const inline = (
    <InlineMarkdown
      annotations={inlineAnnotations}
      components={components}
      onCancelInlineAnnotation={onCancelInlineAnnotation}
      onEditInlineAnnotation={onEditInlineAnnotation}
    >
      {block.content}
    </InlineMarkdown>
  );

  switch (block.type) {
    case "heading": {
      const Tag = `h${block.level ?? 1}` as ElementType;
      return (
        <BlockShell {...shellProps}>
          <Tag data-block-content>{inline}</Tag>
        </BlockShell>
      );
    }

    case "blockquote":
      return (
        <BlockShell {...shellProps}>
          <blockquote data-block-content>{inline}</blockquote>
        </BlockShell>
      );

    case "list-item":
      return (
        <BlockShell {...shellProps}>
          <div
            className="flex items-start gap-3"
            style={{ marginLeft: `${(block.level ?? 0) * 1.25}rem` }}
          >
            <span className="mt-0.5 text-muted-foreground">
              {block.ordered ? `${block.orderedStart ?? 1}.` : "-"}
            </span>
            <div
              className={cn(block.checked && "text-muted-foreground line-through")}
              data-block-content
            >
              {inline}
            </div>
          </div>
        </BlockShell>
      );

    case "code":
      if (block.mermaid) {
        return (
          <BlockShell {...shellProps}>
            <MermaidDiagram blockId={block.id} source={block.content} />
          </BlockShell>
        );
      }

      return (
        <BlockShell {...shellProps}>
          <pre>
            <code data-block-content>
              <AnnotatedText
                annotations={inlineAnnotations}
                components={components}
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
        <BlockShell {...shellProps}>
          <div data-block-content>
            <ReactMarkdown remarkPlugins={[remarkGfm]}>{block.content}</ReactMarkdown>
          </div>
        </BlockShell>
      );

    case "hr":
      return (
        <BlockShell {...shellProps}>
          <hr />
        </BlockShell>
      );

    case "paragraph":
    default:
      return (
        <BlockShell {...shellProps}>
          <p data-block-content>{inline}</p>
        </BlockShell>
      );
  }
}

export function MarkdownViewer({
  blocks,
  components,
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
    <article className="markdown-viewer max-w-none">
      {blocks.map((block) => (
        <MarkdownBlockRenderer
          annotated={annotatedBlockIds.has(block.id)}
          block={block}
          components={components}
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
