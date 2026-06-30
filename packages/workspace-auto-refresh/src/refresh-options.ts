export const AUTO_REFRESH_INTERVAL_MS = 30_000;

export const autoRefreshQueryOptions = {
  refetchInterval: AUTO_REFRESH_INTERVAL_MS,
  refetchIntervalInBackground: false,
  refetchOnWindowFocus: true,
} as const;
