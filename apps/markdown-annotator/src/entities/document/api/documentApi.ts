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

export function installCli(): Promise<CliInstallStatus> {
  return invoke<CliInstallStatus>("install_cli");
}

export function checkCliInstalled(): Promise<CliInstallStatus> {
  return invoke<CliInstallStatus>("check_cli_installed");
}

export function requestOpenDocumentWindow(path: string): Promise<void> {
  return invoke<void>("request_open_document_window", { path });
}

export function requestOpenDocumentTab(path: string): Promise<void> {
  return invoke<void>("request_open_document_tab", { path });
}
