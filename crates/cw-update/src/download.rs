//! Downloading a package and checking it against its `.sha256` file.

use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::release::{Release, agent, user_agent};

/// Largest package accepted. A release is about 490 MB, nearly all of it the
/// NVIDIA libraries beside the worker for the graphics card (`cuda\`);
/// anything far larger is not one.
const MAX_PACKAGE: u64 = 1024 * 1024 * 1024;

/// Download the package of `release` into `dir` and verify it. Returns the
/// path of the verified zip. A package that does not match its checksum is
/// deleted.
pub fn download(release: &Release, dir: &Path) -> Result<PathBuf, String> {
    download_limited(release, dir, MAX_PACKAGE)
}

pub(crate) fn download_limited(
    release: &Release,
    dir: &Path,
    limit: u64,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let expected = {
        let mut response = get(&release.sha256_url)?;
        let text = response
            .body_mut()
            .read_to_string()
            .map_err(|e| format!("could not read {}: {e}", release.sha256_url))?;
        parse_sha256(&text).ok_or_else(|| format!("{} holds no SHA-256", release.sha256_url))?
    };

    let path = dir.join(&release.zip_name);
    let part = dir.join(format!("{}.part", release.zip_name));
    let result = fetch(&release.zip_url, &part, limit).and_then(|actual| {
        if actual == expected {
            Ok(())
        } else {
            Err(format!(
                "{} does not match its checksum (expected {expected}, got {actual})",
                release.zip_name
            ))
        }
    });
    if let Err(e) = result {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    std::fs::rename(&part, &path).map_err(|e| format!("could not keep {}: {e}", path.display()))?;
    Ok(path)
}

fn get(url: &str) -> Result<ureq::http::Response<ureq::Body>, String> {
    // An https address may not be redirected to plain http.
    agent(url.starts_with("https://"))
        .get(url)
        .header("User-Agent", &user_agent())
        .call()
        .map_err(|e| format!("could not download {url}: {e}"))
}

/// Stream `url` into `to`, at most `limit` bytes. Returns the SHA-256 of what
/// was written, in lowercase hex.
fn fetch(url: &str, to: &Path, limit: u64) -> Result<String, String> {
    let mut response = get(url)?;
    let file = File::create(to).map_err(|e| format!("could not create {}: {e}", to.display()))?;
    let mut out = BufWriter::new(file);
    let mut reader = response.body_mut().as_reader();
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 64 * 1024];
    let mut total = 0u64;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|e| format!("the download of {url} broke off: {e}"))?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if total > limit {
            return Err(format!(
                "{url} is larger than {} MB; it is not a CodeWiki package",
                limit / (1024 * 1024)
            ));
        }
        hasher.update(&buffer[..read]);
        out.write_all(&buffer[..read])
            .map_err(|e| format!("could not write {}: {e}", to.display()))?;
    }
    out.flush()
        .map_err(|e| format!("could not write {}: {e}", to.display()))?;
    Ok(hex::encode(hasher.finalize()))
}

/// The hash in a `.sha256` file: `<64 hex>  <file name>`, as written by the
/// release workflow and by `sha256sum`.
pub(crate) fn parse_sha256(text: &str) -> Option<String> {
    let hash = text.split_whitespace().next()?.to_ascii_lowercase();
    (hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())).then_some(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "3a7bd3e2360a3d29eea436fcfb7e44c735d117c42d1c1835420b6b9942dd4f1b";

    #[test]
    fn reads_the_release_workflows_checksum_file() {
        assert_eq!(
            parse_sha256(&format!("{HASH}  CodeWiki-0.2.0-windows-x64.zip\n")).as_deref(),
            Some(HASH)
        );
        // Get-FileHash prints upper case.
        assert_eq!(
            parse_sha256(&HASH.to_ascii_uppercase()).as_deref(),
            Some(HASH)
        );
    }

    #[test]
    fn rejects_anything_that_is_not_a_hash() {
        assert_eq!(parse_sha256(""), None);
        assert_eq!(parse_sha256("Not Found"), None);
        assert_eq!(parse_sha256(&HASH[..63]), None);
        assert_eq!(parse_sha256(&format!("{}g", &HASH[..63])), None);
    }
}
