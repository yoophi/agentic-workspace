import type { PromptDispatchTargetStatus } from "./types";

export type PromptTargetDeliveryState = {
  panelId: string;
  status: PromptDispatchTargetStatus;
  error?: string;
};

export type PromptDispatchState = {
  dispatchId: string | null;
  targets: PromptTargetDeliveryState[];
};

export type PromptDispatchAction =
  | { type: "queued"; dispatchId: string; panelIds: string[] }
  | { type: "sending"; panelId: string }
  | { type: "succeeded"; panelId: string }
  | { type: "failed"; panelId: string; error: string }
  | { type: "reset" };

export const initialPromptDispatchState: PromptDispatchState = {
  dispatchId: null,
  targets: [],
};

export function promptDispatchReducer(
  state: PromptDispatchState,
  action: PromptDispatchAction,
): PromptDispatchState {
  if (action.type === "reset") return initialPromptDispatchState;
  if (action.type === "queued") {
    return {
      dispatchId: action.dispatchId,
      targets: action.panelIds.map((panelId) => ({ panelId, status: "pending" })),
    };
  }
  return {
    ...state,
    targets: state.targets.map((target) =>
      target.panelId !== action.panelId
        ? target
        : action.type === "sending"
          ? { ...target, status: "accepted", error: undefined }
          : action.type === "succeeded"
            ? { ...target, status: "delivered", error: undefined }
            : { ...target, status: "failed", error: action.error },
    ),
  };
}

export function summarizePromptDispatch(state: PromptDispatchState) {
  return {
    total: state.targets.length,
    succeeded: state.targets.filter((target) => target.status === "delivered").length,
    failed: state.targets.filter((target) => target.status === "failed").length,
    pending: state.targets.filter((target) =>
      ["pending", "accepted"].includes(target.status),
    ).length,
  };
}

