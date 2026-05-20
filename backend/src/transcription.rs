use crate::models::{AudioTranscript, AudioTranscriptSegment};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::Utc;
use std::{path::PathBuf, process::Stdio};
use tokio::{fs, process::Command};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub enum LocalTranscriptionEngine {
    Disabled,
    Command {
        command: String,
        model_path: Option<String>,
    },
    WhisperCpp {
        binary_path: String,
        model_path: String,
    },
}

impl LocalTranscriptionEngine {
    pub fn from_env() -> Self {
        match std::env::var("OG_TRANSCRIPTION_ENGINE").as_deref() {
            Ok("command") => {
                let Ok(command) = std::env::var("OG_TRANSCRIPTION_COMMAND") else {
                    tracing::warn!(
                        "OG_TRANSCRIPTION_ENGINE=command requires OG_TRANSCRIPTION_COMMAND"
                    );
                    return Self::Disabled;
                };
                Self::Command {
                    command,
                    model_path: std::env::var("OG_TRANSCRIPTION_MODEL").ok(),
                }
            }
            Ok("whisper_cpp") => {
                let Ok(binary_path) = std::env::var("OG_WHISPER_CPP_BIN") else {
                    tracing::warn!(
                        "OG_TRANSCRIPTION_ENGINE=whisper_cpp requires OG_WHISPER_CPP_BIN"
                    );
                    return Self::Disabled;
                };
                let Ok(model_path) = std::env::var("OG_WHISPER_CPP_MODEL") else {
                    tracing::warn!(
                        "OG_TRANSCRIPTION_ENGINE=whisper_cpp requires OG_WHISPER_CPP_MODEL"
                    );
                    return Self::Disabled;
                };
                Self::WhisperCpp {
                    binary_path,
                    model_path,
                }
            }
            Ok(other) => {
                tracing::warn!(
                    "unsupported OG_TRANSCRIPTION_ENGINE={other}; transcription worker disabled"
                );
                Self::Disabled
            }
            Err(_) => Self::auto_detect(),
        }
    }

    fn auto_detect() -> Self {
        let model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("models")
            .join("ggml-tiny.en.bin");
        if !model_path.exists() {
            return Self::Disabled;
        }

        for command_name in ["whisper-cli", "main"] {
            if let Some(binary_path) = find_command(command_name) {
                tracing::info!(
                    "auto-detected local whisper.cpp transcription with model {}",
                    model_path.display()
                );
                return Self::WhisperCpp {
                    binary_path,
                    model_path: model_path.to_string_lossy().to_string(),
                };
            }
        }

        Self::Disabled
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Command { .. } => "command",
            Self::WhisperCpp { .. } => "whisper_cpp",
        }
    }

    pub async fn transcribe(
        &self,
        recording_id: Uuid,
        data_url: &str,
        mime_type: &str,
        duration_ms: u64,
    ) -> Option<AudioTranscript> {
        if matches!(self, Self::Disabled) {
            return None;
        }

        let input_path = match write_data_url_to_temp_file(recording_id, data_url, mime_type).await
        {
            Ok(path) => path,
            Err(error) => {
                tracing::warn!("failed to prepare local transcription input: {error}");
                return Some(failed_transcript(
                    recording_id,
                    duration_ms,
                    error.to_string(),
                ));
            }
        };

        let transcription_input = self
            .prepare_transcription_input(recording_id, &input_path)
            .await;
        let command_input = transcription_input.as_ref().unwrap_or(&input_path);

        let output = match self.run_command(command_input).await {
            Ok(text) => ready_transcript(recording_id, duration_ms, text),
            Err(error) => {
                tracing::warn!("local transcription failed: {error}");
                failed_transcript(recording_id, duration_ms, error)
            }
        };

        if let Some(transcription_input) = transcription_input {
            let _ = fs::remove_file(transcription_input).await;
        }
        let _ = fs::remove_file(input_path).await;
        Some(output)
    }

    async fn prepare_transcription_input(
        &self,
        recording_id: Uuid,
        input_path: &PathBuf,
    ) -> Option<PathBuf> {
        if !matches!(self, Self::WhisperCpp { .. }) {
            return None;
        }
        let wav_path = std::env::temp_dir().join(format!("og-suite-{recording_id}-whisper.wav"));
        let output = Command::new("ffmpeg")
            .arg("-y")
            .arg("-i")
            .arg(input_path)
            .arg("-ar")
            .arg("16000")
            .arg("-ac")
            .arg("1")
            .arg(&wav_path)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await;
        match output {
            Ok(output) if output.status.success() => Some(wav_path),
            Ok(output) => {
                tracing::warn!(
                    "ffmpeg normalization failed; using original audio: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                let _ = fs::remove_file(&wav_path).await;
                None
            }
            Err(error) => {
                tracing::warn!("ffmpeg normalization unavailable; using original audio: {error}");
                None
            }
        }
    }

    async fn run_command(&self, input_path: &PathBuf) -> Result<String, String> {
        let output = match self {
            Self::Disabled => return Err("transcription engine is disabled".to_string()),
            Self::Command {
                command,
                model_path,
            } => {
                let mut process = Command::new("sh");
                process
                    .arg("-c")
                    .arg(command)
                    .env("INPUT_PATH", input_path)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                if let Some(model_path) = model_path {
                    process.env("MODEL_PATH", model_path);
                }
                process.output().await.map_err(|error| error.to_string())?
            }
            Self::WhisperCpp {
                binary_path,
                model_path,
            } => Command::new(binary_path)
                .arg("-m")
                .arg(model_path)
                .arg("-f")
                .arg(input_path)
                .arg("-nt")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(|error| error.to_string())?,
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if stderr.is_empty() {
                format!("transcription command exited with {}", output.status)
            } else {
                stderr
            });
        }

        let text = clean_transcript_output(&String::from_utf8_lossy(&output.stdout));
        if text.is_empty() {
            return Err("transcription command returned no text".to_string());
        }
        Ok(text)
    }
}

fn clean_transcript_output(output: &str) -> String {
    output
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !line.starts_with("whisper_")
                && !line.starts_with("main:")
                && !line.starts_with("system_info:")
                && !line.starts_with("load_backend:")
                && !line.starts_with("ggml_")
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn find_command(name: &str) -> Option<String> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|path| path.join(name))
            .find(|path| path.is_file())
            .map(|path| path.to_string_lossy().to_string())
    })
}

async fn write_data_url_to_temp_file(
    recording_id: Uuid,
    data_url: &str,
    mime_type: &str,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let encoded = data_url
        .split_once(',')
        .map(|(_, payload)| payload)
        .unwrap_or(data_url);
    let bytes = STANDARD.decode(encoded.as_bytes())?;
    let extension = match mime_type {
        "audio/wav" | "audio/wave" | "audio/x-wav" => "wav",
        "audio/mp4" | "audio/m4a" => "m4a",
        "audio/mpeg" | "audio/mp3" => "mp3",
        "audio/ogg" => "ogg",
        _ => "webm",
    };
    let path = std::env::temp_dir().join(format!("og-suite-{recording_id}.{extension}"));
    fs::write(&path, bytes).await?;
    Ok(path)
}

fn ready_transcript(recording_id: Uuid, duration_ms: u64, text: String) -> AudioTranscript {
    AudioTranscript {
        recording_id,
        status: "ready".to_string(),
        segments: vec![AudioTranscriptSegment {
            id: Uuid::new_v4(),
            recording_id,
            channel: Some(1),
            speaker_label: Some("Speaker 1".to_string()),
            start_ms: 0,
            end_ms: duration_ms,
            text,
        }],
        updated_at: Utc::now(),
    }
}

fn failed_transcript(recording_id: Uuid, duration_ms: u64, reason: String) -> AudioTranscript {
    AudioTranscript {
        recording_id,
        status: "failed".to_string(),
        segments: vec![AudioTranscriptSegment {
            id: Uuid::new_v4(),
            recording_id,
            channel: Some(1),
            speaker_label: Some("System".to_string()),
            start_ms: 0,
            end_ms: duration_ms,
            text: format!("Local transcription failed: {reason}"),
        }],
        updated_at: Utc::now(),
    }
}
