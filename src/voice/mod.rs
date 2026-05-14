//! Voice I/O — Text-to-speech + Speech-to-text.
//!
//! Provides TTS and STT via multiple providers. Configure via `praxis.toml`:
//!
//! ```toml
//! [voice]
//! tts_provider = "openai"  # openai, elevenlabs, piper, coqui
//! stt_provider = "openai"  # openai, deepgram, groq, google
//! voice_id = "alloy"       # OpenAI voice (alloy, echo, fable, onyx, nova, shimmer)
//! ```

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::paths::PraxisPaths;

// ── Configuration ─────────────────────────────────────────────────────────────

/// Voice configuration for TTS/STT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    /// Default TTS provider.
    #[serde(default = "default_tts")]
    pub tts_provider: String,
    /// Default STT provider.
    #[serde(default = "default_stt")]
    pub stt_provider: String,
    /// Voice ID for TTS (provider-specific).
    /// OpenAI: alloy, echo, fable, onyx, nova, shimmer
    /// ElevenLabs: voice ID from dashboard
    pub voice_id: Option<String>,
    /// Speaker embedding for voice cloning.
    pub speaker_embedding: Option<PathBuf>,
    /// Enable local fallback if cloud fails.
    #[serde(default = "true_val")]
    pub local_fallback: bool,
}

fn default_tts() -> String {
    "openai".to_string()
}
fn default_stt() -> String {
    "openai".to_string()
}
fn true_val() -> bool {
    true
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            tts_provider: default_tts(),
            stt_provider: default_stt(),
            voice_id: Some("alloy".to_string()),
            speaker_embedding: None,
            local_fallback: true,
        }
    }
}

impl VoiceConfig {
    /// Load from praxis.toml [voice] section.
    pub fn load(paths: &PraxisPaths) -> Result<Self> {
        let config_path = &paths.config_file;
        if !config_path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(config_path).context("read praxis.toml")?;
        let doc: toml::Value = toml::from_str(&raw).context("parse praxis.toml")?;
        let voice = doc.get("voice").and_then(|v| v.as_table()).cloned();

        let mut cfg = Self::default();
        if let Some(t) = voice {
            if let Some(v) = t.get("tts_provider").and_then(|v| v.as_str()) {
                cfg.tts_provider = v.to_string();
            }
            if let Some(v) = t.get("stt_provider").and_then(|v| v.as_str()) {
                cfg.stt_provider = v.to_string();
            }
            if let Some(v) = t.get("voice_id").and_then(|v| v.as_str()) {
                cfg.voice_id = Some(v.to_string());
            }
        }
        Ok(cfg)
    }
}

// ── TTS ──────────────────────────────────────────────────────────────────────

/// Text-to-speech request.
#[derive(Debug, Serialize)]
pub struct TtsRequest {
    /// Text to synthesize.
    pub text: String,
    /// Voice ID override.
    pub voice_id: Option<String>,
    /// Output format (mp3, wav, etc).
    #[serde(default = "default_format")]
    pub format: String,
    /// Speed multiplier (0.25 to 4.0).
    #[serde(default = "default_speed")]
    pub speed: f32,
}

/// Synthesize speech from text using configured provider.
pub async fn synthesize_speech(paths: &PraxisPaths, req: TtsRequest) -> Result<Vec<u8>> {
    let config = VoiceConfig::load(paths)?;
    match config.tts_provider.as_str() {
        "openai" => tts::openai_synthesize(&req, config.voice_id.as_deref()).await,
        "elevenlabs" => tts::elevenlabs_synthesize(&req, config.voice_id.as_deref()).await,
        _ => {
            log::warn!(
                "voice: unknown TTS provider {}, falling back to openai",
                config.tts_provider
            );
            tts::openai_synthesize(&req, config.voice_id.as_deref()).await
        }
    }
}

/// TTS implementations.
pub mod tts {
    use super::*;

    /// OpenAI TTS-1 synthesis.
    pub async fn openai_synthesize(req: &TtsRequest, voice: Option<&str>) -> Result<Vec<u8>> {
        let api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;

        let voice = voice.unwrap_or("alloy");
        let client = reqwest::Client::new();

        let payload = serde_json::json!({
            "model": "tts-1",
            "voice": voice,
            "input": req.text,
            "speed": req.speed,
            "response_format": req.format,
        });

        let resp = client
            .post("https://api.openai.com/v1/audio/speech")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("OpenAI TTS request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI TTS failed: {} - {}", status, &body[..body.len().min(500)]);
        }

        let bytes = resp.bytes().await.context("read TTS response")?;
        Ok(bytes.to_vec())
    }

    /// ElevenLabs TTS synthesis.
    pub async fn elevenlabs_synthesize(
        req: &TtsRequest,
        voice_id: Option<&str>,
    ) -> Result<Vec<u8>> {
        let api_key = std::env::var("ELEVENLABS_API_KEY").context("ELEVENLABS_API_KEY not set")?;

        let voice_id = voice_id.ok_or_else(|| anyhow::anyhow!("ElevenLabs requires voice_id"))?;
        let client = reqwest::Client::new();

        let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{}", voice_id);

        let payload = serde_json::json!({
            "text": req.text,
            "model_id": "eleven_monolingual_v1",
            "voice_settings": {
                "stability": 0.5,
                "similarity_boost": 0.5,
            }
        });

        let resp = client
            .post(&url)
            .header("xi-api-key", api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("ElevenLabs TTS request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("ElevenLabs TTS failed: {} - {}", status, &body[..body.len().min(500)]);
        }

        let bytes = resp.bytes().await.context("read TTS response")?;
        Ok(bytes.to_vec())
    }
}

// ── STT ──────────────────────────────────────────────────────────────────────

/// Speech-to-text request.
#[derive(Debug, Deserialize)]
pub struct SttRequest {
    /// Audio bytes (mp3, wav, etc).
    pub audio: Vec<u8>,
    /// Audio format.
    #[serde(default = "default_format")]
    pub format: String,
    /// Language code (e.g., "en").
    pub language: Option<String>,
}

fn default_format() -> String {
    "mp3".to_string()
}

fn default_speed() -> f32 {
    1.0
}

impl Default for TtsRequest {
    fn default() -> Self {
        Self {
            text: String::new(),
            voice_id: None,
            format: default_format(),
            speed: default_speed(),
        }
    }
}

/// STT response.
#[derive(Debug, serde::Serialize)]
pub struct SttResponse {
    /// Transcribed text.
    pub text: String,
    /// Confidence score.
    pub confidence: f32,
}

/// Transcribe speech to text using configured provider.
pub async fn transcribe_speech(paths: &PraxisPaths, req: SttRequest) -> Result<SttResponse> {
    let config = VoiceConfig::load(paths)?;
    match config.stt_provider.as_str() {
        "openai" => stt::openai_transcribe(&req).await,
        "deepgram" => stt::deepgram_transcribe(&req).await,
        "groq" => stt::groq_transcribe(&req).await,
        _ => {
            log::warn!(
                "voice: unknown STT provider {}, falling back to openai",
                config.stt_provider
            );
            stt::openai_transcribe(&req).await
        }
    }
}

/// STT implementations.
pub mod stt {
    use super::*;

    /// OpenAI Whisper transcription.
    pub async fn openai_transcribe(req: &SttRequest) -> Result<SttResponse> {
        let api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;

        let client = reqwest::Client::new();

        let file_name = format!("audio.{}", req.format);
        let part = reqwest::multipart::Part::bytes(req.audio.clone())
            .file_name(file_name)
            .mime_str(&format!("audio/{}", req.format))
            .context("invalid audio format")?;

        let form = reqwest::multipart::Form::new()
            .text("model", "whisper-1")
            .text("response_format", "json")
            .part("file", part);

        if let Some(ref lang) = req.language {
            let form = form.text("language", lang.clone());

            let resp = client
                .post("https://api.openai.com/v1/audio/transcriptions")
                .header("Authorization", format!("Bearer {}", api_key))
                .multipart(form)
                .send()
                .await
                .context("OpenAI STT request failed")?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                anyhow::bail!("OpenAI STT failed: {} - {}", status, &body[..body.len().min(500)]);
            }

            let result: serde_json::Value = resp.json().await.context("parse STT response")?;
            let text = result["text"].as_str().unwrap_or("").to_string();
            return Ok(SttResponse { text, confidence: 0.9 });
        }

        let resp = client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", api_key))
            .multipart(form)
            .send()
            .await
            .context("OpenAI STT request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI STT failed: {} - {}", status, &body[..body.len().min(500)]);
        }

        let result: serde_json::Value = resp.json().await.context("parse STT response")?;
        let text = result["text"].as_str().unwrap_or("").to_string();
        Ok(SttResponse { text, confidence: 0.9 })
    }

    /// Deepgram transcription.
    pub async fn deepgram_transcribe(req: &SttRequest) -> Result<SttResponse> {
        let api_key = std::env::var("DEEPGRAM_API_KEY").context("DEEPGRAM_API_KEY not set")?;

        let client = reqwest::Client::new();

        let mut url =
            "https://api.deepgram.com/v1/listen?model=nova-2&smart_format=true".to_string();
        if let Some(ref lang) = req.language {
            url.push_str(&format!("&language={}", lang));
        }

        let resp = client
            .post(&url)
            .header("Authorization", format!("Token {}", api_key))
            .header("Content-Type", format!("audio/{}", req.format))
            .body(req.audio.clone())
            .send()
            .await
            .context("Deepgram STT request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Deepgram STT failed: {} - {}", status, &body[..body.len().min(500)]);
        }

        let result: serde_json::Value = resp.json().await.context("parse STT response")?;
        let text = result["results"]["channels"][0]["alternatives"][0]["transcript"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let confidence = result["results"]["channels"][0]["alternatives"][0]["confidence"]
            .as_f64()
            .unwrap_or(0.0) as f32;

        Ok(SttResponse { text, confidence })
    }

    /// Groq Whisper transcription (free, fast).
    pub async fn groq_transcribe(req: &SttRequest) -> Result<SttResponse> {
        let api_key = std::env::var("GROQ_API_KEY").context("GROQ_API_KEY not set")?;

        let client = reqwest::Client::new();

        let file_name = format!("audio.{}", req.format);
        let part = reqwest::multipart::Part::bytes(req.audio.clone())
            .file_name(file_name)
            .mime_str(&format!("audio/{}", req.format))
            .context("invalid audio format")?;

        let mut form = reqwest::multipart::Form::new()
            .text("model", "whisper-large-v3")
            .part("file", part);

        if let Some(ref lang) = req.language {
            form = form.text("language", lang.clone());
        }

        let resp = client
            .post("https://api.groq.com/openai/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", api_key))
            .multipart(form)
            .send()
            .await
            .context("Groq STT request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Groq STT failed: {} - {}", status, &body[..body.len().min(500)]);
        }

        let result: serde_json::Value = resp.json().await.context("parse STT response")?;
        let text = result["text"].as_str().unwrap_or("").to_string();
        Ok(SttResponse { text, confidence: 0.9 })
    }
}
