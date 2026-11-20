//! Asking GitHub for the latest release.

use std::time::Duration;

use semver::Version;
use serde::Deserialize;

use crate::REPOSITORY;

/// A release newer than the running program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    /// File name of the Windows package, `CodeWiki-<version>-windows-x64.zip`.
    pub zip_name: String,
    pub zip_url: String,
    /// The package's `.sha256` file, written by the release workflow.
    pub sha256_url: String,
}

/// The answer to a check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Check {
    Newer(Release),
    /// The latest release is this version or an older one, or there is no
    /// release yet.
    UpToDate,
    /// No answer to rely on: no network, a rate limit, a release without the
    /// Windows package. The text says why.
    Unavailable(String),
}

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<ApiAsset>,
}

#[derive(Deserialize)]
struct ApiAsset {
    name: String,
    browser_download_url: String,
}

/// `https_only` refuses any request that is not https, a redirect included.
/// It is on for every https address, so that an answer from GitHub cannot
/// send a download on to plain http; a stand-in on http has it off.
pub(crate) fn agent(https_only: bool) -> ureq::Agent {
    ureq::Agent::config_builder()
        .https_only(https_only)
        // No limit on the whole call: a download may take minutes on a slow
        // line. A connection that stops sending is given up instead.
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_per_read(Some(Duration::from_secs(30)))
        .build()
        .into()
}

pub(crate) fn user_agent() -> String {
    format!("CodeWiki/{}", env!("CARGO_PKG_VERSION"))
}

/// Ask `api_base` (GitHub's API, or a stand-in) for the latest release of
/// [`REPOSITORY`] and compare it with `current`.
pub fn check(api_base: &str, current: &str) -> Check {
    let current = match Version::parse(current) {
        Ok(version) => version,
        Err(e) => {
            return Check::Unavailable(format!("this version ({current}) is unreadable: {e}"));
        }
    };
    let url = format!("{api_base}/repos/{REPOSITORY}/releases/latest");
    let response = agent(url.starts_with("https://"))
        .get(&url)
        .header("User-Agent", &user_agent())
        .header("Accept", "application/vnd.github+json")
        .call();
    let mut response = match response {
        Ok(response) => response,
        // GitHub answers 404 while the repository has no release.
        Err(ureq::Error::StatusCode(404)) => return Check::UpToDate,
        Err(ureq::Error::StatusCode(403 | 429)) => {
            return Check::Unavailable(
                "GitHub limits how often it may be asked; try again later".into(),
            );
        }
        Err(e) => return Check::Unavailable(format!("could not reach GitHub: {e}")),
    };
    match response.body_mut().read_to_string() {
        Ok(text) => from_repository(api_base, parse(&text, &current)),
        Err(e) => Check::Unavailable(format!("could not read GitHub's answer: {e}")),
    }
}

/// A release that GitHub described must have its files among this
/// repository's release downloads at GitHub. The package is run before it is
/// installed (`codewiki.exe --version`), and its checksum comes from the same
/// answer, so the address is what says whose package it is. A stand-in
/// (another `api_base`) may serve its files from anywhere.
fn from_repository(api_base: &str, check: Check) -> Check {
    let Check::Newer(release) = &check else {
        return check;
    };
    if api_base != crate::GITHUB_API {
        return check;
    }
    let downloads = format!("https://github.com/{REPOSITORY}/releases/download/").to_lowercase();
    let ours = |url: &str| url.to_lowercase().starts_with(&downloads);
    if ours(&release.zip_url) && ours(&release.sha256_url) {
        check
    } else {
        Check::Unavailable(format!(
            "release {} names files that are not this repository's downloads at GitHub",
            release.version
        ))
    }
}

/// Read GitHub's answer to `GET /repos/{owner}/{repo}/releases/latest`.
pub(crate) fn parse(text: &str, current: &Version) -> Check {
    let release: ApiRelease = match serde_json::from_str(text) {
        Ok(release) => release,
        Err(e) => return Check::Unavailable(format!("GitHub's answer is unreadable: {e}")),
    };
    // `releases/latest` never returns these; a stand-in might.
    if release.draft || release.prerelease {
        return Check::UpToDate;
    }
    let tag = release.tag_name.trim_start_matches('v');
    let version = match Version::parse(tag) {
        Ok(version) => version,
        Err(e) => {
            return Check::Unavailable(format!(
                "the latest release's tag {} is not a version: {e}",
                release.tag_name
            ));
        }
    };
    if !version.pre.is_empty() || version <= *current {
        return Check::UpToDate;
    }
    let zip_name = format!("CodeWiki-{version}-windows-x64.zip");
    let sha256_name = format!("{zip_name}.sha256");
    let url_of = |name: &str| {
        release
            .assets
            .iter()
            .find(|asset| asset.name == name)
            .map(|asset| asset.browser_download_url.clone())
    };
    match (url_of(&zip_name), url_of(&sha256_name)) {
        (Some(zip_url), Some(sha256_url)) => Check::Newer(Release {
            version,
            zip_name,
            zip_url,
            sha256_url,
        }),
        _ => Check::Unavailable(format!(
            "release {version} has no {zip_name} with its .sha256 file"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_github_release_must_name_files_at_this_repository() {
        let release = |zip_url: &str| {
            Check::Newer(Release {
                version: Version::new(0, 2, 0),
                zip_name: "CodeWiki-0.2.0-windows-x64.zip".into(),
                zip_url: zip_url.into(),
                sha256_url: "https://github.com/Fly-onlyone/CodeWiki/releases/download/v0.2.0/CodeWiki-0.2.0-windows-x64.zip.sha256".into(),
            })
        };
        let ours = "https://github.com/Fly-onlyone/CodeWiki/releases/download/v0.2.0/CodeWiki-0.2.0-windows-x64.zip";
        let elsewhere = "https://example.com/CodeWiki-0.2.0-windows-x64.zip";
        let plain = "http://github.com/Fly-onlyone/CodeWiki/releases/download/v0.2.0/CodeWiki-0.2.0-windows-x64.zip";
        let other_repository = "https://github.com/someone/CodeWiki/releases/download/v0.2.0/CodeWiki-0.2.0-windows-x64.zip";

        assert_eq!(
            from_repository(crate::GITHUB_API, release(ours)),
            release(ours)
        );
        for url in [elsewhere, plain, other_repository] {
            assert!(
                matches!(
                    from_repository(crate::GITHUB_API, release(url)),
                    Check::Unavailable(_)
                ),
                "{url}"
            );
        }
        // A stand-in, which tests use, serves from its own address.
        assert_eq!(
            from_repository("http://127.0.0.1:8765", release(elsewhere)),
            release(elsewhere)
        );
        assert_eq!(
            from_repository(crate::GITHUB_API, Check::UpToDate),
            Check::UpToDate
        );
    }

    /// The shape of GitHub's `releases/latest` answer, cut down to the fields
    /// a check reads plus a few it ignores.
    fn answer(tag: &str, prerelease: bool, assets: &[&str]) -> String {
        let assets: Vec<_> = assets
            .iter()
            .map(|name| {
                serde_json::json!({
                    "url": "https://api.github.com/repos/Fly-onlyone/CodeWiki/releases/assets/1",
                    "id": 1,
                    "name": name,
                    "content_type": "application/zip",
                    "state": "uploaded",
                    "size": 26_214_400,
                    "browser_download_url": format!("https://github.com/Fly-onlyone/CodeWiki/releases/download/{tag}/{name}"),
                })
            })
            .collect();
        serde_json::json!({
            "url": "https://api.github.com/repos/Fly-onlyone/CodeWiki/releases/1",
            "html_url": format!("https://github.com/Fly-onlyone/CodeWiki/releases/tag/{tag}"),
            "id": 1,
            "tag_name": tag,
            "name": format!("CodeWiki {}", tag.trim_start_matches('v')),
            "draft": false,
            "prerelease": prerelease,
            "published_at": "2026-11-20T10:00:00Z",
            "assets": assets,
            "body": "notes",
        })
        .to_string()
    }

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn a_newer_release_names_its_package_and_checksum() {
        let text = answer(
            "v0.2.0",
            false,
            &[
                "CodeWiki-0.2.0-windows-x64.zip",
                "CodeWiki-0.2.0-windows-x64.zip.sha256",
            ],
        );
        let Check::Newer(release) = parse(&text, &v("0.1.0")) else {
            panic!("expected a newer release");
        };
        assert_eq!(release.version, v("0.2.0"));
        assert_eq!(release.zip_name, "CodeWiki-0.2.0-windows-x64.zip");
        assert!(
            release
                .zip_url
                .ends_with("/v0.2.0/CodeWiki-0.2.0-windows-x64.zip")
        );
        assert!(release.sha256_url.ends_with(".zip.sha256"));
    }

    #[test]
    fn the_same_or_an_older_release_is_up_to_date() {
        let text = answer("v0.2.0", false, &[]);
        assert_eq!(parse(&text, &v("0.2.0")), Check::UpToDate);
        assert_eq!(parse(&text, &v("0.3.0")), Check::UpToDate);
    }

    #[test]
    fn pre_releases_are_never_offered() {
        let assets = [
            "CodeWiki-0.3.0-rc.1-windows-x64.zip",
            "CodeWiki-0.3.0-rc.1-windows-x64.zip.sha256",
        ];
        assert_eq!(
            parse(&answer("v0.3.0-rc.1", true, &assets), &v("0.2.0")),
            Check::UpToDate
        );
        // Marked as a full release by mistake: still not offered.
        assert_eq!(
            parse(&answer("v0.3.0-rc.1", false, &assets), &v("0.2.0")),
            Check::UpToDate
        );
    }

    #[test]
    fn a_release_without_the_package_or_its_checksum_cannot_be_installed() {
        let only_zip = answer("v0.2.0", false, &["CodeWiki-0.2.0-windows-x64.zip"]);
        assert!(matches!(
            parse(&only_zip, &v("0.1.0")),
            Check::Unavailable(why) if why.contains("CodeWiki-0.2.0-windows-x64.zip")
        ));
    }

    #[test]
    fn unreadable_answers_are_unavailable() {
        assert!(matches!(
            parse("<html>", &v("0.1.0")),
            Check::Unavailable(_)
        ));
        assert!(matches!(
            parse(&answer("nightly", false, &[]), &v("0.1.0")),
            Check::Unavailable(_)
        ));
    }
}
