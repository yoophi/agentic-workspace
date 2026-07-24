import { describe, expect, it } from "vitest";
import { parseTasksKanban } from "@/features/worktree-workspace/model/tasks-kanban";

describe("tasks kanban parser", () => {
  it("keeps only incomplete task sections in needed tasks", () => {
    const result = parseTasksKanban("# Done\n- [x] ship\n# Next\n설명\n- [ ] build");
    expect(result.items).toHaveLength(2);
    expect(result.neededSections).toEqual([expect.objectContaining({ heading: "Next", context: ["설명"], tasks: [expect.objectContaining({ text: "build", completed: false })] })]);
  });
});
