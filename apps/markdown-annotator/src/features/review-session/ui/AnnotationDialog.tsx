import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import type { ReviewAnnotationType } from "@/entities/review-session/model/types";

export function AnnotationDialog({ selectedTexts, onSubmit }: { selectedTexts: string[]; onSubmit: (input: { type: ReviewAnnotationType; comment: string; selectedTexts: string[] }) => void }) {
  const [type, setType] = useState<ReviewAnnotationType>("change-request"); const [comment, setComment] = useState("");
  return <form aria-label="Annotation 작성" className="space-y-2" onSubmit={(event) => { event.preventDefault(); onSubmit({ type, comment, selectedTexts }); setComment(""); }}><select aria-label="Annotation 종류" value={type} onChange={(event) => setType(event.target.value as ReviewAnnotationType)}><option value="change-request">수정 요청</option><option value="question">질문</option><option value="note">메모</option><option value="delete">삭제 요청</option></select><Textarea aria-label="Annotation 내용" value={comment} onChange={(event) => setComment(event.target.value)} /><Button type="submit" disabled={selectedTexts.length === 0 || (type !== "delete" && !comment.trim())}>추가</Button></form>;
}
