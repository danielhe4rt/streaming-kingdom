/// Generate the GTK CSS for the waybar bottom bar.
///
/// Replicates the StreamElements "Recent Events" overlay:
/// - Dark navy background (#090b15)
/// - Monorama SemiBold / CaskaydiaMono Nerd Font
/// - Purple "Recent Events" tag floating above the bar
/// - Right-side fade mask on the event list
pub fn generate() -> String {
    r#"/* ================================================================
   streams-toolkit waybar bottom bar
   Replicates StreamElements "Recent Events" overlay
   ================================================================ */

/* --- Global bar --- */
* {
    font-family: "Monorama SemiBold", "CaskaydiaMono Nerd Font", "Noto Sans", monospace;
    font-size: 14px;
}

window#waybar {
    background-color: #090b15;
    color: #ffffff;
}

/* Remove default padding/margins from all modules */
#custom-stream-tag,
#custom-stream-events {
    padding: 0;
    margin: 0;
}

/* --- "Recent Events" tag --- */
#custom-stream-tag {
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
}

/* --- Event list area --- */
#custom-stream-events {
    padding-left: 24px;
    padding-right: 48px;
    font-size: 14px;
    text-transform: uppercase;
}

#custom-stream-events.empty {
    /* Hide when no events */
    padding: 0;
    margin: 0;
}
"#
    .to_string()
}
