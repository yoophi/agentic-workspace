import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useState } from "react";
import { createFileBrowserRows, type FileBrowserOptions } from "@yoophi/file-browser-core";

import { cancelRootScan, startRootScan } from "@/entities/file-browser/api/file-browser-api";
import type { FileBrowserState, RootScanBatch } from "@/entities/file-browser/model/types";

export const ROOT_SCAN_BATCH_EVENT = "markdown-annotator://root-scan-batch";
export const ROOT_CHANGED_EVENT = "markdown-annotator://root-changed";

export const emptyFileBrowserState: FileBrowserState = {
  scanId: null, lastSequence: -1, entries: [], visitedEntries: 0,
  matchedDocuments: 0, warnings: [], scanning: false,
};

export function reduceRootScanBatch(state: FileBrowserState, batch: RootScanBatch): FileBrowserState {
  if (batch.scanId !== state.scanId || batch.sequence <= state.lastSequence) return state;
  const entriesByPath = new Map(state.entries.map((entry) => [entry.path, entry]));
  for (const entry of batch.entries) entriesByPath.set(entry.path, entry);
  const warnings = [...state.warnings];
  const warningKeys = new Set(warnings.map((warning) => `${warning.code}:${warning.relativePath}`));
  for (const warning of batch.warnings) {
    const key = `${warning.code}:${warning.relativePath}`;
    if (!warningKeys.has(key)) { warningKeys.add(key); warnings.push(warning); }
  }
  return {
    ...state, entries: [...entriesByPath.values()], warnings,
    lastSequence: batch.sequence, visitedEntries: batch.visitedEntries,
    matchedDocuments: batch.matchedDocuments, scanning: !batch.completed,
  };
}

export function useFileBrowser(rootPath: string | null, exclusions: string[], options: FileBrowserOptions) {
  const [state, setState] = useState<FileBrowserState>(emptyFileBrowserState);
  const [rootRevision, setRootRevision] = useState(0);
  useEffect(() => { if (!("__TAURI_INTERNALS__" in window)) return; const unlisten = listen<{ revision: number }>(ROOT_CHANGED_EVENT, ({ payload }) => setRootRevision(payload.revision)); return () => { void unlisten.then((dispose) => dispose()); }; }, []);
  useEffect(() => {
    if (!rootPath || !("__TAURI_INTERNALS__" in window)) return;
    const scanId = crypto.randomUUID();
    setState({ ...emptyFileBrowserState, scanId, scanning: true });
    const unlistenPromise = listen<RootScanBatch>(ROOT_SCAN_BATCH_EVENT, ({ payload }) => {
      setState((current) => reduceRootScanBatch(current, payload));
    });
    void startRootScan(rootPath, exclusions, scanId);
    return () => {
      void cancelRootScan(scanId);
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [rootPath, exclusions, rootRevision]);
  const rows = useMemo(() => createFileBrowserRows(state.entries, options), [options, state.entries]);
  return { ...state, rows };
}
