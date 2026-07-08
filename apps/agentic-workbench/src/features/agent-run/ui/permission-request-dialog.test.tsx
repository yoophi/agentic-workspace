import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { PermissionEvent } from "@/entities/agent-run/model";
import { Dialog } from "@/components/ui/dialog";

import { PermissionRequestDialogFrame } from "./permission-request-dialog";

const longCommand = `gh issue create --body ${"long markdown ".repeat(420)}`;

const longJsonPermission: PermissionEvent = {
  type: "permission",
  permissionId: "perm-json",
  title: "Run shell command",
  input: {
    command: longCommand,
    cwd: "/Users/yoophi/project/agentic-workspace/apps/agentic-workbench",
    payload: {
      markdown: ["# Title", "- item", "```json", '{"deep":true}', "```"].join("\n"),
    },
  },
  options: [
    {
      optionId: "allow",
      kind: "allow_once",
      name: "Allow this command once",
    },
    {
      optionId: "reject",
      kind: "reject_once",
      name: "Reject",
    },
  ],
  requiresResponse: true,
};

const longApprovalPermission: PermissionEvent = {
  ...longJsonPermission,
  permissionId: "perm-approval",
  options: [
    {
      optionId: "allow-long",
      kind: "allow_once",
      name: `gh issue create --body ${"approval prefix ".repeat(80)}`,
    },
    {
      optionId: "reject-long",
      kind: "reject_once",
      name: `Do not allow ${"this risky command ".repeat(40)}`,
    },
  ],
};

describe("PermissionRequestDialogFrame", () => {
  function renderFrame(
    permission: PermissionEvent,
    props: Partial<React.ComponentProps<typeof PermissionRequestDialogFrame>> = {},
  ) {
    return renderToStaticMarkup(
      <Dialog open>
        <PermissionRequestDialogFrame
          permission={permission}
          onSelect={async () => undefined}
          {...props}
        />
      </Dialog>,
    );
  }

  it("renders long details separately from action controls", () => {
    const html = renderFrame(longJsonPermission);

    expect(html).toContain('data-testid="permission-detail"');
    expect(html).toContain('data-testid="permission-actions"');
    expect(html).toContain("gh issue create --body");
    expect(html).toContain("/Users/yoophi/project/agentic-workspace");
    expect(html).toContain('data-long="true"');
  });

  it("uses concise button labels while preserving full option text", () => {
    const html = renderFrame(longApprovalPermission);

    expect(html).toContain(">Allow</span>");
    expect(html).toContain(">Reject</span>");
    expect(html).toContain('data-testid="permission-option-detail"');
    expect(html).toContain("approval prefix");
    expect(html).toContain('data-option-id="allow-long"');
  });

  it("keeps pending labels short and disables actions while submitting", () => {
    const html = renderFrame(longApprovalPermission, {
      submittingOptionId: "allow-long",
      isSubmitting: true,
    });

    expect(html).toContain("Submitting");
    expect(html).toContain("disabled=\"\"");
  });

  it("renders focusable detail and action controls for narrow layouts", () => {
    const html = renderToStaticMarkup(
      <Dialog open>
        <div className="w-[360px]">
          <PermissionRequestDialogFrame
            permission={longJsonPermission}
            onSelect={async () => undefined}
          />
        </div>
      </Dialog>,
    );

    expect(html).toContain("tabindex=\"0\"");
    expect(html).toContain("sm:flex-wrap");
    expect(html).toContain("break-all");
    expect(html).toContain('data-option-id="allow"');
    expect(html).toContain('data-option-id="reject"');
  });
});
