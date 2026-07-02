// worktree session 진입 성능 계측(specs/007 research R12, FR-013).
// dev 빌드에서만 콘솔로 출력하며, measure 이름은 quickstart.md 검증 절차와 계약이다.

export const SESSION_ROUTE_ENTERED_MARK = "session:route-entered";

export type SessionMilestone = "session:shell-rendered" | "session:graph-first-row";

export function markSessionRouteEntered() {
  performance.mark(SESSION_ROUTE_ENTERED_MARK);
}

export function measureSessionMilestone(name: SessionMilestone) {
  try {
    const measure = performance.measure(name, SESSION_ROUTE_ENTERED_MARK);

    if (import.meta.env.DEV) {
      console.info(`[perf] ${name}: ${Math.round(measure.duration)}ms`);
    }
  } catch {
    // route mark 없이 진입한 경우(핫리로드 등)는 계측을 건너뛴다.
  }
}
