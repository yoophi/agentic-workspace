import { useMemo, useRef, useState } from "react";
import { SendIcon } from "lucide-react";

import type { PromptTargetMode } from "@/entities/agent-orchestration";
import {
  checkPromptSize,
  describePromptSizeViolation,
  parseOrchestrationError,
} from "@/entities/agent-orchestration";
import type { AgentRunPanelSlot } from "@/features/agent-run/model/agent-run-panel-slots";
import type { ComposerNotice } from "@/features/agent-run/model/composer-submission";
import {
  decideComposerSubmission,
  noticeForFailure,
} from "@/features/agent-run/model/composer-submission";
import { selectPromptTargets } from "@/features/agent-run/model/prompt-target-selection";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";

type WorkspacePromptComposerProps = {
  slots: AgentRunPanelSlot[];
  focusedPanelId: string;
  disabled?: boolean;
  onSubmit: (input: {
    requestId: string;
    message: string;
    mode: PromptTargetMode;
    panelIds: string[];
    delegate: boolean;
  }) => void | Promise<void>;
};

const modes: Array<{ value: PromptTargetMode; label: string }> = [
  { value: "focused", label: "포커스" },
  { value: "selected", label: "선택" },
  { value: "all", label: "전체" },
  { value: "coordinator", label: "Main 위임" },
];

export function WorkspacePromptComposer({
  slots,
  focusedPanelId,
  disabled = false,
  onSubmit,
}: WorkspacePromptComposerProps) {
  const [message, setMessage] = useState("");
  const [mode, setMode] = useState<PromptTargetMode>("focused");
  const [selectedPanelIds, setSelectedPanelIds] = useState<string[]>([]);
  const [notice, setNotice] = useState<ComposerNotice | null>(null);
  const composerRef = useRef<HTMLTextAreaElement | null>(null);
  const targets = useMemo(
    () => selectPromptTargets(slots, { mode, focusedPanelId, selectedPanelIds }),
    [focusedPanelId, mode, selectedPanelIds, slots],
  );
  const size = useMemo(() => checkPromptSize(message.trim()), [message]);
  const canSubmit = !disabled && Boolean(message.trim()) && targets.valid && size.ok;

  async function submit() {
    const decision = decideComposerSubmission({
      message,
      disabled,
      targetsValid: targets.valid,
    });
    if (decision.kind === "ignored") return;
    if (decision.kind === "blocked") {
      // FR-044: refused locally, so no target is contacted and the text is kept.
      setNotice(decision.notice);
      composerRef.current?.focus();
      return;
    }
    const input = {
      requestId: crypto.randomUUID(),
      message: decision.message,
      mode,
      panelIds: targets.panelIds,
      delegate: targets.delegate,
    };
    setNotice(null);
    setMessage("");
    try {
      await onSubmit(input);
    } catch (error) {
      // FR-022/FR-048: show a distinguishable reason with the next action, and keep the
      // text so a rejected request costs the user nothing.
      setNotice(noticeForFailure(parseOrchestrationError(error)));
      setMessage(decision.message);
    }
    requestAnimationFrame(() => composerRef.current?.focus());
  }

  return (
    <section className="shrink-0 border-t bg-background" aria-label="Workspace prompt composer">
      <div className="flex flex-wrap items-center gap-1.5 border-b px-3 py-1.5">
        <span className="mr-1 text-xs font-medium text-muted-foreground">보낼 대상</span>
        <div role="radiogroup" aria-label="Prompt 대상 모드" className="flex gap-1">
          {modes.map((candidate) => (
            <Button
              key={candidate.value}
              type="button"
              size="sm"
              variant={mode === candidate.value ? "default" : "ghost"}
              role="radio"
              aria-checked={mode === candidate.value}
              onClick={() => setMode(candidate.value)}
            >
              {candidate.label}
            </Button>
          ))}
        </div>
        {mode === "selected" && (
          <div className="flex flex-wrap gap-2" aria-label="선택할 에이전트 패널">
            {slots.map((slot) => (
              <label key={slot.id} className="flex items-center gap-1 text-xs">
                <input
                  type="checkbox"
                  checked={selectedPanelIds.includes(slot.id)}
                  onChange={(event) =>
                    setSelectedPanelIds((current) =>
                      event.target.checked
                        ? [...current, slot.id]
                        : current.filter((panelId) => panelId !== slot.id),
                    )
                  }
                />
                {slot.title}
              </label>
            ))}
          </div>
        )}
      </div>
      <div className="flex items-end gap-2 p-3">
        <Textarea
          ref={composerRef}
          value={message}
          onChange={(event) => setMessage(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
              event.preventDefault();
              void submit();
            }
          }}
          aria-label="모든 에이전트 패널의 공용 prompt"
          placeholder="명령을 입력하세요. ⌘/Ctrl+Enter로 전송"
          className="min-h-16 resize-y"
        />
        <Button
          type="button"
          disabled={!canSubmit}
          onClick={() => void submit()}
          aria-label="Prompt 전송"
        >
          <SendIcon data-icon="inline-start" />
          전송
        </Button>
      </div>
      {!size.ok && (
        <p className="px-3 pb-2 text-xs text-destructive" role="alert">
          {describePromptSizeViolation(size)}
        </p>
      )}
      {!targets.valid && mode === "selected" && (
        <p className="px-3 pb-2 text-xs text-destructive" role="alert">
          하나 이상의 패널을 선택하세요.
        </p>
      )}
      {notice && (
        <div className="px-3 pb-2 text-xs text-destructive" role="alert">
          <p data-testid="composer-notice-reason">{notice.reason}</p>
          {notice.nextAction && (
            <p className="text-muted-foreground" data-testid="composer-notice-next-action">
              {notice.nextAction}
            </p>
          )}
        </div>
      )}
    </section>
  );
}
