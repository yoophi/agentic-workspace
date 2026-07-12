/** Convert supported wikilinks while preserving inline-code spans. */
export function transformWikilinks(markdown: string): string {
  return mapOutsideInlineCode(markdown, (text) =>
    text.replace(/\[\[([^\[\]\n]+)]]/g, (raw, inner: string) => {
      const separator = inner.indexOf("|");
      const target = (separator === -1 ? inner : inner.slice(0, separator)).trim();
      const label = (separator === -1 ? target : inner.slice(separator + 1)).trim();

      if (!target || !label || target.includes("|")) {
        return raw;
      }

      const fileName = target.toLowerCase().endsWith(".md") ? target : `${target}.md`;
      return `[${label}](./${encodeURIComponent(fileName)})`;
    }),
  );
}

function mapOutsideInlineCode(markdown: string, transform: (text: string) => string): string {
  const parts = markdown.split(/(`+[^`]*`+)/g);
  return parts.map((part, index) => (index % 2 === 0 ? transform(part) : part)).join("");
}

/**
 * Hide complete HTML comments outside fenced and inline code. Newlines inside
 * comments are retained so source line anchors remain stable. An unclosed
 * opener is kept as text instead of swallowing the rest of the document.
 */
export function stripHtmlComments(markdown: string): string {
  const lines = markdown.split("\n");
  let fence: string | undefined;
  let inComment = false;
  let pending = "";
  const output: string[] = [];

  for (const line of lines) {
    const trimmed = line.trimStart();
    const fenceMatch = trimmed.match(/^(```+|~~~+)/);
    if (!inComment && fenceMatch) {
      if (!fence) {
        fence = fenceMatch[1][0];
      } else if (fence === fenceMatch[1][0]) {
        fence = undefined;
      }
      output.push(line);
      continue;
    }
    if (fence) {
      output.push(line);
      continue;
    }

    let visible = "";
    let index = 0;
    let inlineTicks = 0;
    while (index < line.length) {
      if (inComment) {
        const close = line.indexOf("-->", index);
        if (close === -1) {
          pending += `${line.slice(index)}\n`;
          index = line.length;
        } else {
          inComment = false;
          pending = "";
          index = close + 3;
        }
        continue;
      }

      if (line[index] === "`") {
        let count = 1;
        while (line[index + count] === "`") count += 1;
        inlineTicks = inlineTicks === count ? 0 : inlineTicks === 0 ? count : inlineTicks;
        visible += line.slice(index, index + count);
        index += count;
        continue;
      }

      if (inlineTicks === 0 && line.startsWith("<!--", index)) {
        const close = line.indexOf("-->", index + 4);
        if (close !== -1) {
          index = close + 3;
          continue;
        }
        inComment = true;
        pending = `${line.slice(index)}\n`;
        index = line.length;
        continue;
      }

      visible += line[index];
      index += 1;
    }
    output.push(visible);
  }

  if (inComment && pending) {
    const pendingLines = pending.slice(0, -1).split("\n");
    const start = output.length - pendingLines.length;
    pendingLines.forEach((line, index) => {
      output[start + index] += line;
    });
  }

  return output.join("\n");
}
