import { create } from "zustand";

import type { Project } from "@/entities/project/model/types";

type ProjectUiStore = {
  editingProject: Project | null;
  deletingProject: Project | null;
  isFormOpen: boolean;
  error: string | null;
  openCreateDialog: () => void;
  openEditDialog: (project: Project) => void;
  openDeleteDialog: (project: Project) => void;
  closeFormDialog: () => void;
  closeDeleteDialog: () => void;
  setError: (error: string | null) => void;
};

export const useProjectUiStore = create<ProjectUiStore>((set) => ({
  editingProject: null,
  deletingProject: null,
  isFormOpen: false,
  error: null,
  openCreateDialog: () =>
    set({ editingProject: null, error: null, isFormOpen: true }),
  openEditDialog: (project) =>
    set({ editingProject: project, error: null, isFormOpen: true }),
  openDeleteDialog: (project) => set({ deletingProject: project, error: null }),
  closeFormDialog: () =>
    set({ editingProject: null, error: null, isFormOpen: false }),
  closeDeleteDialog: () => set({ deletingProject: null }),
  setError: (error) => set({ error }),
}));
