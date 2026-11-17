//! Check GitHub for a release newer than this build.
//!
//! The fetch runs on the same `ehttp` path as the content sources: a worker
//! thread on desktop and Android, `fetch` in the browser build. The UI thread
//! only starts the request and reads its result, and the button's status label
//! is the whole result: nothing is installed or downloaded here.
//!
//! The repository is public, so no token is involved and nothing about the
//! user is sent. The GitHub REST API allows cross-origin browser requests, so
//! the web build does not need the source proxy.

use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::Duration;

use serde::Deserialize;

/// Project addresses shared by the header actions, About, and the update check.
pub(crate) const REPOSITORY_URL: &str = "https://github.com/nicholasgower/Contra-karaoke";
pub(crate) const DOCUMENTATION_URL: &str = "https://contrakaraoke.nicholasgower.com/";
pub(crate) const ISSUES_URL: &str = "https://github.com/nicholasgower/Contra-karaoke/issues";
pub(crate) const RELEASES_URL: &str = "https://github.com/nicholasgower/Contra-karaoke/releases";
const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/nicholasgower/Contra-karaoke/releases/latest";

/// The version compiled into this build.
pub(crate) const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// A response larger than this is not a release description worth reading.
const MAX_RESPONSE_BYTES: usize = 512 * 1024;

/// The numeric part of a release tag such as `v1.2.3`.
///
/// Pre-release and build suffixes (`-beta.1`, `+build`) are ignored for the
/// comparison: the question is whether the release's core version is newer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Version {
    /// Parse `1.2.3` or `v1.2.3`. Anything else, including a fourth component,
    /// is not a version this app can compare.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let text = text.strip_prefix('v').unwrap_or(text);
        let core = text.split(['-', '+']).next()?;
        let mut parts = core.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some(Self {
            major,
            minor,
            patch,
        })
    }
}

/// The part of a GitHub release response this app reads.
#[derive(Deserialize)]
struct ReleaseJson {
    tag_name: String,
    html_url: Option<String>,
    #[serde(default)]
    draft: bool,
}

/// What a completed check found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    UpToDate { latest: String },
    Available { latest: String, url: String },
    NoRelease,
}

/// The result shown under the Preferences title.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    UpToDate {
        latest: String,
    },
    Available {
        latest: String,
        url: String,
    },
    NoRelease,
    Failed {
        message: String,
    },
}

/// One in-flight update check.
#[derive(Default)]
pub(crate) struct UpdateCheck {
    status: UpdateStatus,
    rx: Option<Receiver<Result<Outcome, String>>>,
}

impl UpdateCheck {
    pub(crate) fn status(&self) -> &UpdateStatus {
        &self.status
    }

    pub(crate) fn is_checking(&self) -> bool {
        self.rx.is_some()
    }

    /// Start a check unless one is already running.
    ///
    /// This only starts the request; the result arrives through [`Self::poll`]
    /// on a later frame, so the click can never block the UI on the network.
    pub(crate) fn start(&mut self) {
        if self.rx.is_some() {
            return;
        }
        let (tx, rx) = channel();
        self.rx = Some(rx);
        self.status = UpdateStatus::Checking;
        ehttp::fetch(latest_release_request(), move |response| {
            let result = match response {
                Ok(response) if response.ok => check_release(CURRENT_VERSION, &response.bytes),
                // The repository exists but has no published release yet.
                Ok(response) if response.status == 404 => Ok(Outcome::NoRelease),
                Ok(response) => Err(format!(
                    "GitHub answered HTTP {} {}.",
                    response.status, response.status_text
                )),
                Err(error) => Err(error.to_string()),
            };
            let _ = tx.send(result);
        });
    }

    /// Take a finished result, if there is one. Returns whether the status
    /// changed, so the caller can repaint the label.
    pub(crate) fn poll(&mut self) -> bool {
        let Some(rx) = &self.rx else {
            return false;
        };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => {
                Err("The update check stopped before it answered.".to_string())
            }
        };
        self.rx = None;
        self.status = match result {
            Ok(Outcome::UpToDate { latest }) => UpdateStatus::UpToDate { latest },
            Ok(Outcome::Available { latest, url }) => UpdateStatus::Available { latest, url },
            Ok(Outcome::NoRelease) => UpdateStatus::NoRelease,
            Err(message) => UpdateStatus::Failed { message },
        };
        true
    }
}

fn latest_release_request() -> ehttp::Request {
    #[cfg(not(target_arch = "wasm32"))]
    let headers = [
        ("Accept", "application/vnd.github+json"),
        (
            "User-Agent",
            concat!("Contra-Karaoke/", env!("CARGO_PKG_VERSION")),
        ),
    ];
    // Browsers set their own User-Agent and forbid scripts from replacing it.
    #[cfg(target_arch = "wasm32")]
    let headers = [("Accept", "application/vnd.github+json")];
    let mut request = ehttp::Request::get(LATEST_RELEASE_API);
    request.headers = ehttp::Headers::new(&headers);
    request.timeout = Some(Duration::from_secs(15));
    request
}

/// Read the latest-release response and compare it with `current`.
///
/// Pure: the fetch callback calls this, and the tests call it directly with a
/// fixture body instead of touching the network.
pub(crate) fn check_release(current: &str, body: &[u8]) -> Result<Outcome, String> {
    if body.len() > MAX_RESPONSE_BYTES {
        return Err("The update response was too large.".to_string());
    }
    let json: ReleaseJson = serde_json::from_slice(body)
        .map_err(|error| format!("Could not read the release information: {error}"))?;
    if json.draft {
        return Err("The latest release is still a draft.".to_string());
    }
    let version = Version::parse(&json.tag_name)
        .ok_or_else(|| format!("Could not read a version number in {:?}.", json.tag_name))?;
    let current = Version::parse(current)
        .ok_or_else(|| format!("This build's version ({current}) could not be read."))?;
    let url = json
        .html_url
        .filter(|url| url.starts_with(RELEASES_URL))
        .unwrap_or_else(|| RELEASES_URL.to_string());
    if version > current {
        Ok(Outcome::Available {
            latest: json.tag_name,
            url,
        })
    } else {
        Ok(Outcome::UpToDate {
            latest: json.tag_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release_body(tag: &str) -> Vec<u8> {
        format!(r#"{{"tag_name":"{tag}","html_url":"{RELEASES_URL}/tag/{tag}","draft":false}}"#)
            .into_bytes()
    }

    #[test]
    fn version_tags_parse_with_or_without_a_v() {
        assert_eq!(
            Version::parse("1.2.3"),
            Some(Version {
                major: 1,
                minor: 2,
                patch: 3
            })
        );
        assert_eq!(Version::parse("v0.10.0"), Version::parse("0.10.0"));
        assert_eq!(Version::parse(" v1.0.0-beta.2 ").map(|v| v.patch), Some(0));
        assert_eq!(Version::parse("1.2.3+build.7").map(|v| v.minor), Some(2));
        assert_eq!(Version::parse("latest"), None);
        assert_eq!(Version::parse("1.2"), None);
        assert_eq!(Version::parse("1.2.3.4"), None);
    }

    #[test]
    fn a_newer_release_is_reported_as_available() {
        let outcome = check_release("0.1.0", &release_body("v0.2.0")).expect("valid release");
        assert_eq!(
            outcome,
            Outcome::Available {
                latest: "v0.2.0".into(),
                url: format!("{RELEASES_URL}/tag/v0.2.0"),
            }
        );
    }

    #[test]
    fn the_current_or_an_older_release_is_up_to_date() {
        assert_eq!(
            check_release("0.2.0", &release_body("v0.2.0")).expect("valid release"),
            Outcome::UpToDate {
                latest: "v0.2.0".into()
            }
        );
        assert_eq!(
            check_release("1.0.0", &release_body("v0.9.9")).expect("valid release"),
            Outcome::UpToDate {
                latest: "v0.9.9".into()
            }
        );
    }

    #[test]
    fn version_components_compare_numerically_not_as_text() {
        let outcome = check_release("0.9.0", &release_body("v0.10.0")).expect("valid release");
        assert!(matches!(outcome, Outcome::Available { .. }));
    }

    #[test]
    fn a_link_outside_the_release_pages_falls_back_to_the_releases_page() {
        let body =
            r#"{"tag_name":"v0.2.0","html_url":"https://example.test/v0.2.0","draft":false}"#;
        assert_eq!(
            check_release("0.1.0", body.as_bytes()).expect("valid release"),
            Outcome::Available {
                latest: "v0.2.0".into(),
                url: RELEASES_URL.to_string(),
            }
        );
    }

    #[test]
    fn malformed_or_unusable_responses_are_errors() {
        assert!(check_release("0.1.0", b"not json").is_err());
        assert!(
            check_release(
                "0.1.0",
                br#"{"tag_name":"nightly","html_url":null,"draft":false}"#
            )
            .is_err()
        );
        assert!(
            check_release(
                "0.1.0",
                br#"{"tag_name":"v0.2.0","html_url":null,"draft":true}"#
            )
            .is_err()
        );
        assert!(check_release("0.1.0", &vec![b' '; MAX_RESPONSE_BYTES + 1]).is_err());
    }

    #[test]
    fn a_new_check_starts_idle_and_reports_that_it_is_checking() {
        let mut check = UpdateCheck::default();
        assert_eq!(check.status(), &UpdateStatus::Idle);
        assert!(!check.is_checking());
        assert!(!check.poll(), "an idle check has no result");
    }
}
