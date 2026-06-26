import { listen } from "@tauri-apps/api/event";
import { QueryClient, QueryClientProvider, useQueryClient } from "@tanstack/react-query";
import { PropsWithChildren, useEffect, useState } from "react";
import {
  repositoryKeys,
  startRepositoryWatchers,
  stopRepositoryWatchers,
} from "@/entities/repository";

type RepositoryChangedEvent = {
  repositoryId: string;
  path: string;
};

function RepositoryInvalidationBridge() {
  const queryClient = useQueryClient();

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    startRepositoryWatchers().catch((error) => {
      console.error("Failed to start repository watchers", error);
    });

    listen<RepositoryChangedEvent>("repository-changed", (event) => {
      const repositoryId = event.payload.repositoryId;

      void queryClient.invalidateQueries({ queryKey: repositoryKeys.worktrees(repositoryId) });
      void queryClient.invalidateQueries({ queryKey: repositoryKeys.branches(repositoryId) });
      void queryClient.invalidateQueries({ queryKey: repositoryKeys.historyRoot(repositoryId) });
      void queryClient.invalidateQueries({ queryKey: repositoryKeys.commitGraphRoot(repositoryId) });
      void queryClient.invalidateQueries({
        queryKey: ["repositories", repositoryId, "commits"],
      });
    })
      .then((dispose) => {
        unlisten = dispose;
      })
      .catch((error) => {
        console.error("Failed to listen for repository changes", error);
      });

    return () => {
      unlisten?.();
      stopRepositoryWatchers().catch((error) => {
        console.error("Failed to stop repository watchers", error);
      });
    };
  }, [queryClient]);

  return null;
}

export function QueryProvider({ children }: PropsWithChildren) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            refetchOnWindowFocus: false,
          },
        },
      }),
  );

  return (
    <QueryClientProvider client={queryClient}>
      <RepositoryInvalidationBridge />
      {children}
    </QueryClientProvider>
  );
}
