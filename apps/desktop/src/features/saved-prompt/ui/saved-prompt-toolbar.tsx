import { FormEvent, useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { PencilIcon, PlusIcon, Trash2Icon } from "lucide-react";

import {
  createSavedPrompt,
  deleteSavedPrompt,
  listSavedPrompts,
  updateSavedPrompt,
} from "@/entities/saved-prompt/api/saved-prompt-repository";
import { savedPromptQueryKeys } from "@/entities/saved-prompt/api/query-keys";
import type {
  SavedPrompt,
  SavedPromptInput,
} from "@/entities/saved-prompt/model/types";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Field,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { PromptSuggestion } from "@/components/ui/prompt-suggestion";
import { Textarea } from "@/components/ui/textarea";

type SavedPromptToolbarProps = {
  disabled?: boolean;
  onSendPrompt: (prompt: string) => void;
};

const emptyForm: SavedPromptInput = {
  label: "",
  prompt: "",
};

export function SavedPromptToolbar({
  disabled = false,
  onSendPrompt,
}: SavedPromptToolbarProps) {
  const queryClient = useQueryClient();
  const [dialogMode, setDialogMode] = useState<"create" | "edit" | null>(null);
  const [editingPrompt, setEditingPrompt] = useState<SavedPrompt | null>(null);
  const [form, setForm] = useState<SavedPromptInput>(emptyForm);
  const [error, setError] = useState<string | null>(null);
  const savedPromptsQuery = useQuery({
    queryKey: savedPromptQueryKeys.all,
    queryFn: listSavedPrompts,
  });
  const savedPrompts = savedPromptsQuery.data ?? [];

  const createMutation = useMutation({
    mutationFn: createSavedPrompt,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: savedPromptQueryKeys.all });
    },
  });
  const updateMutation = useMutation({
    mutationFn: ({ id, input }: { id: string; input: SavedPromptInput }) =>
      updateSavedPrompt(id, input),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: savedPromptQueryKeys.all });
    },
  });
  const deleteMutation = useMutation({
    mutationFn: deleteSavedPrompt,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: savedPromptQueryKeys.all });
    },
  });

  useEffect(() => {
    if (!dialogMode) {
      setEditingPrompt(null);
      setForm(emptyForm);
      setError(null);
    }
  }, [dialogMode]);

  function openCreateDialog() {
    setForm(emptyForm);
    setEditingPrompt(null);
    setDialogMode("create");
  }

  function openEditDialog(prompt: SavedPrompt) {
    setForm({
      label: prompt.label,
      prompt: prompt.prompt,
    });
    setEditingPrompt(prompt);
    setDialogMode("edit");
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);

    try {
      if (dialogMode === "edit" && editingPrompt) {
        await updateMutation.mutateAsync({ id: editingPrompt.id, input: form });
      } else {
        await createMutation.mutateAsync(form);
      }

      setDialogMode(null);
    } catch (caughtError) {
      setError(String(caughtError));
    }
  }

  async function handleDelete(id: string) {
    setError(null);

    try {
      await deleteMutation.mutateAsync(id);
    } catch (caughtError) {
      setError(String(caughtError));
    }
  }

  const isSaving = createMutation.isPending || updateMutation.isPending;

  return (
    <div className="flex flex-col gap-2 border-b px-2 py-2">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <span className="text-xs font-medium text-muted-foreground">
          저장 프롬프트
        </span>
        <Button type="button" size="sm" variant="outline" onClick={openCreateDialog}>
          <PlusIcon data-icon="inline-start" />
          추가
        </Button>
      </div>
      {savedPromptsQuery.error && (
        <p className="text-xs text-destructive">{String(savedPromptsQuery.error)}</p>
      )}
      {error && <p className="text-xs text-destructive">{error}</p>}
      {savedPrompts.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {savedPrompts.map((savedPrompt) => (
            <div
              key={savedPrompt.id}
              className="flex max-w-full items-center gap-1 rounded-full border bg-background p-0.5"
            >
              <PromptSuggestion
                size="sm"
                className="min-w-0 max-w-[18rem] justify-start truncate border-transparent"
                disabled={disabled}
                onClick={() => onSendPrompt(savedPrompt.prompt)}
                title={savedPrompt.prompt}
              >
                <span className="truncate">{savedPrompt.label}</span>
              </PromptSuggestion>
              <Button
                type="button"
                variant="ghost"
                size="icon-xs"
                onClick={() => openEditDialog(savedPrompt)}
              >
                <PencilIcon />
                <span className="sr-only">수정</span>
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="icon-xs"
                disabled={deleteMutation.isPending}
                onClick={() => void handleDelete(savedPrompt.id)}
              >
                <Trash2Icon />
                <span className="sr-only">삭제</span>
              </Button>
            </div>
          ))}
        </div>
      )}
      <Dialog open={dialogMode !== null} onOpenChange={(open) => !open && setDialogMode(null)}>
        <DialogContent className="sm:max-w-lg">
          <form onSubmit={handleSubmit} className="contents">
            <DialogHeader>
              <DialogTitle>
                {dialogMode === "edit" ? "저장 프롬프트 수정" : "저장 프롬프트 추가"}
              </DialogTitle>
              <DialogDescription>
                버튼명과 전송할 프롬프트를 입력하세요.
              </DialogDescription>
            </DialogHeader>
            <FieldGroup>
              <Field>
                <FieldLabel htmlFor="saved-prompt-label">버튼명</FieldLabel>
                <Input
                  id="saved-prompt-label"
                  value={form.label}
                  onChange={(event) =>
                    setForm((current) => ({ ...current, label: event.target.value }))
                  }
                  placeholder="Continue"
                />
              </Field>
              <Field>
                <FieldLabel htmlFor="saved-prompt-text">프롬프트</FieldLabel>
                <Textarea
                  id="saved-prompt-text"
                  value={form.prompt}
                  onChange={(event) =>
                    setForm((current) => ({ ...current, prompt: event.target.value }))
                  }
                  placeholder="이어서 진행해주세요."
                  className="min-h-32"
                />
              </Field>
              {error && <FieldError>{error}</FieldError>}
            </FieldGroup>
            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => setDialogMode(null)}
              >
                취소
              </Button>
              <Button type="submit" disabled={isSaving}>
                {isSaving ? "저장 중" : "저장"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  );
}
