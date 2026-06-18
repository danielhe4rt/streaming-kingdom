# Infrastructure — elevenlabs

> **Keep in sync:** this file documents `src/infrastructure/elevenlabs`. Whenever that code changes, update this doc in the same change. (See CLAUDE.md → Layer docs.)

## The service

ElevenLabs is a cloud-based text-to-speech (TTS) API that converts text into natural-sounding audio. We use it to speak donation messages aloud during a stream, triggered by Livepix webhook events. Each TTS request sends text to their `/v1/text-to-speech/<voice_id>` endpoint and receives MP3 audio bytes, which we cache locally and play immediately via `mpv` or `ffplay`.

## What we use from it

**API Endpoint:** `https://api.elevenlabs.io/v1/text-to-speech/{voice_id}` (HTTP POST)

**Request format:**
- Header: `xi-api-key: <ELEVENLABS_API_KEY>`
- Body (JSON):
  ```json
  {
    "text": "string",
    "model_id": "string (default: eleven_v3)",
    "language_code": "string (default: pt for Portuguese)",
    "voice_settings": {
      "stability": 0.5,
      "similarity_boost": 0.75,
      "speed": 0.7
    }
  }
  ```

**Response:** HTTP 200 with binary MP3 audio bytes (or error response with status + body on failure).

**Authentication:** API key via `ELEVENLABS_API_KEY` environment variable (loaded into `TtsConfig::elevenlabs_api_key` at startup). If the key is empty, the TTS worker is never spawned (disabling TTS gracefully).

**Crates used:**
- `reqwest` (0.12) — HTTP client for POST requests
- `tokio` (1.47) — async runtime for the worker task and command receiver loop
- `chrono` (0.4) — timestamp generation for local audio filenames
- `dirs` (6) — locating the user's cache directory (returns `~/.cache/streams-toolkit/tts/`)

## How it's wired

### Public interface

**Function: `spawn(config: TtsConfig) -> Option<mpsc::Sender<TtsRequest>>`**

Spawns a long-running async worker task that processes TTS requests from a bounded MPSC channel (capacity 8). Returns `Some(tx)` if the API key is configured, or `None` if it's empty. The caller uses the returned sender to queue requests.

**Struct: `TtsRequest`**

```rust
pub struct TtsRequest {
    pub text: String,        // The message to synthesize
    pub username: String,    // Source of the message (for logging)
}
```

### Data flow

1. **Initialization** (`main.rs:171`): `infrastructure::elevenlabs::spawn(cfg.livepix.tts.clone())` creates the TTS worker and returns a sender (or `None`).

2. **Request queueing** (`src/infrastructure/livepix/webhook.rs:231–240`): When a Livepix donation webhook arrives, `handle_webhook()` extracts the donation message and attempts to send a `TtsRequest` via `tts_tx.try_send(TtsRequest { text, username })`. If the queue is full (rare), a warning is logged and the request is dropped.

3. **Worker loop** (`mod.rs:25–55`): The `tts_worker()` receives requests from the channel, processes each one (call `process_request()`), and logs errors. The worker exits when the channel closes (application shutdown).

4. **Per-request processing** (`mod.rs:57–117`):
   - Build the ElevenLabs API URL with the configured `voice_id`.
   - POST the JSON payload (with text, model, language, and voice settings) to the API.
   - On success (HTTP 200), receive the MP3 audio bytes.
   - Save the bytes to a local file in `~/.cache/streams-toolkit/tts/message_<timestamp>.mp3`.
   - Play the audio via `mpv` (preferred) or `ffplay` (fallback), blocking until playback finishes.
   - Delete the temporary file.
   - Log any errors (network, API, file I/O, or playback).

### Configuration

Configuration is nested under `config.toml` at `[livepix.tts]`:

```toml
[livepix.tts]
elevenlabs_api_key = "..."      # Required; if empty, TTS is disabled
voice_id = "Qrdut83w0Cr152Yb4Xn3"  # Default voice (can be overridden)
model_id = "eleven_v3"           # Model version (default)
language_code = "pt"             # Language code (default: Portuguese)
```

**Environment override:** The `ELEVENLABS_API_KEY` env var (from `.env` or shell) overrides the config file value if the config value is empty. This is applied in `src/application/config.rs:123–127`.

The `voice_id`, `model_id`, and `language_code` default to Portuguese-friendly settings (voice `Qrdut83w0Cr152Yb4Xn3`, model `eleven_v3`, language `pt`).

### Caching and playback

- **Cache directory**: `~/.cache/streams-toolkit/tts/` (created on worker startup if it doesn't exist).
- **Filenames**: `message_<unix_timestamp>.mp3` (ensures uniqueness and readability).
- **Playback**: Synchronous via subprocess (blocks the worker until audio finishes or player exits).
- **Cleanup**: Temporary MP3 file is deleted after playback (or a warning is logged if deletion fails).

## Gotchas

1. **API key required**: If `ELEVENLABS_API_KEY` is not set or is empty in config, the worker is never spawned. This is graceful — no errors, just silent disable. The livepix webhook will still try to `try_send()` to a `None` sender and will silently drop requests.

2. **Blocking playback**: The TTS worker is single-threaded and blocks on audio playback. If a user has multiple donations in quick succession, the second donation's TTS must wait for the first to finish. The request queue (capacity 8) can absorb bursts, but if the queue fills, new requests are dropped (logged as "TTS queue full").

3. **No error recovery**: If the ElevenLabs API returns an error (e.g., 401 Unauthorized, 429 Rate Limited, 500 Server Error), the request fails and is logged; there is no retry logic or exponential backoff. The worker continues listening for the next request.

4. **Dependency on audio players**: The worker falls back from `mpv` to `ffplay` if `mpv` is not installed. If neither is available, a warning is logged and the audio is never played (though it was already cached). Ensure at least one is installed on the system.

5. **Local cache not auto-cleaned**: The cache directory persists indefinitely. Old MP3 files are only deleted after playback; if the application crashes or is killed during playback, temporary files may accumulate in `~/.cache/streams-toolkit/tts/`.

6. **Voice ID and model hardcoded defaults**: While `voice_id`, `model_id`, and `language_code` can be configured, the defaults are baked into the config schema. Changing the voice will require updating `config.toml` (there is no env var to override these fields, only `ELEVENLABS_API_KEY`).

7. **Portuguese defaults**: The hardcoded `language_code: "pt"` and the specific voice ID assume Portuguese-language streams. Non-Portuguese users should update their config to match their language.

8. **No concurrency safeguards**: The voice settings (stability, similarity_boost, speed) are hardcoded in `process_request()` (lines 72–76). They are not exposed to configuration. If you need to tweak voice characteristics, you must edit the code.
