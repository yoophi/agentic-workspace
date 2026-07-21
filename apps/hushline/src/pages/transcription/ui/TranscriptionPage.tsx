import type { ReactNode } from "react";
import { Check, ChevronDown, CircleAlert, Download, FolderOpen, Languages, ListPlus, LoaderCircle, RotateCcw, Settings2, Trash2, Youtube } from "lucide-react";
import { useTranscriptionWorkspace } from "../../../features/transcribe-video";
import { ModelStatusPanel } from "../../../features/manage-whisper-model";
import { Button, Input, Select } from "../../../shared/ui";
import { AppHeader } from "../../../widgets/app-header";
import { QueuePanel } from "../../../widgets/queue-panel";
import { ResultGrid } from "../../../widgets/result-grid";

export function TranscriptionPage() {
  const { url, setUrl, language, setLanguage, model, setModel, outputDir, stage, progress, message, tools, liveTranscript, models, modelDownload, queue, results, highlightedUrl, urlValid, canStart, selectedModelDownloaded, stageLabel, addToQueue, removeQueueItem, rerun, downloadSelectedModel, deleteSelectedModel, chooseFolder, resetError } = useTranscriptionWorkspace();

  return <main className="min-h-screen bg-[#0b0d0a] text-[#f2f4ed] selection:bg-[#d8ff65] selection:text-black">
    <div className="noise" />
    <AppHeader tools={tools}/>

    <section className="relative z-10 mx-auto grid max-w-[1380px] grid-cols-1 gap-8 px-6 py-10 lg:grid-cols-[minmax(0,1.14fr)_minmax(360px,.86fr)] lg:px-10 lg:py-14">
      <div className="animate-in">
        <div className="panel relative overflow-hidden p-5 sm:p-6">
          <div className="absolute right-0 top-0 h-24 w-24 bg-[#d8ff65]/[.035] blur-2xl" />
          <label className="mb-2.5 block text-xs font-medium text-[#b6bcaf]">YouTube URL</label>
          <div className="flex flex-col gap-3 sm:flex-row">
            <div className="relative flex-1"><Youtube className="absolute left-4 top-1/2 -translate-y-1/2 text-[#737a6e]" size={18}/><Input value={url} onChange={(e) => setUrl(e.target.value)} onKeyDown={(e) => e.key === "Enter" && addToQueue()} className="pl-11 pr-10" placeholder="https://youtube.com/watch?v=…" />{url && <span className="absolute right-4 top-1/2 -translate-y-1/2">{urlValid ? <Check size={16} className="text-[#d8ff65]"/> : <CircleAlert size={16} className="text-amber-400"/>}</span>}</div>
            <Button onClick={addToQueue} disabled={!canStart} className="h-12 min-w-40"><ListPlus size={17}/> Queue에 추가</Button>
          </div>
          {url && !urlValid && <p className="mt-2 text-xs text-amber-300/80">올바른 YouTube 또는 youtu.be 링크를 입력해 주세요.</p>}

          <div className="my-6 h-px bg-white/[.07]" />
          <div className="grid gap-4 sm:grid-cols-3">
            <Setting icon={<Languages size={15}/>} label="음성 언어"><Select value={language} onChange={(e) => setLanguage(e.target.value)}><option value="auto">자동 감지</option><option value="ko">한국어</option><option value="en">English</option><option value="ja">日本語</option><option value="zh">中文</option><option value="es">Español</option><option value="fr">Français</option><option value="de">Deutsch</option><option value="pt">Português</option><option value="ru">Русский</option></Select></Setting>
            <Setting icon={<Settings2 size={15}/>} label="Whisper 모델"><div className="flex gap-2"><Select value={model} onChange={(e) => setModel(e.target.value)}><option value="tiny">Tiny · 빠름</option><option value="base">Base · 균형</option><option value="small">Small · 정밀</option><option value="medium">Medium · 고품질</option><option value="large">Large · 최고품질</option></Select><Button type="button" variant="secondary" size="icon" className={`h-10 shrink-0 ${selectedModelDownloaded ? "text-[#858d80] hover:border-red-400/30 hover:bg-red-400/10 hover:text-red-300" : ""}`} disabled={!!modelDownload || !tools?.whisper} onClick={selectedModelDownloaded ? deleteSelectedModel : downloadSelectedModel} title={selectedModelDownloaded ? "모델 삭제" : "모델 다운로드"}>{modelDownload?.model === model ? <LoaderCircle size={15} className="animate-spin"/> : selectedModelDownloaded ? <Trash2 size={15}/> : <Download size={15}/>}</Button></div></Setting>
            <Setting icon={<FolderOpen size={15}/>} label="저장 위치"><Button variant="secondary" size="sm" className="w-full justify-start overflow-hidden font-normal" onClick={chooseFolder}><span className="truncate">{outputDir || "다운로드 폴더"}</span><ChevronDown size={13} className="ml-auto shrink-0"/></Button></Setting>
          </div>
          <ModelStatusPanel models={models} selectedModel={model} download={modelDownload} onSelect={setModel}/>
        </div>

        {stage !== "idle" && <div className={`panel mt-5 p-5 ${stage === "error" ? "border-red-400/20" : ""}`}>
          <div className="mb-3 flex items-center justify-between"><div className="flex items-center gap-2.5 text-sm font-medium">{stage === "done" ? <Check size={16} className="text-[#d8ff65]"/> : stage === "error" ? <CircleAlert size={16} className="text-red-400"/> : <LoaderCircle size={16} className="animate-spin text-[#d8ff65]"/>}<span>{stageLabel}</span></div><span className="font-mono text-xs text-[#8f9688]">{Math.round(progress)}%</span></div>
          <div className="h-1.5 overflow-hidden rounded-full bg-white/[.06]"><div className="h-full rounded-full bg-[#d8ff65] transition-all duration-500" style={{width: `${progress}%`}} /></div>
          <p className={`mt-3 text-xs ${stage === "error" ? "text-red-300" : "text-[#7e8578]"}`}>{message}</p>
          {stage === "error" && <Button variant="ghost" size="sm" className="mt-3" onClick={resetError}><RotateCcw size={14}/>다시 시도</Button>}
        </div>}
      </div>

      <QueuePanel items={queue} chunks={liveTranscript} onRemove={removeQueueItem}/>
    </section>
    <ResultGrid results={results} highlightedUrl={highlightedUrl} onRerun={rerun}/>
    <footer className="relative z-10 mx-auto flex max-w-[1380px] items-center justify-end border-t border-white/[.06] px-6 py-5 text-[10px] uppercase tracking-[.14em] text-[#50564d] lg:px-10"><span>01 / LOCAL PIPELINE</span></footer>
  </main>;
}

function Setting({icon, label, children}: {icon: ReactNode; label: string; children: ReactNode}) { return <div><div className="mb-2 flex items-center gap-1.5 text-[11px] text-[#777e72]">{icon}{label}</div>{children}</div> }
