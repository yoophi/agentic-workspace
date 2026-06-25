export type Project = {
  id: string;
  name: string;
  workingDirectory: string;
  description?: string | null;
};

export type ProjectInput = {
  name: string;
  workingDirectory: string;
  description: string;
};
