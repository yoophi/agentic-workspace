import { create } from "zustand";

type NavigationState = {
  activePath: string | null;
  backStack: string[];
  forwardStack: string[];
  requestToken: number;
  readingPositions: Record<string, number>;
  rememberPosition: (path: string, offset: number) => void;
  positionFor: (path: string) => number;
  select: (path: string) => number;
  back: () => string | null;
  forward: () => string | null;
};

export const useDocumentNavigationStore = create<NavigationState>((set, get) => ({
  activePath: null,
  backStack: [],
  forwardStack: [],
  requestToken: 0,
  readingPositions: {},
  rememberPosition(path, offset) { set((state) => ({ readingPositions: { ...state.readingPositions, [path]: offset } })); },
  positionFor(path) { return get().readingPositions[path] ?? 0; },
  select(path) {
    const state = get();
    if (state.activePath === path) return state.requestToken;
    const requestToken = state.requestToken + 1;
    set({ activePath: path, backStack: state.activePath ? [...state.backStack, state.activePath] : state.backStack, forwardStack: [], requestToken });
    return requestToken;
  },
  back() {
    const state = get();
    const path = state.backStack[state.backStack.length - 1] ?? null;
    if (!path) return null;
    set({ activePath: path, backStack: state.backStack.slice(0, -1), forwardStack: state.activePath ? [state.activePath, ...state.forwardStack] : state.forwardStack, requestToken: state.requestToken + 1 });
    return path;
  },
  forward() {
    const state = get();
    const path = state.forwardStack[0] ?? null;
    if (!path) return null;
    set({ activePath: path, backStack: state.activePath ? [...state.backStack, state.activePath] : state.backStack, forwardStack: state.forwardStack.slice(1), requestToken: state.requestToken + 1 });
    return path;
  },
}));
