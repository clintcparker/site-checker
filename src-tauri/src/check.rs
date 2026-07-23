use crate::model::{CheckState, Method};

/// A stock Safari string. The spec's "be a polite client" constraint means
/// looking like an ordinary browser rather than announcing an unknown tool that
/// a WAF might rate-limit or block.
pub const USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 \
     (KHTML, like Gecko) Version/17.0 Safari/605.1.15";

const TIMEOUT_SECS: u64 = 10;
const MAX_REDIRECTS: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckOutcome {
    pub state: CheckState,
    /// Short, tooltip-sized explanation. `None` when the site is Up.
    pub reason: Option<String>,
    /// True only when *this* check discovered that HEAD is unsupported. The
    /// caller persists `method_override = GET` when it sees this.
    pub used_get_fallback: bool,
}

/// One client is shared by every site. reqwest keeps no response cache, so the
/// spec's "local HTTP cache disabled" requirement is satisfied by construction —
/// do not add a caching layer.
pub fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
        .build()
        .expect("the HTTP client has no fallible configuration")
}

/// "Is my app working", not "is the box alive": 200-399 is Up.
///
/// A *final* 3xx only appears when the redirect limit is hit; per the spec that
/// still counts as Up.
pub fn classify_status(status: u16) -> CheckState {
    if (200..=399).contains(&status) {
        CheckState::Up
    } else {
        CheckState::Down
    }
}

/// Run one check. Sends HEAD unless the site is already known to need GET.
/// On a 405 or 501 from HEAD, retries once with GET against the same URL.
pub async fn check_url(
    client: &reqwest::Client,
    url: &str,
    method_override: Option<Method>,
) -> CheckOutcome {
    if method_override == Some(Method::Get) {
        return match client.get(url).send().await {
            Ok(response) => outcome_from_status(response.status().as_u16(), false),
            Err(e) => transport_failure(&e, false),
        };
    }

    let head_status = match client.head(url).send().await {
        Ok(response) => response.status().as_u16(),
        Err(e) => return transport_failure(&e, false),
    };

    if head_status != 405 && head_status != 501 {
        return outcome_from_status(head_status, false);
    }

    // This server is HEAD-hostile. Retry with GET and tell the caller to
    // remember it so future checks go straight to GET.
    match client.get(url).send().await {
        Ok(response) => outcome_from_status(response.status().as_u16(), true),
        Err(e) => transport_failure(&e, true),
    }
}

fn outcome_from_status(status: u16, used_get_fallback: bool) -> CheckOutcome {
    let state = classify_status(status);
    CheckOutcome {
        reason: match state {
            CheckState::Up => None,
            CheckState::Down => Some(format!("HTTP {status}")),
        },
        state,
        used_get_fallback,
    }
}

/// reqwest's `Display` chains every source, which is far too long for a hover
/// tooltip. Collapse to a category instead.
fn transport_failure(error: &reqwest::Error, used_get_fallback: bool) -> CheckOutcome {
    let reason = if error.is_timeout() {
        "Timed out after 10s".to_string()
    } else if error.is_connect() {
        "Could not connect".to_string()
    } else if error.is_redirect() {
        "Too many redirects".to_string()
    } else if error.is_body() || error.is_decode() {
        "Bad response".to_string()
    } else {
        "Request failed".to_string()
    };

    CheckOutcome {
        state: CheckState::Down,
        reason: Some(reason),
        used_get_fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::{Method::GET, Method::HEAD, MockServer};

    #[test]
    fn classifies_2xx_and_3xx_as_up() {
        assert_eq!(classify_status(200), CheckState::Up);
        assert_eq!(classify_status(204), CheckState::Up);
        assert_eq!(classify_status(301), CheckState::Up);
        assert_eq!(classify_status(399), CheckState::Up);
    }

    #[test]
    fn classifies_everything_else_as_down() {
        assert_eq!(classify_status(400), CheckState::Down);
        assert_eq!(classify_status(404), CheckState::Down);
        assert_eq!(classify_status(429), CheckState::Down);
        assert_eq!(classify_status(500), CheckState::Down);
        assert_eq!(classify_status(503), CheckState::Down);
        assert_eq!(classify_status(199), CheckState::Down);
    }

    #[tokio::test]
    async fn a_200_on_head_is_up() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(HEAD).path("/");
                then.status(200);
            })
            .await;

        let outcome = check_url(&build_client(), &server.url("/"), None).await;
        assert_eq!(outcome.state, CheckState::Up);
        assert_eq!(outcome.reason, None);
        assert!(!outcome.used_get_fallback);
    }

    #[tokio::test]
    async fn a_404_is_down_with_the_status_as_the_reason() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(HEAD).path("/");
                then.status(404);
            })
            .await;

        let outcome = check_url(&build_client(), &server.url("/"), None).await;
        assert_eq!(outcome.state, CheckState::Down);
        assert_eq!(outcome.reason.as_deref(), Some("HTTP 404"));
    }

    #[tokio::test]
    async fn a_followed_redirect_resolves_to_the_final_status() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(HEAD).path("/old");
                then.status(301).header("location", "/new");
            })
            .await;
        server
            .mock_async(|when, then| {
                when.method(HEAD).path("/new");
                then.status(200);
            })
            .await;

        let outcome = check_url(&build_client(), &server.url("/old"), None).await;
        assert_eq!(outcome.state, CheckState::Up);
    }

    #[tokio::test]
    async fn head_405_falls_back_to_get_and_reports_the_fallback() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(HEAD).path("/");
                then.status(405);
            })
            .await;
        let get_mock = server
            .mock_async(|when, then| {
                when.method(GET).path("/");
                then.status(200);
            })
            .await;

        let outcome = check_url(&build_client(), &server.url("/"), None).await;
        assert_eq!(outcome.state, CheckState::Up);
        assert!(
            outcome.used_get_fallback,
            "the caller needs this to persist method_override = GET"
        );
        get_mock.assert_async().await;
    }

    #[tokio::test]
    async fn head_501_also_falls_back_to_get() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(HEAD).path("/");
                then.status(501);
            })
            .await;
        server
            .mock_async(|when, then| {
                when.method(GET).path("/");
                then.status(200);
            })
            .await;

        let outcome = check_url(&build_client(), &server.url("/"), None).await;
        assert_eq!(outcome.state, CheckState::Up);
        assert!(outcome.used_get_fallback);
    }

    #[tokio::test]
    async fn a_known_get_only_site_skips_head_entirely() {
        let server = MockServer::start_async().await;
        let head_mock = server
            .mock_async(|when, then| {
                when.method(HEAD).path("/");
                then.status(405);
            })
            .await;
        server
            .mock_async(|when, then| {
                when.method(GET).path("/");
                then.status(200);
            })
            .await;

        let outcome = check_url(&build_client(), &server.url("/"), Some(Method::Get)).await;
        assert_eq!(outcome.state, CheckState::Up);
        assert!(
            !outcome.used_get_fallback,
            "already persisted; nothing new to write"
        );
        head_mock.assert_calls_async(0).await;
    }

    #[tokio::test]
    async fn a_get_fallback_that_also_fails_is_down() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(HEAD).path("/");
                then.status(405);
            })
            .await;
        server
            .mock_async(|when, then| {
                when.method(GET).path("/");
                then.status(500);
            })
            .await;

        let outcome = check_url(&build_client(), &server.url("/"), None).await;
        assert_eq!(outcome.state, CheckState::Down);
        assert_eq!(outcome.reason.as_deref(), Some("HTTP 500"));
    }

    #[tokio::test]
    async fn a_connection_failure_is_down_with_a_short_reason() {
        // Port 1 on loopback: nothing listens there, so this refuses immediately.
        let outcome = check_url(&build_client(), "http://127.0.0.1:1/", None).await;
        assert_eq!(outcome.state, CheckState::Down);
        let reason = outcome.reason.expect("a transport failure must explain itself");
        assert!(!reason.is_empty());
        assert!(
            reason.len() <= 80,
            "reason is shown in a hover tooltip, keep it short: {reason}"
        );
    }

    #[tokio::test]
    async fn an_unresolvable_host_is_down() {
        let outcome = check_url(
            &build_client(),
            "https://this-host-does-not-exist.invalid/",
            None,
        )
        .await;
        assert_eq!(outcome.state, CheckState::Down);
        assert!(outcome.reason.is_some());
    }
}
