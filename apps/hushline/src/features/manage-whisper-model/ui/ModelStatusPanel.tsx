import { Check, Download, LoaderCircle } from "lucide-react";
import type { ModelStatus } from "../../../entities/transcription";

type Props = {
  models: ModelStatus[];
  selectedModel: string;
  download: { model: string; progress: number } | null;
  onSelect: (model: string) => void;
};

const labels: Record<string, string> = { tiny: "Tiny", base: "Base", small: "Small", medium: "Medium", large: "Large" };

export function ModelStatusPanel({ models, selectedModel, download, onSelect }: Props) {
  const selected = models.find((item) => item.name === selectedModel);
  return <div className="mt-4 rounded-xl border border-white/[.07] bg-black/10 p-3.5">
    <div className="mb-3 flex items-center justify-between">
      <span className="font-mono text-[9px] uppercase tracking-[.16em] text-[#747c70]">Local model library</span>
      <span className={`flex items-center gap-1.5 text-[10px] ${download ? "text-[#d8ff65]" : selected?.downloaded ? "text-[#9eaa94]" : "text-amber-300/80"}`}>
        {download ? <LoaderCircle size={11} className="animate-spin"/> : selected?.downloaded ? <Check size={11}/> : <Download size={11}/>} 
        {download ? `${labels[download.model] ?? download.model} 다운로드 중` : selected?.downloaded ? `${labels[selectedModel]} 설치됨` : `${labels[selectedModel]} 설치 필요`}
      </span>
    </div>
    <div className="grid grid-cols-5 gap-1.5">
      {models.length ? models.map((item) => <button key={item.name} type="button" onClick={() => onSelect(item.name)} className={`rounded-lg border px-2 py-2 text-left transition-colors ${selectedModel === item.name ? "border-[#d8ff65]/30 bg-[#d8ff65]/[.06]" : "border-white/[.06] bg-white/[.018] hover:bg-white/[.04]"}`}>
        <span className={`mb-1.5 block h-1.5 w-1.5 rounded-full ${download?.model === item.name ? "animate-pulse bg-[#d8ff65]" : item.downloaded ? "bg-[#8ea76a]" : "bg-[#41463f]"}`}/>
        <span className="block truncate text-[9px] text-[#959c90]">{labels[item.name] ?? item.name}</span>
      </button>) : ["tiny", "base", "small", "medium", "large"].map((name) => <div key={name} className="h-[45px] animate-pulse rounded-lg bg-white/[.025]"/>)}
    </div>
    {download && <div className="mt-3"><div className="mb-1.5 flex justify-between font-mono text-[9px] text-[#8f9a84]"><span>모델 파일을 저장하고 있습니다</span><span>{Math.round(download.progress)}%</span></div><div className="h-1 overflow-hidden rounded-full bg-white/[.07]"><div className="h-full rounded-full bg-[#d8ff65] transition-all duration-300" style={{ width: `${download.progress}%` }}/></div></div>}
  </div>;
}
