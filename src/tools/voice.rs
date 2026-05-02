use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::paths::PraxisPaths;

/// Voice tool for Speech-to-Text (STT) and Text-to-Speech (TTS).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceTool {
    pub name: String,
    pub description: String,
    pub parameters: VoiceParameters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceParameters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

impl VoiceTool {
    pub fn new() -> Self {
        Self {
            name: "voice".to_string(),
            description: "Speech-to-Text (STT) and Text-to-Speech (TTS) capabilities.".to_string(),
            parameters: VoiceParameters {
                action: None,
                text: None,
                audio_path: None,
                voice: None,
                language: None,
            },
        }
    }

    /// Execute the voice tool with the given parameters.
    pub fn execute(
        &self,
        params: &VoiceParameters,
        paths: &PraxisPaths,
    ) -> Result<String> {
        let action = params.action.as_deref().unwrap_or("tts");

        match action {
            "stt" => self.speech_to_text(params, paths),
            "tts" => self.text_to_speech(params, paths),
            _ => bail!("Unknown voice action: {action}. Supported: stt, tts"),
        }
    }

    /// Convert speech to text.
    /// Placeholder implementation - requires whisper-rs or external service.
    fn speech_to_text(
        &self,
        params: &VoiceParameters,
        paths: &PraxisPaths,
    ) -> Result<String> {
        let audio_path = params.audio_path.as_deref()
            .ok_or_else(|| anyhow::anyhow!("stt requires 'audio_path' parameter"))?;

        let full_path = if Path::new(audio_path).is_relative() {
            paths.data_dir.join(audio_path)
        } else {
            PathBuf::from(audio_path)
        };

        if !full_path.exists() {
            bail!("Audio file not found: {}", full_path.display());
        }

        // Placeholder: In a real implementation, this would use whisper-rs or an external STT service
        Ok(format!("STT placeholder: Would transcribe {}", full_path.display()))
    }

    /// Convert text to speech.
    /// Placeholder implementation - requires edge-tts or external service.
    fn text_to_speech(
        &self,
        params: &VoiceParameters,
        _paths: &PraxisPaths,
    ) -> Result<String> {
        let text = params.text.as_deref()
            .ok_or_else(|| anyhow::anyhow!("tts requires 'text' parameter"))?;

        let voice = params.voice.as_deref().unwrap_or("en-US-AriaNeural");
        let language = params.language.as_deref().unwrap_or("en-US");

        // Placeholder: In a real implementation, this would use edge-tts or an external TTS service
        Ok(format!("TTS placeholder: Would convert '{}' to speech using voice '{}' in language '{}'", 
            text.chars().take(50).collect::<String>(), voice, language))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_voice_tool_tts() {
        let tool = VoiceTool::new();
        let temp_dir = tempdir().unwrap();
        let paths = PraxisPaths::for_data_dir(temp_dir.path().to_path_buf());
        let params = VoiceParameters {
            action: Some("tts".to_string()),
            text: Some("Hello, world!".to_string()),
            audio_path: None,
            voice: Some("en-US-AriaNeural".to_string()),
            language: Some("en-US".to_string()),
        };

        let result = tool.execute(&params, &paths).unwrap();
        assert!(result.contains("Hello, world!"));
        assert!(result.contains("en-US-AriaNeural"));
    }

    #[test]
    fn test_voice_tool_stt() {
        let tool = VoiceTool::new();
        let temp_dir = tempdir().unwrap();
        let audio_path = temp_dir.path().join("test.wav");
        std::fs::write(&audio_path, "fake audio data").unwrap();

        let paths = PraxisPaths::for_data_dir(temp_dir.path().to_path_buf());
        let params = VoiceParameters {
            action: Some("stt".to_string()),
            text: None,
            audio_path: Some("test.wav".to_string()),
            voice: None,
            language: None,
        };

        let result = tool.execute(&params, &paths).unwrap();
        assert!(result.contains("STT placeholder"));
    }

    #[test]
    fn test_voice_tool_missing_audio() {
        let tool = VoiceTool::new();
        let temp_dir = tempdir().unwrap();
        let paths = PraxisPaths::for_data_dir(temp_dir.path().to_path_buf());
        let params = VoiceParameters {
            action: Some("stt".to_string()),
            text: None,
            audio_path: Some("nonexistent.wav".to_string()),
            voice: None,
            language: None,
        };

        let result = tool.execute(&params, &paths);
        assert!(result.is_err());
    }
}
