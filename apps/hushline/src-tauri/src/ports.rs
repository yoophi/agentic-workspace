use crate::domain::{ModelStatus, ToolStatus, TranscriptChunk, TranscriptionResult, VideoMetadata};
use std::path::{Path, PathBuf};

pub trait EventPort: Send + Sync {
    fn progress(&self, stage: &str, progress: u8, message: &str, detail: Option<String>);
    fn transcript(&self, chunk: TranscriptChunk);
}

pub trait ToolchainPort: Send + Sync {
    fn status(&self) -> ToolStatus;
    fn default_output_dir(&self) -> PathBuf;
    fn ensure_directory(&self, path: &Path) -> Result<(), String>;
    fn metadata(&self, url: &str) -> Result<VideoMetadata, String>;
    fn download_audio(
        &self,
        url: &str,
        output_template: &Path,
        expected_audio: &Path,
        events: &dyn EventPort,
    ) -> Result<(), String>;
    fn transcribe(
        &self,
        audio: &Path,
        output_dir: &Path,
        base_name: &str,
        language: &str,
        model: &str,
        events: &dyn EventPort,
    ) -> Result<PathBuf, String>;
    fn read_text(&self, path: &Path) -> Result<String, String>;
    fn find_result(
        &self,
        output_dir: &Path,
        url: &str,
    ) -> Result<Option<TranscriptionResult>, String>;
    fn save_result(
        &self,
        output_dir: &Path,
        base_name: &str,
        result: &TranscriptionResult,
    ) -> Result<PathBuf, String>;
    fn model_statuses(&self) -> Vec<ModelStatus>;
    fn download_model(&self, model: &str, events: &dyn EventPort) -> Result<(), String>;
    fn delete_model(&self, model: &str) -> Result<(), String>;
}
