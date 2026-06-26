import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";

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
