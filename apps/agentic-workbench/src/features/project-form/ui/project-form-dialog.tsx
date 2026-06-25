import { FormEvent, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpenIcon } from "lucide-react";

import type { Project, ProjectInput } from "@/entities/project/model/types";
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
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";

type ProjectFormDialogProps = {
  project: Project | null;
  open: boolean;
  error: string | null;
  onOpenChange: (open: boolean) => void;
  onSubmit: (input: ProjectInput) => Promise<void>;
  onError: (error: string | null) => void;
};

const emptyForm: ProjectInput = {
  name: "",
  workingDirectory: "",
  description: "",
};

export function ProjectFormDialog({
  project,
  open: isOpen,
  error,
  onOpenChange,
  onSubmit,
  onError,
}: ProjectFormDialogProps) {
  const [form, setForm] = useState<ProjectInput>(emptyForm);
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    setForm(
      project
        ? {
            name: project.name,
            workingDirectory: project.workingDirectory,
            description: project.description ?? "",
          }
        : emptyForm,
    );
  }, [isOpen, project]);

  async function selectWorkingDirectory() {
    onError(null);

    try {
      const selectedPath = await open({
        directory: true,
        multiple: false,
        title: "작업 디렉토리 선택",
      });

      if (typeof selectedPath === "string") {
        setForm((currentForm) => ({
          ...currentForm,
          workingDirectory: selectedPath,
        }));
      }
    } catch (caughtError) {
      onError(String(caughtError));
    }
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setIsSaving(true);

    try {
      await onSubmit(form);
    } finally {
      setIsSaving(false);
    }
  }

  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>
            {project ? "프로젝트 수정" : "프로젝트 생성"}
          </DialogTitle>
          <DialogDescription>
            프로젝트 이름과 작업 디렉토리는 필수입니다.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="flex flex-col gap-4">
          <FieldGroup>
            <Field>
              <FieldLabel htmlFor="project-name">프로젝트 이름</FieldLabel>
              <Input
                id="project-name"
                value={form.name}
                onChange={(event) =>
                  setForm((currentForm) => ({
                    ...currentForm,
                    name: event.target.value,
                  }))
                }
                required
              />
            </Field>
            <Field>
              <FieldLabel htmlFor="working-directory">작업 디렉토리</FieldLabel>
              <div className="flex gap-2">
                <Input
                  id="working-directory"
                  value={form.workingDirectory}
                  placeholder="디렉토리를 선택하세요"
                  readOnly
                  required
                />
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => void selectWorkingDirectory()}
                >
                  <FolderOpenIcon data-icon="inline-start" />
                  선택
                </Button>
              </div>
              <FieldDescription>
                Tauri 파일 선택 다이얼로그에서 로컬 프로젝트의 기준
                디렉토리를 선택하세요.
              </FieldDescription>
            </Field>
            <Field>
              <FieldLabel htmlFor="project-description">설명</FieldLabel>
              <Textarea
                id="project-description"
                value={form.description}
                onChange={(event) =>
                  setForm((currentForm) => ({
                    ...currentForm,
                    description: event.target.value,
                  }))
                }
                rows={4}
              />
            </Field>
            {error && <FieldError>{error}</FieldError>}
          </FieldGroup>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
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
  );
}
