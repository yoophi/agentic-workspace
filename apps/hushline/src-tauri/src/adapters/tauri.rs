use crate::{
    adapters::system::SystemToolchain, application::HushlineService, domain, ports::EventPort,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

#[derive(Serialize)]
struct ToolStatusDto {
    yt_dlp: bool,
    yt_dlp_version: Option<String>,
    yt_dlp_error: Option<String>,
    ffmpeg: bool,
    whisper: bool,
    whisper_command: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProcessRequestDto {
    url: String,
    language: String,
    model: String,
    output_dir: Option<String>,
    #[serde(default)]
    force: bool,
}

#[derive(Serialize)]
struct ProcessResultDto {
    url: String,
    title: String,
    transcript: String,
    transcript_path: String,
    audio_path: String,
    json_path: String,
    language: String,
    model: String,
    cached: bool,
    duration_seconds: Option<f64>,
}

#[derive(Serialize, Clone)]
struct ProgressEventDto {
    stage: String,
    progress: u8,
    message: String,
    detail: Option<String>,
}

#[derive(Serialize, Clone)]
struct TranscriptChunkDto {
    text: String,
    timestamp: Option<String>,
}

#[derive(Serialize)]
struct ModelStatusDto {
    name: String,
    downloaded: bool,
    path: String,
}

#[derive(Deserialize)]
struct ModelRequestDto {
    model: String,
}

struct TauriEvents {
    app: AppHandle,
    model_channel: bool,
}

impl EventPort for TauriEvents {
    fn progress(&self, stage: &str, progress: u8, message: &str, detail: Option<String>) {
        let event = ProgressEventDto {
            stage: stage.into(),
            progress,
            message: message.into(),
            detail,
        };
        let channel = if self.model_channel {
            "model-download-progress"
        } else {
            "pipeline-progress"
        };
        let _ = self.app.emit(channel, event);
    }
    fn transcript(&self, chunk: domain::TranscriptChunk) {
        let _ = self.app.emit(
            "transcript-chunk",
            TranscriptChunkDto {
                text: chunk.text,
                timestamp: chunk.timestamp,
            },
        );
    }
}

fn service<'a>(tools: &'a SystemToolchain, events: &'a TauriEvents) -> HushlineService<'a> {
    HushlineService::new(tools, events)
}

#[tauri::command]
fn check_dependencies(app: AppHandle) -> ToolStatusDto {
    let tools = SystemToolchain::new();
    let events = TauriEvents {
        app,
        model_channel: false,
    };
    let status = service(&tools, &events).dependency_status();
    ToolStatusDto {
        yt_dlp: status.yt_dlp,
        yt_dlp_version: status.yt_dlp_version,
        yt_dlp_error: status.yt_dlp_error,
        ffmpeg: status.ffmpeg,
        whisper: status.whisper,
        whisper_command: status.whisper_command.map(|p| p.display().to_string()),
    }
}

#[tauri::command]
fn get_model_status(app: AppHandle) -> Vec<ModelStatusDto> {
    let tools = SystemToolchain::new();
    let events = TauriEvents {
        app,
        model_channel: true,
    };
    service(&tools, &events)
        .model_statuses()
        .into_iter()
        .map(|m| ModelStatusDto {
            name: m.name,
            downloaded: m.downloaded,
            path: m.path.display().to_string(),
        })
        .collect()
}

#[tauri::command]
async fn download_model(app: AppHandle, request: ModelRequestDto) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let tools = SystemToolchain::new();
        let events = TauriEvents {
            app,
            model_channel: true,
        };
        service(&tools, &events).download_model(&request.model)
    })
    .await
    .map_err(|e| format!("모델 다운로드 작업 오류: {e}"))?
}

#[tauri::command]
fn delete_model(app: AppHandle, request: ModelRequestDto) -> Result<(), String> {
    let tools = SystemToolchain::new();
    let events = TauriEvents {
        app,
        model_channel: true,
    };
    service(&tools, &events).delete_model(&request.model)
}

#[tauri::command]
async fn process_video(
    app: AppHandle,
    request: ProcessRequestDto,
) -> Result<ProcessResultDto, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let tools = SystemToolchain::new();
        let events = TauriEvents {
            app,
            model_channel: false,
        };
        let domain_request = domain::TranscriptionRequest {
            url: request.url,
            language: request.language,
            model: request.model,
            output_dir: request.output_dir.map(Into::into),
            force: request.force,
        };
        service(&tools, &events)
            .transcribe_video(domain_request)
            .map(|result| ProcessResultDto {
                url: result.url,
                title: result.title,
                transcript: result.transcript,
                transcript_path: result.transcript_path.display().to_string(),
                audio_path: result.audio_path.display().to_string(),
                json_path: result.json_path.display().to_string(),
                language: result.language,
                model: result.model,
                cached: result.cached,
                duration_seconds: result.duration_seconds,
            })
    })
    .await
    .map_err(|e| format!("작업 실행 오류: {e}"))?
}

pub fn run() {
    use tauri::{Manager, WindowEvent};
    use acp_agent_core::infrastructure::agent_session_registry::AppState;

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .on_window_event(|window, event| {
            // 창이 닫히면 그 창이 소유한 진행 중 run을 모두 취소해 자원 누수를 막는다(계약 CT-4).
            if let WindowEvent::Destroyed = event {
                let label = window.label().to_string();
                let state = window.state::<AppState>().inner().clone();
                tauri::async_runtime::spawn(async move {
                    state.cancel_runs_owned_by(&label).await;
                });
            }
        })
        .invoke_handler(tauri::generate_handler![
            check_dependencies,
            get_model_status,
            download_model,
            delete_model,
            process_video,
            crate::adapters::agent::start_agent_run,
            crate::adapters::agent::send_prompt_to_run,
            crate::adapters::agent::set_run_permission_mode,
            crate::adapters::agent::cancel_agent_run,
            crate::adapters::agent::respond_agent_permission,
            crate::adapters::agent::save_organized_document,
            crate::adapters::agent::save_chat_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running Hushline");
}
