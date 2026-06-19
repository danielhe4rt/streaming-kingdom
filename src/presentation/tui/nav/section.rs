//! The primary topbar sections (variant D) and pure cycling between them.

/// Primary topbar sections (variant D).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Dashboard,
    Services,
    Overlays,
    Activity,
}

impl Section {
    /// The sections in topbar order.
    pub const ALL: [Section; 4] = [
        Section::Dashboard,
        Section::Services,
        Section::Overlays,
        Section::Activity,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Section::Dashboard => "Dashboard",
            Section::Services => "Services",
            Section::Overlays => "Overlays",
            Section::Activity => "Activity",
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_cycles_both_ways() {
        assert_eq!(Section::Dashboard.next(), Section::Services);
        assert_eq!(Section::Activity.next(), Section::Dashboard);
        assert_eq!(Section::Dashboard.prev(), Section::Activity);
    }
}
