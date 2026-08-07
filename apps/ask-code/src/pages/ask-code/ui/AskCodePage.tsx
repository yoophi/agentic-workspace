import type { LucideIcon } from "lucide-react";
import {
  BotMessageSquareIcon,
  CodeXmlIcon,
  FolderOpenIcon,
  FolderTreeIcon,
} from "lucide-react";

import { Button } from "@yoophi/ui/components/button";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@yoophi/ui/components/resizable";

type WorkspacePaneProps = {
  description: string;
  icon: LucideIcon;
  title: string;
};

function WorkspacePane({ description, icon: Icon, title }: WorkspacePaneProps) {
  return (
    <section className="flex h-full min-h-0 flex-col bg-background" aria-label={title}>
      <header className="flex h-10 shrink-0 items-center gap-2 border-b px-3">
        <Icon className="size-4 text-muted-foreground" aria-hidden="true" />
        <h2 className="text-sm font-medium">{title}</h2>
      </header>
      <div className="flex min-h-0 flex-1 items-center justify-center p-6">
        <p className="max-w-64 text-center text-sm text-muted-foreground">{description}</p>
      </div>
    </section>
  );
}

export function AskCodePage() {
  return (
    <main className="flex h-svh flex-col bg-background text-foreground">
      <header className="flex h-12 shrink-0 items-center justify-between gap-3 border-b px-4">
        <div className="flex min-w-0 items-center gap-2">
          <CodeXmlIcon className="size-5 text-muted-foreground" aria-hidden="true" />
          <h1 className="truncate text-sm font-semibold">Ask Code</h1>
        </div>
        <Button variant="outline" size="sm" disabled title="저장소 열기는 Phase 1에서 연결합니다.">
          <FolderOpenIcon data-icon="inline-start" />
          저장소 열기
        </Button>
      </header>

      <div className="min-h-0 flex-1">
        <ResizablePanelGroup>
          <ResizablePanel id="repository" defaultSize="260px" minSize="220px" maxSize="34%">
            <WorkspacePane
              icon={FolderTreeIcon}
              title="Repository"
              description="저장소를 열면 디렉터리와 파일이 여기에 표시됩니다."
            />
          </ResizablePanel>
          <ResizableHandle />
          <ResizablePanel id="source" minSize="360px">
            <WorkspacePane
              icon={CodeXmlIcon}
              title="Source"
              description="파일을 선택하면 줄 번호가 있는 읽기 전용 코드 뷰어가 열립니다."
            />
          </ResizablePanel>
          <ResizableHandle />
          <ResizablePanel id="questions" defaultSize="360px" minSize="300px" maxSize="42%">
            <WorkspacePane
              icon={BotMessageSquareIcon}
              title="Questions"
              description="선택한 코드에 질문 어노테이션을 작성하면 ACP 답변이 이곳에 스트리밍됩니다."
            />
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>
    </main>
  );
}
