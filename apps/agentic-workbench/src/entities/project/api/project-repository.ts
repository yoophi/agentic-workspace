import { invoke } from "@tauri-apps/api/core";

import type { Project, ProjectInput } from "@/entities/project/model/types";

export async function listProjects() {
  return invoke<Project[]>("list_projects");
}

export async function createProject(input: ProjectInput) {
  return invoke<Project>("create_project", { input });
}

export async function updateProject(id: string, input: ProjectInput) {
  return invoke<Project>("update_project", { id, input });
}

export async function deleteProject(id: string) {
  return invoke("delete_project", { id });
}
