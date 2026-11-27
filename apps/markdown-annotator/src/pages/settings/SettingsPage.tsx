import { useEffect, useState } from "react";
import { loadPreferences, resetPreferences, savePreferences, type GlobalPreferences } from "@/entities/global-preferences/api/preferences-api";
import { trashReviewData } from "@/features/data-management/model/data-management-api";
import { checkCliInstalled, installCli, removeCli } from "@/entities/document/api/documentApi";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

export function SettingsPage() {
  const [prefs, setPrefs] = useState<GlobalPreferences | null>(null);
  const [cli, setCli] = useState("확인 중");
  useEffect(() => { void loadPreferences().then(setPrefs); void checkCliInstalled().then((status) => setCli(status.installed ? "설치됨" : "설치되지 않음")); }, []);
  if (!prefs) return <p>설정을 불러오는 중…</p>;
  return <main className="mx-auto max-w-2xl space-y-6 p-8">
    <h1>설정</h1>
    <label>제외 디렉터리 이름<Input value={prefs.excludedDirectoryNames.join(", ")} onChange={(event) => setPrefs({ ...prefs, excludedDirectoryNames: event.target.value.split(",").map((value) => value.trim()).filter(Boolean) })} /></label>
    <label>글꼴 크기<Input type="number" min={12} max={32} value={prefs.fontSize} onChange={(event) => setPrefs({ ...prefs, fontSize: Number(event.target.value) })} /></label>
    <Button onClick={() => void savePreferences(prefs, prefs.revision).then(setPrefs)}>저장</Button>
    <Button variant="ghost" onClick={() => void resetPreferences().then(setPrefs)}>기본값 복원</Button>
    <section><h2>명령줄 도구</h2><p>{cli}</p><Button onClick={() => void installCli().then(() => setCli("설치됨"))}>ma 설치/재설치</Button><Button variant="ghost" onClick={() => void removeCli().then(() => setCli("설치되지 않음"))}>ma 제거</Button></section>
    <section><h2>데이터 관리</h2><Button variant="destructive" onClick={() => confirm("모든 검토 데이터를 휴지통으로 이동할까요?") && void trashReviewData("all")}>모든 검토 데이터 삭제</Button></section>
  </main>;
}
