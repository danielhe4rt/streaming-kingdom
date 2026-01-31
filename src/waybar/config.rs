use std::path::PathBuf;
use std::{fs, io};

use serde_json::{json, Value};

use super::{data_file_path, stream_events_script_path};

/// Name used to identify our bar object in the waybar config array.
const STREAM_BAR_NAME: &str = "stream-events";

// ---------------------------------------------------------------------------
// Omarchy waybar config paths
// ---------------------------------------------------------------------------

fn omarchy_config_path() -> io::Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no config directory available"))?;
    Ok(config_dir.join("waybar").join("config.jsonc"))
}

// ---------------------------------------------------------------------------
// JSONC → JSON (strip comments before parsing)
// ---------------------------------------------------------------------------

/// Strip single-line (`//`) and multi-line (`/* */`) comments from JSONC,
/// being careful not to touch content inside quoted strings.
fn strip_jsonc_comments(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;

    while let Some(c) = chars.next() {
        if in_string {
            result.push(c);
            if c == '\\' {
                // Push the escaped character too
                if let Some(esc) = chars.next() {
                    result.push(esc);
                }
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }

        match c {
            '"' => {
                in_string = true;
                result.push(c);
            }
            '/' => match chars.peek() {
                Some('/') => {
                    // Line comment — skip until newline
                    chars.next();
                    while let Some(&nc) = chars.peek() {
                        if nc == '\n' {
                            break;
                        }
                        chars.next();
                    }
                }
                Some('*') => {
                    // Block comment — skip until */
                    chars.next();
                    let mut prev = ' ';
                    for nc in chars.by_ref() {
                        if prev == '*' && nc == '/' {
                            break;
                        }
                        prev = nc;
                    }
                }
                _ => result.push(c),
            },
            _ => result.push(c),
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Stream bar object generation
// ---------------------------------------------------------------------------

/// Build the stream bottom-bar JSON object.
pub fn stream_bar_object(output: &str) -> Value {
    let data_path = data_file_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "~/.cache/streams-toolkit/stream_data.json".to_string());

    let script_path = stream_events_script_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "~/.config/streams-toolkit/scripts/stream_events.py".to_string());

    let exec_script = format!(
        "if [ -f '{data}' ] && [ -s '{data}' ]; then python3 '{script}' '{data}'; else echo '{{\"text\":\"\",\"tooltip\":\"No events\",\"class\":\"empty\"}}'; fi",
        data = data_path,
        script = script_path,
    );

    json!({
        "name": STREAM_BAR_NAME,
        "layer": "top",
        "position": "bottom",
        "output": if output.is_empty() { "DP-1" } else { output },
        "height": 40,
        "margin-top": 0,
        "margin-bottom": 0,
        "spacing": 0,
        "modules-left": ["custom/stream-tag", "custom/stream-events"],
        "modules-center": [],
        "modules-right": [],

        "custom/stream-events": {
            "format": "{}",
            "exec": exec_script,
            "return-type": "json",
            "interval": 2,
        },
    })
}

// ---------------------------------------------------------------------------
// Merge / remove stream bar from Omarchy's waybar config
// ---------------------------------------------------------------------------

/// Read Omarchy's waybar config, add the stream bar, and write it back.
/// Backs up the original before modifying.
pub fn add_stream_bar(output: &str) -> io::Result<()> {
    let config_path = omarchy_config_path()?;

    let raw = fs::read_to_string(&config_path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "cannot read Omarchy waybar config at {}: {e}",
                config_path.display()
            ),
        )
    })?;

    // Backup before modifying
    let backup = config_path.with_extension("jsonc.streams-bak");
    fs::write(&backup, &raw)?;
    tracing::debug!("backed up waybar config to {}", backup.display());

    let stripped = strip_jsonc_comments(&raw);

    // Trim the stripped content and handle both array and object forms.
    let trimmed = stripped.trim();

    let mut bars: Vec<Value> = if trimmed.starts_with('[') {
        serde_json::from_str(trimmed).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to parse waybar config as JSON array: {e}"),
            )
        })?
    } else {
        let single: Value = serde_json::from_str(trimmed).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to parse waybar config as JSON object: {e}"),
            )
        })?;
        vec![single]
    };

    // Remove any existing stream bar first (idempotent)
    bars.retain(|bar| bar.get("name").and_then(Value::as_str) != Some(STREAM_BAR_NAME));

    // Append the new stream bar
    bars.push(stream_bar_object(output));

    let merged = serde_json::to_string_pretty(&bars).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to serialize merged config: {e}"),
        )
    })?;

    fs::write(&config_path, &merged)?;
    tracing::info!(
        "merged stream bar into waybar config at {}",
        config_path.display()
    );

    Ok(())
}

/// Read Omarchy's waybar config, remove the stream bar, and write it back.
/// Always preserves the array format (Omarchy uses multiple bars).
pub fn remove_stream_bar() -> io::Result<()> {
    let config_path = omarchy_config_path()?;

    let raw = fs::read_to_string(&config_path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "cannot read Omarchy waybar config at {}: {e}",
                config_path.display()
            ),
        )
    })?;

    let stripped = strip_jsonc_comments(&raw);
    let trimmed = stripped.trim();

    let mut bars: Vec<Value> = if trimmed.starts_with('[') {
        serde_json::from_str(trimmed).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to parse waybar config as JSON array: {e}"),
            )
        })?
    } else {
        let single: Value = serde_json::from_str(trimmed).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to parse waybar config as JSON object: {e}"),
            )
        })?;
        vec![single]
    };

    let before = bars.len();
    bars.retain(|bar| bar.get("name").and_then(Value::as_str) != Some(STREAM_BAR_NAME));
    let removed = before - bars.len();

    if removed == 0 {
        tracing::debug!("stream bar was not present in waybar config, nothing to remove");
        return Ok(());
    }

    let output = serde_json::to_string_pretty(&bars).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to serialize config: {e}"),
        )
    })?;

    fs::write(&config_path, &output)?;
    tracing::info!(
        "removed stream bar from waybar config at {}",
        config_path.display()
    );

    Ok(())
}
