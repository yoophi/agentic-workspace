import { invoke } from "@tauri-apps/api/core";
import type { MarkdownDocument } from "@/entities/document";

export type CliInstallStatus = {
  installed: boolean;
  path: string;
  target: string;
};

export function readMarkdownDocument(path: string): Promise<MarkdownDocument> {
  return invoke<MarkdownDocument>("read_markdown_file", { path });
}

type RootDocumentResponse = { identity: { relativePath: string }; markdownText: string };

export async function readRootMarkdownDocument(rootPath: string, relativePath: string): Promise<MarkdownDocument> {
  const result = await invoke<RootDocumentResponse>("read_root_markdown_document", { rootPath, relativePath });
  return { fileName: relativePath.split("/").pop() ?? relativePath, absolutePath: `${rootPath}/${relativePath}`, markdownText: result.markdownText };
}

export function startMarkdownDocumentWatcher(path: string): Promise<void> {
  return invoke<void>("start_markdown_document_watcher", { path });
}

export function stopMarkdownDocumentWatcher(): Promise<void> {
  return invoke<void>("stop_markdown_document_watcher");
}

export function installCli(): Promise<CliInstallStatus> {
  return invoke<CliInstallStatus>("install_cli");
}

export function checkCliInstalled(): Promise<CliInstallStatus> {
  return invoke<CliInstallStatus>("check_cli_installed");
}
export function removeCli(): Promise<void> { return invoke<void>("remove_cli"); }

export function requestOpenDocumentWindow(path: string): Promise<void> {
  return invoke<void>("request_open_document_window", { path });
}

export function requestOpenDocumentTab(path: string): Promise<void> {
  return invoke<void>("request_open_document_tab", { path });
}
