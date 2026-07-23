use serde::{Deserialize, Serialize};

/// Lowest interval we will ever schedule. Guardrail against hammering an endpoint.
pub const MIN_INTERVAL_SECS: u64 = 10;
/// What a new site gets when the user does not say otherwise.
pub const DEFAULT_INTERVAL_SECS: u64 = 60;

/// The only method we ever persist. HEAD is the default and needs no override;
/// this is written only once a server has told us HEAD is unwelcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Method {
    Get,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Site {
    pub id: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub interval_secs: u64,
    #[serde(default)]
    pub method_override: Option<Method>,
}

/// Emitted state. There is deliberately no `Pending` — that is a UI-only state
/// meaning "no event received yet this session".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckState {
    Up,
    Down,
}

/// Payload of the `site-status` event. Field names stay snake_case to match
/// `Site` and `sites.json`; the frontend reads them as-is.
#[derive(Debug, Clone, Serialize)]
pub struct StatusEvent {
    pub id: String,
    pub state: CheckState,
    /// Epoch milliseconds, taken when the check completed.
    pub checked_at: u64,
    pub reason: Option<String>,
}

/// Validate user input and add a scheme if one is missing.
///
/// Returns the user's own text (trimmed, scheme-prefixed) rather than the
/// re-serialized `Url`, so `example.com` yields `https://example.com` and not
/// `https://example.com/`.
pub fn normalize_url(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Enter a URL".to_string());
    }

    let candidate = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };

    let parsed = url::Url::parse(&candidate).map_err(|_| "Not a valid URL".to_string())?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Only http and https URLs are supported".to_string());
    }
    if parsed.host_str().is_none_or(|h| h.is_empty()) {
        return Err("URL is missing a host".to_string());
    }

    Ok(candidate)
}

/// Raise anything below the floor up to it. Never lowers a value.
pub fn clamp_interval(secs: u64) -> u64 {
    secs.max(MIN_INTERVAL_SECS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_https_scheme_when_missing() {
        assert_eq!(normalize_url("example.com").unwrap(), "https://example.com");
    }

    #[test]
    fn preserves_an_explicit_scheme() {
        assert_eq!(normalize_url("http://example.com").unwrap(), "http://example.com");
        assert_eq!(
            normalize_url("https://api.foo.dev/health").unwrap(),
            "https://api.foo.dev/health"
        );
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(normalize_url("  example.com  ").unwrap(), "https://example.com");
    }

    #[test]
    fn rejects_empty_input() {
        assert!(normalize_url("   ").is_err());
    }

    #[test]
    fn rejects_unparseable_input() {
        assert!(normalize_url("http://").is_err());
        assert!(normalize_url("not a url at all").is_err());
    }

    #[test]
    fn rejects_non_http_schemes() {
        assert!(normalize_url("ftp://example.com").is_err());
        assert!(normalize_url("file:///etc/hosts").is_err());
    }

    #[test]
    fn clamps_intervals_below_the_floor() {
        assert_eq!(clamp_interval(0), 10);
        assert_eq!(clamp_interval(9), 10);
    }

    #[test]
    fn leaves_intervals_at_or_above_the_floor_alone() {
        assert_eq!(clamp_interval(10), 10);
        assert_eq!(clamp_interval(60), 60);
        assert_eq!(clamp_interval(3600), 3600);
    }

    #[test]
    fn method_override_serializes_as_uppercase_get() {
        let json = serde_json::to_string(&Method::Get).unwrap();
        assert_eq!(json, "\"GET\"");
    }

    #[test]
    fn site_omits_absent_optional_fields() {
        let site = Site {
            id: "abc".into(),
            url: "https://example.com".into(),
            label: None,
            interval_secs: 60,
            method_override: None,
        };
        let json = serde_json::to_string(&site).unwrap();
        assert_eq!(
            json,
            r#"{"id":"abc","url":"https://example.com","interval_secs":60,"method_override":null}"#
        );
    }

    #[test]
    fn check_state_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&CheckState::Up).unwrap(), "\"up\"");
        assert_eq!(serde_json::to_string(&CheckState::Down).unwrap(), "\"down\"");
    }
}
