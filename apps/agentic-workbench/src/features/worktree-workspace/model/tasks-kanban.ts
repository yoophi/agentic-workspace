export type TaskKanbanItem = { id: string; text: string; completed: boolean; sectionId: string };
export type NeededTaskSection = { id: string; heading: string; context: string[]; tasks: TaskKanbanItem[] };

export function parseTasksKanban(content: string) {
  const sections: NeededTaskSection[] = [];
  let current: NeededTaskSection = { id: "root", heading: "작업", context: [], tasks: [] };
  sections.push(current);
  content.split("\n").forEach((line, index) => {
    const heading = line.match(/^#{1,6}\s+(.+)$/);
    if (heading) { current = { id: `section-${index}`, heading: heading[1] ?? "작업", context: [], tasks: [] }; sections.push(current); return; }
    const task = line.match(/^\s*[-*]\s+\[([ xX])\]\s+(.+)$/);
    if (task) { current.tasks.push({ id: `task-${index}`, text: task[2] ?? "", completed: task[1]?.toLowerCase() === "x", sectionId: current.id }); return; }
    if (line.trim()) current.context.push(line);
  });
  const items = sections.flatMap((section) => section.tasks);
  return { sections, items, neededSections: sections.filter((section) => section.tasks.some((task) => !task.completed)).map((section) => ({ ...section, tasks: section.tasks.filter((task) => !task.completed) })) };
}
