import { Button } from "@/components/ui/button";
import type { ReviewDecision } from "@/entities/review-session/model/types";
import { useReviewSessionStore } from "../model/review-session-store";

const decisions: Array<[ReviewDecision, string]> = [["draft", "초안"], ["changes-requested", "수정 필요"], ["approved", "승인"], ["stopped", "검토 중단"]];
export function ReviewDecisionPanel() {
  const { session, warning, setDecision, update, removeGroup } = useReviewSessionStore();
  if (!session) return <p>Review를 시작할 문서를 선택하세요.</p>;
  return <section aria-label="Review 결정" className="space-y-3"><div className="flex flex-wrap gap-1">{decisions.map(([value, label]) => <Button key={value} size="sm" variant={session.decision === value ? "default" : "outline"} onClick={() => { if (!setDecision(value) && value === "approved" && window.confirm("열린 요청이 있습니다. 승인할까요?")) setDecision(value, true); }}>{label}</Button>)}</div>{warning ? <p role="alert" className="text-sm text-amber-700">{warning}</p> : null}<ul className="space-y-2">{session.annotations.map((item) => <li key={item.id} className="rounded border p-2 text-sm"><strong>{item.type}</strong><p>{item.selectedText}</p><p>{item.comment}</p><div className="flex gap-1"><Button size="sm" variant="outline" onClick={() => update(item.id, { status: item.status === "open" ? "resolved" : "open" })}>{item.status === "open" ? "해결" : "다시 열기"}</Button><Button size="sm" variant="ghost" onClick={() => removeGroup(item.id)}>삭제</Button></div></li>)}</ul></section>;
}
