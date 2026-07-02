// agent-run query의 신선도 정책(specs/007 research R7). 같은 query key를 쓰는
// 모든 화면이 동일한 캐시 정책을 갖도록 call site가 아닌 api 계층에서 정의한다.

/** agent 카탈로그는 환경 변수 기반이라 세션 중 사실상 불변이다. */
export const agentCatalogQueryOptions = {
  staleTime: 5 * 60_000,
} as const;

export const agentRunSettingsQueryOptions = {
  staleTime: 30_000,
} as const;

export const goalQueryOptions = {
  staleTime: 10_000,
} as const;
