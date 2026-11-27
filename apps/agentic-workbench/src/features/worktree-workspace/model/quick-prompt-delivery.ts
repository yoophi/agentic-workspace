import {
  formatQuickPromptForAgent,
  isQuickPromptSendable,
  type QuickPromptContext,
} from "@yoophi/markdown-annotation-core";

export type WorkbenchQuickPromptTarget = {
  available: boolean;
  unavailableReason?: string;
};

export function createWorkbenchQuickPromptTarget(canSend: boolean): WorkbenchQuickPromptTarget {
  return {
    available: canSend,
    unavailableReason: canSend ? undefined : "연결된 agent prompt 대상이 없습니다.",
  };
}

export function canSendWorkbenchQuickPrompt({
  promptText,
  context,
  target,
}: {
  promptText: string;
  context: QuickPromptContext | null;
  target: WorkbenchQuickPromptTarget;
}) {
  return isQuickPromptSendable({
    promptText,
    context,
    targetAvailable: target.available,
  });
}

export function createWorkbenchQuickPromptPayload({
  promptText,
  context,
  target,
}: {
  promptText: string;
  context: QuickPromptContext;
  target: WorkbenchQuickPromptTarget;
}) {
  if (!canSendWorkbenchQuickPrompt({ promptText, context, target })) {
    throw new Error(target.unavailableReason ?? "Quick prompt is not sendable.");
  }

  return formatQuickPromptForAgent({ promptText, context });
}
