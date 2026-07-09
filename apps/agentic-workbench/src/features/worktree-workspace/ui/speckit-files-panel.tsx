import {
  AlertCircleIcon,
  ArrowDownAZIcon,
  ArrowDownUpIcon,
  CheckCircle2Icon,
  ChevronDownIcon,
  ChevronRightIcon,
  CircleIcon,
  ClipboardListIcon,
  FoldVerticalIcon,
  FileTextIcon,
  FolderKanbanIcon,
  Loader2Icon,
  RefreshCwIcon,
  UnfoldVerticalIcon,
} from "lucide-react";
import { useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type {
  SpeckitDocument,
  SpeckitFeature,
  TaskProgressSummary,
} from "@/features/worktree-workspace/model/speckit-files";
import { cn } from "@/lib/utils";

export type SpeckitFilesPanelProps = {
  features: SpeckitFeature[];
  selectedDocumentPath: string | null;
  loading?: boolean;
  refreshing?: boolean;
  errorMessage?: string;
  staleDocumentPath?: string | null;
  initialExpandedFeatureIds?: string[];
  onSelectDocument: (path: string) => void;
  onRefresh: () => void;
};

export function SpeckitFilesPanel({
  features,
  selectedDocumentPath,
  loading = false,
  refreshing = false,
  errorMessage,
  staleDocumentPath,
  initialExpandedFeatureIds = [],
  onSelectDocument,
  onRefresh,
}: SpeckitFilesPanelProps) {
  const [expandedFeatureIds, setExpandedFeatureIds] = useState<Set<string>>(
    () => new Set(initialExpandedFeatureIds),
  );
  const [sortMode, setSortMode] = useState<"path" | "task">("path");
  const [sortDirection, setSortDirection] = useState<"asc" | "desc">("desc");
  const sortedFeatures = [...features].sort((left, right) => {
    let result: number;
    if (sortMode === "task") {
      const taskOrder = taskStateRank(left) - taskStateRank(right);
      if (taskOrder !== 0) {
        result = taskOrder;
        return sortDirection === "asc" ? result : -result;
      }
    }

    result = left.relativePath.localeCompare(right.relativePath);
    return sortDirection === "asc" ? result : -result;
  });

  function toggleFeature(featureId: string) {
    setExpandedFeatureIds((current) => {
      const next = new Set(current);
      if (next.has(featureId)) {
        next.delete(featureId);
      } else {
        next.add(featureId);
      }
      return next;
    });
  }

  function collapseAllFeatures() {
    setExpandedFeatureIds(new Set());
  }

  function expandAllFeatures() {
    setExpandedFeatureIds(new Set(features.map((feature) => feature.id)));
  }

  const allFeaturesExpanded =
    features.length > 0 && features.every((feature) => expandedFeatureIds.has(feature.id));

  return (
    <div className="flex h-full min-h-0 flex-col border-r">
      <header className="flex shrink-0 items-center justify-between gap-2 border-b px-4 py-3">
        <div className="flex min-w-0 items-center gap-2">
          <FolderKanbanIcon className="size-4 shrink-0 text-muted-foreground" />
          <div className="min-w-0">
            <h2 className="truncate text-sm font-medium">Speckit files</h2>
            <p className="truncate text-xs text-muted-foreground">
              {features.length} features
            </p>
            <div className="mt-1 flex items-center gap-1.5">
              {refreshing && !loading ? <Badge variant="secondary">Refreshing</Badge> : null}
              {staleDocumentPath ? <Badge variant="destructive">Stale document</Badge> : null}
              {errorMessage ? <Badge variant="destructive">Error</Badge> : null}
            </div>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          <div className="flex rounded-md border p-0.5" aria-label="Speckit 정렬 방식">
            <Button
              type="button"
              size="sm"
              variant={sortMode === "path" ? "secondary" : "ghost"}
              aria-pressed={sortMode === "path"}
              onClick={() => setSortMode("path")}
            >
              <ArrowDownAZIcon data-icon="inline-start" />
              Path
            </Button>
            <Button
              type="button"
              size="sm"
              variant={sortMode === "task" ? "secondary" : "ghost"}
              aria-pressed={sortMode === "task"}
              onClick={() => setSortMode("task")}
            >
              <ClipboardListIcon data-icon="inline-start" />
              Task
            </Button>
          </div>
          <Button
            type="button"
            size="icon-sm"
            variant="ghost"
            aria-label={
              sortDirection === "asc"
                ? "Speckit 정렬을 역순으로 변경"
                : "Speckit 정렬을 정순으로 변경"
            }
            aria-pressed={sortDirection === "desc"}
            onClick={() => setSortDirection((current) => (current === "asc" ? "desc" : "asc"))}
          >
            <ArrowDownUpIcon className={cn(sortDirection === "desc" && "rotate-180")} />
          </Button>
          <Button
            type="button"
            size="icon-sm"
            variant="ghost"
            aria-label={allFeaturesExpanded ? "모든 Speckit feature 접기" : "모든 Speckit feature 펼치기"}
            disabled={features.length === 0}
            onClick={allFeaturesExpanded ? collapseAllFeatures : expandAllFeatures}
          >
            {allFeaturesExpanded ? <FoldVerticalIcon /> : <UnfoldVerticalIcon />}
          </Button>
          <Button
            type="button"
            size="icon-sm"
            variant="ghost"
            aria-label="Speckit files 새로고침"
            disabled={loading || refreshing}
            onClick={onRefresh}
          >
            <RefreshCwIcon className={cn((loading || refreshing) && "animate-spin")} />
          </Button>
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-auto p-2">
        {loading ? (
          <SpeckitInlineState icon={Loader2Icon} title="Speckit 문서를 불러오는 중입니다." spinning />
        ) : errorMessage ? (
          <SpeckitInlineState
            icon={AlertCircleIcon}
            title="Speckit 문서를 불러오지 못했습니다."
            description={errorMessage}
            variant="destructive"
          />
        ) : features.length === 0 ? (
          <SpeckitEmptyState />
        ) : (
          <div className="grid gap-2">
            {sortedFeatures.map((feature) => (
              <SpeckitFeatureSection
                expanded={expandedFeatureIds.has(feature.id)}
                feature={feature}
                key={feature.id}
                onSelectDocument={onSelectDocument}
                onToggleFeature={toggleFeature}
                selectedDocumentPath={selectedDocumentPath}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function taskStateRank(feature: SpeckitFeature) {
  switch (feature.taskProgress?.state) {
    case "inProgress":
      return 0;
    case "notStarted":
      return 1;
    case "noTasks":
      return 2;
    case "complete":
      return 3;
    default:
      return 4;
  }
}

function SpeckitFeatureSection({
  expanded,
  feature,
  selectedDocumentPath,
  onSelectDocument,
  onToggleFeature,
}: {
  expanded: boolean;
  feature: SpeckitFeature;
  selectedDocumentPath: string | null;
  onSelectDocument: (path: string) => void;
  onToggleFeature: (featureId: string) => void;
}) {
  const ToggleIcon = expanded ? ChevronDownIcon : ChevronRightIcon;
  const isComplete = feature.taskProgress?.state === "complete";

  return (
    <section
      className={cn(
        "rounded-md border bg-background",
        isComplete && "border-green-500/40 bg-green-50/70",
      )}
      data-complete={isComplete}
    >
      <header className={cn("px-3 py-2", expanded && "border-b")}>
        <button
          type="button"
          aria-expanded={expanded}
          className="grid w-full min-w-0 grid-cols-[auto_minmax(0,1fr)] items-start gap-2 rounded-sm text-left"
          onClick={() => onToggleFeature(feature.id)}
        >
          <ToggleIcon className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
          <div className="min-w-0">
            <h3 className="truncate text-sm font-medium">{feature.name}</h3>
            <div className="mt-2 flex flex-wrap items-center gap-2">
              <RequiredDocumentStatus feature={feature} />
              <TaskProgressBadge progress={feature.taskProgress} />
            </div>
          </div>
        </button>
      </header>
      {expanded ? (
        <div className="grid py-1">
          {getSpeckitDocumentGroups(feature.documents).map((group) =>
            group.kind === "planArtifacts" ? (
              <div
                className="ml-6 grid border-l border-border pl-1"
                key="plan-artifacts"
              >
                {group.documents.map((document) => (
                  <SpeckitDocumentButton
                    document={document}
                    key={document.id}
                    nestedUnderPlan
                    onSelectDocument={onSelectDocument}
                    selected={document.relativePath === selectedDocumentPath}
                  />
                ))}
              </div>
            ) : (
              <SpeckitDocumentButton
                document={group.document}
                key={group.document.id}
                onSelectDocument={onSelectDocument}
                selected={group.document.relativePath === selectedDocumentPath}
              />
            ),
          )}
        </div>
      ) : null}
    </section>
  );
}

function RequiredDocumentStatus({ feature }: { feature: SpeckitFeature }) {
  const documentTypes = new Set(feature.documents.map((document) => document.type));
  const statuses = [
    ["spec", "Spec"],
    ["plan", "Plan"],
    ["tasks", "Tasks"],
  ] as const;

  return (
    <div className="flex shrink-0 flex-wrap items-center gap-1">
      {statuses.map(([type, label]) => {
        const exists = documentTypes.has(type);
        return (
          <Badge
            className={cn(
              exists
                ? "border-foreground bg-foreground text-background"
                : "border-border bg-background text-muted-foreground",
            )}
            key={type}
            variant="outline"
          >
            {label}
          </Badge>
        );
      })}
    </div>
  );
}

function SpeckitDocumentButton({
  document,
  nestedUnderPlan = false,
  selected,
  onSelectDocument,
}: {
  document: SpeckitDocument;
  nestedUnderPlan?: boolean;
  selected: boolean;
  onSelectDocument: (path: string) => void;
}) {
  return (
    <button
      type="button"
      className={cn(
        "grid min-h-8 min-w-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2 rounded-sm px-2 py-1 text-left text-sm hover:bg-muted data-[selected=true]:bg-muted",
        nestedUnderPlan && "pl-3",
      )}
      data-document-path={document.relativePath}
      data-plan-artifact={nestedUnderPlan}
      data-selected={selected}
      onClick={() => onSelectDocument(document.relativePath)}
    >
      <FileTextIcon className="size-4 shrink-0 text-muted-foreground" />
      <span className="min-w-0">
        <span className="flex min-w-0 items-center gap-1.5">
          <Badge variant={document.group === "core" || nestedUnderPlan ? "secondary" : "outline"}>
            {document.label}
          </Badge>
          <span className="truncate font-mono text-xs text-muted-foreground">
            {document.relativePath}
          </span>
        </span>
      </span>
      {document.errorMessage ? (
        <AlertCircleIcon className="size-4 shrink-0 text-destructive" />
      ) : null}
    </button>
  );
}

type SpeckitDocumentRenderGroup =
  | {
      kind: "document";
      document: SpeckitDocument;
    }
  | {
      kind: "planArtifacts";
      documents: SpeckitDocument[];
    };

function getSpeckitDocumentGroups(documents: SpeckitDocument[]): SpeckitDocumentRenderGroup[] {
  const specDocuments = documents.filter((document) => document.type === "spec");
  const planDocuments = documents.filter((document) => document.type === "plan");
  const planArtifactDocuments = documents
    .filter(isPlanArtifactDocument)
    .sort((left, right) => {
      const rankOrder = planArtifactRank(left) - planArtifactRank(right);
      if (rankOrder !== 0) {
        return rankOrder;
      }
      return left.relativePath.localeCompare(right.relativePath);
    });
  const taskDocuments = documents.filter((document) => document.type === "tasks");
  const remainingDocuments = documents.filter(
    (document) =>
      document.type !== "spec" &&
      document.type !== "plan" &&
      document.type !== "tasks" &&
      !isPlanArtifactDocument(document),
  );

  const groups: SpeckitDocumentRenderGroup[] = [
    ...specDocuments.map((document) => ({ kind: "document" as const, document })),
    ...planDocuments.map((document) => ({ kind: "document" as const, document })),
  ];
  if (planArtifactDocuments.length > 0) {
    groups.push({ kind: "planArtifacts", documents: planArtifactDocuments });
  }
  groups.push(
    ...taskDocuments.map((document) => ({ kind: "document" as const, document })),
    ...remainingDocuments.map((document) => ({ kind: "document" as const, document })),
  );
  return groups;
}

function isPlanArtifactDocument(document: SpeckitDocument) {
  return (
    document.type === "research" ||
    document.type === "dataModel" ||
    document.type === "contract" ||
    document.type === "quickstart"
  );
}

function planArtifactRank(document: SpeckitDocument) {
  switch (document.type) {
    case "research":
      return 0;
    case "dataModel":
      return 1;
    case "contract":
      return 2;
    case "quickstart":
      return 3;
    default:
      return 4;
  }
}

function TaskProgressBadge({ progress }: { progress: TaskProgressSummary | null }) {
  if (!progress) {
    return (
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        <ClipboardListIcon className="size-3.5" />
        <span>No tasks.md</span>
      </div>
    );
  }

  if (progress.state === "noTasks") {
    return (
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        <CircleIcon className="size-3.5" />
        <span>No checkbox tasks</span>
      </div>
    );
  }

  const label = `${progress.completed}/${progress.total} done · ${progress.remaining} remaining`;
  return (
    <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
      <CheckCircle2Icon
        className={cn(
          "size-3.5",
          progress.state === "complete" ? "text-green-600" : "text-muted-foreground",
        )}
      />
      <span>{label}</span>
    </div>
  );
}

function SpeckitEmptyState() {
  return (
    <div className="flex min-h-56 items-center justify-center rounded-md border border-dashed bg-muted/20 p-4 text-center">
      <div className="max-w-sm">
        <p className="text-sm font-medium">Speckit 문서 없음</p>
        <p className="mt-1 text-sm leading-6 text-muted-foreground">
          현재 worktree에서 표시할 `specs/*` markdown 산출물을 찾지 못했습니다.
        </p>
      </div>
    </div>
  );
}

function SpeckitInlineState({
  icon: Icon,
  title,
  description,
  spinning = false,
  variant = "muted",
}: {
  icon: typeof AlertCircleIcon;
  title: string;
  description?: string;
  spinning?: boolean;
  variant?: "muted" | "destructive";
}) {
  return (
    <div
      className={cn(
        "flex min-h-40 items-center justify-center rounded-md border border-dashed p-4 text-center",
        variant === "destructive"
          ? "border-destructive/30 bg-destructive/5 text-destructive"
          : "bg-muted/20",
      )}
    >
      <div className="max-w-sm">
        <Icon
          className={cn(
            "mx-auto size-5",
            spinning && "animate-spin",
            variant === "muted" && "text-muted-foreground",
          )}
        />
        <p className="mt-2 text-sm font-medium">{title}</p>
        {description ? (
          <p className="mt-1 break-words text-sm leading-6 text-muted-foreground">
            {description}
          </p>
        ) : null}
      </div>
    </div>
  );
}
