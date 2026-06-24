import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { ChevronsUpDownIcon } from "lucide-react";

import { listGitBranches } from "@/entities/project/api/git-branch-repository";
import { projectQueryKeys } from "@/entities/project/api/query-keys";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { EllipsisPopoverText } from "@/shared/ui/ellipsis-popover-text";

type GitReferenceComboboxProps = {
  workingDirectory: string;
  value: string;
  onValueChange: (value: string) => void;
};

export function GitReferenceCombobox({
  workingDirectory,
  value,
  onValueChange,
}: GitReferenceComboboxProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [search, setSearch] = useState("");
  const branchesQuery = useQuery({
    queryKey: projectQueryKeys.gitBranches(workingDirectory),
    queryFn: () => listGitBranches(workingDirectory),
  });
  const branches = branchesQuery.data ?? [];
  const trimmedSearch = search.trim();
  const canUseCustomReference =
    trimmedSearch.length > 0 &&
    !branches.some((branch) => branch.name === trimmedSearch);

  function selectReference(reference: string) {
    onValueChange(reference);
    setSearch("");
    setIsOpen(false);
  }

  return (
    <Popover
      open={isOpen}
      onOpenChange={(open) => {
        setIsOpen(open);

        if (open) {
          setSearch("");
        }
      }}
    >
      <PopoverTrigger asChild>
        <Button
          type="button"
          variant="outline"
          className="w-full justify-between"
          aria-expanded={isOpen}
        >
          <EllipsisPopoverText
            value={value || "브랜치 또는 커밋 해시"}
            className="flex-1"
            focusable={false}
          />
          <ChevronsUpDownIcon data-icon="inline-end" />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-(--radix-popover-trigger-width) p-0" align="start">
        <Command shouldFilter={true}>
          <CommandInput
            placeholder="브랜치 검색 또는 커밋 해시 입력"
            value={search}
            onValueChange={setSearch}
          />
          <CommandList>
            <CommandEmpty>
              {trimmedSearch
                ? "입력값을 커밋 해시로 사용할 수 있습니다."
                : "브랜치가 없습니다."}
            </CommandEmpty>
            {canUseCustomReference && (
              <CommandGroup heading="직접 입력">
                <CommandItem
                  value={trimmedSearch}
                  onSelect={() => selectReference(trimmedSearch)}
                >
                  <EllipsisPopoverText
                    value={trimmedSearch}
                    className="font-mono"
                    contentClassName="font-mono text-xs"
                    focusable={false}
                  />
                </CommandItem>
              </CommandGroup>
            )}
            <CommandGroup heading="브랜치">
              {branches.map((branch) => (
                <CommandItem
                  key={branch.name}
                  value={branch.name}
                  data-checked={value === branch.name}
                  onSelect={() => selectReference(branch.name)}
                >
                  <EllipsisPopoverText value={branch.name} focusable={false} />
                  {branch.isCurrent && <Badge variant="secondary">current</Badge>}
                  {branch.isRemote && <Badge variant="outline">remote</Badge>}
                </CommandItem>
              ))}
            </CommandGroup>
          </CommandList>
        </Command>
        {branchesQuery.error && (
          <p className="px-3 py-2 text-sm text-destructive">
            {String(branchesQuery.error)}
          </p>
        )}
      </PopoverContent>
    </Popover>
  );
}
