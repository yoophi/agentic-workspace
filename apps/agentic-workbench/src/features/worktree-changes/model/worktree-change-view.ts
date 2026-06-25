import type { WorktreeChange, WorktreeChangeType } from "@/entities/agent-run/model/types";

export type ChangeBadgeVariant = "default" | "secondary" | "destructive" | "outline";

const CHANGE_LABELS: Record<WorktreeChangeType, string> = {
  added: "생성",
  modified: "수정",
  deleted: "삭제",
  renamed: "이름변경",
  untracked: "생성",
};

const CHANGE_BADGE_VARIANTS: Record<WorktreeChangeType, ChangeBadgeVariant> = {
  added: "default",
  modified: "secondary",
  deleted: "destructive",
  renamed: "outline",
  untracked: "default",
};

export function changeTypeLabel(changeType: WorktreeChangeType) {
  return CHANGE_LABELS[changeType];
}

export function changeBadgeVariant(changeType: WorktreeChangeType): ChangeBadgeVariant {
  return CHANGE_BADGE_VARIANTS[changeType];
}

/** 파일 확장자로 shiki 하이라이팅 언어를 추정한다. 모르면 plaintext로 둔다. */
export function languageFromPath(path: string): string {
  const match = /\.([a-zA-Z0-9]+)$/.exec(path);
  const extension = match?.[1]?.toLowerCase();
  if (!extension) {
    return "text";
  }

  const languageByExtension: Record<string, string> = {
    ts: "typescript",
    tsx: "tsx",
    js: "javascript",
    jsx: "jsx",
    mjs: "javascript",
    cjs: "javascript",
    json: "json",
    rs: "rust",
    py: "python",
    go: "go",
    java: "java",
    rb: "ruby",
    php: "php",
    c: "c",
    h: "c",
    cpp: "cpp",
    hpp: "cpp",
    cs: "csharp",
    css: "css",
    scss: "scss",
    html: "html",
    md: "markdown",
    sh: "bash",
    bash: "bash",
    zsh: "bash",
    yml: "yaml",
    yaml: "yaml",
    toml: "toml",
    sql: "sql",
  };

  return languageByExtension[extension] ?? "text";
}

/**
 * 변경 항목을 어떻게 표시할지 결정한다.
 * - binary: 미리보기 불가 안내
 * - diff: unified diff 본문
 * - content: 새 파일 전체 내용 미리보기
 * - empty: 표시할 내용 없음(예: 읽기 실패)
 */
export type ChangePreview =
  | { kind: "binary" }
  | { kind: "diff"; text: string; language: "diff" }
  | { kind: "content"; text: string; language: string }
  | { kind: "empty" };

export function previewForChange(change: WorktreeChange): ChangePreview {
  if (change.binary) {
    return { kind: "binary" };
  }
  if (change.diff !== null) {
    return { kind: "diff", text: change.diff, language: "diff" };
  }
  if (change.content !== null) {
    return { kind: "content", text: change.content, language: languageFromPath(change.path) };
  }
  return { kind: "empty" };
}
