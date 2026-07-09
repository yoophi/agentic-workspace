import type { WorktreeFileEntry } from "@/entities/worktree-file/model/types";

export type SpeckitDocumentType =
  | "spec"
  | "plan"
  | "tasks"
  | "research"
  | "dataModel"
  | "quickstart"
  | "contract"
  | "checklist"
  | "other";

export type SpeckitDocumentGroup = "core" | "contracts" | "checklists" | "other";

export type TaskProgressState = "noTasks" | "notStarted" | "inProgress" | "complete";

export type SpeckitFeatureStatus = "ready" | "partial" | "error";

export type SpeckitPanelLoadState = "idle" | "loading" | "ready" | "empty" | "error";

export type TaskProgressSummary = {
  total: number;
  completed: number;
  remaining: number;
  state: TaskProgressState;
  sourcePath: string;
};

export type SpeckitDocument = {
  id: string;
  featureId: string;
  type: SpeckitDocumentType;
  label: string;
  relativePath: string;
  group: SpeckitDocumentGroup;
  size: number;
  modifiedMs: number | null;
  readState: "unknown" | "readable" | "unreadable";
  errorMessage?: string;
};

export type SpeckitFeature = {
  id: string;
  name: string;
  relativePath: string;
  documents: SpeckitDocument[];
  taskProgress: TaskProgressSummary | null;
  status: SpeckitFeatureStatus;
};

export type SpeckitPanelState = {
  selectedFeatureId: string | null;
  selectedDocumentPath: string | null;
  loadState: SpeckitPanelLoadState;
  features: SpeckitFeature[];
  errorMessage?: string;
};

export const speckitCoreDocumentOrder: SpeckitDocumentType[] = [
  "spec",
  "plan",
  "tasks",
  "research",
  "dataModel",
  "quickstart",
  "contract",
  "checklist",
  "other",
];

const coreDocumentTypesByName: Record<string, SpeckitDocumentType> = {
  "spec.md": "spec",
  "plan.md": "plan",
  "tasks.md": "tasks",
  "research.md": "research",
  "data-model.md": "dataModel",
  "quickstart.md": "quickstart",
};

const documentLabels: Record<SpeckitDocumentType, string> = {
  spec: "Spec",
  plan: "Plan",
  tasks: "Tasks",
  research: "Research",
  dataModel: "Data model",
  quickstart: "Quickstart",
  contract: "Contract",
  checklist: "Checklist",
  other: "Document",
};

export function buildSpeckitFeatures(
  entries: readonly WorktreeFileEntry[],
  taskContentsByPath: Record<string, string | undefined> = {},
): SpeckitFeature[] {
  const features = new Map<string, SpeckitFeature>();

  for (const entry of entries) {
    if (entry.isDir || !isMarkdownFile(entry.relativePath)) {
      continue;
    }

    const featurePath = getSpeckitFeaturePath(entry.relativePath);
    if (!featurePath) {
      continue;
    }

    const feature = ensureFeature(features, featurePath);
    feature.documents.push(createSpeckitDocument(entry, featurePath));
  }

  return Array.from(features.values())
    .map((feature) => {
      const documents = sortSpeckitDocuments(feature.documents);
      const tasksDocument = documents.find((document) => document.type === "tasks");
      return {
        ...feature,
        documents,
        taskProgress:
          tasksDocument && taskContentsByPath[tasksDocument.relativePath] !== undefined
            ? summarizeTaskProgress(
                taskContentsByPath[tasksDocument.relativePath] ?? "",
                tasksDocument.relativePath,
              )
            : null,
      };
    })
    .filter((feature) => feature.documents.length > 0)
    .sort((left, right) => left.relativePath.localeCompare(right.relativePath));
}

export function classifySpeckitDocument(relativePath: string): {
  type: SpeckitDocumentType;
  group: SpeckitDocumentGroup;
  label: string;
} {
  const normalized = normalizePath(relativePath);
  const segments = normalized.split("/");
  const fileName = (segments[segments.length - 1] ?? "").toLowerCase();
  const afterFeature = segments.slice(2);

  if (afterFeature[0] === "contracts") {
    return { type: "contract", group: "contracts", label: documentLabels.contract };
  }
  if (afterFeature[0] === "checklists") {
    return { type: "checklist", group: "checklists", label: documentLabels.checklist };
  }

  const type = coreDocumentTypesByName[fileName] ?? "other";
  return {
    type,
    group: type === "other" ? "other" : "core",
    label: documentLabels[type],
  };
}

export function summarizeTaskProgress(content: string, sourcePath: string): TaskProgressSummary {
  const matches = Array.from(content.matchAll(/^\s*[-*]\s+\[([ xX])\]\s+/gm));
  const total = matches.length;
  const completed = matches.filter((match) => match[1]?.toLowerCase() === "x").length;
  const remaining = total - completed;

  return {
    total,
    completed,
    remaining,
    state: getTaskProgressState(total, completed),
    sourcePath,
  };
}

export function getTaskProgressState(total: number, completed: number): TaskProgressState {
  if (total === 0) {
    return "noTasks";
  }
  if (completed === 0) {
    return "notStarted";
  }
  if (completed === total) {
    return "complete";
  }
  return "inProgress";
}

export function getTaskDocumentPaths(features: readonly SpeckitFeature[]) {
  return features
    .flatMap((feature) => feature.documents)
    .filter((document) => document.type === "tasks")
    .map((document) => document.relativePath);
}

export function isSpeckitEmpty(features: readonly SpeckitFeature[]) {
  return features.length === 0;
}

function ensureFeature(features: Map<string, SpeckitFeature>, featurePath: string) {
  const existing = features.get(featurePath);
  if (existing) {
    return existing;
  }

  const feature: SpeckitFeature = {
    id: featurePath,
    name: featurePath.split("/").pop() ?? featurePath,
    relativePath: featurePath,
    documents: [],
    taskProgress: null,
    status: "ready",
  };
  features.set(featurePath, feature);
  return feature;
}

function createSpeckitDocument(entry: WorktreeFileEntry, featurePath: string): SpeckitDocument {
  const classification = classifySpeckitDocument(entry.relativePath);
  return {
    id: entry.relativePath,
    featureId: featurePath,
    type: classification.type,
    label: classification.label,
    relativePath: entry.relativePath,
    group: classification.group,
    size: entry.size,
    modifiedMs: entry.modifiedMs ?? null,
    readState: "unknown",
  };
}

function sortSpeckitDocuments(documents: SpeckitDocument[]) {
  return [...documents].sort((left, right) => {
    const groupOrder = groupRank(left.group) - groupRank(right.group);
    if (groupOrder !== 0) {
      return groupOrder;
    }

    const typeOrder =
      speckitCoreDocumentOrder.indexOf(left.type) -
      speckitCoreDocumentOrder.indexOf(right.type);
    if (typeOrder !== 0) {
      return typeOrder;
    }

    return left.relativePath.localeCompare(right.relativePath);
  });
}

function groupRank(group: SpeckitDocumentGroup) {
  switch (group) {
    case "core":
      return 0;
    case "contracts":
      return 1;
    case "checklists":
      return 2;
    case "other":
      return 3;
  }
}

function getSpeckitFeaturePath(relativePath: string) {
  const normalized = normalizePath(relativePath);
  const segments = normalized.split("/");
  if (segments.length < 3 || segments[0] !== "specs" || !segments[1]) {
    return null;
  }
  return `specs/${segments[1]}`;
}

function isMarkdownFile(path: string) {
  const lowered = path.toLowerCase();
  return lowered.endsWith(".md") || lowered.endsWith(".markdown") || lowered.endsWith(".mdx");
}

function normalizePath(path: string) {
  return path.replace(/\\/g, "/").replace(/^\/+/, "");
}
