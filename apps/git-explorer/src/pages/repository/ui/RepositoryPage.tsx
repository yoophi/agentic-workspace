import { useState } from "react";
import type { Layout } from "react-resizable-panels";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@yoophi/ui/components/resizable";
import type { Repository } from "@/entities/repository";
import { ChangesPanel } from "@/widgets/changes-panel";
import { RepositorySidebar } from "@/widgets/repository-sidebar";

const LAYOUT_STORAGE_KEY = "repository-layout";

function loadLayout(): Layout | undefined {
  try {
    return JSON.parse(localStorage.getItem(LAYOUT_STORAGE_KEY) ?? "");
  } catch {
    return undefined;
  }
}

function saveLayout(layout: Layout) {
  localStorage.setItem(LAYOUT_STORAGE_KEY, JSON.stringify(layout));
}

export function RepositoryPage() {
  const [selectedRepository, setSelectedRepository] = useState<Repository>();

  function handleDeleteRepository(repository: Repository) {
    if (repository.id === selectedRepository?.id) {
      setSelectedRepository(undefined);
    }
  }

  return (
    <main className="h-svh bg-background text-foreground">
      <ResizablePanelGroup defaultLayout={loadLayout()} onLayoutChanged={saveLayout}>
        <ResizablePanel id="repository-sidebar" defaultSize="300px" minSize="260px" maxSize="42%">
          <RepositorySidebar
            selectedRepositoryId={selectedRepository?.id}
            onDeleteRepository={handleDeleteRepository}
            onSelectRepository={setSelectedRepository}
          />
        </ResizablePanel>
        <ResizableHandle />
        <ResizablePanel id="changes">
          <ChangesPanel selectedRepository={selectedRepository} />
        </ResizablePanel>
      </ResizablePanelGroup>
    </main>
  );
}
