// markdown 파일 판별. 백엔드 scope(kind: "markdown") 필터(fs_worktree_file_provider의
// is_markdown_path)와 동일한 확장자 목록을 유지해야 한다.
export function isMarkdownPath(path: string) {
  const normalized = path.toLowerCase();
  return (
    normalized.endsWith(".md") ||
    normalized.endsWith(".markdown") ||
    normalized.endsWith(".mdx")
  );
}
