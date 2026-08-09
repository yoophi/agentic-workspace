export type InternalLinkIntent =
  | { kind: "document"; relativePath: string; heading: string | null }
  | { kind: "heading"; heading: string }
  | { kind: "external"; url: string };

export function resolveInternalLink(currentRelativePath: string, href: string): InternalLinkIntent {
  if (/^https?:\/\//i.test(href)) {
    const url = new URL(href);
    if (url.protocol !== "http:" && url.protocol !== "https:") throw new Error("지원하지 않는 외부 링크입니다.");
    return { kind: "external", url: url.toString() };
  }
  if (href.startsWith("#")) {
    const heading = decode(href.slice(1));
    if (!heading) throw new Error("빈 heading 링크입니다.");
    return { kind: "heading", heading };
  }
  if (href.includes(":") || href.startsWith("/") || href.startsWith("\\")) throw new Error("root 밖 링크는 열 수 없습니다.");
  const [rawPath, rawHeading] = href.split("#", 2);
  const decodedPath = decode((rawPath ?? "").replace(/^\.\//, ""));
  const baseSegments = currentRelativePath.split("/").slice(0, -1);
  const segments = [...baseSegments];
  for (const segment of decodedPath.split("/")) {
    if (!segment || segment === ".") continue;
    if (segment === "..") {
      if (segments.length === 0) throw new Error("root 밖 링크는 열 수 없습니다.");
      segments.pop();
    } else if (segment.includes("\\") || segment.includes("\0")) throw new Error("올바르지 않은 링크 경로입니다.");
    else segments.push(segment);
  }
  const relativePath = segments.join("/");
  if (!/\.(md|markdown)$/i.test(relativePath)) throw new Error("Markdown 문서 링크만 열 수 있습니다.");
  return { kind: "document", relativePath, heading: rawHeading ? decode(rawHeading) : null };
}

function decode(value: string) {
  try { return decodeURIComponent(value); } catch { throw new Error("링크 인코딩이 올바르지 않습니다."); }
}

export async function openExternalLink(url: string) { const parsed = new URL(url); if (parsed.protocol !== "http:" && parsed.protocol !== "https:") throw new Error("지원하지 않는 외부 링크입니다."); const { invoke } = await import("@tauri-apps/api/core"); return invoke<void>("open_external_https", { url: parsed.toString() }); }
