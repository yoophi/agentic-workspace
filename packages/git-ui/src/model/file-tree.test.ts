import { describe, expect, it } from "vitest";

import type { GitCommitFileChange } from "@yoophi/git-graph";

import { buildFileTreeRows, getFileFolderPaths } from "./file-tree";

function file(path: string, status = "M"): GitCommitFileChange {
  return { path, status } as GitCommitFileChange;
}

describe("buildFileTreeRows", () => {
  it("nests files under folders and compresses single-child chains", () => {
    const files = [file("src/app/main.ts"), file("src/app/util.ts"), file("README.md")];
    const expanded = getFileFolderPaths(files);

    const rows = buildFileTreeRows(files, expanded);

    // "src/app" 체인은 하나의 폴더 행으로 압축된다.
    const folderRows = rows.filter((row) => row.type === "folder");
    expect(folderRows).toHaveLength(1);
    expect(folderRows[0]).toMatchObject({ name: "src/app", path: "src/app" });

    const fileRows = rows.filter((row) => row.type === "file");
    expect(fileRows.map((row) => row.name)).toEqual(["main.ts", "util.ts", "README.md"]);
  });

  it("hides files under collapsed folders", () => {
    const files = [file("src/a.ts"), file("docs/b.md")];

    const rows = buildFileTreeRows(files, new Set());

    expect(rows.every((row) => row.type === "folder")).toBe(true);
    expect(rows).toHaveLength(2);
  });

  it("preserves Korean folder and file names in expanded rows", () => {
    const files = [
      file("문서/키오스크.md", "A"),
      file("문서/릴리즈-노트_01.md", "M"),
      file("README.md", "M"),
    ];

    const rows = buildFileTreeRows(files, getFileFolderPaths(files));
    const folderRows = rows.filter((row) => row.type === "folder");
    const fileRows = rows.filter((row) => row.type === "file");

    expect(folderRows.map((row) => row.name)).toContain("문서");
    expect(fileRows.map((row) => row.name)).toEqual([
      "릴리즈-노트_01.md",
      "키오스크.md",
      "README.md",
    ]);
    expect(fileRows.map((row) => row.file.path)).toContain("문서/키오스크.md");
    expect(fileRows.map((row) => row.file.path).join("\n")).not.toContain("\\355");
  });

  it("keeps the readable Korean file path on rows used for selection", () => {
    const selectedPath = "문서/키오스크.md";
    const rows = buildFileTreeRows(
      [file(selectedPath, "M")],
      getFileFolderPaths([file(selectedPath)]),
    );
    const selectedRow = rows.find((row) => row.type === "file");

    expect(selectedRow?.type).toBe("file");
    if (selectedRow?.type === "file") {
      expect(selectedRow.file.path).toBe(selectedPath);
      expect(selectedRow.id).toBe(`file:M:${selectedPath}`);
    }
  });

  it("displays renamed Korean current paths without octal byte escapes", () => {
    const rows = buildFileTreeRows(
      [file("문서/한글-new.md", "R100")],
      getFileFolderPaths([file("문서/한글-new.md", "R100")]),
    );
    const fileRows = rows.filter((row) => row.type === "file");

    expect(fileRows).toHaveLength(1);
    expect(fileRows[0].name).toBe("한글-new.md");
    expect(fileRows[0].file.path).toBe("문서/한글-new.md");
    expect(fileRows[0].file.path).not.toContain("\\355");
  });
});
