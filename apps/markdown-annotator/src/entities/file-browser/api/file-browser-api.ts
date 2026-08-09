import { invoke } from "@tauri-apps/api/core";

export function startRootScan(rootPath: string, excludedDirectoryNames: string[], scanId: string) {
  return invoke<void>("start_root_scan", { rootPath, excludedDirectoryNames, scanId });
}

export function cancelRootScan(scanId: string) {
  return invoke<void>("cancel_root_scan", { scanId });
}

export const startRootWatcher = (rootId: string, rootPath: string) => invoke<void>("start_root_watcher", { rootId, rootPath });
export const stopRootWatcher = (rootId: string) => invoke<void>("stop_root_watcher", { rootId });
