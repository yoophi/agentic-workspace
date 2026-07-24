import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { SddWorkflowControls } from "@/features/worktree-workspace/ui/sdd-workflow-controls";

describe("SddWorkflowControls", () => {
  it("shows ordered stages and unavailable guidance", () => {
    const html = renderToStaticMarkup(<SddWorkflowControls pointer={{ status: "unavailable", featurePath: null, reason: "missing" }} stages={["specify", "plan", "tasks", "implement"].map((stage) => ({ stage: stage as "specify", status: "unavailable" as const, canStart: stage === "specify", requiresConfirmation: false }))} onRequest={() => undefined} />);
    expect(html).toContain("SDD workflow");
    expect(html).toContain("초안 주입");
    expect(html).toContain("missing");
  });
});
