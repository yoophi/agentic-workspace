use std::path::PathBuf;

pub const MODELS: [&str; 5] = ["tiny", "base", "small", "medium", "large"];

#[derive(Clone)]
pub struct TranscriptionRequest {
    pub url: String,
    pub language: String,
    pub model: String,
    pub output_dir: Option<PathBuf>,
    pub force: bool,
}

#[derive(Clone)]
pub struct TranscriptionResult {
    pub url: String,
    pub title: String,
    pub transcript: String,
    pub transcript_path: PathBuf,
    pub audio_path: PathBuf,
    pub json_path: PathBuf,
    pub language: String,
    pub model: String,
    pub cached: bool,
    pub duration_seconds: Option<f64>,
}

pub struct VideoMetadata {
    pub title: String,
    pub id: String,
    pub duration_seconds: Option<f64>,
}
pub struct ModelStatus {
    pub name: String,
    pub downloaded: bool,
    pub path: PathBuf,
}
pub struct ToolStatus {
    pub yt_dlp: bool,
    pub yt_dlp_version: Option<String>,
    pub yt_dlp_error: Option<String>,
    pub ffmpeg: bool,
    pub whisper: bool,
    pub whisper_command: Option<PathBuf>,
}

#[derive(Clone)]
pub struct TranscriptChunk {
    pub text: String,
    pub timestamp: Option<String>,
}

pub fn validate_request(request: &TranscriptionRequest) -> Result<(), String> {
    let parsed = url::Url::parse(&request.url).map_err(|_| "올바른 URL이 아닙니다.".to_string())?;
    let host = parsed
        .host_str()
        .unwrap_or_default()
        .trim_start_matches("www.");
    if !matches!(
        host,
        "youtube.com" | "m.youtube.com" | "youtu.be" | "music.youtube.com"
    ) {
        return Err("YouTube URL만 사용할 수 있습니다.".into());
    }
    validate_model(&request.model)
}

pub fn validate_model(model: &str) -> Result<(), String> {
    if MODELS.contains(&model) {
        Ok(())
    } else {
        Err("지원하지 않는 모델입니다.".into())
    }
}

pub fn model_file(model: &str) -> &'static str {
    match model {
        "tiny" => "tiny.pt",
        "base" => "base.pt",
        "small" => "small.pt",
        "medium" => "medium.pt",
        "large" => "large-v3.pt",
        _ => unreachable!("validated model"),
    }
}

pub fn safe_base_name(title: &str, id: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!(
        "{}-{}",
        cleaned
            .trim()
            .chars()
            .take(80)
            .collect::<String>()
            .trim()
            .replace(' ', "_"),
        id
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_non_youtube_urls() {
        let request = TranscriptionRequest {
            url: "https://example.com/video".into(),
            language: "auto".into(),
            model: "base".into(),
            output_dir: None,
            force: false,
        };
        assert!(validate_request(&request).is_err());
    }
    #[test]
    fn sanitizes_output_names() {
        assert_eq!(safe_base_name("hello/world?", "abc"), "hello_world_-abc");
    }
}
