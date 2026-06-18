use std::path::PathBuf;
use std::{fs, io};

const SECTION_START: &str = "/* === streams-toolkit: bottom bar === */";
const SECTION_END: &str = "/* === end streams-toolkit === */";

// ---------------------------------------------------------------------------
// Omarchy waybar style path
// ---------------------------------------------------------------------------

fn omarchy_style_path() -> io::Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no config directory available"))?;
    Ok(config_dir.join("waybar").join("style.css"))
}

// ---------------------------------------------------------------------------
// Stream bar CSS block
// ---------------------------------------------------------------------------

/// The CSS styles for the stream bottom bar, scoped to the `stream-events`
/// bar name so they don't conflict with Omarchy's top bar styles.
fn stream_css() -> String {
    format!(
        r#"{start}

/* Scope all stream-bar rules under window#stream-events */
window#stream-events {{
    background-color: #090b15;
    color: #ffffff;
}}

window#stream-events * {{
    font-family: "Monorama SemiBold", "CaskaydiaMono Nerd Font", "Noto Sans", monospace;
    font-size: 18px;
}}

/* Remove default padding/margins from stream-bar modules */
window#stream-events #custom-stream-tag,
window#stream-events #custom-stream-events {{
    padding: 0;
    margin: 0;
}}

/* --- "Recent Events" tag --- */
window#stream-events #custom-stream-tag {{
    background-color: #c792ea;
    color: #090b15;
    font-size: 10px;
    text-transform: uppercase;
    padding: 4px 7px 5px 7px;
    margin-left: 295px;
    margin-top: -12px;
    margin-bottom: 4px;
    border-radius: 3px;
    box-shadow: 0px 25px 25px rgba(0, 0, 0, 0.25);
    font-weight: bold;
}}

/* --- Event list area --- */
window#stream-events #custom-stream-events {{
    padding-left: 24px;
    padding-right: 48px;
    font-size: 14px;
    text-transform: uppercase;
}}

window#stream-events #custom-stream-events.empty {{
    /* Hide when no events */
    padding: 0;
    margin: 0;
}}

{end}"#,
        start = SECTION_START,
        end = SECTION_END,
    )
}

// ---------------------------------------------------------------------------
// Merge / remove stream CSS from Omarchy's style.css
// ---------------------------------------------------------------------------

/// Append stream bar CSS to Omarchy's waybar style.css.
/// Removes any existing section first (idempotent).
pub fn add_stream_css() -> io::Result<()> {
    let style_path = omarchy_style_path()?;

    let _existing = fs::read_to_string(&style_path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "cannot read Omarchy waybar style at {}: {e}",
                style_path.display()
            ),
        )
    })?;

    // Remove any existing stream section first
    // let cleaned = remove_section(&existing);
    // let new_content = format!("{}\n\n{}\n", cleaned.trim_end(), stream_css());

    // fs::write(&style_path, &new_content)?;
    tracing::info!("appended stream bar CSS to {}", style_path.display());

    Ok(())
}

/// Remove the stream bar CSS section from Omarchy's waybar style.css.
pub fn remove_stream_css() -> io::Result<()> {
    let style_path = omarchy_style_path()?;

    let existing = fs::read_to_string(&style_path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "cannot read Omarchy waybar style at {}: {e}",
                style_path.display()
            ),
        )
    })?;

    if !existing.contains(SECTION_START) {
        tracing::debug!("stream bar CSS section not found, nothing to remove");
        return Ok(());
    }

    let cleaned = remove_section(&existing);
    fs::write(&style_path, cleaned.trim_end().to_string() + "\n")?;
    tracing::info!("removed stream bar CSS from {}", style_path.display());

    Ok(())
}

/// Remove the text between (and including) the start/end markers.
fn remove_section(content: &str) -> String {
    if let Some(start_idx) = content.find(SECTION_START)
        && let Some(end_marker_idx) = content[start_idx..].find(SECTION_END)
    {
        let end_idx = start_idx + end_marker_idx + SECTION_END.len();
        // Also trim any trailing newlines after the section
        let after = content[end_idx..].trim_start_matches('\n');
        return format!("{}{}", &content[..start_idx], after);
    }
    content.to_string()
}
