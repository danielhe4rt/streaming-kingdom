const FILTER_NAME: &str = "Composite Blur";
const FILTER_KIND: &str = "obs_composite_blur";

/// Try to connect to OBS. Returns `None` (with a logged warning) when OBS
/// isn't running or the connection is refused – this is expected during normal
/// desktop use.
pub async fn try_connect_obs(host: &str, port: u16, password: &str) -> Option<obws::Client> {
    let pw: Option<&str> = if password.is_empty() {
        None
    } else {
        Some(password)
    };

    match obws::Client::connect(host, port, pw).await {
        Ok(client) => Some(client),
        Err(e) => {
            tracing::warn!("OBS not available: {e}");
            None
        }
    }
}

/// Ensure a blur filter exists and is **enabled** on `capture_source`.
pub async fn enable_blur(client: &obws::Client, capture_source: &str) {
    let source = obws::requests::sources::SourceId::Name(capture_source);

    // Check whether the filter already exists.
    match client.filters().get(source, FILTER_NAME).await {
        Ok(existing) => {
            if !existing.enabled {
                let req: obws::requests::filters::SetEnabled<'_> =
                    obws::requests::filters::SetEnabled {
                        source,
                        filter: FILTER_NAME,
                        enabled: true,
                    };
                if let Err(e) = client.filters().set_enabled(req).await {
                    tracing::error!("failed to enable blur filter: {e}");
                }
            }
        }
        Err(_) => {
            // Filter doesn't exist yet – create it enabled.
            let settings = serde_json::json!({
                "speed_x": 0.0,
                "speed_y": 0.0,
                "cx": 240.0,
                "cy": 240.0,
            });
            let req = obws::requests::filters::Create {
                source,
                filter: FILTER_NAME,
                kind: FILTER_KIND,
                settings: Some(settings),
            };
            if let Err(e) = client.filters().create(req).await {
                tracing::error!("failed to create blur filter: {e}");
            }
        }
    }
}

/// Disable (but keep) the blur filter so it can be quickly re-enabled.
pub async fn disable_blur(client: &obws::Client, capture_source: &str) {
    let source = obws::requests::sources::SourceId::Name(capture_source);
    let req = obws::requests::filters::SetEnabled {
        source,
        filter: FILTER_NAME,
        enabled: false,
    };
    if let Err(e) = client.filters().set_enabled(req).await {
        tracing::debug!("blur filter disable skipped: {e}");
    }
}
