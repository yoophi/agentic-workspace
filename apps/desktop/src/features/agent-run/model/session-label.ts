import type { ProviderSession } from "@/entities/agent-run/model";

/**
 * provider 네이티브 세션을 재사용 드롭다운에 표시할 라벨로 포맷한다.
 * `제목 · N msgs · 시각` 형태이며, 제목이 없으면 세션 id 앞 8자를 쓰고
 * updatedAt이 없거나 파싱 불가하면 시각을 생략한다.
 */
export function formatSessionLabel(session: ProviderSession): string {
  const title = session.title?.trim() || session.id.slice(0, 8);
  const parts = [title, `${session.messageCount} msgs`];
  if (session.updatedAt) {
    const when = new Date(session.updatedAt);
    if (!Number.isNaN(when.getTime())) {
      parts.push(when.toLocaleString());
    }
  }
  return parts.join(" · ");
}
