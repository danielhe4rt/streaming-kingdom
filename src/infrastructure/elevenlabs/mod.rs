use std::path::Path;

use tokio::process::Command;
use tokio::sync::mpsc;

use crate::application::TtsConfig;

pub struct TtsRequest {
    pub text: String,
    pub username: String,
}

/// Spawn the TTS worker task. Returns `None` if the API key is not configured.
pub fn spawn(config: TtsConfig) -> Option<mpsc::Sender<TtsRequest>> {
    if config.elevenlabs_api_key.is_empty() {
        tracing::info!("ElevenLabs API key not configured, TTS disabled");
        return None;
    }

    let (tx, rx) = mpsc::channel(8);
    tokio::spawn(tts_worker(rx, config));
    Some(tx)
}

pub async fn tts_worker(mut rx: mpsc::Receiver<TtsRequest>, config: TtsConfig) {
    let client = reqwest::Client::new();

    let cache_dir = match dirs::cache_dir() {
        Some(d) => d.join("streams-toolkit").join("tts"),
        None => {
            tracing::error!("could not determine cache directory, TTS worker exiting");
            return;
        }
    };

    if let Err(e) = std::fs::create_dir_all(&cache_dir) {
        tracing::error!(
            "failed to create TTS cache dir {}: {e}",
            cache_dir.display()
        );
        return;
    }

    tracing::info!("TTS worker started (voice_id={})", config.voice_id);

    while let Some(req) = rx.recv().await {
        tracing::info!(username = %req.username, "processing TTS request");

        if let Err(e) = process_request(&client, &config, &req, &cache_dir).await {
            tracing::error!(username = %req.username, "TTS failed: {e}");
        }
    }

    tracing::info!("TTS worker shutting down (channel closed)");
}

async fn process_request(
    client: &reqwest::Client,
    config: &TtsConfig,
    req: &TtsRequest,
    cache_dir: &Path,
) -> Result<(), String> {
    let url = format!(
        "https://api.elevenlabs.io/v1/text-to-speech/{}",
        config.voice_id,
    );

    let payload = serde_json::json!({
        "text": req.text,
        "model_id": config.model_id,
        "language_code": config.language_code,
        "voice_settings": {
            "stability": 0.5,
            "similarity_boost": 0.75,
            "speed": 0.7
        }
    });

    let response = client
        .post(&url)
        .header("xi-api-key", &config.elevenlabs_api_key)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("request error: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("API error {status}: {body}"));
    }

    let audio_bytes = response
        .bytes()
        .await
        .map_err(|e| format!("failed to read audio bytes: {e}"))?;

    tracing::info!(bytes = audio_bytes.len(), "TTS audio received");

    // Save to cache directory
    let filename = format!("message_{}.mp3", chrono::Local::now().timestamp());
    let filepath = cache_dir.join(&filename);

    std::fs::write(&filepath, &audio_bytes)
        .map_err(|e| format!("failed to write {}: {e}", filepath.display()))?;

    // Play audio (blocks until playback finishes)
    play_audio(&filepath).await;

    // Clean up
    if let Err(e) = std::fs::remove_file(&filepath) {
        tracing::warn!("failed to remove {}: {e}", filepath.display());
    }

    Ok(())
}

async fn play_audio(path: &Path) {
    // Try mpv first
    if let Ok(mut child) = Command::new("mpv")
        .arg("--no-video")
        .arg("--really-quiet")
        .arg(path)
        .spawn()
    {
        if let Err(e) = child.wait().await {
            tracing::warn!("mpv exited with error: {e}");
        }
        return;
    }

    // Fallback to ffplay
    match Command::new("ffplay")
        .arg("-nodisp")
        .arg("-autoexit")
        .arg("-loglevel")
        .arg("quiet")
        .arg(path)
        .spawn()
    {
        Ok(mut child) => {
            if let Err(e) = child.wait().await {
                tracing::warn!("ffplay exited with error: {e}");
            }
        }
        Err(_) => {
            tracing::warn!("no audio player available (mpv or ffplay)");
        }
    }
}
