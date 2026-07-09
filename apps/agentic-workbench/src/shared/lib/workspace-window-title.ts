export const MCP_WINDOW_TITLE_EVENT = "workspace://mcp-window-title";
export const MCP_WINDOW_TITLE_FALLBACK_EVENT = "mcp-window-title-fallback";

export type McpWindowTitleEvent = {
  title: string;
};

export function dispatchMcpWindowTitle(title: string) {
  if (!title) {
    return;
  }

  window.dispatchEvent(
    new CustomEvent<McpWindowTitleEvent>(MCP_WINDOW_TITLE_FALLBACK_EVENT, {
      detail: { title },
    }),
  );
}
