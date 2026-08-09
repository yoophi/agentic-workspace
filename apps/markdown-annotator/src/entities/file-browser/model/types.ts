import type { FileBrowserEntry } from "@yoophi/file-browser-core";

export type RootScanWarning = { relativePath: string; code: string };
export type RootScanBatch = {
  scanId: string;
  sequence: number;
  entries: FileBrowserEntry[];
  visitedEntries: number;
  matchedDocuments: number;
  warnings: RootScanWarning[];
  completed: boolean;
};

export type FileBrowserState = {
  scanId: string | null;
  lastSequence: number;
  entries: FileBrowserEntry[];
  visitedEntries: number;
  matchedDocuments: number;
  warnings: RootScanWarning[];
  scanning: boolean;
};
