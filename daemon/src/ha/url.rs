use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    pub origin: String,
    pub websocket: String,
    pub rest: String,
    pub plaintext: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlError {
    Empty,
    Malformed,
    UnsupportedScheme(String),
}

impl std::fmt::Display for UrlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "no Home Assistant URL set"),
            Self::Malformed => write!(f, "that does not look like a URL"),
            Self::UnsupportedScheme(s) => write!(f, "unsupported scheme “{s}”"),
        }
    }
}

pub fn parse(input: &str) -> Result<Endpoint, UrlError> {
    // Trailing slashes come off the parsed path below, not here: stripping them
    // first turns a scheme-only "https://" into "https:", which then re-prefixes
    // into a URL that parses but points elsewhere.
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(UrlError::Empty);
    }

    let has_scheme = trimmed
        .split_once("://")
        .is_some_and(|(s, _)| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.'));
    let candidate = if has_scheme {
        trimmed.to_string()
    } else {
        // Guessing TLS upward costs a connection error; guessing downward sends
        // the access token in the clear.
        format!("https://{trimmed}")
    };

    let url = Url::parse(&candidate).map_err(|_| UrlError::Malformed)?;
    let secure = match url.scheme() {
        "https" | "wss" => true,
        "http" | "ws" => false,
        other => return Err(UrlError::UnsupportedScheme(other.to_string())),
    };
    let host = url.host_str().ok_or(UrlError::Malformed)?;
    if host.is_empty() {
        return Err(UrlError::Malformed);
    }

    let authority = match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    };
    let base = url.path().trim_end_matches('/');

    let (http, ws) = if secure { ("https", "wss") } else { ("http", "ws") };
    Ok(Endpoint {
        origin: format!("{http}://{authority}{base}"),
        websocket: format!("{ws}://{authority}{base}/api/websocket"),
        rest: format!("{http}://{authority}{base}/api"),
        plaintext: !secure,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(input: &str) -> Endpoint {
        parse(input).unwrap_or_else(|e| panic!("{input} should parse: {e}"))
    }

    #[test]
    fn https_with_a_port_round_trips() {
        let e = ok("https://ha.example.com:8123");
        assert_eq!(e.origin, "https://ha.example.com:8123");
        assert_eq!(e.websocket, "wss://ha.example.com:8123/api/websocket");
        assert_eq!(e.rest, "https://ha.example.com:8123/api");
        assert!(!e.plaintext);
    }

    #[test]
    fn a_missing_scheme_assumes_tls() {
        let e = ok("homeassistant.local:8123");
        assert_eq!(e.websocket, "wss://homeassistant.local:8123/api/websocket");
        assert!(!e.plaintext);
    }

    #[test]
    fn plaintext_is_reported_not_silently_accepted() {
        assert!(ok("http://192.168.1.10:8123").plaintext);
        assert!(ok("ws://192.168.1.10:8123").plaintext);
    }

    #[test]
    fn websocket_schemes_map_onto_their_http_twin() {
        assert_eq!(ok("wss://ha.example.com").rest, "https://ha.example.com/api");
        assert_eq!(ok("ws://ha.example.com").rest, "http://ha.example.com/api");
    }

    #[test]
    fn a_proxied_subpath_is_preserved() {
        let e = ok("https://example.com/homeassistant");
        assert_eq!(e.websocket, "wss://example.com/homeassistant/api/websocket");
        assert_eq!(e.rest, "https://example.com/homeassistant/api");
    }

    #[test]
    fn trailing_slashes_do_not_double_up() {
        assert_eq!(ok("https://ha.example.com/").websocket, "wss://ha.example.com/api/websocket");
    }

    #[test]
    fn unknown_schemes_are_refused_rather_than_downgraded() {
        assert_eq!(
            parse("ftp://ha.example.com"),
            Err(UrlError::UnsupportedScheme("ftp".into()))
        );
        assert!(matches!(parse("gopher://ha"), Err(UrlError::UnsupportedScheme(_))));
    }

    #[test]
    fn empty_and_malformed_input_is_rejected() {
        assert_eq!(parse(""), Err(UrlError::Empty));
        assert_eq!(parse("   "), Err(UrlError::Empty));
        assert!(parse("https://").is_err());
    }
}
