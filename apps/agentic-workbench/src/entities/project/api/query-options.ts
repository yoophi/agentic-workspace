export const gitStateRefreshIntervalMs = 3_000;

export const gitStateRefreshQueryOptions = {
  refetchInterval: gitStateRefreshIntervalMs,
  refetchIntervalInBackground: true,
  refetchOnWindowFocus: true,
} as const;
