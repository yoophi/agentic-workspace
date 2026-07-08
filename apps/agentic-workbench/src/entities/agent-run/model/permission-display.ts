import type { PermissionOption, RunEvent } from "./types";

export type PermissionEvent = Extract<RunEvent, { type: "permission" }>;

export type CommandDetailDisplay = {
  label: string;
  text: string;
  isMultiline: boolean;
  isLong: boolean;
};

export type ApprovalOptionDisplay = {
  optionId: string;
  kind: string;
  fullLabel: string;
  buttonLabel: string;
  isDestructiveOrReject: boolean;
};

export type PermissionDisplayModel = {
  permissionId: string | null;
  title: string;
  detail: CommandDetailDisplay | null;
  options: ApprovalOptionDisplay[];
};

const LONG_TEXT_THRESHOLD = 600;
const SHORT_LABEL_MAX = 32;

export function createPermissionDisplayModel(
  permission: PermissionEvent | null,
): PermissionDisplayModel {
  const permissionId = cleanText(permission?.permissionId) || null;
  const title = cleanText(permission?.title) || "Tool request";

  return {
    permissionId,
    title,
    detail: createCommandDetailDisplay(permission?.input),
    options: (permission?.options ?? []).map(createApprovalOptionDisplay),
  };
}

export function createCommandDetailDisplay(input: unknown): CommandDetailDisplay | null {
  if (input === undefined) {
    return null;
  }

  const text = formatPermissionInput(input);
  return {
    label: "Request detail",
    text,
    isMultiline: text.includes("\n"),
    isLong: text.length > LONG_TEXT_THRESHOLD || text.split("\n").length > 12,
  };
}

export function formatPermissionInput(input: unknown): string {
  if (typeof input === "string") {
    return input;
  }

  if (input === null) {
    return "null";
  }

  try {
    return JSON.stringify(input, null, 2);
  } catch (error) {
    const message = error instanceof Error ? error.message : "unknown serialization error";
    return `[Unable to serialize permission input: ${message}]`;
  }
}

export function createApprovalOptionDisplay(option: PermissionOption): ApprovalOptionDisplay {
  const kind = cleanText(option.kind) || "option";
  const optionName = cleanText(option.name);
  const fullLabel = optionName || kind || cleanText(option.optionId) || "Option";
  const isDestructiveOrReject = isRejectLikeOption(option);

  return {
    optionId: option.optionId,
    kind,
    fullLabel,
    buttonLabel: summarizeOptionLabel(optionName, kind, isDestructiveOrReject),
    isDestructiveOrReject,
  };
}

export function summarizeOptionLabel(
  label: string,
  kind: string,
  isDestructiveOrReject = isRejectLikeText(`${kind} ${label}`),
): string {
  const cleaned = cleanText(label);
  if (cleaned && cleaned.length <= SHORT_LABEL_MAX && !isCommandLikeLabel(cleaned)) {
    return cleaned;
  }

  if (isDestructiveOrReject) {
    return "Reject";
  }

  const normalizedKind = cleanText(kind).toLowerCase();
  if (normalizedKind.includes("allow")) {
    return "Allow";
  }
  if (normalizedKind.includes("approve")) {
    return "Approve";
  }
  if (normalizedKind.includes("accept")) {
    return "Accept";
  }

  return "Approve";
}

function isRejectLikeOption(option: PermissionOption) {
  return isRejectLikeText(`${option.kind} ${option.name} ${option.optionId}`);
}

function isRejectLikeText(text: string) {
  return /\b(reject|deny|decline|cancel|disallow)\b/i.test(text);
}

function isCommandLikeLabel(label: string) {
  return (
    label.includes("\n") ||
    label.includes("{") ||
    label.includes("}") ||
    /\s--[\w-]+/.test(label) ||
    /\b(npm|pnpm|yarn|gh|git|cargo|python|node|bash|zsh|sh|rm|sudo)\b/.test(label)
  );
}

function cleanText(value: unknown) {
  return typeof value === "string" ? value.trim() : "";
}
