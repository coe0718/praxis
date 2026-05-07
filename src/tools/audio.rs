//! Centralized Audio Routing — manage audio I/O across tools and platforms.
//!
//! Provides a single point of control for:
//! - Audio format conversion (WAV, MP3, FLAC, OGG)
//! - Audio file storage and lifecycle
//! - Routing audio between tools (voice → Telegram, browser → STT, etc.)
//! - FLAC compression for efficient storage/transmission

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::paths::PraxisPaths;

/// Supported audio formats for routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    Wav,
    Mp3,
    Flac,
    Ogg,
}

impl AudioFormat {
    /// Detect format from file extension.
    pub fn from_extension(path: &Path) -> Option<Self> {
        match path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()).as_deref() {
            Some("wav") => Some(Self::Wav),
            Some("mp3") => Some(Self::Mp3),
            Some("flac") => Some(Self::Flac),
            Some("ogg") => Some(Self::Ogg),
            _ => None,
        }
    }

    /// File extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Mp3 => "mp3",
            Self::Flac => "flac",
            Self::Ogg => "ogg",
        }
    }

    /// MIME type for this format.
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Wav => "audio/wav",
            Self::Mp3 => "audio/mpeg",
            Self::Flac => "audio/flac",
            Self::Ogg => "audio/ogg",
        }
    }
}

/// Audio routing manager — handles storage, conversion, and dispatch.
pub struct AudioRouter {
    audio_dir: PathBuf,
}

impl AudioRouter {
    /// Create a new router for the given data directory.
    pub fn new(paths: &PraxisPaths) -> Self {
        Self {
            audio_dir: paths.data_dir.join("audio"),
        }
    }

    /// Ensure the audio directory exists and return its path.
    pub fn ensure_dir(&self) -> Result<&Path> {
        fs::create_dir_all(&self.audio_dir)
            .with_context(|| format!("failed to create {}", self.audio_dir.display()))?;
        Ok(&self.audio_dir)
    }

    /// Generate a unique file path in the audio directory.
    pub fn new_path(&self, prefix: &str, format: AudioFormat) -> Result<PathBuf> {
        self.ensure_dir()?;
        let ts = chrono::Utc::now().timestamp_millis();
        let filename = format!("{prefix}_{ts}.{}", format.extension());
        Ok(self.audio_dir.join(filename))
    }

    /// Store audio data to a file, returning the path.
    pub fn store(&self, data: &[u8], prefix: &str, format: AudioFormat) -> Result<PathBuf> {
        let path = self.new_path(prefix, format)?;
        fs::write(&path, data).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(path)
    }

    /// List all audio files in the audio directory, sorted by modification time (newest first).
    pub fn list(&self) -> Result<Vec<PathBuf>> {
        if !self.audio_dir.exists() {
            return Ok(Vec::new());
        }
        let mut files: Vec<PathBuf> = fs::read_dir(&self.audio_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| {
                        matches!(ext.to_lowercase().as_str(), "wav" | "mp3" | "flac" | "ogg")
                    })
                    .unwrap_or(false)
            })
            .map(|e| e.path())
            .collect();

        // Sort newest first
        files.sort_by(|a, b| {
            let a_time = fs::metadata(a).and_then(|m| m.modified()).ok();
            let b_time = fs::metadata(b).and_then(|m| m.modified()).ok();
            b_time.cmp(&a_time)
        });

        Ok(files)
    }

    /// Clean up audio files older than `max_age_secs`.
    pub fn cleanup(&self, max_age_secs: u64) -> Result<usize> {
        if !self.audio_dir.exists() {
            return Ok(0);
        }
        let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(max_age_secs);
        let mut removed = 0;
        for entry in fs::read_dir(&self.audio_dir)? {
            let entry = entry?;
            if let Ok(modified) = entry.metadata().and_then(|m| m.modified())
                && modified < cutoff
                    && fs::remove_file(entry.path()).is_ok() {
                        removed += 1;
                    }
        }
        Ok(removed)
    }

    /// Convert audio from one format to another using ffmpeg.
    /// Falls back to a no-op copy if ffmpeg is not available and formats match.
    pub fn convert(&self, input: &Path, target_format: AudioFormat) -> Result<PathBuf> {
        let source_format = AudioFormat::from_extension(input)
            .ok_or_else(|| anyhow::anyhow!("unknown audio format for {}", input.display()))?;

        if source_format == target_format {
            return Ok(input.to_path_buf());
        }

        let output = self.new_path("converted", target_format)?;

        let status = std::process::Command::new("ffmpeg")
            .args(["-y", "-i"])
            .arg(input)
            .arg(&output)
            .output()
            .context("ffmpeg not found — install ffmpeg for audio conversion")?;

        if !status.status.success() {
            let stderr = String::from_utf8_lossy(&status.stderr);
            bail!("ffmpeg conversion failed: {}", stderr.chars().take(200).collect::<String>());
        }

        Ok(output)
    }
}
