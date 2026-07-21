use crate::{
    domain::{
        safe_base_name, validate_model, validate_request, ModelStatus, ToolStatus,
        TranscriptionRequest, TranscriptionResult,
    },
    ports::{EventPort, ToolchainPort},
};

pub struct HushlineService<'a> {
    tools: &'a dyn ToolchainPort,
    events: &'a dyn EventPort,
}

impl<'a> HushlineService<'a> {
    pub fn new(tools: &'a dyn ToolchainPort, events: &'a dyn EventPort) -> Self {
        Self { tools, events }
    }
    pub fn dependency_status(&self) -> ToolStatus {
        self.tools.status()
    }
    pub fn model_statuses(&self) -> Vec<ModelStatus> {
        self.tools.model_statuses()
    }

    pub fn download_model(&self, model: &str) -> Result<(), String> {
        validate_model(model)?;
        self.events.progress(
            "model",
            1,
            "Whisper 모델 다운로드를 시작합니다…",
            Some(model.into()),
        );
        self.tools.download_model(model, self.events)?;
        self.events.progress(
            "model",
            100,
            "모델 다운로드가 완료되었습니다",
            Some(model.into()),
        );
        Ok(())
    }

    pub fn delete_model(&self, model: &str) -> Result<(), String> {
        validate_model(model)?;
        self.tools.delete_model(model)
    }

    pub fn transcribe_video(
        &self,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult, String> {
        validate_request(&request)?;
        let status = self.tools.status();
        if !status.yt_dlp {
            return Err("yt-dlp가 필요합니다. `brew install yt-dlp`로 설치해 주세요.".into());
        }
        if !status.ffmpeg {
            return Err("FFmpeg가 필요합니다. `brew install ffmpeg`로 설치해 주세요.".into());
        }
        if !status.whisper {
            return Err("Whisper가 필요합니다. `pipx install openai-whisper`로 설치한 뒤 앱을 다시 시작해 주세요.".into());
        }
        let output_dir = request
            .output_dir
            .clone()
            .unwrap_or_else(|| self.tools.default_output_dir());
        self.tools.ensure_directory(&output_dir)?;
        if !request.force {
            if let Some(mut cached) = self.tools.find_result(&output_dir, &request.url)? {
                cached.cached = true;
                self.events
                    .progress("done", 100, "이미 변환된 결과를 불러왔습니다", None);
                return Ok(cached);
            }
        }

        self.events
            .progress("metadata", 6, "영상 제목과 길이를 확인하는 중…", None);
        let metadata = self.tools.metadata(&request.url)?;
        let base_name = safe_base_name(&metadata.title, &metadata.id);
        let output_template = output_dir.join(format!("{base_name}.%(ext)s"));
        let audio_path = output_dir.join(format!("{base_name}.wav"));

        self.events.progress(
            "download",
            18,
            "영상에서 오디오 스트림을 내려받는 중…",
            None,
        );
        self.tools
            .download_audio(&request.url, &output_template, &audio_path, self.events)?;
        self.events
            .progress("extract", 48, "Whisper에 맞게 오디오를 정리하는 중…", None);
        self.events.progress(
            "transcribe",
            58,
            "Whisper가 음성을 듣고 있습니다. 영상 길이에 따라 시간이 걸릴 수 있어요.",
            None,
        );
        let transcript_path = self.tools.transcribe(
            &audio_path,
            &output_dir,
            &base_name,
            &request.language,
            &request.model,
            self.events,
        )?;
        let transcript = self.tools.read_text(&transcript_path)?;
        let mut result = TranscriptionResult {
            url: request.url,
            title: metadata.title,
            transcript,
            transcript_path,
            audio_path,
            json_path: output_dir.join(format!("{base_name}.hushline.json")),
            language: request.language,
            model: request.model,
            cached: false,
            duration_seconds: metadata.duration_seconds,
        };
        result.json_path = self.tools.save_result(&output_dir, &base_name, &result)?;
        self.events
            .progress("done", 100, "텍스트 변환이 완료되었습니다", None);
        Ok(result)
    }
}
