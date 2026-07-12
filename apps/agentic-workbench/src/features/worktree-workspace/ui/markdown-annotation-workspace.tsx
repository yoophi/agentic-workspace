import type { RefObject } from "react";
import { PencilLineIcon, SendIcon, StickyNoteIcon, Trash2Icon } from "lucide-react";
import type { MarkdownBlock, TocEntry } from "@yoophi/markdown-annotation-core/types";
import { AnnotationInputDialog, MarkdownViewer, scrollToBlock } from "@yoophi/markdown-annotation-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { MarkdownAnnotationWorkspaceModel } from "@/features/worktree-workspace/model/use-markdown-annotation-workspace";
import { annotationDialogComponents } from "./annotation-dialog-components";
import { MarkdownPreviewToc } from "./markdown-preview-toc";
import { markdownViewerComponents } from "./markdown-viewer-components";

export function MarkdownAnnotationWorkspace({
  blocks,
  contentRef,
  model,
  onSendAnnotationPrompt,
  previewRef,
  tocEntries,
}: {
  blocks: MarkdownBlock[];
  contentRef: RefObject<HTMLDivElement | null>;
  model: MarkdownAnnotationWorkspaceModel;
  onSendAnnotationPrompt?: (prompt: string) => void;
  previewRef: RefObject<HTMLDivElement | null>;
  tocEntries: TocEntry[];
}) {
  return (
    <>
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_20rem]">
        <div ref={contentRef} className="relative min-w-0">
          <MarkdownViewer
            blocks={blocks}
            components={markdownViewerComponents}
            annotatedBlockIds={model.viewerMaps.annotatedBlockIds}
            deletedBlockIds={model.viewerMaps.deletedBlockIds}
            inlineAnnotationsByBlock={model.viewerMaps.inlineAnnotationsByBlock}
            noteAnnotationsByBlock={model.viewerMaps.noteAnnotationsByBlock}
            onCancelInlineAnnotation={model.removeAnnotation}
            onEditInlineAnnotation={model.editAnnotation}
            onRequestBlockComment={model.requestBlockComment}
            onRequestBlockDelete={model.toggleBlockDelete}
          />
          {model.selectionHighlightRects.map((rect, index) => (
            <div aria-hidden="true" className="pointer-events-none absolute bg-yellow-200/50" key={`${rect.left}-${rect.top}-${index}`} style={{ height: rect.height, left: rect.left, top: rect.top, width: rect.width }} />
          ))}
          {model.selectionToolbarPosition ? (
            <div className="absolute z-10 flex items-center gap-1 rounded-lg border bg-popover p-1 shadow-sm" style={{ left: model.selectionToolbarPosition.left, top: model.selectionToolbarPosition.top }} onMouseDown={(event) => { event.preventDefault(); event.stopPropagation(); }} onMouseUp={(event) => event.stopPropagation()} onClick={(event) => event.stopPropagation()}>
              <Button type="button" size="icon-sm" variant="ghost" aria-label="선택 영역 삭제" onClick={model.deleteSelection}><Trash2Icon /></Button>
              <Button type="button" size="icon-sm" variant="ghost" aria-label="선택 영역에 노트 추가" onClick={model.requestSelectionNote}><StickyNoteIcon /></Button>
            </div>
          ) : null}
        </div>
        <aside className="flex flex-col gap-3">
          <div className="rounded-md border p-3">
            <div className="flex items-center justify-between gap-2"><h3 className="text-sm font-medium">Annotations</h3><Badge variant="outline">{model.annotations.length}</Badge></div>
            {model.annotations.length === 0 ? <p className="mt-3 text-sm text-muted-foreground">No annotations.</p> : (
              <div className="mt-3 grid gap-2">
                {model.annotations.map((annotation) => (
                  <div key={annotation.id} className="rounded-md border p-2">
                    <div className="flex items-start justify-between gap-2">
                      <div className="min-w-0"><Badge variant={annotation.type === "delete" ? "destructive" : "secondary"}>{annotation.type}</Badge><p className="mt-1 text-xs text-muted-foreground">Lines {annotation.anchor.startLine}-{annotation.anchor.endLine}</p></div>
                      <div className="flex shrink-0 gap-1"><Button type="button" size="icon-xs" variant="ghost" aria-label="Annotation 편집" onClick={() => model.editAnnotation(annotation.id)}><PencilLineIcon /></Button><Button type="button" size="icon-xs" variant="ghost" aria-label="Annotation 삭제" onClick={() => model.removeAnnotation(annotation.id)}><Trash2Icon /></Button></div>
                    </div>
                    <p className="mt-2 line-clamp-3 whitespace-pre-wrap break-words text-xs text-muted-foreground">{annotation.selectedText}</p>
                    {annotation.comment ? <p className="mt-2 break-words text-sm">{annotation.comment}</p> : null}
                  </div>
                ))}
              </div>
            )}
          </div>
          {model.annotations.length > 0 ? (
            <div className="rounded-md border"><div className="flex items-center justify-between border-b px-3 py-2"><span className="text-sm font-medium">Agent prompt</span><Button type="button" size="sm" disabled={!onSendAnnotationPrompt} onClick={() => onSendAnnotationPrompt?.(model.annotationPrompt)}><SendIcon data-icon="inline-start" />Send</Button></div><pre className="max-h-80 overflow-auto p-3 text-xs leading-5"><code>{model.annotationPrompt}</code></pre></div>
          ) : null}
          <MarkdownPreviewToc className="sticky bottom-4 mt-auto shadow-sm" entries={tocEntries} onEntrySelect={(entry) => scrollToBlock(previewRef.current, entry.blockId)} />
        </aside>
      </div>
      <AnnotationInputDialog
        open={model.draftTarget !== null}
        onOpenChange={(open) => { if (!open) { model.resetDraft(); model.resetSelection(); } }}
        isEditing={model.editingAnnotationId !== null}
        selectedText={model.draftTarget ? model.draftTarget.kind === "selection" ? model.draftTarget.text : model.draftTarget.block.rawContent : ""}
        type={model.draftType}
        onTypeChange={model.setDraftType}
        comment={model.draftComment}
        onCommentChange={model.setDraftComment}
        onSubmit={model.saveAnnotation}
        components={annotationDialogComponents}
      />
    </>
  );
}
