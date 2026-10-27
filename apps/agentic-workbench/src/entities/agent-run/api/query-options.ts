// agent-run query의 신선도 정책(specs/007 research R7). 같은 query key를 쓰는
// 모든 화면이 동일한 캐시 정책을 갖도록 call site가 아닌 api 계층에서 정의한다.

/** 계정별 CLI 모델 조회 결과를 5분 캐시한다. */
export const agentCatalogQueryOptions = {
  staleTime: 5 * 60_000,
} as const;

export const agentRunSettingsQueryOptions = {
  staleTime: 30_000,
} as const;

export const agentToolCommandCandidateQueryOptions = {
  staleTime: 5_000,
  gcTime: 30_000,
  retry: false,
} as const;
