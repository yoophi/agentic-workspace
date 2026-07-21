import { Check, Database, ListVideo, LoaderCircle, RotateCcw, X } from "lucide-react";
import type { QueueItem, TranscriptChunk } from "../../../entities/transcription";
import { Button } from "../../../shared/ui";

export function QueuePanel({ items, chunks, onRemove }: { items: QueueItem[]; chunks?: TranscriptChunk[]; onRemove: (id: string) => void }) {
  return <aside className="animate-in delay-1"><div className="panel flex h-[520px] min-h-0 flex-col overflow-hidden">
    <div className="flex shrink-0 items-center justify-between border-b border-white/[.07] px-5 py-4"><div className="flex items-center gap-2"><ListVideo size={16} className="text-[#d8ff65]"/><span className="text-sm font-semibold">Processing queue</span></div><span className="rounded-full bg-white/[.05] px-2 py-1 font-mono text-[9px] text-[#747c70]">{items.length} ITEMS</span></div>
    <div className="transcript-scroll min-h-0 flex-1 space-y-2 overflow-y-auto p-3">
      {items.length ? items.map((item, index) => <QueueRow key={item.id} item={item} index={index} onRemove={onRemove}/>) : <div className="flex h-full flex-col items-center justify-center text-center"><ListVideo size={30} strokeWidth={1.25} className="mb-3 text-[#3e453b]"/><p className="text-sm text-[#7c8477]">Queue가 비어 있습니다</p><p className="mt-1 text-[11px] text-[#50574e]">URL을 추가하면 순서대로 처리합니다</p></div>}
    </div>
    {!!chunks?.length && <div className="shrink-0 border-t border-white/[.07] bg-black/10 px-5 py-3"><p className="mb-1.5 text-[9px] uppercase tracking-[.15em] text-[#596156]">Live transcript</p><p className="line-clamp-3 text-xs leading-5 text-[#9fa69a]">{chunks.slice(-3).map((chunk) => chunk.text).join(" ")}</p></div>}
  </div></aside>;
}

function QueueRow({item, index, onRemove}: {item: QueueItem; index: number; onRemove: (id: string) => void}) {
  const icon = item.status === "processing" ? <LoaderCircle size={14} className="animate-spin text-[#d8ff65]"/> : item.status === "cached" ? <Database size={14} className="text-sky-300"/> : item.status === "completed" ? <Check size={14} className="text-[#d8ff65]"/> : item.status === "error" ? <RotateCcw size={14} className="text-red-300"/> : <span className="font-mono text-[10px] text-[#5f675b]">{String(index + 1).padStart(2, "0")}</span>;
  return <div className={`relative overflow-hidden rounded-xl border p-3 ${item.status === "processing" ? "border-[#d8ff65]/20 bg-[#d8ff65]/[.035]" : item.status === "cached" ? "border-sky-300/15 bg-sky-300/[.025]" : "border-white/[.07] bg-white/[.02]"}`}>
    <div className="flex items-start gap-3"><div className="mt-0.5 grid h-7 w-7 shrink-0 place-items-center rounded-lg bg-black/20">{icon}</div><div className="min-w-0 flex-1"><p className="truncate text-xs text-[#bbc1b6]">{item.url}</p><p className={`mt-1 truncate text-[10px] ${item.status === "error" ? "text-red-300/70" : "text-[#626a5f]"}`}>{item.message} · {item.language === "auto" ? "자동 감지" : item.language.toUpperCase()}</p></div><Button variant="ghost" size="icon" className="h-7 w-7 shrink-0" disabled={item.status === "processing"} onClick={() => onRemove(item.id)} title="Queue에서 삭제"><X size={13}/></Button></div>
    {(item.status === "processing" || item.progress > 0) && <div className="mt-3 h-1 overflow-hidden rounded-full bg-white/[.06]"><div className={`h-full rounded-full transition-all ${item.status === "cached" ? "bg-sky-300" : "bg-[#d8ff65]"}`} style={{width: `${item.progress}%`}}/></div>}
  </div>;
}
