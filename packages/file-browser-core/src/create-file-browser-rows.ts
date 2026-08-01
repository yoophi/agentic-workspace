import type {
  FileBrowserChildState,
  FileBrowserEntry,
  FileBrowserMatchRange,
  FileBrowserOptions,
  FileBrowserRow,
} from "./types";

type Node = {
  path: string;
  name: string;
  kind: "directory" | "file";
  children: Map<string, Node>;
  childState?: FileBrowserChildState;
  modifiedAt?: string;
  size?: number;
};

export class FileBrowserPathError extends Error {
  constructor(readonly path: string) {
    super(`Invalid file browser path: ${JSON.stringify(path)}`);
    this.name = "FileBrowserPathError";
  }
}

const naturalCollator = new Intl.Collator(undefined, {
  numeric: true,
  sensitivity: "base",
});

function validatePath(path: string) {
  if (
    path.length === 0 ||
    path.startsWith("/") ||
    path.includes("\\") ||
    path.includes("\0") ||
    path.split("/").some((segment) => segment === "" || segment === "." || segment === "..")
  ) {
    throw new FileBrowserPathError(path);
  }
}

function createNode(path: string, name: string, kind: Node["kind"]): Node {
  return { path, name, kind, children: new Map() };
}

function mergeEntries(entries: readonly FileBrowserEntry[]) {
  const root = createNode("", "", "directory");

  for (const entry of entries) {
    validatePath(entry.path);
    const segments = entry.path.split("/");
    let parent = root;
    let currentPath = "";

    segments.forEach((segment, index) => {
      currentPath = currentPath ? `${currentPath}/${segment}` : segment;
      const isLeaf = index === segments.length - 1;
      const kind = isLeaf ? entry.kind : "directory";
      const existing = parent.children.get(segment);

      if (existing && existing.kind !== kind) {
        throw new FileBrowserPathError(entry.path);
      }

      const node = existing ?? createNode(currentPath, segment, kind);
      parent.children.set(segment, node);
      parent = node;

      if (isLeaf) {
        node.modifiedAt ??= entry.modifiedAt;
        node.size ??= entry.size;
        if (entry.childState) node.childState = entry.childState;
      }
    });
  }

  return root;
}

function nodeMatches(node: Node, query: string): boolean {
  return node.path.toLocaleLowerCase().includes(query);
}

function hasMatch(node: Node, query: string): boolean {
  if (nodeMatches(node, query)) return true;
  return [...node.children.values()].some((child) => hasMatch(child, query));
}

function compareNodes(left: Node, right: Node, options: FileBrowserOptions) {
  if (options.directoriesFirst !== false && left.kind !== right.kind) {
    return left.kind === "directory" ? -1 : 1;
  }
  const sort = options.sort ?? { by: "name", direction: "asc" as const };
  let comparison = 0;
  if (sort.by === "modifiedAt") {
    comparison = (left.modifiedAt ?? "").localeCompare(right.modifiedAt ?? "");
  } else {
    comparison = naturalCollator.compare(
      sort.by === "path" ? left.path : left.name,
      sort.by === "path" ? right.path : right.name,
    );
  }
  if (comparison === 0) comparison = naturalCollator.compare(left.path, right.path);
  return sort.direction === "desc" ? -comparison : comparison;
}

function matchRanges(label: string, query: string): readonly FileBrowserMatchRange[] {
  if (!query) return [];
  const ranges: FileBrowserMatchRange[] = [];
  const target = label.toLocaleLowerCase();
  let from = 0;
  while (from <= target.length - query.length) {
    const start = target.indexOf(query, from);
    if (start < 0) break;
    ranges.push({ start, end: start + query.length });
    from = start + query.length;
  }
  return ranges;
}

export function createFileBrowserRows(
  entries: readonly FileBrowserEntry[],
  options: FileBrowserOptions,
): readonly FileBrowserRow[] {
  const root = mergeEntries(entries);
  const rows: FileBrowserRow[] = [];
  const query = options.searchQuery?.trim().toLocaleLowerCase() ?? "";
  const compress = options.compressSingleDirectoryChains !== false;

  const append = (parent: Node, depth: number) => {
    const children = [...parent.children.values()]
      .filter((node) => !query || hasMatch(node, query))
      .sort((left, right) => compareNodes(left, right, options));

    for (const initialNode of children) {
      let node = initialNode;
      const chainPaths = [node.path];
      const labels = [node.name];

      if (compress && node.kind === "directory") {
        while (node.children.size === 1) {
          const onlyChild = node.children.values().next().value as Node | undefined;
          if (!onlyChild || onlyChild.kind !== "directory") break;
          node = onlyChild;
          chainPaths.push(node.path);
          labels.push(node.name);
        }
      }

      const label = labels.join("/");
      const expanded = node.kind === "directory" && (query !== "" || options.expandedPaths.has(node.path));
      rows.push({
        id: `${node.kind}:${node.path}`,
        path: node.path,
        label,
        kind: node.kind,
        depth,
        expanded,
        hasChildren: node.children.size > 0 || node.childState === "unknown" || node.childState === "loading",
        childState: node.childState,
        modifiedAt: node.modifiedAt,
        size: node.size,
        matchRanges: matchRanges(
          label.toLocaleLowerCase().includes(query) ? label : node.path,
          query,
        ),
        chainPaths,
      });

      if (node.kind === "directory" && expanded) append(node, depth + 1);
    }
  };

  append(root, 0);
  return rows;
}
