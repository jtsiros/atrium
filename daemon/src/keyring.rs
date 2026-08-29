use std::process::Stdio;
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::process::Command;

const DEADLINE: Duration = Duration::from_secs(20);

const SERVICE: &str = "atrium";

// Resolved absolutely, and PATH pinned for the child: the token is written to
// this process's stdin, so a `secret-tool` planted earlier in an inherited PATH
// would receive it.
const CANDIDATES: &[&str] = &["/usr/bin/secret-tool", "/bin/secret-tool", "/usr/local/bin/secret-tool"];
const SAFE_PATH: &str = "/usr/bin:/bin";

fn helper() -> Option<&'static str> {
    CANDIDATES
        .iter()
        .copied()
        .find(|path| std::path::Path::new(path).is_file())
}

#[derive(Debug)]
pub enum Error {
    Unavailable,
    Timeout,
    NotFound,
    Failed(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => write!(
                f,
                "secret-tool is not installed, so the access token cannot be stored"
            ),
            Self::Timeout => write!(f, "the system keyring did not respond"),
            Self::NotFound => write!(f, "no access token is stored for that address"),
            Self::Failed(m) => write!(f, "the system keyring refused: {m}"),
        }
    }
}

async fn run(args: &[&str], stdin: Option<&str>) -> Result<String, Error> {
    let Some(program) = helper() else {
        return Err(Error::Unavailable);
    };
    let mut command = Command::new(program);
    command
        .env("PATH", SAFE_PATH)
        .args(args)
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => Error::Unavailable,
        _ => Error::Failed(e.to_string()),
    })?;

    if let Some(secret) = stdin {
        if let Some(mut pipe) = child.stdin.take() {
            let _ = pipe.write_all(secret.as_bytes()).await;
            let _ = pipe.shutdown().await;
        }
    }

    let output = match tokio::time::timeout(DEADLINE, child.wait_with_output()).await {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => return Err(Error::Failed(e.to_string())),
        Err(_) => return Err(Error::Timeout),
    };

    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        // secret-tool exits 1 with nothing on stderr when the lookup simply
        // matched no entry. Any other shape is a real failure and must not be
        // reported to the caller as an absent token.
        if message.is_empty() && output.status.code() == Some(1) {
            return Err(Error::NotFound);
        }
        return Err(Error::Failed(if message.is_empty() {
            format!("exit status {}", output.status)
        } else {
            message
        }));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub async fn store(origin: &str, token: &str) -> Result<(), Error> {
    run(
        &[
            "store",
            "--label",
            &format!("Atrium — {origin}"),
            "service",
            SERVICE,
            "origin",
            origin,
        ],
        Some(token),
    )
    .await
    .map(|_| ())
}

pub async fn lookup(origin: &str) -> Result<Option<String>, Error> {
    match run(&["lookup", "service", SERVICE, "origin", origin], None).await {
        Ok(out) => {
            let token = out.trim().to_string();
            Ok((!token.is_empty()).then_some(token))
        }
        Err(Error::NotFound) => Ok(None),
        Err(e) => Err(e),
    }
}

pub async fn clear(origin: &str) -> Result<(), Error> {
    match run(&["clear", "service", SERVICE, "origin", origin], None).await {
        Ok(_) => Ok(()),
        Err(Error::NotFound) => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled() -> bool {
        std::env::var("ATRIUM_KEYRING_TESTS").is_ok()
    }

    #[tokio::test]
    async fn a_missing_origin_reads_as_none_not_an_error() {
        if !enabled() {
            return;
        }
        let origin = "https://atrium-test.invalid:1";
        clear(origin).await.expect("clearing an absent entry is fine");
        assert!(lookup(origin).await.expect("lookup should succeed").is_none());
    }

    #[tokio::test]
    async fn a_token_round_trips_and_is_scoped_to_its_origin() {
        if !enabled() {
            return;
        }
        let a = "https://atrium-test.invalid:2";
        let b = "https://atrium-test.invalid:3";
        store(a, "token-for-a").await.unwrap();
        assert_eq!(lookup(a).await.unwrap().as_deref(), Some("token-for-a"));
        assert!(lookup(b).await.unwrap().is_none());

        clear(a).await.unwrap();
        assert!(lookup(a).await.unwrap().is_none());
    }

    #[test]
    fn the_helper_is_never_taken_from_an_inherited_path() {
        for candidate in CANDIDATES {
            assert!(candidate.starts_with('/'), "{candidate} must be absolute");
        }
        assert!(!SAFE_PATH.split(':').any(|p| p.is_empty() || !p.starts_with('/')));
    }

    #[tokio::test]
    async fn clearing_one_origin_leaves_the_other_alone() {
        if !enabled() {
            return;
        }
        let a = "https://atrium-test.invalid:4";
        let b = "https://atrium-test.invalid:5";
        store(a, "aaa").await.unwrap();
        store(b, "bbb").await.unwrap();
        clear(a).await.unwrap();
        assert!(lookup(a).await.unwrap().is_none());
        assert_eq!(lookup(b).await.unwrap().as_deref(), Some("bbb"));
        clear(b).await.unwrap();
    }
}
