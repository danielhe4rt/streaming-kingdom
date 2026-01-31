#!/bin/bash
# sync-check.sh - Run this on project startup to verify Python files are in sync

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
