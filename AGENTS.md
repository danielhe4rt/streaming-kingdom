# AGENTS.md - Development Guidelines

## Project: streams-toolkit

## On Startup - REQUIRED CHECKS

Every time you start working on this project, you MUST perform the following:

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
