export const AUTO_REFRESH_INTERVAL_MS = 3_000;

export const autoRefreshQueryOptions = {
  refetchInterval: AUTO_REFRESH_INTERVAL_MS,
  refetchIntervalInBackground: true,
  refetchOnWindowFocus: true,
} as const;
