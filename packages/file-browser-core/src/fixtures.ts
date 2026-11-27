import type { FileBrowserEntry } from "./types";

export const compressedMarkdownFixture: readonly FileBrowserEntry[] = [
  { kind: "file", path: "a/file.md" },
  { kind: "file", path: "b/b1/file2.md" },
  { kind: "file", path: "d/file10.md" },
  { kind: "file", path: "d/file2.md" },
];

export const unicodeFixture: readonly FileBrowserEntry[] = [
  { kind: "file", path: "문서/키오스크10.md" },
  { kind: "file", path: "문서/키오스크2.md" },
  { kind: "file", path: "README.md" },
];
