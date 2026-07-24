import { useMemo, useState } from "react";
import { MarkdownViewer } from "@yoophi/markdown-annotation-react";
import { transformWikilinks } from "@yoophi/markdown-annotation-core";
import type { MarkdownBlock } from "@yoophi/markdown-annotation-core/types";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

import { Button } from "@/components/ui/button";
import { parseTasksKanban } from "@/features/worktree-workspace/model/tasks-kanban";
import { markdownViewerComponents } from "@/features/worktree-workspace/ui/markdown-viewer-components";

export function TasksKanbanPanel({ content, blocks }: { content: string; blocks: MarkdownBlock[] }) {
  const [mode, setMode] = useState<"all" | "open" | "done">("all");
  const [density, setDensity] = useState<"compact" | "detailed">("compact");
  const data = useMemo(() => parseTasksKanban(content), [content]);
  const visible = data.items.filter((task) => mode === "all" || (mode === "open" ? !task.completed : task.completed));

  return <div className="p-4"><Controls density={density} mode={mode} onDensity={setDensity} onMode={setMode} />
    {density === "detailed" ? <DetailedMarkdown blocks={blocks} mode={mode} /> : <div className="grid gap-3 md:grid-cols-2"><TaskColumn title="진행할 작업" tasks={mode === "done" ? [] : visible.filter((task) => !task.completed)} /><TaskColumn title="완료된 작업" tasks={mode === "open" ? [] : visible.filter((task) => task.completed)} /></div>}
  </div>;
}

function Controls({ mode, density, onMode, onDensity }: { mode: "all" | "open" | "done"; density: "compact" | "detailed"; onMode: (mode: "all" | "open" | "done") => void; onDensity: (density: "compact" | "detailed") => void }) {
  return <div className="mb-3 flex flex-wrap items-center gap-2"><Button size="sm" variant={mode === "all" ? "secondary" : "outline"} onClick={() => onMode("all")}>모든 작업</Button><Button size="sm" variant={mode === "done" ? "secondary" : "outline"} onClick={() => onMode("done")}>완료된 작업</Button><Button size="sm" variant={mode === "open" ? "secondary" : "outline"} onClick={() => onMode("open")}>미완료 작업</Button><span className="ml-auto text-xs text-muted-foreground">표시 밀도</span><Button size="sm" variant={density === "compact" ? "secondary" : "outline"} onClick={() => onDensity("compact")}>간단히</Button><Button size="sm" variant={density === "detailed" ? "secondary" : "outline"} onClick={() => onDensity("detailed")}>자세히</Button></div>;
}

function DetailedMarkdown({ blocks, mode }: { blocks: MarkdownBlock[]; mode: "all" | "open" | "done" }) {
  const visibleBlocks = mode === "open" ? getIncompleteTaskSectionBlocks(blocks) : blocks;
  const segments = groupDetailedBlocks(visibleBlocks);

  return (
    <div>
      {segments.map((segment, index) =>
        segment.kind === "markdown" ? (
          <MarkdownViewer blocks={segment.blocks} components={markdownViewerComponents} key={`markdown-${index}`} />
        ) : (
          <section className="my-3 rounded-md border bg-muted/20 p-3" key={`tasks-${index}`}>
            <DetailedTaskSwimlanes blocks={segment.blocks} mode={mode} />
          </section>
        ),
      )}
    </div>
  );
}

function getIncompleteTaskSectionBlocks(blocks: MarkdownBlock[]): MarkdownBlock[] {
  const sections = blocks.reduce<MarkdownBlock[][]>((result, block) => {
    if (block.type === "heading" || result.length === 0) {
      result.push([]);
    }
    result[result.length - 1]?.push(block);
    return result;
  }, []);

  return sections.flatMap((section) => {
    const hasIncompleteTask = section.some((block) => block.type === "list-item" && block.checked === false);
    return hasIncompleteTask
      ? section.filter((block) => !(block.type === "list-item" && block.checked === true))
      : [];
  });
}

function DetailedTaskSwimlanes({ blocks, mode }: { blocks: MarkdownBlock[]; mode: "all" | "open" | "done" }) {
  const tasks = blocks.map((block) => ({
    id: block.id,
    text: block.content,
    completed: block.checked ?? false,
    sectionId: "detailed",
  }));

  return (
    <div className="grid gap-3 md:grid-cols-2">
      {mode !== "done" ? <TaskSwimlane title="Todo" tasks={tasks.filter((task) => !task.completed)} /> : null}
      {mode !== "open" ? <TaskSwimlane title="Done" tasks={tasks.filter((task) => task.completed)} /> : null}
    </div>
  );
}

function TaskSwimlane({ title, tasks }: { title: string; tasks: ReturnType<typeof parseTasksKanban>["items"] }) {
  return (
    <section>
      <h3 className="font-medium">{title}</h3>
      <div className="mt-3 grid gap-2">
        {tasks.map((task) => <TaskCard key={task.id} task={task} />)}
      </div>
      {tasks.length === 0 ? <p className="mt-3 text-sm text-muted-foreground">표시할 작업이 없습니다.</p> : null}
    </section>
  );
}

type DetailedBlockSegment = { kind: "markdown" | "tasks"; blocks: MarkdownBlock[] };

function groupDetailedBlocks(blocks: MarkdownBlock[]): DetailedBlockSegment[] {
  return blocks.reduce<DetailedBlockSegment[]>((segments, block) => {
    const kind = block.type === "list-item" && block.checked !== undefined ? "tasks" : "markdown";
    const previous = segments[segments.length - 1];

    if (previous?.kind === kind) {
      previous.blocks.push(block);
    } else {
      segments.push({ kind, blocks: [block] });
    }

    return segments;
  }, []);
}

function TaskColumn({ title, tasks }: { title: string; tasks: ReturnType<typeof parseTasksKanban>["items"] }) { return <section className="rounded-md border bg-muted/20 p-3"><h3 className="font-medium">{title}</h3><div className="mt-3 grid gap-2">{tasks.map((task) => <TaskCard key={task.id} task={task} />)}</div>{tasks.length === 0 ? <p className="mt-3 text-sm text-muted-foreground">표시할 작업이 없습니다.</p> : null}</section>; }
function TaskCard({ task }: { task: ReturnType<typeof parseTasksKanban>["items"][number] }) {
  return (
    <article className="rounded-md border bg-background p-3 shadow-sm">
      <span className="text-xs text-muted-foreground">{task.completed ? "완료" : "진행 필요"}</span>
      <div className="mt-1 text-sm leading-6">
        <ReactMarkdown components={{ p: ({ children }) => <>{children}</> }} remarkPlugins={[remarkGfm]}>
          {transformWikilinks(task.text)}
        </ReactMarkdown>
      </div>
    </article>
  );
}
