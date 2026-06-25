import type { Project } from "@/entities/project/model/types";

export function sortProjectsByName(projects: Project[]) {
  return [...projects].sort((first, second) =>
    first.name.localeCompare(second.name, "ko"),
  );
}
