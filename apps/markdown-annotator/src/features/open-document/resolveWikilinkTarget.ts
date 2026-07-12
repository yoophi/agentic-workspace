export type ResolvedWikilinkTarget = {
  fileName: string;
  path: string;
};

export function resolveWikilinkTarget(
  currentDocumentPath: string,
  href: string,
): ResolvedWikilinkTarget {
  if (!href.startsWith("./") || href.includes(":") || href.includes("?") || href.includes("#")) {
    throw new Error(`허용되지 않은 wikilink 대상입니다: ${href}`);
  }

  let fileName: string;
  try {
    fileName = decodeURIComponent(href.slice(2));
  } catch {
    throw new Error(`wikilink 대상 인코딩이 올바르지 않습니다: ${href}`);
  }

  if (
    !fileName ||
    fileName === "." ||
    fileName === ".." ||
    fileName.includes("/") ||
    fileName.includes("\\") ||
    !fileName.toLowerCase().endsWith(".md")
  ) {
    throw new Error(`현재 문서 디렉터리 밖의 wikilink는 열 수 없습니다: ${href}`);
  }

  const separator = currentDocumentPath.includes("\\") ? "\\" : "/";
  const lastSeparator = Math.max(
    currentDocumentPath.lastIndexOf("/"),
    currentDocumentPath.lastIndexOf("\\"),
  );
  const directory = lastSeparator === -1 ? "" : currentDocumentPath.slice(0, lastSeparator);

  return {
    fileName,
    path: directory ? `${directory}${separator}${fileName}` : fileName,
  };
}
