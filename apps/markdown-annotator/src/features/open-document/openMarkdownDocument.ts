import { open } from "@tauri-apps/plugin-dialog";
import { requestOpenDocumentTab, requestOpenDocumentWindow } from "@/entities/document/api/documentApi";
import type { MarkdownDocument } from "@/entities/document";

function isTauriRuntime() {
  return "__TAURI_INTERNALS__" in window;
}

function openMarkdownDocumentFromBrowser(): Promise<MarkdownDocument | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".md,.markdown,.mdx,text/markdown,text/plain";
    input.style.display = "none";

    input.addEventListener(
      "change",
      async () => {
        const file = input.files?.[0];
        input.remove();

        if (!file) {
          resolve(null);
          return;
        }

        resolve({
          fileName: file.name,
          absolutePath: file.name,
          markdownText: await file.text(),
        });
      },
      { once: true },
    );

    input.addEventListener(
      "cancel",
      () => {
        input.remove();
        resolve(null);
      },
      { once: true },
    );

    document.body.append(input);
    input.click();
  });
}

export async function openMarkdownDocument(): Promise<MarkdownDocument | null> {
  if (!isTauriRuntime()) {
    return openMarkdownDocumentFromBrowser();
  }

  const selected = await selectMarkdownDocumentPath();
  if (!selected) {
    return null;
  }

  await requestOpenDocumentWindow(selected);
  return null;
}

export async function openMarkdownDocumentTab(): Promise<MarkdownDocument | null> {
  if (!isTauriRuntime()) {
    return openMarkdownDocumentFromBrowser();
  }

  const selected = await selectMarkdownDocumentPath();
  if (!selected) {
    return null;
  }

  await requestOpenDocumentTab(selected);
  return null;
}

export function getDocumentPathFromWindowQuery() {
  return new URLSearchParams(window.location.search).get("path");
}

async function selectMarkdownDocumentPath() {
  const selected = await open({
    multiple: false,
    filters: [{ name: "Markdown", extensions: ["md", "markdown", "mdx"] }],
  });

  return typeof selected === "string" ? selected : null;
}
