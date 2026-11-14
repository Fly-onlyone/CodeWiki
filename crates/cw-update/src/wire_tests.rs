//! Drives the check and the download against a stand-in for GitHub, so no
//! network is needed.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use sha2::{Digest, Sha256};

use crate::download::download_limited;
use crate::{Check, Release, Version, check, download};

enum Reply {
    Status(u16, Vec<u8>),
    /// A 302 to another path, as GitHub sends an asset to its download host.
    Redirect(String),
}

type Routes = Arc<Mutex<HashMap<String, Reply>>>;

/// Answer requests by path until the test ends. Routes can be added after
/// the server is bound, once its address is known.
fn serve() -> (String, Routes) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let routes: Routes = Arc::default();
    let shared = routes.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { return };
            let mut reader = BufReader::new(stream);
            let mut request_line = String::new();
            if reader.read_line(&mut request_line).is_err() {
                continue;
            }
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).is_err() || line.trim_end().is_empty() {
                    break;
                }
            }
            let path = request_line.split_whitespace().nth(1).unwrap_or("");
            let mut stream = reader.into_inner();
            let routes = shared.lock().unwrap();
            let _ = match routes.get(path) {
                Some(Reply::Redirect(to)) => write!(
                    stream,
                    "HTTP/1.1 302 Found\r\nLocation: {to}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                ),
                Some(Reply::Status(status, body)) => write!(
                    stream,
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .and_then(|()| stream.write_all(body)),
                None => write!(
                    stream,
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                ),
            };
            let _ = stream.flush();
        }
    });
    (url, routes)
}

const LATEST: &str = "/repos/Fly-onlyone/CodeWiki/releases/latest";
const ZIP: &str = "CodeWiki-0.2.0-windows-x64.zip";

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// A stand-in that offers 0.2.0: `package`, behind a redirect, and a
/// `.sha256` file naming `sha256`.
fn github(package: &[u8], sha256: &str) -> String {
    let (url, routes) = serve();
    let release = serde_json::json!({
        "tag_name": "v0.2.0",
        "draft": false,
        "prerelease": false,
        "assets": [
            { "name": ZIP, "browser_download_url": format!("{url}/download/v0.2.0/{ZIP}") },
            { "name": format!("{ZIP}.sha256"), "browser_download_url": format!("{url}/download/v0.2.0/{ZIP}.sha256") },
        ],
    });
    let mut routes = routes.lock().unwrap();
    routes.insert(
        LATEST.to_string(),
        Reply::Status(200, release.to_string().into_bytes()),
    );
    routes.insert(
        format!("/download/v0.2.0/{ZIP}"),
        Reply::Redirect(format!("/objects/{ZIP}")),
    );
    routes.insert(
        format!("/objects/{ZIP}"),
        Reply::Status(200, package.to_vec()),
    );
    routes.insert(
        format!("/download/v0.2.0/{ZIP}.sha256"),
        Reply::Status(200, format!("{sha256}  {ZIP}\n").into_bytes()),
    );
    url
}

fn release_at(url: &str) -> Release {
    match check(url, "0.1.0") {
        Check::Newer(release) => release,
        other => panic!("expected a newer release, got {other:?}"),
    }
}

#[test]
fn a_newer_release_is_found_and_downloaded_through_a_redirect() {
    let package = b"zip bytes ".repeat(1000);
    let url = github(&package, &sha256_hex(&package));
    let release = release_at(&url);
    assert_eq!(release.version, Version::new(0, 2, 0));
    assert_eq!(check(&url, "0.2.0"), Check::UpToDate);

    let dir = tempfile::tempdir().unwrap();
    let path = download(&release, dir.path()).unwrap();
    assert_eq!(path, dir.path().join(ZIP));
    assert_eq!(std::fs::read(&path).unwrap(), package);
    assert!(!dir.path().join(format!("{ZIP}.part")).exists());
}

#[test]
fn a_package_that_does_not_match_its_checksum_is_deleted() {
    let package = b"zip bytes".to_vec();
    let url = github(&package, &sha256_hex(b"other bytes"));
    let release = release_at(&url);
    let dir = tempfile::tempdir().unwrap();
    let error = download(&release, dir.path()).unwrap_err();
    assert!(error.contains("does not match its checksum"), "{error}");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn a_package_larger_than_the_limit_is_refused() {
    let package = vec![7u8; 4096];
    let url = github(&package, &sha256_hex(&package));
    let release = release_at(&url);
    let dir = tempfile::tempdir().unwrap();
    let error = download_limited(&release, dir.path(), 1024).unwrap_err();
    assert!(error.contains("larger than"), "{error}");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn no_release_yet_is_up_to_date() {
    let (url, _routes) = serve();
    assert_eq!(check(&url, "0.1.0"), Check::UpToDate);
}

#[test]
fn a_rate_limit_is_not_an_answer() {
    let (url, routes) = serve();
    routes.lock().unwrap().insert(
        LATEST.to_string(),
        Reply::Status(403, br#"{"message":"API rate limit exceeded"}"#.to_vec()),
    );
    assert!(matches!(
        check(&url, "0.1.0"),
        Check::Unavailable(why) if why.contains("try again later")
    ));
}

#[test]
fn no_network_is_not_an_answer() {
    // Bound and released at once: nothing listens there.
    let url = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}", listener.local_addr().unwrap())
    };
    assert!(matches!(check(&url, "0.1.0"), Check::Unavailable(_)));
}
