export type TocLevel = 1 | 2 | 3;

export type TocEntry = {
  blockId: string;
  level: TocLevel;
  text: string;
  startLine: number;
  taskSummary?: {
    completed: number;
    open: number;
  };
};
