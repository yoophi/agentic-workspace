import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";

import type { GitWorktreeChanges, GitWorktreeFileDiff } from "@yoophi/git-graph";
import { WorktreeChangesView } from "@yoophi/git-ui";

import type { Repository } from "@/entities/repository";
import { sampleRepositories } from "@/shared/storybook/sample-data";
import { ChangesPanel } from "@/widgets/changes-panel";
import { RepositorySidebar } from "@/widgets/repository-sidebar";

const meta = {
  title: "Atomic Design/Organisms/Registered Components",
  parameters: {
    docs: {
      description: {
        component: "여러 feature/entity 데이터를 조합하고 사용자 워크플로를 소유하는 위젯입니다.",
      },
    },
  },
} satisfies Meta;

export default meta;
type Story = StoryObj<typeof meta>;

export const RepositorySelection: Story = {
  render: () => {
    const [selectedRepository, setSelectedRepository] = useState<Repository>(sampleRepositories[0]);

    return (
      <div className="h-[720px] max-w-sm overflow-hidden rounded-md border">
        <RepositorySidebar
          selectedRepositoryId={selectedRepository.id}
          onDeleteRepository={() => setSelectedRepository(sampleRepositories[0])}
          onSelectRepository={setSelectedRepository}
        />
      </div>
    );
  },
};

export const CommitInspection: Story = {
  render: () => (
    <div className="h-[720px] overflow-hidden rounded-md border">
      <ChangesPanel selectedRepository={sampleRepositories[0]} />
    </div>
  ),
};

export const EmptyCommitInspection: Story = {
  render: () => (
    <div className="h-[520px] overflow-hidden rounded-md border">
      <ChangesPanel />
    </div>
  ),
};

const sampleWorktreeChanges: GitWorktreeChanges = {
  workingDirectory: "/Users/dev/project/sample-repo",
  files: [
    {
      path: "src/merge-target.ts",
      oldPath: null,
      stagedStatus: "U",
      unstagedStatus: "U",
      group: "conflicted",
    },
    {
      path: "src/entities/repository/api.ts",
      oldPath: null,
      stagedStatus: "M",
      unstagedStatus: null,
      group: "staged",
    },
    {
      path: "src/widgets/changes-panel/ui/ChangesPanel.tsx",
      oldPath: "src/widgets/ChangesPanel.tsx",
      stagedStatus: "R",
      unstagedStatus: null,
      group: "staged",
    },
    {
      path: "src/lib/format.ts",
      oldPath: null,
      stagedStatus: null,
      unstagedStatus: "M",
      group: "unstaged",
    },
    {
      path: "docs/notes.md",
      oldPath: null,
      stagedStatus: null,
      unstagedStatus: null,
      group: "untracked",
    },
  ],
  stagedCount: 2,
  unstagedCount: 1,
  untrackedCount: 1,
  conflictedCount: 1,
};

const emptyWorktreeChanges: GitWorktreeChanges = {
  workingDirectory: "/Users/dev/project/sample-repo",
  files: [],
  stagedCount: 0,
  unstagedCount: 0,
  untrackedCount: 0,
  conflictedCount: 0,
};

const sampleWorktreeDiff: GitWorktreeFileDiff = {
  path: "src/lib/format.ts",
  content: `diff --git a/src/lib/format.ts b/src/lib/format.ts
index 1a2b3c4..5d6e7f8 100644
--- a/src/lib/format.ts
+++ b/src/lib/format.ts
@@ -1,6 +1,9 @@
 export function formatCount(count: number): string {
-  return String(count);
+  if (count > 999) {
+    return "999+";
+  }
+  return String(count);
 }
`,
  isBinary: false,
  isTruncated: false,
};

/** 미커밋(working-tree) 변경 뷰. 데이터·콜백을 props로 주입받는 공유 컴포넌트라
 * 스토리에서는 선택 상태만 로컬로 관리한다. */
export const WorktreeInspection: Story = {
  render: () => {
    const [selectedFilePath, setSelectedFilePath] = useState<string | undefined>(
      "src/lib/format.ts",
    );

    return (
      <div className="max-w-2xl rounded-md border p-3">
        <WorktreeChangesView
          changes={sampleWorktreeChanges}
          diff={selectedFilePath === "src/lib/format.ts" ? sampleWorktreeDiff : undefined}
          onSelectFile={setSelectedFilePath}
          selectedFilePath={selectedFilePath}
        />
      </div>
    );
  },
};

export const CleanWorktreeInspection: Story = {
  render: () => (
    <div className="max-w-2xl rounded-md border p-3">
      <WorktreeChangesView changes={emptyWorktreeChanges} onSelectFile={() => {}} />
    </div>
  ),
};

export const WorktreeInspectionLoadingDiff: Story = {
  render: () => (
    <div className="max-w-2xl rounded-md border p-3">
      <WorktreeChangesView
        changes={sampleWorktreeChanges}
        diffLoading
        onSelectFile={() => {}}
        selectedFilePath="src/lib/format.ts"
      />
    </div>
  ),
};

export const WorktreeInspectionDiffError: Story = {
  render: () => (
    <div className="max-w-2xl rounded-md border p-3">
      <WorktreeChangesView
        changes={sampleWorktreeChanges}
        diffError="Failed to read diff: repository is locked by another process."
        onSelectFile={() => {}}
        selectedFilePath="src/lib/format.ts"
      />
    </div>
  ),
};

export const WorktreeInspectionBinaryDiff: Story = {
  render: () => (
    <div className="max-w-2xl rounded-md border p-3">
      <WorktreeChangesView
        changes={sampleWorktreeChanges}
        diff={{ path: "assets/logo.png", content: "", isBinary: true, isTruncated: false }}
        onSelectFile={() => {}}
        selectedFilePath="assets/logo.png"
      />
    </div>
  ),
};

export const WorktreeInspectionTruncatedDiff: Story = {
  render: () => (
    <div className="max-w-2xl rounded-md border p-3">
      <WorktreeChangesView
        changes={sampleWorktreeChanges}
        diff={{ ...sampleWorktreeDiff, isTruncated: true }}
        diffClassName="max-h-64"
        onSelectFile={() => {}}
        selectedFilePath="src/lib/format.ts"
      />
    </div>
  ),
};
