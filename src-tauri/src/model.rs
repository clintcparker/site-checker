use serde::{Deserialize, Serialize};

/// Lowest interval we will ever schedule. Guardrail against hammering an endpoint.
pub const MIN_INTERVAL_SECS: u64 = 10;

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

/// Byte index of the `://` ending a leading scheme, or `None` if there isn't
/// one. A scheme counts only at the very start and only when every character
/// before the `://` is ASCII alphanumeric or one of `+`, `-`, `.`. A bare
/// `contains("://")` also matches a `://` inside a query string, which would
/// stop a scheme-less URL from getting one.
///
/// Returns the index rather than a bool so `normalize_url` can lowercase
/// exactly the scheme and nothing else, without scanning the string twice. The
/// index is safe to slice at: `find` returns a character boundary, and the
/// character guard proves every byte before it is ASCII.
fn leading_scheme_end(s: &str) -> Option<usize> {
    match s.find("://") {
        Some(0) | None => None,
        Some(i) => s[..i]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
            .then_some(i),
    }
}

/// Validate user input, add a scheme if one is missing, and lowercase the
/// scheme if one is present.
///
/// Returns the user's own text (trimmed, scheme-prefixed) rather than the
/// re-serialized `Url`, so `example.com` yields `https://example.com` and not
/// `https://example.com/`. That is why the lowercasing happens here on a slice
/// of the input instead of by handing back `parsed`'s rendering — `url::Url`
/// lowercases the scheme for free, but it would also put the trailing slash
/// back. Only the scheme is touched: hosts are case-insensitive too, but
/// rewriting them is more of the user's text than was asked for, and paths and
/// query values are case-*sensitive*.
///
/// There is no migration. `load` does not call this, so a site already stored
/// as `HTTPS://…` keeps that value until the user next edits it — and on that
/// edit it counts as a URL change, so `main.ts`'s `upsertSite` drops the row to
/// Pending and `update_site` clears `method_override`, costing one request to
/// re-learn HEAD support. Surprising exactly once per affected site.
pub fn normalize_url(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Enter a URL".to_string());
    }

    let candidate = match leading_scheme_end(trimmed) {
        // `to_ascii_lowercase`, not `to_lowercase`: a scheme is ASCII by the
        // rule `leading_scheme_end` enforces, and Unicode folding here would
        // only mislead.
        Some(i) => format!("{}{}", trimmed[..i].to_ascii_lowercase(), &trimmed[i..]),
        None => format!("https://{trimmed}"),
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

/// Decide whether an address already in the site list may be handed to the
/// operating system to open, returning it **byte-identical** when it may.
///
/// The contrast with `normalize_url` directly above is why the two live
/// adjacent. `normalize_url` *repairs* what a user just typed: it trims, it
/// prepends a missing `https://`, it lowercases the scheme. This one repairs
/// nothing. It is handed a value that is already stored — possibly hand-edited
/// into `sites.json` — and either hands it back untouched or refuses it.
/// `example.com` is `Ok("https://example.com")` there and an `Err` here, and
/// that difference is the whole point: prepending a scheme to a stored value
/// would turn an address the user never approved into one this app opens.
///
/// It must therefore never call `normalize_url`.
///
/// Returning the input rather than `parsed`'s rendering is the same trap
/// `normalize_url` documents: `url::Url` would put a trailing slash back, so
/// `https://example.com` would come back as `https://example.com/`. The parse
/// is only ever consulted for its verdict.
pub fn openable_url(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("This site has no address, so there is nothing to open.".to_string());
    }

    let parsed = url::Url::parse(trimmed).map_err(|_| {
        format!(
            "\"{trimmed}\" is not a complete web address, so it cannot be opened. \
             Edit the site to give it an http:// or https:// address."
        )
    })?;

    // `url::Url` lowercases the scheme for this check, which is what lets a
    // stored `HTTPS://…` through without the returned value being touched.
    let scheme = parsed.scheme();
    if !matches!(scheme, "http" | "https") {
        return Err(format!(
            "Only http and https addresses can be opened, and this one uses \"{scheme}\". \
             Nothing was opened."
        ));
    }

    // Unreachable for the special schemes above, which `Url::parse` already
    // refuses without a host. Kept so the guarantee is stated where it is
    // relied on rather than inferred from another crate's behaviour.
    if parsed.host_str().is_none_or(|h| h.is_empty()) {
        return Err(format!("\"{trimmed}\" has no host, so there is nothing to open."));
    }

    Ok(input.to_string())
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
    fn adds_a_scheme_when_the_query_contains_a_url() {
        assert_eq!(
            normalize_url("example.com?next=http://x.dev").unwrap(),
            "https://example.com?next=http://x.dev"
        );
    }

    #[test]
    fn lowercases_an_uppercase_scheme() {
        assert_eq!(normalize_url("HTTPS://example.com").unwrap(), "https://example.com");
        assert_eq!(
            normalize_url("HtTp://example.com/health").unwrap(),
            "http://example.com/health"
        );
    }

    #[test]
    fn lowercases_only_the_scheme() {
        // The guard against a lazy `to_lowercase()` on the whole string, which
        // would quietly corrupt case-sensitive paths and query values.
        assert_eq!(
            normalize_url("HTTP://Example.COM/Path?Q=1").unwrap(),
            "http://Example.COM/Path?Q=1"
        );
        assert_eq!(normalize_url("https://EXAMPLE.com").unwrap(), "https://EXAMPLE.com");
    }

    #[test]
    fn an_uppercase_scheme_in_a_query_is_left_alone() {
        // The first `://` is inside the query, so this takes the prepend branch
        // and the embedded scheme is not a leading scheme to lowercase.
        assert_eq!(
            normalize_url("example.com?next=HTTP://x.dev").unwrap(),
            "https://example.com?next=HTTP://x.dev"
        );
    }

    #[test]
    fn rejects_a_non_http_scheme_regardless_of_case() {
        assert!(normalize_url("FTP://example.com").is_err());
    }

    #[test]
    fn openable_url_returns_an_http_or_https_address_byte_identical() {
        // Byte-identical is the requirement, not merely "equivalent": handing
        // back `parsed`'s rendering would turn this into
        // `https://example.com/` and open an address the user never stored.
        assert_eq!(openable_url("https://example.com").unwrap(), "https://example.com");
        assert_eq!(
            openable_url("http://example.com/health?q=A").unwrap(),
            "http://example.com/health?q=A"
        );
    }

    #[test]
    fn openable_url_accepts_an_uppercase_scheme_without_touching_it() {
        // `normalize_url` would lowercase this. Here the stored value is
        // handed back exactly as stored; `open` copes with the case.
        assert_eq!(openable_url("HTTPS://example.com").unwrap(), "HTTPS://example.com");
    }

    #[test]
    fn openable_url_refuses_every_other_scheme() {
        assert!(openable_url("ftp://example.com").is_err());
        assert!(openable_url("file:///etc/hosts").is_err());
        assert!(openable_url("javascript:alert(1)").is_err());
    }

    #[test]
    fn openable_url_refuses_a_scheme_less_address_rather_than_repairing_it() {
        // The test that fails the moment `openable_url` delegates to
        // `normalize_url`, which would answer `Ok("https://example.com")`.
        assert!(openable_url("example.com").is_err());
    }

    #[test]
    fn openable_url_refuses_an_empty_or_hostless_address() {
        assert!(openable_url("").is_err());
        assert!(openable_url("   ").is_err());
        assert!(openable_url("https://").is_err());
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
