#!/usr/bin/env python3
"""
Stream Events Formatter for Waybar

This script reads stream events from a JSON data file and formats them
for display in the waybar status bar.

Usage:
    python3 stream_events.py <data_file_path>

Or as a shell script integration:
    if [ -f '/path/to/data.json' ] && [ -s '/path/to/data.json' ]; then
        python3 stream_events.py '/path/to/data.json'
    else
        echo '{"text":"","tooltip":"No events","class":"empty"}'
    fi
"""

import json
import sys
import html
import subprocess
import os

# Color mapping for different event types
COLORS = {
    'follow': '#676e95',
    'sub': '#c792ea',
    'newSubscriber': '#c792ea',
    'recurringSubscriber': '#c792ea',
    'giftSubscriber': '#89ddff',
    'directGiftSubscriber': '#89ddff',
    'donation': '#f78c6c',
    'cheer': '#ffcb6b',
    'raid': '#f07178',
}

# Label mapping for different event types
LABELS = {
    'follow': 'follow',
    'sub': 'sub',
    'newSubscriber': 'sub',
    'recurringSubscriber': 'resub',
    'giftSubscriber': 'gift',
    'directGiftSubscriber': 'gift',
    'donation': 'tip',
    'cheer': 'cheer',
    'raid': 'raid',
}


def get_spotify_info():
    """Get current Spotify track information using playerctl."""
    try:
        # Check if Spotify is playing
        result = subprocess.run(
            ['playerctl', '--player=spotify', 'metadata', '--format', '{{artist}} - {{title}}'],
            capture_output=True,
            text=True,
            timeout=1
        )
        if result.returncode == 0 and result.stdout.strip():
            track_info = result.stdout.strip()
            # Truncate if too long
            if len(track_info) > 50:
                track_info = track_info[:47] + '...'
            return track_info
    except (subprocess.SubprocessError, FileNotFoundError):
        pass
    return None


def format_events(data_path: str) -> dict:
    """
    Read events from the data file and format them for waybar display.
    
    Args:
        data_path: Path to the JSON file containing stream events
        
    Returns:
        dict with 'text', 'tooltip', and 'class' keys for waybar
    """
    try:
        with open(data_path) as f:
            events = json.load(f)
    except (json.JSONDecodeError, FileNotFoundError, IOError):
        events = []

    if not events:
        return {'text': '', 'tooltip': 'No events', 'class': 'empty'}

    # Keep only the last 4 events
    events = events[-4:]

    parts = []
    # Add Spotify music info as header
    spotify_info = get_spotify_info()
    if spotify_info:
        header = '<span color="#1DB954" size="large">Spotify:</span> '
        header += f'<span color="#ffffff" size="large">{html.escape(spotify_info)}</span> '
        header += '<span color="#676e95">||</span> '
    else:
        header = '<span color="#676e95" size="large" style="italic">No music playing</span> '
        header += '<span color="#676e95">||</span> '
    parts.append(header)
    
    # Add Recent Events label
    recent_header = '<span color="#c792ea" size="large" weight="bold">Recent Events</span> '
    recent_header += '<span color="#676e95">//</span> '
    parts.append(recent_header)
    
    # Opacity levels for fading effect (100%, 75%, 50%, 25%)
    OPACITIES = ['ff', 'bf', '80', '40']
    
    for i, ev in enumerate(events):
        event_type = ev.get('event_type', '')
        user = html.escape(ev.get('username', '?'))
        color = COLORS.get(event_type, '#676e95')
        label = LABELS.get(event_type, event_type)
        amount = ev.get('amount', '')

        # Build amount string based on event type
        amount_str = ''
        if amount:
            if event_type == 'donation':
                amount_str = f' R${amount}'
            elif event_type == 'cheer':
                amount_str = f' {amount} bits'
            elif 'sub' in event_type.lower() or 'gift' in event_type.lower():
                amount_str = f' x{amount}'
            else:
                amount_str = f' {amount}'

        # Get opacity for this event position (gradually fading)
        alpha = OPACITIES[i]
        
        # Format the event display with fading opacity
        if i == 0:
            # First (most recent) event: large, bold username, full opacity
            part = f'<span size="x-large" weight="bold">{user}</span> '
            part += f'<span size="large" color="{color}{alpha}">{label}{amount_str}</span>'
        else:
            # Subsequent events: smaller with separator, fading opacity
            sep = f'<span color="#24283a{alpha}" size="large"> // </span>'
            part = sep + f'<span color="#ffffff{alpha}">{user} </span>'
            part += f'<span color="{color}{alpha}">{label}{amount_str}</span>'

        parts.append(part)
        
        # Add gap between events (but not after the last one)
        if i < len(events) - 1:
            gap_alpha = OPACITIES[i + 1] if i + 1 < len(OPACITIES) else '40'
            parts.append(f'<span color="#24283a{gap_alpha}">   </span>')

    text = ''.join(parts)
    return {
        'text': text,
        'tooltip': f'{len(events)} recent events',
        'class': 'has-events'
    }


def main():
    if len(sys.argv) < 2:
        print(json.dumps({'text': '', 'tooltip': 'No data file specified', 'class': 'empty'}))
        sys.exit(1)

    data_path = sys.argv[1]
    
    # Check if file exists and is not empty
    import os
    if not os.path.exists(data_path) or os.path.getsize(data_path) == 0:
        print(json.dumps({'text': '', 'tooltip': 'No events', 'class': 'empty'}))
        sys.exit(0)

    result = format_events(data_path)
    print(json.dumps(result))


if __name__ == '__main__':
    main()
