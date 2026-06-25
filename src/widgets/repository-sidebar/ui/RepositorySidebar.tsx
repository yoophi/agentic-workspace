import { ChangeEvent, FormEvent, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import {
  AlertCircle,
  Check,
  FolderOpen,
  GitBranch,
  History,
  Loader2,
  Plus,
  Search,
} from "lucide-react";
import { Button } from "@yoophi/ui/components/button";
import { Input } from "@yoophi/ui/components/input";
import {
  createRepository,
  listRepositories,
  repositoryKeys,
  startRepositoryWatchers,
  type Repository,
} from "@/entities/repository";

type RepositorySidebarProps = {
  selectedRepositoryId?: string;
  onSelectRepository: (repository: Repository) => void;
};

function getErrorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

export function RepositorySidebar({
  selectedRepositoryId,
  onSelectRepository,
}: RepositorySidebarProps) {
  const [path, setPath] = useState("");
  const [search, setSearch] = useState("");
  const queryClient = useQueryClient();

  const repositoriesQuery = useQuery({
    queryKey: repositoryKeys.all,
    queryFn: listRepositories,
  });

  const createRepositoryMutation = useMutation({
    mutationFn: createRepository,
    onSuccess: (repository) => {
      setPath("");
      queryClient.setQueryData<Repository[]>(repositoryKeys.all, (repositories = []) => [
        ...repositories,
        repository,
      ]);
      onSelectRepository(repository);
      startRepositoryWatchers().catch((error) => {
        console.error("Failed to refresh repository watchers", error);
      });
    },
  });
  const isRegistering = createRepositoryMutation.isPending;

  const repositories = repositoriesQuery.data ?? [];
  const visibleRepositories = useMemo(() => {
    const keyword = search.trim().toLowerCase();

    if (!keyword) {
      return repositories;
    }

    return repositories.filter(
      (repository) =>
        repository.name.toLowerCase().includes(keyword) ||
        repository.path.toLowerCase().includes(keyword),
    );
  }, [repositories, search]);

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    createRepositoryMutation.mutate(path);
  }

  function handlePathChange(event: ChangeEvent<HTMLInputElement>) {
    setPath(event.target.value);
  }

  function handleSearchChange(event: ChangeEvent<HTMLInputElement>) {
    setSearch(event.target.value);
  }

  async function handleSelectDirectory() {
    const selectedPath = await open({
      directory: true,
      multiple: false,
      title: "Select Git repository",
    });

    if (typeof selectedPath !== "string") {
      return;
    }

    setPath(selectedPath);
    createRepositoryMutation.mutate(selectedPath);
  }

  return (
    <aside className="flex h-full min-h-0 flex-col border-r bg-sidebar">
      <header className="border-b px-3 py-3">
        <div className="flex items-center gap-2">
          <GitBranch className="size-4 text-muted-foreground" />
          <h1 className="text-sm font-medium">Git Explorer</h1>
        </div>
        <form className="mt-3 grid gap-2" onSubmit={handleSubmit}>
          <div className="flex gap-2">
            <Input
              aria-label="Repository path"
              value={path}
              onChange={handlePathChange}
              placeholder="/path/to/git/repo"
            />
            <Button
              aria-label="Select repository folder"
              disabled={isRegistering}
              size="icon"
              type="button"
              variant="outline"
              onClick={handleSelectDirectory}
            >
              <FolderOpen />
            </Button>
            <Button
              aria-label="Register repository"
              disabled={isRegistering}
              size="icon"
              type="submit"
            >
              {isRegistering ? <Loader2 className="animate-spin" /> : <Plus />}
            </Button>
          </div>
          {createRepositoryMutation.isError ? (
            <p className="flex items-start gap-1.5 text-xs leading-5 text-red-600">
              <AlertCircle className="mt-0.5 size-3.5 shrink-0" />
              <span>{getErrorMessage(createRepositoryMutation.error)}</span>
            </p>
          ) : null}
        </form>
        <div className="relative mt-3">
          <Search className="pointer-events-none absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
          <Input
            className="pl-7"
            value={search}
            onChange={handleSearchChange}
            placeholder="Search repositories"
          />
        </div>
      </header>
      <nav className="min-h-0 flex-1 overflow-auto p-2">
        {repositoriesQuery.isLoading ? (
          <p className="flex items-center gap-2 px-2 py-3 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            Loading repositories
          </p>
        ) : null}
        {repositoriesQuery.isError ? (
          <p className="flex items-start gap-1.5 px-2 py-3 text-sm leading-5 text-red-600">
            <AlertCircle className="mt-0.5 size-4 shrink-0" />
            <span>{getErrorMessage(repositoriesQuery.error)}</span>
          </p>
        ) : null}
        {!repositoriesQuery.isLoading && !repositoriesQuery.isError && repositories.length === 0 ? (
          <p className="px-2 py-3 text-sm leading-5 text-muted-foreground">
            Register a local Git repository to start tracking it.
          </p>
        ) : null}
        {visibleRepositories.map((repository) => {
          const isSelected = repository.id === selectedRepositoryId;

          return (
            <button
              className="flex w-full items-center gap-2 rounded-md px-2 py-2 text-left text-sm hover:bg-muted data-[selected=true]:bg-muted"
              data-selected={isSelected}
              key={repository.id}
              type="button"
              onClick={() => onSelectRepository(repository)}
            >
              <History className="size-4 text-muted-foreground" />
              <span className="min-w-0 flex-1">
                <span className="block truncate font-medium">{repository.name}</span>
                <span className="block truncate text-xs text-muted-foreground">
                  {repository.path}
                </span>
              </span>
              {isSelected ? <Check className="size-4 text-muted-foreground" /> : null}
            </button>
          );
        })}
      </nav>
    </aside>
  );
}
