import type { Meta, StoryObj } from "@storybook/react-vite";

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
