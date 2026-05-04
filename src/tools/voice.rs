use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

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

/// Check whether a command exists on `$PATH` by running `command --version`.
fn command_exists(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Also try `command -h` as a fallback (espeak uses this).
fn command_exists_alt(name: &str) -> bool {
    if command_exists(name) {
        return true;
    }
    Command::new(name)
        .arg("-h")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
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
    pub fn execute(&self, params: &VoiceParameters, paths: &PraxisPaths) -> Result<String> {
        let action = params.action.as_deref().unwrap_or("tts");

        match action {
            "stt" => self.speech_to_text(params, paths),
            "tts" => self.text_to_speech(params, paths),
            _ => bail!("Unknown voice action: {action}. Supported: stt, tts"),
        }
    }

    /// Convert speech to text using whisper CLI.
    fn speech_to_text(&self, params: &VoiceParameters, paths: &PraxisPaths) -> Result<String> {
        let audio_path = params
            .audio_path
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("stt requires 'audio_path' parameter"))?;

        let full_path = if Path::new(audio_path).is_relative() {
            paths.data_dir.join(audio_path)
        } else {
            PathBuf::from(audio_path)
        };

        if !full_path.exists() {
            bail!("Audio file not found: {}", full_path.display());
        }

        let language = params.language.as_deref().unwrap_or("en");

        // Try whisper CLI first
        if command_exists("whisper") {
            return self.stt_whisper(&full_path, language);
        }

        bail!(
            "No STT backend found. Install one of:\n  \
             - whisper (pip install openai-whisper)\n  \
             Then retry the voice stt command."
        );
    }

    /// Run OpenAI whisper CLI for speech-to-text.
    fn stt_whisper(&self, audio_path: &Path, language: &str) -> Result<String> {
        let output = Command::new("whisper")
            .arg(audio_path)
            .arg("--language")
            .arg(language)
            .arg("--model")
            .arg("base")
            .arg("--output_format")
            .arg("txt")
            .arg("--output_dir")
            .arg("-")
            .output()
            .context("Failed to execute whisper command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("whisper failed: {}", stderr.trim());
        }

        let transcript = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if transcript.is_empty() {
            bail!("whisper produced empty transcript for {}", audio_path.display());
        }

        Ok(format!("Transcript (whisper): {}", transcript))
    }

    /// Convert text to speech. Tries espeak, then edge-tts.
    /// Saves audio to `paths.data_dir/audio/`.
    fn text_to_speech(&self, params: &VoiceParameters, paths: &PraxisPaths) -> Result<String> {
        let text = params
            .text
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("tts requires 'text' parameter"))?;

        let language = params.language.as_deref().unwrap_or("en-US");

        // Ensure output directory exists
        let audio_dir = paths.data_dir.join("audio");
        std::fs::create_dir_all(&audio_dir).context("Failed to create audio output directory")?;

        // Generate a safe filename from a hash of the text + timestamp
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let text_hash = simple_hash(text);
        let output_path = audio_dir.join(format!("tts_{}_{}.wav", timestamp, text_hash));

        // Try backends in order of preference
        if command_exists_alt("espeak") {
            return self.tts_espeak(text, language, &output_path);
        }

        if command_exists("edge-tts") {
            return self.tts_edge_tts(text, params.voice.as_deref(), language, &output_path);
        }

        bail!(
            "No TTS backend found. Install one of:\n  \
             - espeak (apt install espeak / brew install espeak)\n  \
             - edge-tts (pip install edge-tts)\n  \
             Then retry the voice tts command."
        );
    }

    /// TTS via espeak — writes WAV to `output_path`.
    fn tts_espeak(&self, text: &str, language: &str, output_path: &Path) -> Result<String> {
        // espeak uses ISO 639-1 codes; try to extract from "en-US" style tag
        let lang_code = language.split('-').next().unwrap_or("en");

        let output = Command::new("espeak")
            .arg("-v")
            .arg(lang_code)
            .arg("--stdout")
            .arg(text)
            .output()
            .context("Failed to execute espeak")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("espeak failed: {}", stderr.trim());
        }

        if output.stdout.is_empty() {
            bail!("espeak produced no audio data");
        }

        std::fs::write(output_path, &output.stdout).context("Failed to write TTS audio file")?;

        Ok(format!(
            "TTS audio saved to {} (espeak, {} bytes)",
            output_path.display(),
            output.stdout.len()
        ))
    }

    /// TTS via edge-tts (Python) — writes MP3 to `output_path`.
    fn tts_edge_tts(
        &self,
        text: &str,
        voice: Option<&str>,
        language: &str,
        output_path: &Path,
    ) -> Result<String> {
        // Default voice based on language
        let voice = voice.unwrap_or_else(|| match language {
            "en-US" => "en-US-AriaNeural",
            "en-GB" => "en-GB-SoniaNeural",
            "fr-FR" => "fr-FR-DeniseNeural",
            "de-DE" => "de-DE-KatjaNeural",
            "es-ES" => "es-ES-ElviraNeural",
            "ja-JP" => "ja-JP-NanamiNeural",
            "zh-CN" => "zh-CN-XiaoxiaoNeural",
            _ => "en-US-AriaNeural",
        });

        // edge-tts outputs mp3; adjust extension
        let mp3_path = output_path.with_extension("mp3");

        let output = Command::new("edge-tts")
            .arg("--voice")
            .arg(voice)
            .arg("--text")
            .arg(text)
            .arg("--write-media")
            .arg(&mp3_path)
            .output()
            .context("Failed to execute edge-tts")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("edge-tts failed: {}", stderr.trim());
        }

        if !mp3_path.exists() {
            bail!("edge-tts did not produce output file");
        }

        let file_size = std::fs::metadata(&mp3_path)?.len();

        Ok(format!(
            "TTS audio saved to {} (edge-tts, voice={}, {} bytes)",
            mp3_path.display(),
            voice,
            file_size
        ))
    }
}

/// Simple deterministic hash for filename generation (not cryptographic).
fn simple_hash(s: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:08x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Helper: create a PraxisPaths pointing at a temp dir.
    fn test_paths(temp: &tempfile::TempDir) -> PraxisPaths {
        PraxisPaths::for_data_dir(temp.path().to_path_buf())
    }

    // ── TTS ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_tts_missing_text_param() {
        let tool = VoiceTool::new();
        let temp = tempdir().unwrap();
        let paths = test_paths(&temp);
        let params = VoiceParameters {
            action: Some("tts".into()),
            text: None,
            audio_path: None,
            voice: None,
            language: None,
        };
        let result = tool.execute(&params, &paths);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("'text' parameter"),
            "should complain about missing text param"
        );
    }

    #[test]
    fn test_tts_no_backend_available() {
        // This test verifies graceful fallback when no TTS tool is installed.
        // If espeak or edge-tts IS installed on the CI machine the test still
        // passes — it just exercises the real backend instead.
        let tool = VoiceTool::new();
        let temp = tempdir().unwrap();
        let paths = test_paths(&temp);
        let params = VoiceParameters {
            action: Some("tts".into()),
            text: Some("Hello, world!".into()),
            audio_path: None,
            voice: Some("en-US-AriaNeural".into()),
            language: Some("en-US".into()),
        };

        let result = tool.execute(&params, &paths);
        if let Ok(msg) = &result {
            // A backend was available — verify it produced output
            assert!(
                msg.contains("TTS audio saved to") || msg.contains("tts_"),
                "TTS should report saved file: {msg}"
            );
        } else {
            // No backend available — verify the error is helpful
            let err = result.unwrap_err().to_string();
            assert!(err.contains("No TTS backend found"), "should explain missing backend: {err}");
        }
    }

    // ── STT ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_stt_missing_audio_param() {
        let tool = VoiceTool::new();
        let temp = tempdir().unwrap();
        let paths = test_paths(&temp);
        let params = VoiceParameters {
            action: Some("stt".into()),
            text: None,
            audio_path: None,
            voice: None,
            language: None,
        };
        let result = tool.execute(&params, &paths);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("'audio_path' parameter"),
            "should complain about missing audio_path param"
        );
    }

    #[test]
    fn test_stt_file_not_found() {
        let tool = VoiceTool::new();
        let temp = tempdir().unwrap();
        let paths = test_paths(&temp);
        let params = VoiceParameters {
            action: Some("stt".into()),
            text: None,
            audio_path: Some("nonexistent.wav".into()),
            voice: None,
            language: None,
        };
        let result = tool.execute(&params, &paths);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("not found"),
            "should report missing file"
        );
    }

    #[test]
    fn test_stt_no_backend_available() {
        let tool = VoiceTool::new();
        let temp = tempdir().unwrap();
        // Write a dummy audio file so we get past the file-exists check
        let audio_path = temp.path().join("test.wav");
        std::fs::write(&audio_path, b"fake audio data").unwrap();
        let paths = test_paths(&temp);
        let params = VoiceParameters {
            action: Some("stt".into()),
            text: None,
            audio_path: Some("test.wav".into()),
            voice: None,
            language: None,
        };

        let result = tool.execute(&params, &paths);
        if let Ok(msg) = &result {
            // whisper was available — it probably failed on fake audio but
            // that's fine; we just verify it didn't panic.
            assert!(
                msg.contains("Transcript") || msg.contains("whisper"),
                "STT should mention whisper: {msg}"
            );
        } else {
            // No backend — verify the helpful error
            let err = result.unwrap_err().to_string();
            assert!(err.contains("No STT backend found"), "should explain missing backend: {err}");
        }
    }

    // ── Utility ──────────────────────────────────────────────────────────────

    #[test]
    fn test_simple_hash_deterministic() {
        let h1 = simple_hash("hello");
        let h2 = simple_hash("hello");
        assert_eq!(h1, h2, "same input must produce same hash");
        let h3 = simple_hash("world");
        assert_ne!(h1, h3, "different input should produce different hash");
    }

    #[test]
    fn test_unknown_action() {
        let tool = VoiceTool::new();
        let temp = tempdir().unwrap();
        let paths = test_paths(&temp);
        let params = VoiceParameters {
            action: Some("foobar".into()),
            text: None,
            audio_path: None,
            voice: None,
            language: None,
        };
        let result = tool.execute(&params, &paths);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("Unknown voice action"),
            "should reject unknown action"
        );
    }
}
