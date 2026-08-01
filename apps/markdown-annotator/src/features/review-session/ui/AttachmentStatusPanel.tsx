import type { ReviewAnnotation } from "@/entities/review-session/model/types";
import { Button } from "@/components/ui/button";
export function AttachmentStatusPanel({ annotations, onDiscard, onRelink }: { annotations: ReviewAnnotation[]; onDiscard: (id: string) => void; onRelink: (id: string) => void }) {
  const detached = annotations.filter((item) => item.attachmentState !== "attached");
  if (!detached.length) return null;
  return <section aria-label="연결 상태" className="space-y-2"><h3>확인이 필요한 annotation</h3>{detached.map((item) => <div key={item.id}><p>{item.attachmentState === "conflict" ? "같은 문구가 여러 곳에 있습니다." : item.attachmentState === "missing" ? "원문에서 선택 문구를 찾지 못했습니다." : "원문과 연결되지 않았습니다."}</p><Button onClick={() => onRelink(item.id)}>다시 연결</Button><Button variant="ghost" onClick={() => onDiscard(item.id)}>폐기</Button></div>)}</section>;
}
