//! Voice I/O — Text-to-speech + Speech-to-text.
//!
//! Moltis has 8 TTS + 7 STT providers. This module provides the integration layer.
//!
//! # TTS Providers
//! - ElevenLabs
//! - OpenAI TTS
//! - Piper (local)
//! - Coqui (local)
//!
//! # STT Providers
//! - whisper.cpp (local)
//! - OpenAI Whisper
//! - Groq
//! - Deepgram
//! - Google Speech-to-Text
//! - Mistral Voxtral
//! - SherpaOnnx

use std::path::PathBuf;

use anyhow::Result;
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
    pub voice_id: Option<String>,
    /// Speaker embedding for voice cloning.
    pub speaker_embedding: Option<PathBuf>,
    /// Enable local fallback if cloud fails.
    #[serde(default = "true_val")]
    pub local_fallback: bool,
}

fn default_tts() -> String { "openai".to_string() }
fn default_stt() -> String { "whisper".to_string() }
fn true_val() -> bool { true }

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            tts_provider: default_tts(),
            stt_provider: default_stt(),
            voice_id: None,
            speaker_embedding: None,
            local_fallback: true,
        }
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
    #[serde(default = "default_audio_format")]
    pub format: String,
    /// Speed multiplier.
    #[serde(default = "default_speed")]
    pub speed: f32,
}

fn default_audio_format() -> String { "mp3".to_string() }
fn default_speed() -> f32 { 1.0 }

/// Synthesize speech from text.
pub async fn synthesize_speech(
    _paths: &PraxisPaths,
    _req: TtsRequest,
) -> Result<Vec<u8>> {
    // Placeholder - would integrate with providers
    anyhow::bail!("TTS not yet implemented")
}

// ── STT ──────────────────────────────────────────────────────────────────────

/// Speech-to-text request.
#[derive(Debug, Deserialize)]
pub struct SttRequest {
    /// Audio bytes (mp3, wav, etc).
    pub audio: Vec<u8>,
    /// Audio format.
    pub format: String,
    /// Language code.
    #[serde(default)]
    pub language: Option<String>,
}

/// Transcribe speech to text.
pub async fn transcribe_speech(
    _paths: &PraxisPaths,
    _req: SttRequest,
) -> Result<SttResponse> {
    // Placeholder - would integrate with providers
    anyhow::bail!("STT not yet implemented")
}

/// STT response.
#[derive(Debug, Serialize)]
pub struct SttResponse {
    /// Transcribed text.
    pub text: String,
    /// Confidence score (0-1).
    pub confidence: Option<f32>,
    /// Detected language.
    pub language: Option<String>,
}

// ── Provider Implementations ──────────────────────────────────────────────

/// Available TTS providers.
pub mod tts {
    use super::*;
    
    pub struct ElevenLabs;
    pub struct OpenAiTts;
    pub struct Piper;
    pub struct Coqui;
    
    impl ElevenLabs {
        pub async fn synthesize(&self, _text: &str) -> Result<Vec<u8>> {
            anyhow::bail!("ElevenLabs TTS not implemented")
        }
    }
}

/// Available STT providers.
pub mod stt {
    use super::*;
    
    pub struct WhisperCpp;
    pub struct OpenAiStt;
    pub struct Groq;
    pub struct Deepgram;
    pub struct GoogleStt;
    pub struct Voxtral;
    pub struct SherpaOnnx;
    
    impl WhisperCpp {
        pub async fn transcribe(&self, _audio: &[u8]) -> Result<SttResponse> {
            anyhow::bail!("Whisper.cpp STT not implemented")
        }
    }
}