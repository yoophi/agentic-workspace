import { readFile, readdir } from "node:fs/promises";
import path from "node:path";

const appsDirectory = path.resolve("apps");
const sourceExtensions = new Set([".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"]);
const violations = [];

async function visit(directory, owningApp) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    if (entry.name === "node_modules" || entry.name === "dist" || entry.name === "target") continue;
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      await visit(entryPath, owningApp);
      continue;
    }
    if (!sourceExtensions.has(path.extname(entry.name))) continue;

    const source = await readFile(entryPath, "utf8");
    for (const match of source.matchAll(/(?:from\s+|import\s*\(|require\s*\()["']([^"']+)["']/g)) {
      const specifier = match[1];
      if (!specifier) continue;
      const resolvedSpecifier = specifier.startsWith(".")
        ? path.resolve(path.dirname(entryPath), specifier)
        : specifier;
      const relativeToRoot = path.relative(process.cwd(), resolvedSpecifier).split(path.sep).join("/");
      const appPath = specifier.startsWith("apps/") ? specifier : relativeToRoot;
      const otherApp = appPath.match(/^apps\/([^/]+)/)?.[1];
      if (otherApp && otherApp !== owningApp) violations.push(`${entryPath}: ${specifier}`);
    }
  }
}

for (const app of await readdir(appsDirectory, { withFileTypes: true })) {
  if (app.isDirectory()) await visit(path.join(appsDirectory, app.name), app.name);
}

if (violations.length > 0) {
  console.error("Direct app-to-app imports are forbidden:\n" + violations.join("\n"));
  process.exitCode = 1;
}
