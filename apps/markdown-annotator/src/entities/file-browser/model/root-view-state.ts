export type RootViewState = { sort: "name-asc" | "name-desc"; expanded: string[]; leftPanelOpen: boolean; rightPanelOpen: boolean };
const key = (rootId: string) => `ma:root-view:${rootId}`;
export const rootViewStateRepository = {
  load(rootId: string): RootViewState { try { return JSON.parse(localStorage.getItem(key(rootId)) ?? "") as RootViewState; } catch { return { sort: "name-asc", expanded: [], leftPanelOpen: true, rightPanelOpen: true }; } },
  save(rootId: string, state: RootViewState) { localStorage.setItem(key(rootId), JSON.stringify(state)); },
};
