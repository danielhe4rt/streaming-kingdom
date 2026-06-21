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
