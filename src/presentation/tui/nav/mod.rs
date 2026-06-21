//! M4 — the two-level nav model (prototype variant **D**), sliced by concern.
//!
//! Topbar picks a **Section**; the sidebar shows that section's contextual
//! **sub-nav**, and the content panel renders the selected sub-item. All
//! transitions here are **pure** functions of the current [`NavState`] — no I/O,
//! no rendering — so they're unit-testable in isolation, and the sub-nav lists
//! are derived from the data-driven Service registry (no fixed counts, no magic
//! numbers).
//!
//! Slices:
//! - [`section`] — the topbar [`Section`]s and cycling between them
//! - [`overlay`] — the [`OverlayId`] registry + data-driven [`overlay_url`]
//! - [`sub_item`] — the [`SubItem`]s and registry-derived lists/labels
//! - [`nav_state`] — [`NavState`] and the pure transitions over it

mod nav_state;
mod overlay;
mod section;
mod sub_item;

pub use nav_state::NavState;
pub use overlay::{OverlayDef, OverlayId, overlay_url};
pub use section::Section;
pub use sub_item::{SubItem, default_sub, sub_items, sub_label};
