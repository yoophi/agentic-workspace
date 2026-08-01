import { useEffect, useMemo, useRef, useState } from "react";

import type { FileBrowserTreeProps } from "./types";
import { nextTreeIndex, parentRowIndex } from "./use-tree-keyboard-navigation";

export function FileBrowserTree({
  rows,
  selectedPath = null,
  activePath = null,
  height = 400,
  rowHeight = 32,
  overscan = 4,
  ariaLabel,
  className,
  style,
  renderRow,
  onSelect,
  onToggle,
}: FileBrowserTreeProps) {
  const initialIndex = Math.max(0, rows.findIndex((row) => row.path === (activePath ?? selectedPath)));
  const [activeIndex, setActiveIndex] = useState(initialIndex);
  const [scrollTop, setScrollTop] = useState(0);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const index = rows.findIndex((row) => row.path === activePath);
    if (index >= 0) setActiveIndex(index);
  }, [activePath, rows]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container || activeIndex < 0) return;
    const top = activeIndex * rowHeight;
    const bottom = top + rowHeight;
    if (top < container.scrollTop) container.scrollTop = top;
    else if (bottom > container.scrollTop + height) container.scrollTop = bottom - height;
    setScrollTop(container.scrollTop);
    requestAnimationFrame(() => {
      container.querySelector<HTMLElement>(`[data-tree-index="${activeIndex}"]`)?.focus();
    });
  }, [activeIndex, height, rowHeight]);

  const range = useMemo(() => {
    const start = Math.max(0, Math.floor(scrollTop / rowHeight) - overscan);
    const count = Math.ceil(height / rowHeight) + overscan * 2;
    return { start, end: Math.min(rows.length, start + count) };
  }, [height, overscan, rowHeight, rows.length, scrollTop]);

  return (
    <div
      ref={containerRef}
      role="tree"
      aria-label={ariaLabel}
      className={className}
      style={{ ...style, height, overflow: "auto", position: "relative" }}
      onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
    >
      <div style={{ height: rows.length * rowHeight, position: "relative" }}>
        {rows.slice(range.start, range.end).map((row, offset) => {
          const index = range.start + offset;
          const selected = row.path === selectedPath;
          const active = index === activeIndex;
          return (
            <div
              key={row.id}
              role="treeitem"
              aria-level={row.depth + 1}
              aria-expanded={row.kind === "directory" ? row.expanded : undefined}
              aria-selected={selected}
              tabIndex={active ? 0 : -1}
              data-tree-index={index}
              style={{ position: "absolute", top: index * rowHeight, height: rowHeight, width: "100%" }}
              onFocus={() => setActiveIndex(index)}
              onDoubleClick={() => row.kind === "directory" ? onToggle(row) : onSelect(row)}
              onKeyDown={(event) => {
                const next = nextTreeIndex(event, rows, index);
                if (next !== index) {
                  event.preventDefault();
                  setActiveIndex(next);
                  return;
                }
                if (event.key === "ArrowRight" && row.kind === "directory" && !row.expanded) {
                  event.preventDefault(); onToggle(row);
                } else if (event.key === "ArrowLeft" && row.kind === "directory" && row.expanded) {
                  event.preventDefault(); onToggle(row);
                } else if (event.key === "ArrowLeft") {
                  const parent = parentRowIndex(rows, index);
                  if (parent !== index) { event.preventDefault(); setActiveIndex(parent); }
                } else if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  row.kind === "directory" ? onToggle(row) : onSelect(row);
                }
              }}
              onClick={() => row.kind === "directory" ? onToggle(row) : onSelect(row)}
            >
              {renderRow(row, { active, selected })}
            </div>
          );
        })}
      </div>
    </div>
  );
}
