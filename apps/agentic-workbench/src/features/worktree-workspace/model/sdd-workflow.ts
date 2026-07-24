import type { SpeckitFeature } from "@/features/worktree-workspace/model/speckit-files";

export type SddStage = "specify" | "plan" | "tasks" | "implement";
export type SddDelivery = "queue" | "draft";
export type SddStageStatus = "complete" | "current" | "pending" | "unavailable";

export type ActiveFeaturePointer =
  | { status: "loading"; featurePath: null; reason?: string }
  | { status: "active"; featurePath: string; reason?: string }
  | { status: "unavailable" | "error"; featurePath: null; reason: string };

export type SddStageState = {
  stage: SddStage;
  status: SddStageStatus;
  canStart: boolean;
  requiresConfirmation: boolean;
  blockedReason?: string;
};

export type SddActionRequest = { text: string; delivery: SddDelivery };

const stageFiles: Record<Exclude<SddStage, "implement">, string> = {
  specify: "spec.md",
  plan: "plan.md",
  tasks: "tasks.md",
};

export function readActiveFeaturePointer(
  content: string | undefined,
  features: readonly SpeckitFeature[],
  loading = false,
): ActiveFeaturePointer {
  if (loading) return { status: "loading", featurePath: null };
  if (!content) return { status: "unavailable", featurePath: null, reason: "활성 기능이 설정되지 않았습니다." };
  try {
    const value: unknown = JSON.parse(content);
    const path = typeof value === "object" && value !== null && "feature_directory" in value
      ? (value as { feature_directory?: unknown }).feature_directory
      : undefined;
    if (typeof path !== "string" || !isFeaturePath(path)) {
      return { status: "unavailable", featurePath: null, reason: "활성 기능 경로가 유효하지 않습니다." };
    }
    if (!features.some((feature) => feature.relativePath === path)) {
      return { status: "unavailable", featurePath: null, reason: "활성 기능이 현재 Speckit 목록에 없습니다." };
    }
    return { status: "active", featurePath: path };
  } catch {
    return { status: "error", featurePath: null, reason: "활성 기능 정보를 읽을 수 없습니다." };
  }
}

export function getSddStageStates(feature: SpeckitFeature | undefined, pointer: ActiveFeaturePointer): SddStageState[] {
  if (pointer.status !== "active" || !feature) {
    return (["specify", "plan", "tasks", "implement"] as SddStage[]).map((stage) => ({
      stage, status: "unavailable", canStart: stage === "specify", requiresConfirmation: false,
      blockedReason: "활성 SDD 기능을 설정한 뒤 진행할 수 있습니다.",
    }));
  }
  const names = new Set(feature.documents.map((document) => {
    const parts = document.relativePath.split("/");
    return parts[parts.length - 1];
  }));
  const spec = names.has(stageFiles.specify);
  const plan = names.has(stageFiles.plan);
  const tasks = names.has(stageFiles.tasks);
  return [
    { stage: "specify", status: spec ? "complete" : "current", canStart: true, requiresConfirmation: spec },
    { stage: "plan", status: plan ? "complete" : spec ? "current" : "pending", canStart: spec, requiresConfirmation: true, blockedReason: spec ? undefined : "spec.md가 필요합니다." },
    { stage: "tasks", status: tasks ? "complete" : plan ? "current" : "pending", canStart: plan, requiresConfirmation: true, blockedReason: plan ? undefined : "plan.md가 필요합니다." },
    { stage: "implement", status: feature.taskProgress?.state === "complete" ? "complete" : tasks ? "current" : "pending", canStart: tasks, requiresConfirmation: Boolean(tasks), blockedReason: tasks ? undefined : "tasks.md가 필요합니다." },
  ];
}

export function createSddAction(stage: SddStage, pointer: ActiveFeaturePointer): SddActionRequest {
  if (pointer.status !== "active") {
    return { delivery: "draft", text: "현재 SDD 활성 기능이 설정되지 않았습니다. 기능 설명을 확인한 뒤 $speckit-specify 로 새 spec을 생성하고 .specify/feature.json의 feature_directory를 설정해 주세요." };
  }
  return { delivery: "queue", text: `$speckit-${stage}\n\n현재 활성 기능: ${pointer.featurePath}` };
}

function isFeaturePath(path: string) {
  const parts = path.split("/");
  return parts.length === 2 && parts[0] === "specs" && Boolean(parts[1]) && !path.startsWith("/") && !parts.includes("..");
}
