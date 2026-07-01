import type { AgentTarget, QuickPromptContext } from "@yoophi/markdown-annotation-core";
import { formatQuickPromptForAgent, isQuickPromptSendable } from "@yoophi/markdown-annotation-core";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Textarea } from "@/components/ui/textarea";

export type QuickPromptDialogStatus = "editing" | "sending" | "sent" | "failed";

export type QuickPromptDialogProps = {
  open: boolean;
  context: QuickPromptContext | null;
  promptText: string;
  onPromptTextChange: (value: string) => void;
  onOpenChange: (open: boolean) => void;
  target: AgentTarget;
  status?: QuickPromptDialogStatus;
  errorMessage?: string | null;
  onSend?: (formattedPrompt: string, context: QuickPromptContext) => void | Promise<void>;
};

function formatRange(context: QuickPromptContext) {
  if (context.scope.kind === "document") {
    return context.scope.endLine ? `전체 문서 · ${context.scope.endLine} lines` : "전체 문서";
  }

  if (context.scope.startLine === undefined) {
    return context.scope.label;
  }

  if (context.scope.endLine === undefined || context.scope.startLine === context.scope.endLine) {
    return `${context.scope.label} · line ${context.scope.startLine}`;
  }

  return `${context.scope.label} · lines ${context.scope.startLine}-${context.scope.endLine}`;
}

export function QuickPromptDialog({
  open,
  context,
  promptText,
  onPromptTextChange,
  onOpenChange,
  target,
  status = "editing",
  errorMessage,
  onSend,
}: QuickPromptDialogProps) {
  const targetAvailable = target.availability === "available";
  const canSend = isQuickPromptSendable({ promptText, context, targetAvailable }) && status !== "sending";
  const blockedReason = !context
    ? "첨부 컨텍스트가 없습니다."
    : !promptText.trim()
      ? "프롬프트 내용을 입력하세요."
      : !targetAvailable
        ? target.unavailableReason ?? "Agent target is unavailable."
        : null;

  async function handleSend() {
    if (!context || !canSend) {
      return;
    }

    await onSend?.(formatQuickPromptForAgent({ promptText, context }), context);
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Quick prompt</DialogTitle>
          <DialogDescription>첨부 컨텍스트를 확인하고 agent에게 보낼 프롬프트를 작성합니다.</DialogDescription>
        </DialogHeader>

        <div className="grid gap-4">
          {context ? (
            <section className="grid gap-2 rounded-lg border bg-muted/30 p-3" aria-label="Quick prompt context">
              <div className="flex flex-wrap items-center gap-2">
                <Badge variant="secondary">{context.scope.label}</Badge>
                <Badge variant={context.lengthState === "complete" ? "outline" : "destructive"}>
                  {context.lengthState === "complete"
                    ? `${context.includedLength} chars`
                    : `${context.includedLength}/${context.originalLength} chars`}
                </Badge>
                <span className="text-xs text-muted-foreground">{formatRange(context)}</span>
              </div>
              <p className="text-xs text-muted-foreground">
                {context.documentPath || context.sourceLabel} · {context.documentRevisionLabel}
              </p>
              {context.lengthState === "reduced" ? (
                <p className="text-xs text-destructive">{context.reductionReason}</p>
              ) : null}
              <pre className="max-h-44 overflow-auto whitespace-pre-wrap rounded-md bg-background p-3 text-xs leading-5">
                <code>{context.content}</code>
              </pre>
            </section>
          ) : (
            <div className="rounded-lg border bg-muted/30 p-3 text-sm text-muted-foreground">
              첨부할 수 있는 문서 컨텍스트가 없습니다.
            </div>
          )}

          <div className="grid gap-2">
            <label className="text-sm font-medium" htmlFor="quick-prompt-text">
              Prompt
            </label>
            <Textarea
              id="quick-prompt-text"
              autoFocus
              rows={5}
              value={promptText}
              onChange={(event) => onPromptTextChange(event.target.value)}
              placeholder="이 컨텍스트를 바탕으로 agent에게 요청할 내용을 입력하세요."
            />
          </div>

          <div className="rounded-lg border p-3 text-sm">
            <div className="flex items-center justify-between gap-3">
              <span className="font-medium">{target.label}</span>
              <Badge variant={targetAvailable ? "default" : target.availability === "busy" ? "outline" : "destructive"}>
                {target.availability}
              </Badge>
            </div>
            {blockedReason ? <p className="mt-2 text-xs text-muted-foreground">{blockedReason}</p> : null}
            {status === "sent" ? <p className="mt-2 text-xs text-muted-foreground">Prompt sent.</p> : null}
            {status === "failed" && errorMessage ? <p className="mt-2 text-xs text-destructive">{errorMessage}</p> : null}
          </div>
        </div>

        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button type="button" disabled={!canSend} onClick={() => void handleSend()}>
            {status === "sending" ? "Sending" : "Send"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
