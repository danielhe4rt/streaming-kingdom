//! Deciding whether a window title matches a configured sensitive pattern.

pub(super) fn is_sensitive(title: &str, patterns: &[String]) -> bool {
    let lower = title.to_lowercase();
    patterns.iter().any(|p| lower.contains(&p.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_matching() {
        let patterns: Vec<String> = vec![
            ".env".into(),
            "credentials".into(),
            ".secret".into(),
            ".pem".into(),
            "id_rsa".into(),
        ];

        assert!(is_sensitive(".env - nvim", &patterns));
        assert!(is_sensitive("credentials.json - code", &patterns));
        assert!(is_sensitive("server.pem - cat", &patterns));
        assert!(is_sensitive("~/.ssh/id_rsa - vim", &patterns));
        assert!(is_sensitive("MY_APP.ENV.LOCAL - nano", &patterns));

        assert!(!is_sensitive("main.rs - code", &patterns));
        assert!(!is_sensitive("README.md - nvim", &patterns));
        assert!(!is_sensitive("", &patterns));
    }
}
