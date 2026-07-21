use crate::{
    domain::{
        model_file, ModelStatus, ToolStatus, TranscriptChunk, TranscriptionResult, VideoMetadata,
        MODELS,
    },
    ports::{EventPort, ToolchainPort},
};
use std::{
    env, fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
};

pub struct SystemToolchain;

#[derive(serde::Serialize, serde::Deserialize)]
struct StoredResult {
    schema_version: u8,
    url: String,
    title: String,
    transcript: String,
    transcript_path: String,
    audio_path: String,
    language: String,
    model: String,
    duration_seconds: Option<f64>,
}

impl StoredResult {
    fn from_domain(result: &TranscriptionResult) -> Self {
        Self {
            schema_version: 1,
            url: result.url.clone(),
            title: result.title.clone(),
            transcript: result.transcript.clone(),
            transcript_path: result.transcript_path.display().to_string(),
            audio_path: result.audio_path.display().to_string(),
            language: result.language.clone(),
            model: result.model.clone(),
            duration_seconds: result.duration_seconds,
        }
    }
    fn into_domain(self, json_path: PathBuf) -> TranscriptionResult {
        TranscriptionResult {
            url: self.url,
            title: self.title,
            transcript: self.transcript,
            transcript_path: self.transcript_path.into(),
            audio_path: self.audio_path.into(),
            json_path,
            language: self.language,
            model: self.model,
            cached: true,
            duration_seconds: self.duration_seconds,
        }
    }
}

impl SystemToolchain {
    pub fn new() -> Self {
        Self
    }

    fn candidates(name: &str) -> Vec<PathBuf> {
        let mut values = vec![PathBuf::from(name)];
        for base in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"] {
            values.push(Path::new(base).join(name));
        }
        if let Ok(home) = env::var("HOME") {
            values.push(Path::new(&home).join(".local/bin").join(name));
            values.push(Path::new(&home).join(".pyenv/shims").join(name));
        }
        values
    }

    fn find(names: &[&str]) -> Option<PathBuf> {
        names
            .iter()
            .flat_map(|name| Self::candidates(name))
            .find(|path| {
                if path.components().count() > 1 {
                    path.is_file()
                } else {
                    Command::new("which")
                        .arg(path)
                        .output()
                        .map(|o| o.status.success())
                        .unwrap_or(false)
                }
            })
            .and_then(|path| {
                if path.components().count() > 1 {
                    Some(path)
                } else {
                    Command::new("which")
                        .arg(path)
                        .output()
                        .ok()
                        .and_then(|o| String::from_utf8(o.stdout).ok())
                        .map(|s| PathBuf::from(s.trim()))
                }
            })
    }

    fn cache_dir() -> PathBuf {
        env::var("XDG_CACHE_HOME")
            .map(|p| PathBuf::from(p).join("whisper"))
            .or_else(|_| env::var("HOME").map(|h| Path::new(&h).join(".cache/whisper")))
            .unwrap_or_else(|_| env::temp_dir().join("whisper"))
    }

    fn whisper_python(whisper: &Path) -> Option<PathBuf> {
        let script = fs::read_to_string(whisper).ok()?;
        let first = script.lines().next()?.strip_prefix("#!")?.trim();
        let executable = first.split_whitespace().next()?;
        if executable.ends_with("env") {
            Self::find(&["python3", "python"])
        } else {
            Some(PathBuf::from(executable))
        }
    }

    fn run_streaming<F>(command: &mut Command, mut on_line: F) -> Result<(bool, String), String>
    where
        F: FnMut(&str),
    {
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("프로세스 실행 실패: {e}"))?;
        let stdout = child.stdout.take().ok_or("stdout 연결 실패")?;
        let stderr = child.stderr.take().ok_or("stderr 연결 실패")?;
        let (tx, rx) = mpsc::channel::<String>();
        for reader in [
            Box::new(BufReader::new(stdout)) as Box<dyn BufRead + Send>,
            Box::new(BufReader::new(stderr)) as Box<dyn BufRead + Send>,
        ] {
            let sender = tx.clone();
            thread::spawn(move || {
                for line in reader.lines().map_while(Result::ok) {
                    let _ = sender.send(line);
                }
            });
        }
        drop(tx);
        let mut log = Vec::new();
        for line in rx {
            if !line.trim().is_empty() {
                on_line(&line);
                log.push(line);
            }
        }
        let status = child
            .wait()
            .map_err(|e| format!("프로세스 종료 확인 실패: {e}"))?;
        Ok((status.success(), log.join("\n")))
    }

    fn percent(line: &str) -> Option<f32> {
        let prefix = &line[..line.find('%')?];
        prefix
            .split(|c: char| !(c.is_ascii_digit() || c == '.'))
            .filter(|s| !s.is_empty())
            .next_back()?
            .parse()
            .ok()
    }

    fn chunk(line: &str) -> Option<TranscriptChunk> {
        let trimmed = line.trim();
        if !trimmed.starts_with('[') || !trimmed.contains("-->") {
            return None;
        }
        let closing = trimmed.find(']')?;
        let text = trimmed.get(closing + 1..)?.trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some(TranscriptChunk {
                text,
                timestamp: Some(trimmed.get(1..closing)?.trim().into()),
            })
        }
    }

    fn tail_error(label: &str, log: &str, lines: usize) -> String {
        let tail = log
            .lines()
            .rev()
            .take(lines)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        format!("{label} 실패: {tail}")
    }
}

impl ToolchainPort for SystemToolchain {
    fn status(&self) -> ToolStatus {
        let downloader = Self::find(&["yt-dlp", "youtube-dl"]);
        let (yt_dlp, yt_dlp_version, yt_dlp_error) = match downloader {
            Some(path) => match Command::new(&path).arg("--version").output() {
                Ok(output) if output.status.success() => (
                    true,
                    Some(String::from_utf8_lossy(&output.stdout).trim().to_string()),
                    None,
                ),
                Ok(output) => (
                    false,
                    None,
                    Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
                ),
                Err(error) => (false, None, Some(format!("실행 실패: {error}"))),
            },
            None => (false, None, Some("실행 파일을 찾지 못했습니다".into())),
        };
        let whisper = Self::find(&["whisper", "whisper-cli"]);
        ToolStatus {
            yt_dlp,
            yt_dlp_version,
            yt_dlp_error,
            ffmpeg: Self::find(&["ffmpeg"]).is_some(),
            whisper: whisper.is_some(),
            whisper_command: whisper,
        }
    }

    fn default_output_dir(&self) -> PathBuf {
        env::var("HOME")
            .map(|h| Path::new(&h).join("Downloads/Hushline"))
            .unwrap_or_else(|_| env::temp_dir().join("Hushline"))
    }

    fn ensure_directory(&self, path: &Path) -> Result<(), String> {
        fs::create_dir_all(path).map_err(|e| format!("저장 폴더를 만들 수 없습니다: {e}"))
    }

    fn metadata(&self, url: &str) -> Result<VideoMetadata, String> {
        let downloader =
            Self::find(&["yt-dlp", "youtube-dl"]).ok_or("yt-dlp를 찾지 못했습니다.")?;
        let output = Command::new(downloader)
            .args(["--dump-single-json", "--no-playlist", url])
            .output()
            .map_err(|e| format!("yt-dlp 실행 실패: {e}"))?;
        if !output.status.success() {
            return Err(Self::tail_error(
                "영상 정보 조회",
                &String::from_utf8_lossy(&output.stderr),
                8,
            ));
        }
        let value: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("영상 정보를 읽지 못했습니다: {e}"))?;
        Ok(VideoMetadata {
            title: value["title"].as_str().unwrap_or("YouTube audio").into(),
            id: value["id"].as_str().unwrap_or("video").into(),
            duration_seconds: value["duration"].as_f64(),
        })
    }

    fn download_audio(
        &self,
        url: &str,
        template: &Path,
        expected: &Path,
        events: &dyn EventPort,
    ) -> Result<(), String> {
        let downloader =
            Self::find(&["yt-dlp", "youtube-dl"]).ok_or("yt-dlp를 찾지 못했습니다.")?;
        let mut command = Command::new(downloader);
        command
            .args([
                "--no-playlist",
                "--extract-audio",
                "--audio-format",
                "wav",
                "--audio-quality",
                "0",
                "--newline",
                "--progress",
                "--output",
            ])
            .arg(template)
            .arg(url);
        let (ok, log) = Self::run_streaming(&mut command, |line| {
            let progress = Self::percent(line)
                .map(|p| 18 + (p * 0.28) as u8)
                .unwrap_or(46)
                .min(47);
            events.progress(
                "download",
                progress,
                "영상에서 오디오 스트림을 내려받는 중…",
                Some(line.into()),
            );
        })?;
        if !ok {
            return Err(Self::tail_error("오디오 다운로드", &log, 8));
        }
        if !expected.exists() {
            return Err("오디오 파일이 생성되지 않았습니다. FFmpeg 설정을 확인해 주세요.".into());
        }
        Ok(())
    }

    fn transcribe(
        &self,
        audio: &Path,
        output_dir: &Path,
        base_name: &str,
        language: &str,
        model: &str,
        events: &dyn EventPort,
    ) -> Result<PathBuf, String> {
        let whisper =
            Self::find(&["whisper", "whisper-cli"]).ok_or("Whisper를 찾지 못했습니다.")?;
        let transcript_path = output_dir.join(format!("{base_name}.txt"));
        let mut command = Command::new(&whisper);
        if whisper.file_name().and_then(|n| n.to_str()) == Some("whisper-cli") {
            let model_path = env::var("WHISPER_MODEL_PATH").map_err(|_| "whisper.cpp 사용 시 WHISPER_MODEL_PATH 환경 변수에 ggml 모델 경로를 지정해 주세요.".to_string())?;
            command
                .arg("-m")
                .arg(model_path)
                .arg("-f")
                .arg(audio)
                .arg("-otxt")
                .arg("-of")
                .arg(output_dir.join(base_name));
            if language != "auto" {
                command.arg("-l").arg(language);
            }
        } else {
            command
                .env("PYTHONUNBUFFERED", "1")
                .arg(audio)
                .arg("--model")
                .arg(model)
                .arg("--output_dir")
                .arg(output_dir)
                .args(["--output_format", "txt", "--fp16", "False"]);
            if language != "auto" {
                command.arg("--language").arg(language);
            }
        }
        let mut segments = 0u8;
        let (ok, log) = Self::run_streaming(&mut command, |line| {
            if let Some(chunk) = Self::chunk(line) {
                segments = segments.saturating_add(1);
                events.transcript(chunk);
            }
            let progress = Self::percent(line)
                .map(|p| 58 + (p * 0.40) as u8)
                .unwrap_or(58u8.saturating_add(segments.min(35)))
                .min(98);
            events.progress(
                "transcribe",
                progress,
                "Whisper가 음성을 텍스트로 변환하고 있습니다…",
                Some(line.into()),
            );
        })?;
        if !ok {
            return Err(Self::tail_error("텍스트 변환", &log, 10));
        }
        Ok(transcript_path)
    }

    fn read_text(&self, path: &Path) -> Result<String, String> {
        fs::read_to_string(path).map_err(|e| format!("변환된 텍스트를 읽지 못했습니다: {e}"))
    }

    fn find_result(
        &self,
        output_dir: &Path,
        url: &str,
    ) -> Result<Option<TranscriptionResult>, String> {
        let entries = match fs::read_dir(output_dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("결과 폴더를 읽지 못했습니다: {error}")),
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".hushline.json"))
            {
                continue;
            }
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            let Ok(stored) = serde_json::from_str::<StoredResult>(&content) else {
                continue;
            };
            if stored.url == url {
                return Ok(Some(stored.into_domain(path)));
            }
        }
        Ok(None)
    }

    fn save_result(
        &self,
        output_dir: &Path,
        base_name: &str,
        result: &TranscriptionResult,
    ) -> Result<PathBuf, String> {
        let path = output_dir.join(format!("{base_name}.hushline.json"));
        let json = serde_json::to_string_pretty(&StoredResult::from_domain(result))
            .map_err(|e| format!("JSON 결과 생성 실패: {e}"))?;
        fs::write(&path, json).map_err(|e| format!("JSON 결과 저장 실패: {e}"))?;
        Ok(path)
    }

    fn model_statuses(&self) -> Vec<ModelStatus> {
        MODELS
            .iter()
            .map(|name| {
                let path = Self::cache_dir().join(model_file(name));
                ModelStatus {
                    name: (*name).into(),
                    downloaded: path.exists(),
                    path,
                }
            })
            .collect()
    }

    fn download_model(&self, model: &str, events: &dyn EventPort) -> Result<(), String> {
        let whisper = Self::find(&["whisper"])
            .ok_or("모델 다운로드에는 OpenAI Whisper 설치가 필요합니다.")?;
        let python = Self::whisper_python(&whisper)
            .ok_or("Whisper의 Python 실행 환경을 찾지 못했습니다.")?;
        let mut command = Command::new(python);
        command.env("PYTHONUNBUFFERED", "1").args([
            "-c",
            "import sys, whisper; whisper.load_model(sys.argv[1]); print('MODEL_READY')",
            model,
        ]);
        let (ok, log) = Self::run_streaming(&mut command, |line| {
            let progress = Self::percent(line).unwrap_or(if line.contains("MODEL_READY") {
                100.0
            } else {
                2.0
            }) as u8;
            events.progress(
                "model",
                progress,
                &format!("{model} 모델 다운로드 중…"),
                Some(line.into()),
            );
        })?;
        if ok {
            Ok(())
        } else {
            Err(Self::tail_error("모델 다운로드", &log, 8))
        }
    }

    fn delete_model(&self, model: &str) -> Result<(), String> {
        let path = Self::cache_dir().join(model_file(model));
        if path.exists() {
            fs::remove_file(path)
                .map_err(|e| format!("{model} 모델을 삭제하지 못했습니다: {e}"))?;
        }
        Ok(())
    }
}
