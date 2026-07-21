import { useState } from "react";
import { Check, Copy, Database, ExternalLink, Languages, RotateCcw } from "lucide-react";
import type { TranscriptionResult } from "../../../entities/transcription";
import { backend } from "../../../shared/api";
import { Button, Select } from "../../../shared/ui";

const languages = [["auto", "자동 감지"], ["ko", "한국어"], ["en", "English"], ["ja", "日本語"], ["zh", "中文"], ["es", "Español"], ["fr", "Français"], ["de", "Deutsch"]];

export function ResultGrid({ results, highlightedUrl, onRerun }: { results: TranscriptionResult[]; highlightedUrl: string | null; onRerun: (result: TranscriptionResult, language: string) => void }) {
  if (!results.length) return null;
  return <section className="relative z-10 mx-auto max-w-[1380px] px-6 pb-12 lg:px-10"><div className="mb-4 flex items-end justify-between"><div><p className="font-mono text-[9px] uppercase tracking-[.18em] text-[#697164]">Saved artifacts</p><h2 className="mt-1 font-display text-2xl">변환 결과</h2></div><span className="text-[10px] text-[#596056]">JSON + TRANSCRIPT · {results.length}</span></div><div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">{results.map((result) => <ResultCard key={result.url} result={result} highlighted={highlightedUrl === result.url} onRerun={onRerun}/>)}</div></section>;
}

function ResultCard({result, highlighted, onRerun}: {result: TranscriptionResult; highlighted: boolean; onRerun: (result: TranscriptionResult, language: string) => void}) {
  const [language, setLanguage] = useState(result.language);
  const [copied, setCopied] = useState(false);
  async function copy() { await navigator.clipboard.writeText(result.transcript); setCopied(true); setTimeout(() => setCopied(false), 1200); }
  return <article className={`panel flex min-h-[300px] flex-col overflow-hidden transition-all duration-500 ${highlighted ? "border-[#d8ff65]/60 shadow-[0_0_0_3px_rgba(216,255,101,.08),0_20px_60px_rgba(216,255,101,.08)]" : ""}`}>
    <div className="border-b border-white/[.07] p-4"><div className="mb-2 flex items-center justify-between gap-3"><span className={`inline-flex items-center gap-1.5 rounded-full px-2 py-1 font-mono text-[9px] uppercase tracking-wider ${result.cached ? "bg-sky-300/10 text-sky-200" : "bg-[#d8ff65]/10 text-[#d8ff65]"}`}>{result.cached ? <Database size={10}/> : <Check size={10}/>} {result.cached ? "Cached result" : "Converted"}</span><span className="font-mono text-[9px] text-[#555d52]">{result.model}</span></div><h3 className="line-clamp-2 text-sm font-semibold leading-5 text-[#d9ddd3]">{result.title}</h3><p className="mt-1 truncate text-[10px] text-[#555d52]">{result.url}</p></div>
    <div className="transcript-scroll min-h-0 flex-1 overflow-y-auto p-4"><p className="line-clamp-6 whitespace-pre-wrap text-xs leading-5 text-[#969e91]">{result.transcript}</p></div>
    <div className="border-t border-white/[.07] p-3"><div className="mb-2 flex gap-2"><div className="relative flex-1"><Languages size={12} className="absolute left-2.5 top-1/2 z-10 -translate-y-1/2 text-[#6c7468]"/><Select className="h-9 pl-8 text-xs" value={language} onChange={(event) => setLanguage(event.target.value)}>{languages.map(([value, label]) => <option key={value} value={value}>{label}</option>)}</Select></div><Button variant="secondary" size="sm" onClick={() => onRerun(result, language)} disabled={language === result.language}><RotateCcw size={13}/>재실행</Button></div><div className="grid grid-cols-2 gap-2"><Button variant="ghost" size="sm" onClick={copy}>{copied ? <Check size={13}/> : <Copy size={13}/>} {copied ? "복사됨" : "텍스트 복사"}</Button><Button variant="ghost" size="sm" onClick={() => backend.openArtifact(result.json_path)}><ExternalLink size={13}/>JSON 열기</Button></div></div>
  </article>;
}
