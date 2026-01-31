use std::path::PathBuf;
use std::{fs, io};

use regex::Regex;
use serde_json::{json, Value};

use super::data_file_path;

/// Name used to identify our bar object in the waybar config array.
const STREAM_BAR_NAME: &str = "stream-events";

// ---------------------------------------------------------------------------
// Omarchy waybar config paths
// ---------------------------------------------------------------------------

fn omarchy_config_path() -> io::Result<PathBuf> {
    let config_dir = dirs::config_dir().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "no config directory available")
    })?;
    Ok(config_dir.join("waybar").join("config.jsonc"))
}

// ---------------------------------------------------------------------------
// JSONC → JSON (strip comments before parsing)
// ---------------------------------------------------------------------------

/// Strip single-line (`//`) and multi-line (`/* */`) comments from JSONC,
/// being careful not to touch strings.
fn strip_jsonc_comments(input: &str) -> String {
    // Replace block comments first, then line comments.
    let block = Regex::new(r"/\*[\s\S]*?\*/").expect("valid regex");
    let no_blocks = block.replace_all(input, "");

    // Line comments: only outside of strings. A rough but effective approach
    // for waybar configs which don't embed `//` in string values.
    let line = Regex::new(r"//[^\n]*").expect("valid regex");
    line.replace_all(&no_blocks, "").to_string()
}

// ---------------------------------------------------------------------------
// Stream bar object generation
// ---------------------------------------------------------------------------

/// Build the stream bottom-bar JSON object.
pub fn stream_bar_object(output: &str) -> Value {
    let data_path = data_file_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "~/.cache/streams-toolkit/stream_data.json".to_string());

    let exec_script = format!(
        concat!(
            "if [ -f '{data}' ] && [ -s '{data}' ]; then ",
            "python3 -c \"\n",
            "import json, sys, html\n",
            "COLORS = {{\n",
            "  'follow': '#676e95',\n",
            "  'sub': '#c792ea',\n",
            "  'newSubscriber': '#c792ea',\n",
            "  'recurringSubscriber': '#c792ea',\n",
            "  'giftSubscriber': '#89ddff',\n",
            "  'directGiftSubscriber': '#89ddff',\n",
            "  'donation': '#f78c6c',\n",
            "  'cheer': '#ffcb6b',\n",
            "  'raid': '#f07178',\n",
            "}}\n",
            "LABELS = {{\n",
            "  'follow': 'follow',\n",
            "  'sub': 'sub',\n",
            "  'newSubscriber': 'sub',\n",
            "  'recurringSubscriber': 'resub',\n",
            "  'giftSubscriber': 'gift',\n",
            "  'directGiftSubscriber': 'gift',\n",
            "  'donation': 'tip',\n",
            "  'cheer': 'cheer',\n",
            "  'raid': 'raid',\n",
            "}}\n",
            "try:\n",
            "  with open('{data}') as f:\n",
            "    events = json.load(f)\n",
            "except: events = []\n",
            "if not events:\n",
            "  print(json.dumps({{'text': '', 'tooltip': 'No events', 'class': 'empty'}}))\n",
            "  sys.exit(0)\n",
            "parts = []\n",
            "for i, ev in enumerate(events):\n",
            "  t = ev.get('event_type', '')\n",
            "  user = html.escape(ev.get('username', '?'))\n",
            "  color = COLORS.get(t, '#676e95')\n",
            "  label = LABELS.get(t, t)\n",
            "  amt = ev.get('amount', '')\n",
            "  amt_str = ''\n",
            "  if amt:\n",
            "    if t == 'donation': amt_str = f' R${{amt}}'\n",
            "    elif t == 'cheer': amt_str = f' {{amt}} bits'\n",
            "    elif 'sub' in t.lower() or 'gift' in t.lower(): amt_str = f' x{{amt}}'\n",
            "    else: amt_str = f' {{amt}}'\n",
            "  if i == 0:\n",
            "    part = f'<span size=\\\"x-large\\\" weight=\\\"bold\\\">{{user}}</span> '\n",
            "    part += f'<span size=\\\"large\\\" color=\\\"{{color}}\\\">{{label}}{{amt_str}}</span>'\n",
            "  else:\n",
            "    sep = '<span color=\\\"#24283a\\\" size=\\\"large\\\"> // </span>'\n",
            "    part = sep + f'<span color=\\\"#ffffff80\\\">{{user}} </span>'\n",
            "    part += f'<span color=\\\"{{color}}80\\\">{{label}}{{amt_str}}</span>'\n",
            "  parts.append(part)\n",
            "text = ''.join(parts)\n",
            "print(json.dumps({{'text': text, 'tooltip': f'{{len(events)}} recent events', 'class': 'has-events'}}))\n",
            "\"; ",
            "else echo '{{\"text\":\"\",\"tooltip\":\"No events\",\"class\":\"empty\"}}'; fi",
        ),
        data = data_path,
    );

    json!({
        "name": STREAM_BAR_NAME,
        "layer": "top",
        "position": "bottom",
        "output": if output.is_empty() { "DP-1" } else { output },
        "height": 65,
        "margin-top": 0,
        "margin-bottom": 0,
        "spacing": 0,
        "modules-left": ["custom/stream-tag", "custom/stream-events"],
        "modules-center": [],
        "modules-right": [],

        "custom/stream-tag": {
            "format": "{}",
            "exec": "echo '<span size=\"small\">⊕ Recent Events</span>'",
            "interval": "once",
            "return-type": "",
        },

        "custom/stream-events": {
            "format": "{}",
            "exec": exec_script,
            "return-type": "json",
            "interval": 2,
            "escape": false,
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
/// If only one bar remains, unwraps it from the array.
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

    // If only one bar remains, unwrap from array to single object
    let output: String = if bars.len() == 1 {
        serde_json::to_string_pretty(&bars.into_iter().next().unwrap()).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to serialize config: {e}"),
            )
        })?
    } else {
        serde_json::to_string_pretty(&bars).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to serialize config: {e}"),
            )
        })?
    };

    fs::write(&config_path, &output)?;
    tracing::info!(
        "removed stream bar from waybar config at {}",
        config_path.display()
    );

    Ok(())
}
