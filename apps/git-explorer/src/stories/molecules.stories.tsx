import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, within } from "storybook/test";

import { CollapsibleResizablePanels } from "@yoophi/ui/components/collapsible-resizable-panels";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@yoophi/ui/components/resizable";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@yoophi/ui/components/table";
import { sampleBranches, sampleCommitDetail, sampleWorktrees } from "@/shared/storybook/sample-data";

const meta = {
  title: "Atomic Design/Molecules/Registered Components",
  parameters: {
    docs: {
      description: {
        component: "여러 primitive를 조합해 특정 데이터 표현을 담당하는 컴포넌트 예시입니다.",
      },
    },
  },
} satisfies Meta;

export default meta;
type Story = StoryObj<typeof meta>;

function StatusBadge({ status }: { status: string }) {
  const colorByStatus: Record<string, string> = {
    A: "border-green-500/30 bg-green-500/10 text-green-700 dark:text-green-300",
    M: "border-yellow-500/30 bg-yellow-500/10 text-yellow-700 dark:text-yellow-300",
    D: "border-red-500/30 bg-red-500/10 text-red-700 dark:text-red-300",
    R: "border-blue-500/30 bg-blue-500/10 text-blue-700 dark:text-blue-300",
  };

  return (
    <span
      className={`rounded-sm border px-1.5 py-0.5 font-mono text-[10px] leading-none ${colorByStatus[status] ?? "border-border text-muted-foreground"}`}
    >
      {status}
    </span>
  );
}

export const Tables: Story = {
  render: () => (
    <div className="grid gap-6 lg:grid-cols-2">
      <section className="grid gap-2">
        <h3 className="text-sm font-medium">Worktree data</h3>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Path</TableHead>
              <TableHead className="w-32">Branch</TableHead>
              <TableHead className="w-20">Kind</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {sampleWorktrees.map((worktree) => (
              <TableRow key={worktree.path}>
                <TableCell className="max-w-0 truncate font-mono text-xs">
                  {worktree.path}
                </TableCell>
                <TableCell>{worktree.branch}</TableCell>
                <TableCell>{worktree.isMain ? "Main" : "Linked"}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </section>
      <section className="grid gap-2">
        <h3 className="text-sm font-medium">Changed file status data</h3>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-20">Status</TableHead>
              <TableHead>File</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {sampleCommitDetail.files.map((file) => (
              <TableRow key={`${file.status}:${file.path}`}>
                <TableCell>
                  <StatusBadge status={file.status} />
                </TableCell>
                <TableCell className="max-w-0 truncate font-mono text-xs">{file.path}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </section>
    </div>
  ),
};

export const ResizableColumns: Story = {
  render: () => (
    <div className="h-80 overflow-hidden rounded-md border">
      <ResizablePanelGroup>
        <ResizablePanel defaultSize="220px" minSize="180px">
          <div className="flex h-full flex-col gap-2 border-r p-3">
            <h3 className="text-sm font-medium">Branches</h3>
            {sampleBranches.map((branch) => (
              <div className="truncate text-sm" key={branch.fullName}>
                {branch.name}
              </div>
            ))}
          </div>
        </ResizablePanel>
        <ResizableHandle />
        <ResizablePanel>
          <div className="flex h-full flex-col gap-2 p-3">
            <h3 className="text-sm font-medium">Commit detail</h3>
            <p className="text-sm text-muted-foreground">
              Resizable pane 조합은 repository 정보, history, diff 같은 밀도 높은 데이터를
              나란히 표시할 때 사용합니다.
            </p>
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  ),
};

export const ResizableThreeColumns: Story = {
  render: () => (
    <div className="h-80 overflow-hidden rounded-md border">
      <ResizablePanelGroup orientation="horizontal">
        <ResizablePanel defaultSize="25%" id="three-column-navigation" minSize="15%">
          <div className="flex h-full flex-col gap-2 p-3">
            <h3 className="font-medium">Navigation</h3>
            <p className="text-sm text-muted-foreground">
              저장소와 브랜치 같은 탐색 항목을 표시합니다.
            </p>
          </div>
        </ResizablePanel>
        <ResizableHandle />
        <ResizablePanel defaultSize="45%" id="three-column-content" minSize="25%">
          <div className="flex h-full flex-col gap-2 p-3">
            <h3 className="font-medium">Content</h3>
            <p className="text-sm text-muted-foreground">
              선택한 항목의 주요 작업 콘텐츠를 표시합니다.
            </p>
          </div>
        </ResizablePanel>
        <ResizableHandle />
        <ResizablePanel defaultSize="30%" id="three-column-inspector" minSize="20%">
          <div className="flex h-full flex-col gap-2 p-3">
            <h3 className="font-medium">Inspector</h3>
            <p className="text-sm text-muted-foreground">
              메타데이터와 상세 정보를 보조 영역에 표시합니다.
            </p>
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const separators = canvas.getAllByRole("separator");
    await expect(separators).toHaveLength(2);
    separators[0]?.focus();
    await userEvent.keyboard("{ArrowRight}");
    await expect(separators[0]).toHaveAttribute("aria-valuenow");
  },
};

export const CollapsibleResizablePanelsBothOpen: Story = {
  render: () => (
    <div className="h-96 overflow-hidden rounded-md border">
      <CollapsibleResizablePanels
        leftPanel={{
          id: "storybook-panel-a",
          title: "Panel A",
          content: (
            <p className="text-sm text-muted-foreground">
              세로 구분선을 좌우로 끌어 두 패널의 너비를 조절합니다.
            </p>
          ),
          defaultSize: "40%",
          minSize: "20%",
          maxSize: "70%",
        }}
        rightPanel={{
          id: "storybook-panel-b",
          title: "Panel B",
          content: (
            <p className="text-sm text-muted-foreground">
              초기 너비는 서로 다르며 각 패널의 최소·최대 경계를 유지합니다.
            </p>
          ),
          defaultSize: "60%",
          minSize: "30%",
          maxSize: "80%",
        }}
      />
    </div>
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const separator = canvas.getByRole("separator");
    separator.focus();
    await userEvent.keyboard("{ArrowRight}");
    await expect(separator).not.toHaveAttribute("aria-disabled", "true");
    await expect(canvas.getByRole("button", { name: "Panel A" })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
  },
};

export const CollapsibleResizablePanelsOnlyPanelACollapsible: Story = {
  render: () => (
    <div className="h-96 overflow-hidden rounded-md border">
      <CollapsibleResizablePanels
        leftPanel={{
          id: "only-a-collapsible-panel-a",
          title: "Panel A",
          content: (
            <p className="text-sm text-muted-foreground">
              Panel A의 title 영역을 눌러 이 패널만 접고 펼칠 수 있습니다.
            </p>
          ),
          defaultSize: "35%",
          minSize: "20%",
        }}
        rightPanel={{
          id: "only-a-collapsible-panel-b",
          title: "Panel B",
          content: (
            <div className="flex h-full flex-col gap-2">
              <h3 className="font-medium">Panel B content</h3>
              <p className="text-sm text-muted-foreground">
                Panel B는 title 영역 없이 콘텐츠만 표시되며 접을 수 없습니다.
              </p>
            </div>
          ),
          collapsible: false,
          defaultSize: "65%",
          minSize: "30%",
          showTitle: false,
        }}
      />
    </div>
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.queryByRole("button", { name: "Panel B" })).not.toBeInTheDocument();
    const panelAButton = canvas.getByRole("button", { name: "Panel A" });
    await userEvent.click(panelAButton);
    await expect(panelAButton).toHaveAttribute("aria-expanded", "false");
    await expect(canvas.getByText("Panel B content")).toBeVisible();
    await userEvent.click(panelAButton);
    await expect(canvas.getByRole("button", { name: "Panel A" })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
  },
};

export const NestedCollapsibleResizablePanels: Story = {
  render: () => (
    <div className="h-[32rem] overflow-hidden rounded-md border">
      <CollapsibleResizablePanels
        leftPanel={{
          id: "nested-outer-panel-a",
          title: "Panel A",
          content: (
            <p className="text-sm text-muted-foreground">
              외부 Panel A는 독립적으로 접고 펼칠 수 있습니다.
            </p>
          ),
          defaultSize: "30%",
          minSize: "20%",
        }}
        rightPanel={{
          id: "nested-outer-panel-b",
          title: "Panel B",
          content: (
            <div className="h-full min-h-0 overflow-hidden">
              <CollapsibleResizablePanels
                leftPanel={{
                  id: "nested-inner-panel-b1",
                  title: "Panel B1",
                  content: (
                    <p className="text-sm text-muted-foreground">
                      B 내부의 첫 번째 패널입니다.
                    </p>
                  ),
                  defaultSize: "45%",
                  minSize: "25%",
                }}
                rightPanel={{
                  id: "nested-inner-panel-b2",
                  title: "Panel B2",
                  content: (
                    <p className="text-sm text-muted-foreground">
                      B 내부의 두 번째 패널입니다.
                    </p>
                  ),
                  defaultSize: "55%",
                  minSize: "25%",
                }}
              />
            </div>
          ),
          collapsible: false,
          contentClassName: "p-0",
          defaultSize: "70%",
          minSize: "40%",
          showTitle: false,
        }}
      />
    </div>
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.queryByRole("button", { name: "Panel B" })).not.toBeInTheDocument();
    await expect(canvas.getByRole("button", { name: "Panel A" })).toBeEnabled();
    await expect(canvas.getByRole("button", { name: "Panel B1" })).toBeEnabled();
    await expect(canvas.getByRole("button", { name: "Panel B2" })).toBeEnabled();
    await expect(canvas.getAllByRole("separator")).toHaveLength(2);

    const panelB1Button = canvas.getByRole("button", { name: "Panel B1" });
    await userEvent.click(panelB1Button);
    await expect(panelB1Button).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(panelB1Button);
    await expect(canvas.getByRole("button", { name: "Panel B1" })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
  },
};

export const CollapsibleResizablePanelsPanelACollapsed: Story = {
  render: () => (
    <div className="h-96 overflow-hidden rounded-md border">
      <CollapsibleResizablePanels
        leftPanel={{
          id: "collapsed-a-panel-a",
          title: "Panel A",
          content: "Panel A는 왼쪽의 회전된 title rail로 접힙니다.",
          defaultSize: "38%",
          minSize: "20%",
        }}
        rightPanel={{
          id: "collapsed-a-panel-b",
          title: "Panel B",
          content: "Panel B는 남은 가용 너비를 사용하며 제목이 비활성화됩니다.",
          defaultSize: "62%",
          minSize: "30%",
        }}
      />
    </div>
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const separator = canvas.getByRole("separator");
    separator.focus();
    await userEvent.keyboard("{ArrowRight}");
    const panelAButton = canvas.getByRole("button", { name: "Panel A" });
    await userEvent.click(panelAButton);
    await expect(panelAButton).toHaveAttribute("aria-expanded", "false");
    const collapsedPanel = panelAButton.closest<HTMLElement>("[data-panel]");
    await expect(collapsedPanel).not.toBeNull();
    await expect(collapsedPanel?.getBoundingClientRect().width).toBeLessThanOrEqual(41);
    const collapsedPanelRect = collapsedPanel?.getBoundingClientRect();
    const collapsedButtonRect = panelAButton.getBoundingClientRect();
    await expect(collapsedButtonRect.height).toBe(collapsedPanelRect?.height);
    await expect(collapsedButtonRect.width).toBe(collapsedPanelRect?.width);
    const collapsedLabel = panelAButton.querySelector("span");
    await expect(collapsedLabel).not.toBeNull();
    const collapsedLabelRect = collapsedLabel?.getBoundingClientRect();
    await expect(
      Math.abs(
        collapsedButtonRect.left + collapsedButtonRect.width / 2 -
          ((collapsedLabelRect?.left ?? 0) + (collapsedLabelRect?.width ?? 0) / 2),
      ),
    ).toBeLessThanOrEqual(1);
    await expect((collapsedLabelRect?.top ?? 0) - collapsedButtonRect.top).toBeLessThanOrEqual(9);
    await expect(canvas.getByRole("button", { name: "Panel B" })).toBeDisabled();
    await expect(canvas.getByRole("separator")).toHaveAttribute("aria-disabled", "true");

    for (const verticalRatio of [0.05, 0.5, 0.95]) {
      const collapsedButton = canvas.getByRole("button", { name: "Panel A" });
      const buttonRect = collapsedButton.getBoundingClientRect();
      collapsedButton.dispatchEvent(
        new MouseEvent("click", {
          bubbles: true,
          clientX: buttonRect.left + buttonRect.width / 2,
          clientY: buttonRect.top + buttonRect.height * verticalRatio,
        }),
      );
      await expect(canvas.getByRole("button", { name: "Panel A" })).toHaveAttribute(
        "aria-expanded",
        "true",
      );
      if (verticalRatio !== 0.95) {
        await userEvent.click(canvas.getByRole("button", { name: "Panel A" }));
        await expect(canvas.getByRole("button", { name: "Panel A" })).toHaveAttribute(
          "aria-expanded",
          "false",
        );
      }
    }

    const expandedPanelAButton = canvas.getByRole("button", { name: "Panel A" });
    expandedPanelAButton.focus();
    await userEvent.keyboard("{Enter}");
    const keyboardCollapsedButton = canvas.getByRole("button", { name: "Panel A" });
    await expect(keyboardCollapsedButton).toHaveAttribute("aria-expanded", "false");
    await expect(keyboardCollapsedButton.querySelector("span")).not.toHaveAttribute("tabindex");
    keyboardCollapsedButton.focus();
    await userEvent.keyboard(" ");
    await expect(canvas.getByRole("button", { name: "Panel A" })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
  },
};

export const CollapsibleResizablePanelsPanelBCollapsed: Story = {
  render: () => (
    <div className="h-96 overflow-hidden rounded-md border">
      <CollapsibleResizablePanels
        leftPanel={{
          id: "collapsed-b-panel-a",
          title: "Panel A",
          content: "Panel A는 남은 가용 너비를 사용하며 제목이 비활성화됩니다.",
          defaultSize: "58%",
          minSize: "30%",
        }}
        rightPanel={{
          id: "collapsed-b-panel-b",
          title: "Panel B",
          content: "Panel B는 오른쪽의 회전된 title rail로 접힙니다.",
          defaultSize: "42%",
          minSize: "20%",
        }}
      />
    </div>
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const panelBButton = canvas.getByRole("button", { name: "Panel B" });
    await userEvent.click(panelBButton);
    await expect(panelBButton).toHaveAttribute("aria-expanded", "false");
    const collapsedPanel = panelBButton.closest<HTMLElement>("[data-panel]");
    await expect(collapsedPanel).not.toBeNull();
    const collapsedPanelRect = collapsedPanel?.getBoundingClientRect();
    const collapsedButtonRect = panelBButton.getBoundingClientRect();
    await expect(collapsedButtonRect.height).toBe(collapsedPanelRect?.height);
    await expect(collapsedButtonRect.width).toBe(collapsedPanelRect?.width);
    const collapsedLabel = panelBButton.querySelector("span");
    await expect(collapsedLabel).not.toBeNull();
    const collapsedLabelRect = collapsedLabel?.getBoundingClientRect();
    await expect(
      Math.abs(
        collapsedButtonRect.left + collapsedButtonRect.width / 2 -
          ((collapsedLabelRect?.left ?? 0) + (collapsedLabelRect?.width ?? 0) / 2),
      ),
    ).toBeLessThanOrEqual(1);
    await expect((collapsedLabelRect?.top ?? 0) - collapsedButtonRect.top).toBeLessThanOrEqual(9);
    await expect(canvas.getByRole("button", { name: "Panel A" })).toBeDisabled();
    await expect(canvas.getByRole("separator")).toHaveAttribute("aria-disabled", "true");
  },
};

export const CollapsibleResizablePanelsLongTitle: Story = {
  render: () => (
    <div className="h-96 overflow-hidden rounded-md border">
      <CollapsibleResizablePanels
        leftPanel={{
          id: "long-title-panel-a",
          title: "Panel A — 매우 긴 탐색 및 변경 파일 목록 제목",
          content: "긴 제목은 펼침 상태에서 잘리고 회전 상태에서는 rail 밖으로 넘치지 않습니다.",
          defaultSize: "34%",
          minSize: "20%",
        }}
        rightPanel={{
          id: "long-title-panel-b",
          title: "Panel B — 선택한 파일의 상세 미리보기 제목",
          content: "접근 가능한 이름은 시각적 잘림과 관계없이 유지됩니다.",
          defaultSize: "66%",
          minSize: "30%",
        }}
      />
    </div>
  ),
};

export const CollapsibleResizablePanelsEmptyContent: Story = {
  render: () => (
    <div className="h-96 overflow-hidden rounded-md border">
      <CollapsibleResizablePanels
        leftPanel={{
          id: "empty-content-panel-a",
          title: "Empty Panel",
          content: null,
          defaultSize: "45%",
          minSize: "20%",
        }}
        rightPanel={{
          id: "empty-content-panel-b",
          title: "Available Panel",
          content: "한쪽 콘텐츠가 비어 있어도 제목과 separator는 계속 사용할 수 있습니다.",
          defaultSize: "55%",
          minSize: "20%",
        }}
      />
    </div>
  ),
};

export const CollapsibleResizablePanelsOverflowingContent: Story = {
  render: () => (
    <div className="h-96 overflow-hidden rounded-md border">
      <CollapsibleResizablePanels
        leftPanel={{
          id: "overflow-panel-a",
          title: "Files",
          content: (
            <div className="flex flex-col gap-2">
              {Array.from({ length: 40 }, (_, index) => (
                <div className="truncate text-sm" key={index}>
                  src/features/example/very-long-file-name-{index + 1}.tsx
                </div>
              ))}
            </div>
          ),
          defaultSize: "32%",
          minSize: "20%",
        }}
        rightPanel={{
          id: "overflow-panel-b",
          title: "Preview",
          content: (
            <p className="min-w-[720px] text-sm text-muted-foreground">
              매우 넓고 긴 콘텐츠는 자체 영역에서 스크롤되며 패널 제목과 separator를 밀어내지
              않습니다.
            </p>
          ),
          defaultSize: "68%",
          minSize: "30%",
        }}
      />
    </div>
  ),
};
