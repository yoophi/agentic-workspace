import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, type PropsWithChildren } from "react";

export const ROOT_DOCUMENT_SELECTED_EVENT = "ma:root-document-selected";
const TAURI_ROOT_DOCUMENT_SELECTED_EVENT = "markdown-annotator://root-document-selected";

export function ApplicationEventsProvider({ children }: PropsWithChildren) {
  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    const unlistenPromise = listen<string>(TAURI_ROOT_DOCUMENT_SELECTED_EVENT, ({ payload }) => {
      const url = new URL(window.location.href);
      url.searchParams.set("path", payload);
      window.history.replaceState(null, "", url);
      window.dispatchEvent(new CustomEvent(ROOT_DOCUMENT_SELECTED_EVENT, { detail: payload }));
    });
    return () => { void unlistenPromise.then((unlisten) => unlisten()); };
  }, []);
  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    const rootPath = new URL(window.location.href).searchParams.get("root");
    if (!rootPath) return;
    const rootId = new URL(window.location.href).searchParams.get("rootId") ?? rootPath;
    void invoke("start_root_watcher", { rootId, rootPath });
    return () => { void invoke("stop_root_watcher", { rootId }); };
  }, []);

  return children;
}
