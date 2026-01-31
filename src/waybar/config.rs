use serde_json::{json, Value};

use super::data_file_path;

/// Generate the waybar config JSON string.
///
/// `output` is the Wayland output name (e.g. "DP-1") from config.toml.
pub fn generate(output: &str) -> String {
    let data_path = data_file_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "~/.cache/streams-toolkit/stream_data.json".to_string());

    // Script that reads the JSON data file and produces Pango markup for waybar.
    // We use `return-type: json` so we can set both text and tooltip.
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

    let config: Value = json!({
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
    });

    serde_json::to_string_pretty(&config).expect("waybar config serialization cannot fail")
}
