import { CheckCircle2Icon, CircleIcon, PlayIcon } from "lucide-react";
import { useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { ActiveFeaturePointer, SddActionRequest, SddStageState } from "@/features/worktree-workspace/model/sdd-workflow";
import { createSddAction } from "@/features/worktree-workspace/model/sdd-workflow";

export function SddWorkflowControls({ pointer, stages, onRequest }: { pointer: ActiveFeaturePointer; stages: SddStageState[]; onRequest: (request: SddActionRequest) => void }) {
  const [confirming, setConfirming] = useState<SddStageState | null>(null);
  function request(stage: SddStageState) {
    if (!stage.canStart) return;
    if (stage.requiresConfirmation && pointer.status === "active") { setConfirming(stage); return; }
    onRequest(createSddAction(stage.stage, pointer));
  }
  return <section className="border-b p-3" aria-label="SDD 작업 단계">
    <div className="mb-2 flex items-center justify-between gap-2"><div><h2 className="text-sm font-medium">SDD workflow</h2><p className="text-xs text-muted-foreground">{pointer.status === "active" ? pointer.featurePath : pointer.status === "loading" ? "활성 기능 확인 중" : pointer.reason}</p></div>{pointer.status === "active" ? <Badge>현재 작업 중</Badge> : null}</div>
    <div className="grid gap-1">{stages.map((stage) => <div className={cn("flex items-center justify-between gap-2 rounded-md border p-2", stage.status === "complete" && "border-green-600/30 bg-green-500/10", stage.status === "current" && "border-blue-600/30 bg-blue-500/10", stage.status === "pending" && "bg-muted/40", stage.status === "unavailable" && "border-destructive/30 bg-destructive/5")} key={stage.stage}>
      <span className="flex items-center gap-2 text-sm">{stage.status === "complete" ? <CheckCircle2Icon className="size-4 text-green-600" /> : <CircleIcon className="size-4 text-muted-foreground" />}<span className="capitalize">{stage.stage}</span><span className="text-xs text-muted-foreground">{stage.status}</span></span>
      <Button type="button" size="sm" disabled={!stage.canStart} title={stage.blockedReason} onClick={() => request(stage)}><PlayIcon data-icon="inline-start" />{stage.stage === "specify" && pointer.status !== "active" ? "초안 주입" : "시작"}</Button>
    </div>)}</div>
    {confirming ? <div className="mt-2 rounded-md border bg-muted/30 p-2 text-sm"><p>{confirming.stage} 단계는 기존 산출물 또는 검토 결과에 영향을 줄 수 있습니다. 계속할까요?</p><div className="mt-2 flex gap-2"><Button size="sm" variant="outline" onClick={() => setConfirming(null)}>취소</Button><Button size="sm" onClick={() => { onRequest(createSddAction(confirming.stage, pointer)); setConfirming(null); }}>계속</Button></div></div> : null}
  </section>;
}
