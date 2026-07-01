import {
  CheckCircle2,
  ClipboardCopy,
  FileText,
  FolderOpen,
  MessageSquarePlus,
  PanelLeftClose,
  PanelLeftOpen,
  StickyNote,
  Terminal,
  Trash2,
} from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useRef, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import type { AnnotationAnchor, AnnotationDraft, AnnotationType } from "@/entities/annotation";
import {
  extractTocEntries,
  formatAnnotationsForAgent,
  parseMarkdownToBlocks,
  type AgentPromptGoal,
  type TocEntry,
} from "@yoophi/markdown-annotation-core";
import {
  checkCliInstalled,
  installCli,
  readMarkdownDocument,
  startMarkdownDocumentWatcher,
  stopMarkdownDocumentWatcher,
} from "@/entities/document/api/documentApi";
import { exampleMarkdownDocuments } from "@/entities/document/model/examples";
import type { MarkdownDocument } from "@/entities/document";
import type { MarkdownBlock } from "@/entities/markdown-block";
import {
  getDocumentPathFromWindowQuery,
  openMarkdownDocumentTab,
} from "@/features/open-document/openMarkdownDocument";
import { shouldSwapDocument } from "./model/document-reload";
import {
  AnnotationInputDialog,
  MarkdownToc,
  MarkdownViewer,
  annotationTypes,
  buildViewerAnnotationMaps,
  getAnnotationCommentLabel as annotationCommentLabel,
  getAnnotationCommentPlaceholder as annotationCommentPlaceholder,
  getSelectionAnchors,
  getSelectionRects,
  requiresComment,
  scrollToBlock,
  type SelectionRect,
} from "@yoophi/markdown-annotation-react";
import {
  AUTO_REFRESH_INTERVAL_MS,
  MARKDOWN_DOCUMENT_CHANGED_EVENT,
  findStaleMarkdownDocument,
  type MarkdownDocumentChangedEvent,
  type StaleSelection,
} from "@yoophi/workspace-auto-refresh";
import { annotationDialogComponents } from "@/shared/ui/annotation-dialog-components";
import { markdownViewerComponents } from "@/shared/ui/markdown-viewer-components";

const WINDOW_HIGHLIGHT_EVENT = "markdown-annotator://window-highlight";

const promptGoals: Array<{ value: AgentPromptGoal; label: string; description: string }> = [
  {
    value: "edit-document",
    label: "문서 수정",
    description: "Agent가 annotation을 실제 문서 변경 요청으로 해석합니다.",
  },
  {
    value: "review-reference",
    label: "검토 참고",
    description: "Agent가 annotation을 수정 지시가 아닌 리뷰 메모로 참고합니다.",
  },
  {
    value: "custom",
    label: "기타",
    description: "아래 사용자 지침을 우선 따릅니다.",
  },
];

type SelectionToolbarPosition = {
  left: number;
  top: number;
};

function isTauriRuntime() {
  return "__TAURI_INTERNALS__" in window;
}

export function AnnotatorPage() {
  const documentPaneRef = useRef<HTMLDivElement>(null);
  const [selectedExampleId, setSelectedExampleId] = useState(exampleMarkdownDocuments[0]?.id ?? "");
  const [document, setDocument] = useState<MarkdownDocument>(() => {
    const initial = exampleMarkdownDocuments[0];
    return {
      fileName: initial.fileName,
      absolutePath: `examples/markdown-annotator/${initial.fileName}`,
      markdownText: initial.markdownText,
    };
  });
  const latestDocumentRef = useRef(document);
  const [selection, setSelection] = useState("");
  const [selectionAnchor, setSelectionAnchor] = useState<AnnotationAnchor | null>(null);
  const [selectionAnchors, setSelectionAnchors] = useState<AnnotationAnchor[]>([]);
  const [annotationType, setAnnotationType] = useState<AnnotationType>("change-request");
  const [comment, setComment] = useState("");
  const [promptGoal, setPromptGoal] = useState<AgentPromptGoal>("edit-document");
  const [promptInstruction, setPromptInstruction] = useState("수정 요청은 문서에 반영하고, note는 참고 정보로만 사용하세요.");
  const [promptFilePath, setPromptFilePath] = useState(() => {
    const initial = exampleMarkdownDocuments[0];
    return initial ? `examples/markdown-annotator/${initial.fileName}` : "";
  });
  const [annotations, setAnnotations] = useState<AnnotationDraft[]>([]);
  const [editingAnnotationId, setEditingAnnotationId] = useState<string | null>(null);
  const [noteDialogOpen, setNoteDialogOpen] = useState(false);
  const [isCliInstalled, setCliInstalled] = useState(false);
  const [isCliInstalling, setCliInstalling] = useState(false);
  const [selectionHighlightRects, setSelectionHighlightRects] = useState<SelectionRect[]>([]);
  const [selectionToolbarPosition, setSelectionToolbarPosition] = useState<SelectionToolbarPosition | null>(null);
  const [status, setStatus] = useState("예제 문서를 선택하거나 로컬 Markdown 파일을 열 수 있습니다.");
  const [isDocumentRefreshing, setDocumentRefreshing] = useState(false);
  const [documentReloadError, setDocumentReloadError] = useState<string | null>(null);
  const [staleDocument, setStaleDocument] = useState<StaleSelection | null>(null);
  const [isTocOpen, setTocOpen] = useState(true);

  latestDocumentRef.current = document;

  const title = document.fileName;
  const isReloadableDocument =
    isTauriRuntime() && !document.absolutePath.startsWith("examples/markdown-annotator/");
  const blocks = useMemo(() => parseMarkdownToBlocks(document.markdownText), [document.markdownText]);
  const tocEntries = useMemo(() => extractTocEntries(blocks), [blocks]);
  const exportText = useMemo(
    () =>
      formatAnnotationsForAgent(document.fileName, annotations, blocks, {
        filePath: promptFilePath || document.absolutePath,
        goal: promptGoal,
        instruction: promptInstruction,
      }),
    [annotations, blocks, document.absolutePath, document.fileName, promptFilePath, promptGoal, promptInstruction],
  );
  const { annotatedBlockIds, deletedBlockIds, inlineAnnotationsByBlock, noteAnnotationsByBlock } =
    useMemo(() => buildViewerAnnotationMaps(annotations, blocks), [annotations, blocks]);
  const visibleAnnotations = useMemo(() => {
    const seen = new Set<string>();

    return annotations.filter((annotation) => {
      const key = annotation.groupId ?? annotation.id;
      if (seen.has(key)) {
        return false;
      }

      seen.add(key);
      return true;
    });
  }, [annotations]);

  function resetSelectionState() {
    setSelection("");
    setSelectionAnchor(null);
    setSelectionAnchors([]);
    setSelectionHighlightRects([]);
    setSelectionToolbarPosition(null);
    window.getSelection()?.removeAllRanges();
  }

  function loadDocumentIntoWindow(opened: MarkdownDocument, message: string) {
    setSelectedExampleId("");
    setDocument(opened);
    setPromptFilePath(opened.absolutePath);
    setAnnotations([]);
    setDocumentReloadError(null);
    setStaleDocument(null);
    resetSelectionState();
    setEditingAnnotationId(null);
    setNoteDialogOpen(false);
    setComment("");
    setStatus(message);
  }

  function activeSelectionAnchors() {
    if (selectionAnchors.length > 0) {
      return selectionAnchors;
    }

    return selectionAnchor ? [selectionAnchor] : [];
  }

  function makeAnnotationsFromSelection(type: AnnotationType, annotationComment: string) {
    const anchors = activeSelectionAnchors();
    const groupId = anchors.length > 1 ? crypto.randomUUID() : undefined;
    const createdAt = new Date().toISOString();

    return anchors.map((anchor) => ({
      id: crypto.randomUUID(),
      groupId,
      fileName: document.fileName,
      anchor,
      selectedText: anchor.selectedText ?? selection,
      comment: annotationComment,
      type,
      createdAt,
    }));
  }

  function loadExample(exampleId: string | null) {
    if (!exampleId) {
      return;
    }

    const example = exampleMarkdownDocuments.find((candidate) => candidate.id === exampleId);
    if (!example) {
      return;
    }

    setSelectedExampleId(example.id);
    setDocument({
      fileName: example.fileName,
      absolutePath: `examples/markdown-annotator/${example.fileName}`,
      markdownText: example.markdownText,
    });
    setPromptFilePath(`examples/markdown-annotator/${example.fileName}`);
    setAnnotations([]);
    setDocumentReloadError(null);
    setStaleDocument(null);
    resetSelectionState();
    setEditingAnnotationId(null);
    setNoteDialogOpen(false);
    setComment("");
    setStatus(`${example.fileName} 예제를 불러왔습니다.`);
  }

  async function handleOpenFileAsTab() {
    try {
      const opened = await openMarkdownDocumentTab();
      if (!opened) {
        return;
      }

      loadDocumentIntoWindow(opened, `${opened.fileName} 파일을 열었습니다.`);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "파일을 탭으로 열 수 없습니다.");
    }
  }

  async function handleInstallCli() {
    setCliInstalling(true);
    try {
      const status = await installCli();
      setCliInstalled(status.installed);
      setStatus(`CLI를 설치했습니다: ${status.path}`);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "CLI를 설치할 수 없습니다.");
    } finally {
      setCliInstalling(false);
    }
  }

  async function reloadActiveDocument({ announceSuccess = false }: { announceSuccess?: boolean } = {}) {
    const activeDocument = latestDocumentRef.current;
    const isActiveDocumentReloadable =
      isTauriRuntime() && !activeDocument.absolutePath.startsWith("examples/markdown-annotator/");

    if (!isActiveDocumentReloadable) {
      return;
    }

    setDocumentRefreshing(true);
    try {
      const reloaded = await readMarkdownDocument(activeDocument.absolutePath);
      setDocumentReloadError(null);
      setStaleDocument(null);
      // Only swap document state when the content actually changed. Re-setting an
      // identical document on every poll would re-parse the markdown and reset the
      // reader's annotation/selection context unnecessarily.
      if (shouldSwapDocument(latestDocumentRef.current, reloaded)) {
        latestDocumentRef.current = reloaded;
        setDocument(reloaded);
        setPromptFilePath(reloaded.absolutePath);
      }
      if (announceSuccess) {
        setStatus(`${reloaded.fileName} 파일을 다시 불러왔습니다.`);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : "Markdown 파일을 다시 읽을 수 없습니다.";
      setDocumentReloadError(message);
      setStaleDocument(
        findStaleMarkdownDocument({
          absolutePath: activeDocument.absolutePath,
          readable: false,
        }),
      );
      setStatus(message);
    } finally {
      setDocumentRefreshing(false);
    }
  }

  function blockAnchor(block: MarkdownBlock): AnnotationAnchor {
    return {
      blockId: block.id,
      startOffset: 0,
      endOffset: block.content.length,
      selectedText: block.content,
      startLine: block.startLine,
      endLine: block.endLine,
    };
  }

  function requestBlockComment(block: MarkdownBlock) {
    const anchor = blockAnchor(block);
    setSelection(block.content);
    setSelectionAnchor(anchor);
    setSelectionAnchors([anchor]);
    setSelectionHighlightRects([]);
    setSelectionToolbarPosition(null);
    setEditingAnnotationId(null);
    setAnnotationType("note");
    setComment("");
    setNoteDialogOpen(true);
    setStatus("블록 코멘트 입력을 시작했습니다.");
  }

  function requestBlockDelete(block: MarkdownBlock) {
    const existingDelete = annotations.find(
      (annotation) => annotation.type === "delete" && annotation.anchor.blockId === block.id,
    );

    if (existingDelete) {
      deleteAnnotation(existingDelete.id);
      setStatus("블록 삭제 annotation을 취소했습니다.");
      return;
    }

    const anchor = blockAnchor(block);
    setAnnotations((current) => [
      ...current,
      {
        id: crypto.randomUUID(),
        fileName: document.fileName,
        anchor,
        selectedText: block.content,
        comment: "Remove this block.",
        type: "delete",
        createdAt: new Date().toISOString(),
      },
    ]);
    setStatus("블록 삭제 annotation을 추가했습니다.");
  }

  function captureSelection() {
    const anchors = getSelectionAnchors(documentPaneRef.current);
    if (anchors.length > 0) {
      const highlightRects = getSelectionRects(documentPaneRef.current);
      const lastRect = highlightRects[highlightRects.length - 1];
      const selectedText =
        window.getSelection()?.toString().trim() ?? anchors.map((anchor) => anchor.selectedText).join("\n");

      setSelection(selectedText);
      setSelectionAnchor(anchors[0] ?? null);
      setSelectionAnchors(anchors);
      setSelectionHighlightRects(highlightRects);
      setSelectionToolbarPosition(
        lastRect ? { left: lastRect.left + lastRect.width + 8, top: lastRect.top } : null,
      );
      setStatus("선택 anchor를 저장했습니다.");
      return;
    }

    setSelectionAnchor(null);
    setSelectionToolbarPosition(null);
    setSelectionHighlightRects([]);
    setSelectionAnchors([]);
  }

  function scheduleCaptureSelection() {
    window.setTimeout(captureSelection, 10);
  }

  useEffect(() => {
    function handleDocumentMouseUp() {
      window.setTimeout(() => {
        const selection = window.getSelection();
        const pane = documentPaneRef.current;
        if (
          !selection ||
          selection.isCollapsed ||
          selection.rangeCount === 0 ||
          !pane ||
          (!selection.anchorNode || !pane.contains(selection.anchorNode)) ||
          (!selection.focusNode || !pane.contains(selection.focusNode))
        ) {
          return;
        }

        captureSelection();
      }, 10);
    }

    globalThis.document.addEventListener("mouseup", handleDocumentMouseUp);
    return () => globalThis.document.removeEventListener("mouseup", handleDocumentMouseUp);
  });

  useEffect(() => {
    const documentPath = getDocumentPathFromWindowQuery();
    if (!documentPath || !isTauriRuntime()) {
      return;
    }

    let cancelled = false;
    void readMarkdownDocument(documentPath)
      .then((opened) => {
        if (!cancelled) {
          loadDocumentIntoWindow(opened, `${opened.fileName} 파일을 열었습니다.`);
        }
      })
      .catch((error) => {
        if (!cancelled) {
          setStatus(error instanceof Error ? error.message : "파일을 열 수 없습니다.");
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    let timeoutId: number | undefined;
    const unlistenPromise = listen(WINDOW_HIGHLIGHT_EVENT, () => {
      setStatus("이미 열린 문서 창을 전면으로 가져왔습니다.");
      if (timeoutId !== undefined) {
        window.clearTimeout(timeoutId);
      }
      timeoutId = window.setTimeout(() => setStatus("문서 창을 사용할 수 있습니다."), 900);
    });

    return () => {
      if (timeoutId !== undefined) {
        window.clearTimeout(timeoutId);
      }
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    if (!isReloadableDocument) {
      return;
    }

    let disposed = false;
    let unlisten: (() => void) | undefined;

    startMarkdownDocumentWatcher(document.absolutePath).catch((error) => {
      console.error("Failed to start markdown document watcher", error);
    });

    listen<MarkdownDocumentChangedEvent>(MARKDOWN_DOCUMENT_CHANGED_EVENT, (event) => {
      if (!disposed && event.payload.path === document.absolutePath) {
        void reloadActiveDocument();
      }
    })
      .then((dispose) => {
        unlisten = dispose;
      })
      .catch((error) => {
        console.error("Failed to listen for markdown document changes", error);
      });

    return () => {
      disposed = true;
      unlisten?.();
      stopMarkdownDocumentWatcher().catch((error) => {
        console.error("Failed to stop markdown document watcher", error);
      });
    };
  }, [document.absolutePath, isReloadableDocument]);

  useEffect(() => {
    if (!isReloadableDocument) {
      return;
    }

    let cancelled = false;
    const intervalId = window.setInterval(() => {
      if (!cancelled && !globalThis.document.hidden) {
        void reloadActiveDocument();
      }
    }, AUTO_REFRESH_INTERVAL_MS);

    return () => {
      cancelled = true;
      window.clearInterval(intervalId);
    };
  }, [document.absolutePath, isReloadableDocument]);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    void checkCliInstalled()
      .then((status) => setCliInstalled(status.installed))
      .catch(() => setCliInstalled(false));
  }, []);

  function handleTocEntrySelect(entry: TocEntry) {
    scrollToBlock(documentPaneRef.current, entry.blockId);
  }

  function requestSelectionNote() {
    if (!selection || activeSelectionAnchors().length === 0) {
      setStatus("텍스트를 선택하세요.");
      return;
    }

    setAnnotationType("note");
    setComment("");
    setEditingAnnotationId(null);
    setSelectionToolbarPosition(null);
    setNoteDialogOpen(true);
    setStatus("선택 영역 노트 입력을 시작했습니다.");
  }

  function requestSelectionDelete() {
    if (!selection || activeSelectionAnchors().length === 0) {
      setStatus("텍스트를 선택하세요.");
      return;
    }

    const nextAnnotations = makeAnnotationsFromSelection("delete", "Remove this selection.");
    setAnnotations((current) => [...current, ...nextAnnotations]);
    resetSelectionState();
    setEditingAnnotationId(null);
    setNoteDialogOpen(false);
    setStatus(
      nextAnnotations.length > 1
        ? `${nextAnnotations.length}개 블록에 선택 영역 삭제 annotation을 추가했습니다.`
        : "선택 영역 삭제 annotation을 추가했습니다.",
    );
  }

  function editInlineAnnotation(annotationId: string) {
    const annotation = annotations.find((candidate) => candidate.id === annotationId);
    if (!annotation || annotation.type !== "note") {
      return;
    }
    const groupAnnotations = annotation.groupId
      ? annotations.filter((candidate) => candidate.groupId === annotation.groupId)
      : [annotation];

    setSelection(groupAnnotations.map((candidate) => candidate.selectedText).join("\n"));
    setSelectionAnchor(groupAnnotations[0]?.anchor ?? annotation.anchor);
    setSelectionAnchors(groupAnnotations.map((candidate) => candidate.anchor));
    setAnnotationType("note");
    setComment(annotation.comment);
    setEditingAnnotationId(annotation.id);
    setSelectionHighlightRects([]);
    setSelectionToolbarPosition(null);
    setNoteDialogOpen(true);
    setStatus("노트 annotation 수정 모드입니다.");
  }

  function closeNoteDialog() {
    setNoteDialogOpen(false);
    setEditingAnnotationId(null);
    setComment("");
    resetSelectionState();
  }

  function saveDialogAnnotation() {
    if (!selection || activeSelectionAnchors().length === 0 || (requiresComment(annotationType) && !comment.trim())) {
      setStatus("Annotation 내용을 입력하세요.");
      return;
    }

    if (editingAnnotationId) {
      const editingAnnotation = annotations.find((annotation) => annotation.id === editingAnnotationId);
      const editingGroupId = editingAnnotation?.groupId;

      setAnnotations((current) =>
        current.map((annotation) =>
          annotation.id === editingAnnotationId || (editingGroupId !== undefined && annotation.groupId === editingGroupId)
            ? {
                ...annotation,
                comment: comment.trim(),
                type: annotationType,
              }
            : annotation,
        ),
      );
      setStatus("Annotation을 수정했습니다.");
    } else {
      const nextAnnotations = makeAnnotationsFromSelection(annotationType, comment.trim());
      setAnnotations((current) => [...current, ...nextAnnotations]);
      setStatus(
        nextAnnotations.length > 1
          ? `${nextAnnotations.length}개 블록에 annotation을 추가했습니다.`
          : "Annotation을 추가했습니다.",
      );
    }

    setComment("");
    resetSelectionState();
    setEditingAnnotationId(null);
    setNoteDialogOpen(false);
  }

  function addAnnotation() {
    if (!selection || activeSelectionAnchors().length === 0 || (requiresComment(annotationType) && !comment.trim())) {
      setStatus("텍스트를 선택하고 comment를 입력하세요.");
      return;
    }

    if (editingAnnotationId) {
      const editingAnnotation = annotations.find((annotation) => annotation.id === editingAnnotationId);
      const editingGroupId = editingAnnotation?.groupId;

      setAnnotations((current) =>
        current.map((annotation) =>
          annotation.id === editingAnnotationId || (editingGroupId !== undefined && annotation.groupId === editingGroupId)
            ? {
                ...annotation,
                comment: comment.trim(),
                type: annotationType,
              }
            : annotation,
        ),
      );
      setComment("");
      resetSelectionState();
      setEditingAnnotationId(null);
      setNoteDialogOpen(false);
      setStatus("Annotation을 수정했습니다.");
      return;
    }

    const nextAnnotations = makeAnnotationsFromSelection(annotationType, comment.trim());
    setAnnotations((current) => [...current, ...nextAnnotations]);
    setComment("");
    resetSelectionState();
    setEditingAnnotationId(null);
    setNoteDialogOpen(false);
    setStatus(
      nextAnnotations.length > 1
        ? `${nextAnnotations.length}개 블록에 annotation을 추가했습니다.`
        : "Annotation을 추가했습니다.",
    );
  }

  function deleteAnnotation(annotationId: string) {
    const target = annotations.find((annotation) => annotation.id === annotationId);
    const targetGroupId = target?.groupId;

    setAnnotations((current) =>
      current.filter((annotation) =>
        targetGroupId ? annotation.groupId !== targetGroupId : annotation.id !== annotationId,
      ),
    );
    if (editingAnnotationId === annotationId) {
      setEditingAnnotationId(null);
      resetSelectionState();
      setComment("");
    }
    setStatus(targetGroupId ? "묶음 annotation을 삭제했습니다." : "Annotation을 삭제했습니다.");
  }

  async function copyExport() {
    await navigator.clipboard.writeText(exportText);
    setStatus("Agent용 Markdown 출력을 클립보드에 복사했습니다.");
  }

  return (
    <main className="flex h-screen min-h-0 flex-col bg-background text-foreground">
      <header className="flex min-h-16 items-center justify-between border-b bg-card px-4">
        <div className="flex min-w-0 items-center gap-3">
          <div className="flex size-9 items-center justify-center rounded-lg bg-muted">
            <FileText aria-hidden="true" />
          </div>
          <div>
            <h1 className="text-base font-semibold">Markdown Annotator</h1>
            <p className="max-w-[56vw] truncate text-sm text-muted-foreground">{title}</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Select value={selectedExampleId} onValueChange={loadExample}>
            <SelectTrigger className="w-[260px]">
              <SelectValue placeholder="예제 선택" />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectLabel>Examples</SelectLabel>
                {exampleMarkdownDocuments.map((example) => (
                  <SelectItem key={example.id} value={example.id}>
                    {example.title}
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>
          <Button type="button" variant="outline" onClick={handleOpenFileAsTab}>
            <FolderOpen data-icon="inline-start" aria-hidden="true" />
            Open
          </Button>
          {isTauriRuntime() ? (
            <Button
              type="button"
              variant={isCliInstalled ? "default" : "outline"}
              onClick={handleInstallCli}
              disabled={isCliInstalling}
            >
              <Terminal data-icon="inline-start" aria-hidden="true" />
              {isCliInstalling ? "Installing" : isCliInstalled ? "CLI installed" : "Install CLI"}
            </Button>
          ) : null}
          <Button type="button" onClick={copyExport}>
            <ClipboardCopy data-icon="inline-start" aria-hidden="true" />
            Copy prompt
          </Button>
        </div>
      </header>

      <section
        className={
          tocEntries.length > 0
            ? "grid min-h-0 flex-1 grid-cols-[auto_minmax(0,1fr)_420px]"
            : "grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_420px]"
        }
      >
        {tocEntries.length > 0 ? (
          <div className="flex min-h-0 border-r bg-card">
            {isTocOpen ? (
              <div className="flex min-h-0 w-64 flex-col">
                <div className="flex items-center justify-between border-b px-3 py-2">
                  <h2 className="text-sm font-medium">Contents</h2>
                  <Button
                    aria-label="Collapse table of contents"
                    size="icon-sm"
                    type="button"
                    variant="ghost"
                    onClick={() => setTocOpen(false)}
                  >
                    <PanelLeftClose aria-hidden="true" />
                  </Button>
                </div>
                <div className="min-h-0 flex-1 overflow-y-auto p-2">
                  <MarkdownToc entries={tocEntries} onEntrySelect={handleTocEntrySelect} />
                </div>
              </div>
            ) : (
              <div className="p-2">
                <Button
                  aria-label="Expand table of contents"
                  size="icon-sm"
                  type="button"
                  variant="ghost"
                  onClick={() => setTocOpen(true)}
                >
                  <PanelLeftOpen aria-hidden="true" />
                </Button>
              </div>
            )}
          </div>
        ) : null}
        <div className="min-h-0 bg-muted/30">
          <ScrollArea className="h-full">
            <div className="relative mx-auto max-w-5xl p-6" ref={documentPaneRef} onMouseUp={scheduleCaptureSelection}>
              <Card>
                <CardHeader>
                  <CardTitle>{title}</CardTitle>
                  <CardDescription>{document.absolutePath}</CardDescription>
                  <CardAction>
                    <div className="flex items-center gap-2">
                      {isDocumentRefreshing ? <Badge variant="outline">Refreshing</Badge> : null}
                      {staleDocument ? <Badge variant="destructive">Stale</Badge> : null}
                      {documentReloadError ? (
                        <Button
                          type="button"
                          size="sm"
                          variant="outline"
                          onClick={() => void reloadActiveDocument({ announceSuccess: true })}
                        >
                          Retry
                        </Button>
                      ) : null}
                      <Badge variant="secondary">{annotations.length} annotations</Badge>
                    </div>
                  </CardAction>
                </CardHeader>
                <CardContent>
                  <MarkdownViewer
                    components={markdownViewerComponents}
                    blocks={blocks}
                    annotatedBlockIds={annotatedBlockIds}
                    deletedBlockIds={deletedBlockIds}
                    inlineAnnotationsByBlock={inlineAnnotationsByBlock}
                    noteAnnotationsByBlock={noteAnnotationsByBlock}
                    onCancelInlineAnnotation={deleteAnnotation}
                    onEditInlineAnnotation={editInlineAnnotation}
                    onRequestBlockComment={requestBlockComment}
                    onRequestBlockDelete={requestBlockDelete}
                  />
                </CardContent>
              </Card>
              {selectionHighlightRects.map((rect, index) => (
                <div
                  aria-hidden="true"
                  className="pointer-events-none absolute bg-yellow-200/50"
                  key={`${rect.left}-${rect.top}-${index}`}
                  style={{
                    height: rect.height,
                    left: rect.left,
                    top: rect.top,
                    width: rect.width,
                  }}
                />
              ))}
              {selectionToolbarPosition ? (
                <div
                  className="absolute flex items-center gap-1 rounded-lg border bg-popover p-1 shadow-sm"
                  style={{
                    left: selectionToolbarPosition.left,
                    top: selectionToolbarPosition.top,
                  }}
                  onMouseDown={(event) => {
                    event.preventDefault();
                    event.stopPropagation();
                  }}
                  onMouseUp={(event) => event.stopPropagation()}
                  onClick={(event) => event.stopPropagation()}
                >
                  <Button
                    aria-label="Delete selected text"
                    size="icon-sm"
                    type="button"
                    variant="ghost"
                    onClick={requestSelectionDelete}
                  >
                    <Trash2 aria-hidden="true" />
                  </Button>
                  <Button
                    aria-label="Add note to selected text"
                    size="icon-sm"
                    type="button"
                    variant="ghost"
                    onClick={requestSelectionNote}
                  >
                    <StickyNote aria-hidden="true" />
                  </Button>
                </div>
              ) : null}
            </div>
          </ScrollArea>
        </div>

        <aside className="min-h-0 border-l bg-card">
          <Tabs defaultValue="annotate" className="h-full gap-0">
            <div className="border-b p-3">
              <TabsList className="grid w-full grid-cols-2">
                <TabsTrigger value="annotate">Annotate</TabsTrigger>
                <TabsTrigger value="prompt">Prompt</TabsTrigger>
              </TabsList>
            </div>

            <TabsContent value="annotate" className="min-h-0 flex-1">
              <ScrollArea className="h-[calc(100vh-8rem)]">
                <div className="flex flex-col gap-4 p-4">
                  <Card>
                    <CardHeader>
                      <CardTitle>Add annotation</CardTitle>
                      <CardDescription>문서에서 텍스트를 선택한 뒤 comment를 입력합니다.</CardDescription>
                    </CardHeader>
                    <CardContent>
                      <FieldGroup>
                        <Field>
                          <FieldLabel>Selected text</FieldLabel>
                          <div className="min-h-16 rounded-lg border bg-muted/50 p-3 text-sm text-muted-foreground">
                            {selection || "문서에서 텍스트를 드래그하세요."}
                          </div>
                          <FieldDescription>
                            {selectionAnchors.length > 0 || selectionAnchor
                              ? "선택 범위가 저장되었습니다."
                              : "선택하면 block anchor와 offset을 내부적으로 저장합니다."}
                          </FieldDescription>
                        </Field>
                        <Field>
                          <FieldLabel>Type</FieldLabel>
                          <Select
                            value={annotationType}
                            onValueChange={(value) => {
                              if (value) {
                                setAnnotationType(value as AnnotationType);
                              }
                            }}
                          >
                            <SelectTrigger className="w-full">
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              <SelectGroup>
                                <SelectLabel>Annotation type</SelectLabel>
                                {annotationTypes.map((type) => (
                                  <SelectItem key={type.value} value={type.value}>
                                    {type.label}
                                  </SelectItem>
                                ))}
                              </SelectGroup>
                            </SelectContent>
                          </Select>
                        </Field>
                        <Field>
                          <FieldLabel htmlFor="annotation-comment">{annotationCommentLabel(annotationType)}</FieldLabel>
                          <Textarea
                            id="annotation-comment"
                            value={comment}
                            onChange={(event) => setComment(event.target.value)}
                            placeholder={annotationCommentPlaceholder(annotationType)}
                            rows={5}
                          />
                        </Field>
                        <Button type="button" onClick={addAnnotation}>
                          <MessageSquarePlus data-icon="inline-start" aria-hidden="true" />
                          {editingAnnotationId ? "Save annotation" : "Add annotation"}
                        </Button>
                      </FieldGroup>
                    </CardContent>
                  </Card>

                  <Separator />

                  <div className="flex items-center justify-between">
                    <h2 className="text-sm font-medium">Annotations</h2>
                    <Badge variant="outline">{visibleAnnotations.length}</Badge>
                  </div>

                  {annotations.length === 0 ? (
                    <Empty>
                      <EmptyHeader>
                        <EmptyMedia variant="icon">
                          <CheckCircle2 aria-hidden="true" />
                        </EmptyMedia>
                        <EmptyTitle>No annotations yet</EmptyTitle>
                        <EmptyDescription>
                          Select text in the document to create structured feedback.
                        </EmptyDescription>
                      </EmptyHeader>
                    </Empty>
                  ) : (
                    visibleAnnotations.map((annotation) => {
                      const groupAnnotations = annotation.groupId
                        ? annotations.filter((candidate) => candidate.groupId === annotation.groupId)
                        : [annotation];
                      const selectedText = groupAnnotations.map((candidate) => candidate.selectedText).join("\n");

                      return (
                        <Card key={annotation.id} size="sm">
                          <CardHeader>
                            <CardTitle className="line-clamp-2 text-sm">{selectedText}</CardTitle>
                            <CardDescription>{annotation.type}</CardDescription>
                            <CardAction>
                              <Button
                                type="button"
                                variant="ghost"
                                size="icon-sm"
                                onClick={() => deleteAnnotation(annotation.id)}
                                aria-label="Delete annotation"
                              >
                                <Trash2 aria-hidden="true" />
                              </Button>
                            </CardAction>
                          </CardHeader>
                          <CardContent className="flex flex-col gap-2">
                            <Badge className="w-fit" variant="secondary">
                              {annotation.type}
                              {groupAnnotations.length > 1 ? ` · ${groupAnnotations.length} blocks` : ""}
                            </Badge>
                            <p className="text-sm text-muted-foreground">{annotation.comment}</p>
                          </CardContent>
                        </Card>
                      );
                    })
                  )}
                </div>
              </ScrollArea>
            </TabsContent>

            <TabsContent value="prompt" className="min-h-0 flex-1">
              <div className="flex h-[calc(100vh-8rem)] flex-col gap-3 p-4">
                <Card size="sm">
                  <CardHeader>
                    <CardTitle className="text-sm">Prompt instruction</CardTitle>
                    <CardDescription>Agent가 annotation을 어떻게 처리할지 지정합니다.</CardDescription>
                  </CardHeader>
                  <CardContent>
                    <FieldGroup>
                      <Field>
                        <FieldLabel htmlFor="prompt-file-path">File path</FieldLabel>
                        <Input
                          id="prompt-file-path"
                          value={promptFilePath}
                          onChange={(event) => setPromptFilePath(event.target.value)}
                          placeholder="/absolute/path/to/document.md"
                        />
                        <FieldDescription>
                          Agent가 파일을 직접 수정해야 한다면 절대경로를 입력하는 것이 안전합니다.
                        </FieldDescription>
                      </Field>
                      <Field>
                        <FieldLabel>Goal</FieldLabel>
                        <Select
                          value={promptGoal}
                          onValueChange={(value) => {
                            if (value) {
                              setPromptGoal(value as AgentPromptGoal);
                            }
                          }}
                        >
                          <SelectTrigger className="w-full">
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectGroup>
                              <SelectLabel>Agent goal</SelectLabel>
                              {promptGoals.map((goal) => (
                                <SelectItem key={goal.value} value={goal.value}>
                                  {goal.label}
                                </SelectItem>
                              ))}
                            </SelectGroup>
                          </SelectContent>
                        </Select>
                        <FieldDescription>
                          {promptGoals.find((goal) => goal.value === promptGoal)?.description}
                        </FieldDescription>
                      </Field>
                      <Field>
                        <FieldLabel htmlFor="prompt-instruction">User instruction</FieldLabel>
                        <Textarea
                          id="prompt-instruction"
                          value={promptInstruction}
                          onChange={(event) => setPromptInstruction(event.target.value)}
                          placeholder="예: change-request는 문서에 반영하고 note는 참고만 하세요."
                          rows={3}
                        />
                      </Field>
                    </FieldGroup>
                  </CardContent>
                </Card>
                <div className="flex items-center justify-between">
                  <div>
                    <h2 className="text-sm font-medium">Agent prompt</h2>
                    <p className="text-sm text-muted-foreground">
                      파일 경로와 사용자 지침을 포함해 agent session에 전달할 Markdown입니다.
                    </p>
                  </div>
                  <Button type="button" size="sm" onClick={copyExport}>
                    <ClipboardCopy data-icon="inline-start" aria-hidden="true" />
                    Copy
                  </Button>
                </div>
                <Textarea className="min-h-0 flex-1 resize-none font-mono text-xs" readOnly value={exportText} />
              </div>
            </TabsContent>
          </Tabs>
        </aside>
      </section>

      <AnnotationInputDialog
        open={noteDialogOpen}
        onOpenChange={(open) => {
          if (open) {
            setNoteDialogOpen(true);
            return;
          }

          closeNoteDialog();
        }}
        isEditing={editingAnnotationId !== null}
        selectedText={selection}
        type={annotationType}
        onTypeChange={setAnnotationType}
        comment={comment}
        onCommentChange={setComment}
        onSubmit={saveDialogAnnotation}
        components={annotationDialogComponents}
      />

      <footer className="border-t bg-card px-4 py-2 text-xs text-muted-foreground">{status}</footer>
    </main>
  );
}
