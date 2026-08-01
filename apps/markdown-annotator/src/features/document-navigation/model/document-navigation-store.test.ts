import { beforeEach, describe, expect, it } from "vitest";
import { useDocumentNavigationStore } from "./document-navigation-store";

describe("document navigation store", () => {
  beforeEach(() => useDocumentNavigationStore.setState({ activePath: null, backStack: [], forwardStack: [], requestToken: 0 }));
  it("maintains one active document with back and forward history", () => {
    useDocumentNavigationStore.getState().select("a.md");
    const token = useDocumentNavigationStore.getState().select("b.md");
    expect(token).toBe(2);
    expect(useDocumentNavigationStore.getState().back()).toBe("a.md");
    expect(useDocumentNavigationStore.getState().forward()).toBe("b.md");
  });
});
