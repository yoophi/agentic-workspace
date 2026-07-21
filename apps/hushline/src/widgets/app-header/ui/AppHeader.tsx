import { AudioLines } from "lucide-react";
import type { ToolStatus } from "../../../entities/transcription";

export function AppHeader({ tools }: { tools: ToolStatus | null }) {
  const ready = Boolean(tools?.yt_dlp && tools.ffmpeg && tools.whisper);
  return <header className="relative z-10 flex h-16 items-center justify-between border-b border-white/[.07] px-6 lg:px-10" data-tauri-drag-region>
    <div className="flex items-center gap-3"><div className="grid h-8 w-8 place-items-center rounded-[10px] bg-[#d8ff65] text-[#11150d]"><AudioLines size={18} strokeWidth={2.5}/></div><span className="font-display text-[17px] tracking-tight">Hushline</span><span className="rounded-full border border-white/10 px-2 py-0.5 font-mono text-[9px] tracking-[.16em] text-[#7f8777]">LOCAL STT</span></div>
    <div className="flex items-center gap-2">
      <nav className="hidden items-center gap-1.5 md:flex" aria-label="로컬 엔진 상태">
        <ToolIndicator name="yt-dlp" ok={tools?.yt_dlp} detail={tools?.yt_dlp ? tools.yt_dlp_version : tools?.yt_dlp_error}/>
        <ToolIndicator name="FFmpeg" ok={tools?.ffmpeg}/>
        <ToolIndicator name="Whisper" ok={tools?.whisper}/>
      </nav>
      <div className="ml-2 flex items-center gap-2 border-l border-white/[.07] pl-4 text-xs text-[#868d7e]" title={tools?.yt_dlp ? `yt-dlp ${tools.yt_dlp_version ?? ""} 실행 가능` : tools?.yt_dlp_error}><span className={`h-1.5 w-1.5 rounded-full ${ready ? "bg-[#d8ff65] shadow-[0_0_10px_#d8ff65]" : "bg-amber-400"}`}/><span className="hidden lg:inline">{tools ? ready ? "엔진 준비됨" : "설정 확인 필요" : "엔진 확인 중"}</span></div>
    </div>
  </header>;
}

function ToolIndicator({ name, ok, detail }: { name: string; ok?: boolean; detail?: string }) {
  return <div className={`flex h-8 items-center gap-2 rounded-lg border px-2.5 ${ok === false ? "border-red-400/15 bg-red-400/[.035]" : "border-white/[.07] bg-white/[.025]"}`} title={detail || (ok ? `${name} 실행 가능` : `${name} 상태 확인 중`)}><span className={`h-1.5 w-1.5 rounded-full ${ok ? "bg-[#d8ff65]" : ok === false ? "bg-red-400" : "bg-[#494e46]"}`}/><span className="font-mono text-[9px] uppercase tracking-wider text-[#747b6f]">{name}</span>{name === "yt-dlp" && ok && detail && <span className="hidden text-[9px] text-[#535b50] xl:inline">{detail}</span>}</div>;
}
