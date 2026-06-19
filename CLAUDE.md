# AGENTS.md - Development Guidelines

## Project: streams-toolkit

## On Startup - REQUIRED CHECKS

Every time you start working on this project, you MUST perform the following:

### 0. Read the Context Map first

Before exploring code, read `CONTEXT-MAP.md` at the repo root — it maps each DDD layer to its `CONTEXT.md` glossary. That is enough to orient. **Only after that, and only if the task needs deeper detail on a specific module or external service**, read the matching `docs/arch/*.md` (see the Layer docs table below). Don't read the arch docs upfront or wholesale — they're reference material, pulled in on demand.

### 1. Python Files Sync Check

Compare the Python files in the project with those in `~/.config/streams-toolkit/`. If they differ, the project version is the source of truth - copy project files to `.config`.

**Quick check command:**
```bash
diff /home/danielhe4rt/dev/lives/streams-toolkit/scripts/stream_events.py /home/danielhe4rt/.config/streams-toolkit/scripts/stream_events.py >/dev/null 2>&1 && echo "SYNCED" || echo "NEEDS SYNC"
```

**Sync command:**
```bash
cp /home/danielhe4rt/dev/lives/streams-toolkit/scripts/stream_events.py /home/danielhe4rt/.config/streams-toolkit/scripts/stream_events.py
```

**Files to sync:**
- `scripts/stream_events.py`

### 2. Checksum Verification Script

Run this to verify and sync all tracked files:

```bash
#!/bin/bash
# sync-check.sh

echo "Checking Python files for sync..."

PROJECT_DIR="/home/danielhe4rt/dev/lives/streams-toolkit"
CONFIG_DIR="$HOME/.config/streams-toolkit"

# Check stream_events.py
if ! diff "$PROJECT_DIR/scripts/stream_events.py" "$CONFIG_DIR/scripts/stream_events.py" >/dev/null 2>&1; then
    echo "⚠️  stream_events.py differs - syncing..."
    cp "$PROJECT_DIR/scripts/stream_events.py" "$CONFIG_DIR/scripts/stream_events.py"
    echo "✅ Synced stream_events.py"
else
    echo "✅ stream_events.py is synced"
fi

echo "Done!"
```

## Architecture Notes

### File Locations
- **Project files**: `/home/danielhe4rt/dev/lives/streams-toolkit/`
- **Active config**: `~/.config/streams-toolkit/`

### Sync Rule
**Project files are the SOURCE OF TRUTH.** Always copy FROM project TO .config, never the reverse.

## Current Features

- Stream Events formatter for Waybar
- Spotify integration showing current track
- Recent events display with gradual opacity fade (4 events max)
- Event types: follow, sub, newSubscriber, recurringSubscriber, giftSubscriber, directGiftSubscriber, donation, cheer, raid

## Dependencies

- playerctl (for Spotify integration)
- Python 3
- Waybar

## Agent skills

### Issue tracker

Issues live in this repo's GitHub Issues (uses the `gh` CLI). Note: no git remote is configured yet. See `docs/agents/issue-tracker.md`.

### Triage labels

Canonical triage roles use their default strings, plus `area:*` (DDD layer) and `infra:*` (external integration) labels derived from `src/`. See `docs/agents/triage-labels.md`.

### Domain docs

Multi-context: `CONTEXT-MAP.md` at the root points to one `CONTEXT.md` per layer (domain, application, infrastructure, presentation). See `docs/agents/domain.md`.

## Layer docs

Robust prose docs live in `docs/arch/`, one numbered file per domain module and per infrastructure service. They complement the glossaries in each `src/<layer>/CONTEXT.md` (which hold the ubiquitous-language terms, not prose).

**Maintenance rule — these docs are code-coupled. When you change a module, update its doc in the SAME change:**

| When you edit…                       | Update…                            |
| ------------------------------------ | ---------------------------------- |
| `src/domain/events.rs`               | `docs/arch/1-domain-events.md`     |
| `src/domain/chat.rs`                 | `docs/arch/2-domain-chat.md`       |
| `src/domain/commands.rs`             | `docs/arch/3-domain-commands.md`   |
| `src/domain/stats.rs`                | `docs/arch/4-domain-stats.md`      |
| `src/domain/media_player.rs`         | `docs/arch/12-domain-media-player.md` |
| `src/domain/app_event.rs`            | `docs/arch/5-domain-app-event.md`  |
| `src/infrastructure/twitch/`         | `docs/arch/6-infra-twitch.md`      |
| `src/infrastructure/elevenlabs/`     | `docs/arch/7-infra-elevenlabs.md`  |
| `src/infrastructure/hyprland/`       | `docs/arch/8-infra-hyprland.md`    |
| `src/infrastructure/livepix/`        | `docs/arch/9-infra-livepix.md`     |
| `src/infrastructure/obs/`            | `docs/arch/10-infra-obs.md`        |
| `src/infrastructure/waybar/`         | `docs/arch/11-infra-waybar.md`     |
| `src/infrastructure/media_player/`   | `docs/arch/13-infra-media-player.md` |
| `src/infrastructure/discord/`        | `docs/arch/14-infra-discord.md`    |
| `src/domain/voice.rs`                | `docs/arch/15-domain-voice.md`     |

For **infrastructure** docs especially: whenever the external service usage changes (new endpoint, scope, env var, protocol, crate), reflect it in the matching doc. Each doc carries a "Keep in sync" header pointing back here. When you add a new domain module or infra service, add a numbered doc and a row above.
