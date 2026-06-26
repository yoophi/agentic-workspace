import type { Meta, StoryObj } from "@storybook/react-vite";
import { FolderOpen, Plus, RefreshCw, Search, Trash2 } from "lucide-react";

import { Button } from "@yoophi/ui/components/button";
import { Input } from "@yoophi/ui/components/input";

const meta = {
  title: "Atomic Design/Atoms/Registered Components",
  parameters: {
    docs: {
      description: {
        component: "단일 UI primitive와 작고 독립적인 입력/액션 요소입니다.",
      },
    },
  },
} satisfies Meta;

export default meta;
type Story = StoryObj<typeof meta>;

export const Buttons: Story = {
  render: () => (
    <div className="grid gap-5">
      <section className="grid gap-2">
        <h3 className="text-sm font-medium">Repository actions</h3>
        <div className="flex flex-wrap items-center gap-2">
          <Button>
            <Plus />
            Add repository
          </Button>
          <Button variant="secondary">
            <RefreshCw />
            Refresh
          </Button>
          <Button variant="outline">
            <FolderOpen />
            Browse
          </Button>
          <Button variant="ghost">
            <Search />
            Search
          </Button>
        </div>
      </section>
      <section className="grid gap-2">
        <h3 className="text-sm font-medium">Icon and disabled states</h3>
        <div className="flex flex-wrap items-center gap-2">
          <Button aria-label="Refresh repository data" size="icon">
            <RefreshCw />
          </Button>
          <Button aria-label="Delete repository" size="icon-sm" variant="ghost">
            <Trash2 />
          </Button>
          <Button disabled>Disabled</Button>
        </div>
      </section>
    </div>
  ),
};

export const Inputs: Story = {
  render: () => (
    <div className="grid max-w-2xl gap-5">
      <label className="grid gap-2 text-sm font-medium">
        Repository path
        <Input defaultValue="/Users/yoophi/project/tauri-git-explorer" />
      </label>
      <label className="grid gap-2 text-sm font-medium">
        Long worktree path
        <Input defaultValue="/Users/yoophi/project/worktrees/tauri-git-explorer/feature/storybook-long-path-layout-validation" />
      </label>
      <label className="grid gap-2 text-sm font-medium">
        Search
        <Input placeholder="Search repositories" />
      </label>
      <label className="grid gap-2 text-sm font-medium">
        Disabled
        <Input disabled value="Repository watcher is running" />
      </label>
    </div>
  ),
};
