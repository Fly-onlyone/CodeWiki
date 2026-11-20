//! The models on disk.
//!
//! An installed model is the folder `<data dir>\models\<id>` with the files
//! the catalog names and a `manifest.json`. A download is made in
//! `<data dir>\models\.partial\<id>` and moved into place whole, after every
//! file matched its SHA-256, so a folder under `models` is never half a
//! model. A download that broke off, or was cancelled, goes on from the
//! bytes it has.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cw_core::{Error, Result};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::catalog::{self, Entry};

/// Where the models are fetched from when nothing else is named.
pub const DEFAULT_BASE: &str = "https://huggingface.co";
/// Environment variable that names another host, for tests and mirrors.
pub const BASE_ENV: &str = "CODEWIKI_MODELS_URL";

const MANIFEST: &str = "manifest.json";
const PARTIAL: &str = ".partial";
const CHUNK: usize = 64 * 1024;

/// How an install ended when it did not fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Installed,
    /// Stopped on request. What was downloaded stays for the next try.
    Cancelled,
}

pub struct Store {
    root: PathBuf,
    base: String,
}

fn io(path: &Path) -> impl Fn(std::io::Error) -> Error + '_ {
    move |source| Error::io(path, source)
}

/// `https_only` refuses any request that is not https, a redirect included,
/// so that an answer cannot send a download on to plain http. A stand-in on
/// http has it off.
fn agent(https_only: bool) -> ureq::Agent {
    ureq::Agent::config_builder()
        .https_only(https_only)
        // No limit on the whole call: a model is hundreds of megabytes. A
        // connection that stops sending is given up instead.
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_per_read(Some(Duration::from_secs(30)))
        .build()
        .into()
}

fn sha256_of(path: &Path, hasher: &mut Sha256, cancel: &AtomicBool) -> Result<bool> {
    let mut file = File::open(path).map_err(io(path))?;
    let mut buffer = vec![0; CHUNK];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Ok(false);
        }
        let read = file.read(&mut buffer).map_err(io(path))?;
        if read == 0 {
            return Ok(true);
        }
        hasher.update(&buffer[..read]);
    }
}

impl Store {
    /// The store under `data_dir`. `base` replaces the host the files come
    /// from; `None` is Hugging Face.
    pub fn new(data_dir: &Path, base: Option<&str>) -> Self {
        Self {
            root: data_dir.join("models"),
            base: base
                .unwrap_or(DEFAULT_BASE)
                .trim_end_matches('/')
                .to_string(),
        }
    }

    /// The folder that holds the installed models.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The folder of an installed model.
    pub fn dir(&self, entry: &Entry) -> PathBuf {
        self.root.join(&entry.id)
    }

    fn partial(&self, entry: &Entry) -> PathBuf {
        self.root.join(PARTIAL).join(&entry.id)
    }

    /// Whether the model is on disk at the catalog's revision, every file
    /// with its size. The hashes were checked when it was installed.
    pub fn is_installed(&self, entry: &Entry) -> bool {
        let dir = self.dir(entry);
        let Ok(text) = fs::read_to_string(dir.join(MANIFEST)) else {
            return false;
        };
        let Ok(manifest) = serde_json::from_str::<Value>(&text) else {
            return false;
        };
        manifest["revision"].as_str() == Some(entry.revision.as_str())
            && entry.files.iter().all(|file| {
                fs::metadata(dir.join(&file.path)).is_ok_and(|found| found.len() == file.size)
            })
    }

    /// The catalog's models that are installed.
    pub fn installed(&self) -> Vec<&'static Entry> {
        catalog::all()
            .iter()
            .filter(|entry| self.is_installed(entry))
            .collect()
    }

    /// Bytes of the model that are already downloaded.
    fn have(&self, entry: &Entry) -> u64 {
        let partial = self.partial(entry);
        entry
            .files
            .iter()
            .map(|file| {
                let done = partial.join(&file.path);
                let part = partial.join(format!("{}.part", file.path));
                fs::metadata(&done)
                    .or_else(|_| fs::metadata(&part))
                    .map_or(0, |found| found.len().min(file.size))
            })
            .sum()
    }

    /// Download, verify and install the model. `progress` hears the bytes
    /// there are and the bytes there will be, many times a second; `cancel`
    /// stops the download at the next chunk.
    pub fn install(
        &self,
        entry: &Entry,
        progress: &mut dyn FnMut(u64, u64),
        cancel: &AtomicBool,
    ) -> Result<Outcome> {
        let partial = self.partial(entry);
        fs::create_dir_all(&partial).map_err(io(&partial))?;
        // One installer at a time: the app's backend and the command line
        // may both be asked. The lock goes when this process ends, however
        // it ends, so a crash leaves nothing to clear.
        let lock_path = self.root.join(PARTIAL).join(format!("{}.lock", entry.id));
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock_path)
            .map_err(io(&lock_path))?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                return Err(Error::invalid(format!(
                    "another program is installing {}",
                    entry.name
                )));
            }
            Err(TryLockError::Error(source)) => return Err(Error::io(&lock_path, source)),
        }

        let total = entry.size();
        let mut done = self.have(entry);
        progress(done, total);
        for file in &entry.files {
            let target = partial.join(&file.path);
            if fs::metadata(&target).is_ok_and(|found| found.len() == file.size) {
                // Verified by an earlier try: a file gets this name only
                // after its hash matched.
                continue;
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(io(parent))?;
            }
            let url = format!(
                "{}/{}/resolve/{}/{}",
                self.base, entry.repository, entry.revision, file.path
            );
            let part = partial.join(format!("{}.part", file.path));
            let fetched = fetch(&url, &part, file.size, cancel, &mut |change: i64| {
                done = done.saturating_add_signed(change);
                progress(done, total);
            })?;
            let Some(sha256) = fetched else {
                return Ok(Outcome::Cancelled);
            };
            if sha256 != file.sha256 {
                let _ = fs::remove_file(&part);
                return Err(Error::invalid(format!(
                    "{} of {} does not match its checksum (expected {}, got {sha256})",
                    file.path, entry.name, file.sha256
                )));
            }
            fs::rename(&part, &target).map_err(io(&target))?;
        }

        let manifest = json!({
            "id": entry.id,
            "revision": entry.revision,
            "files": entry.files,
            "installed": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_secs()),
        });
        let manifest_path = partial.join(MANIFEST);
        fs::write(&manifest_path, manifest.to_string()).map_err(io(&manifest_path))?;
        let dir = self.dir(entry);
        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(io(&dir))?;
        }
        fs::rename(&partial, &dir).map_err(io(&dir))?;
        drop(lock);
        let _ = fs::remove_file(&lock_path);
        Ok(Outcome::Installed)
    }

    /// Delete the model and whatever was downloaded of it. Not an error when
    /// there is nothing.
    pub fn remove(&self, entry: &Entry) -> Result<()> {
        for dir in [self.dir(entry), self.partial(entry)] {
            match fs::remove_dir_all(&dir) {
                Ok(()) => {}
                Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => return Err(Error::io(&dir, source)),
            }
        }
        Ok(())
    }
}

/// Bring `part` to the `size` bytes of `url`, going on from the bytes it
/// holds. Returns the SHA-256 of the whole file, or `None` when cancelled.
/// `moved` hears each change of the byte count: positive as bytes arrive,
/// negative when a server that cannot resume makes the file start over.
fn fetch(
    url: &str,
    part: &Path,
    size: u64,
    cancel: &AtomicBool,
    moved: &mut dyn FnMut(i64),
) -> Result<Option<String>> {
    let mut have = fs::metadata(part).map_or(0, |found| found.len());
    if have > size {
        // Not this file's bytes.
        fs::remove_file(part).map_err(io(part))?;
        moved(-(size as i64));
        have = 0;
    }
    let mut hasher = Sha256::new();
    if have > 0 && !sha256_of(part, &mut hasher, cancel)? {
        return Ok(None);
    }
    if have < size {
        let mut request = agent(url.starts_with("https://")).get(url).header(
            "User-Agent",
            format!("CodeWiki/{}", env!("CARGO_PKG_VERSION")),
        );
        if have > 0 {
            request = request.header("Range", format!("bytes={have}-"));
        }
        let mut response = request
            .call()
            .map_err(|error| Error::external(format!("downloading {url}"), error))?;
        let resumed = response.status().as_u16() == 206;
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(part)
            .map_err(io(part))?;
        if have > 0 && !resumed {
            // The server sent the whole file again.
            file.set_len(0).map_err(io(part))?;
            hasher = Sha256::new();
            moved(-(have as i64));
            have = 0;
        }
        file.seek(SeekFrom::Start(have)).map_err(io(part))?;
        let mut reader = response.body_mut().as_reader();
        let mut buffer = vec![0; CHUNK];
        loop {
            if cancel.load(Ordering::Relaxed) {
                file.flush().map_err(io(part))?;
                return Ok(None);
            }
            let read = match reader.read(&mut buffer) {
                Ok(read) => read,
                Err(error) => {
                    // What arrived stays for the next try.
                    let _ = file.flush();
                    return Err(Error::external(
                        format!("the download of {url} broke off"),
                        error,
                    ));
                }
            };
            if read == 0 {
                break;
            }
            have += read as u64;
            if have > size {
                drop(file);
                let _ = fs::remove_file(part);
                return Err(Error::invalid(format!(
                    "{url} is larger than the {size} bytes the catalog names"
                )));
            }
            hasher.update(&buffer[..read]);
            file.write_all(&buffer[..read]).map_err(io(part))?;
            moved(read as i64);
        }
        file.flush().map_err(io(part))?;
        if have < size {
            return Err(Error::invalid(format!(
                "the download of {url} ended after {have} of {size} bytes"
            )));
        }
    }
    Ok(Some(hex::encode(hasher.finalize())))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::catalog::{Family, File as CatalogFile, Pooling, Tier};

    /// What a stand-in host does with a request for the large file.
    #[derive(Clone, Copy, PartialEq)]
    enum Mode {
        Normal,
        /// Close the connection after this many bytes of the first answer.
        CutFirst(usize),
        /// Answer a `Range` request with the whole file, as a host that
        /// cannot resume does.
        IgnoreRange,
    }

    /// `(path, Range header)` of every request, in order.
    type Requests = Arc<Mutex<Vec<(String, Option<String>)>>>;

    struct Host {
        url: String,
        requests: Requests,
    }

    /// Serve `files` by path until the test ends.
    fn host(files: HashMap<String, Vec<u8>>, mode: Mode) -> Host {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests: Requests = Arc::new(Mutex::new(Vec::new()));
        let seen = requests.clone();
        std::thread::spawn(move || {
            let mut cut_done = false;
            for stream in listener.incoming() {
                let Ok(stream) = stream else { break };
                let mut reader = BufReader::new(stream);
                let mut line = String::new();
                if reader.read_line(&mut line).is_err() {
                    continue;
                }
                let path = line.split_whitespace().nth(1).unwrap_or("").to_string();
                let mut range = None;
                loop {
                    let mut header = String::new();
                    if reader.read_line(&mut header).is_err() || header.trim_end().is_empty() {
                        break;
                    }
                    if let Some((key, value)) = header.trim_end().split_once(':')
                        && key.eq_ignore_ascii_case("range")
                    {
                        range = Some(value.trim().to_string());
                    }
                }
                seen.lock().unwrap().push((path.clone(), range.clone()));
                let mut stream = reader.into_inner();
                let Some(body) = files.get(&path) else {
                    let _ = write!(
                        stream,
                        "HTTP/1.1 404 X\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    );
                    continue;
                };
                let from = range
                    .as_deref()
                    .filter(|_| mode != Mode::IgnoreRange)
                    .and_then(|range| range.strip_prefix("bytes="))
                    .and_then(|range| range.strip_suffix('-'))
                    .and_then(|from| from.parse::<usize>().ok());
                let (status, body) = match from {
                    Some(from) => (206, &body[from..]),
                    None => (200, &body[..]),
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                match mode {
                    Mode::CutFirst(after) if !cut_done && body.len() > after => {
                        cut_done = true;
                        let _ = stream.write_all(&body[..after]);
                    }
                    _ => {
                        let _ = stream.write_all(body);
                    }
                }
                let _ = stream.flush();
            }
        });
        Host { url, requests }
    }

    fn sha256(bytes: &[u8]) -> String {
        hex::encode(Sha256::digest(bytes))
    }

    /// A model of two files: a small one, and one of several chunks.
    fn model() -> (Entry, HashMap<String, Vec<u8>>) {
        let config = br#"{"model_type":"modernbert"}"#.to_vec();
        let weights: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
        let revision = "a".repeat(40);
        let entry = Entry {
            id: "test-model".into(),
            name: "Test Model".into(),
            tier: Tier::Small,
            family: Family::ModernBert,
            repository: "owner/test-model".into(),
            revision: revision.clone(),
            license: "Apache-2.0".into(),
            languages: "English".into(),
            dimensions: 4,
            max_tokens: 8,
            pooling: Pooling::First,
            query_prefix: String::new(),
            batch: 2,
            files: vec![
                CatalogFile {
                    path: "config.json".into(),
                    size: config.len() as u64,
                    sha256: sha256(&config),
                },
                CatalogFile {
                    path: "model.safetensors".into(),
                    size: weights.len() as u64,
                    sha256: sha256(&weights),
                },
            ],
        };
        let at = |path: &str| format!("/owner/test-model/resolve/{revision}/{path}");
        let files = HashMap::from([
            (at("config.json"), config),
            (at("model.safetensors"), weights),
        ]);
        (entry, files)
    }

    fn install(store: &Store, entry: &Entry) -> Result<Outcome> {
        store.install(entry, &mut |_, _| {}, &AtomicBool::new(false))
    }

    #[test]
    fn a_model_is_downloaded_verified_and_moved_into_place() {
        let (entry, files) = model();
        let weights = files
            .iter()
            .find(|(path, _)| path.ends_with("model.safetensors"))
            .unwrap()
            .1
            .clone();
        let host = host(files, Mode::Normal);
        let data = tempfile::tempdir().unwrap();
        let store = Store::new(data.path(), Some(&host.url));
        assert!(!store.is_installed(&entry));

        let mut seen = Vec::new();
        let outcome = store
            .install(
                &entry,
                &mut |done, total| seen.push((done, total)),
                &AtomicBool::new(false),
            )
            .unwrap();
        assert_eq!(outcome, Outcome::Installed);
        assert!(store.is_installed(&entry));
        let dir = data.path().join("models").join("test-model");
        assert_eq!(fs::read(dir.join("model.safetensors")).unwrap(), weights);
        assert!(dir.join("manifest.json").is_file());
        assert!(
            !data
                .path()
                .join("models")
                .join(".partial")
                .join("test-model")
                .exists()
        );
        // Progress starts at nothing, never goes back, and ends at the size.
        assert_eq!(seen.first(), Some(&(0, entry.size())));
        assert_eq!(seen.last(), Some(&(entry.size(), entry.size())));
        assert!(seen.windows(2).all(|pair| pair[0].0 <= pair[1].0));

        // Another revision in the catalog: this one no longer counts.
        let mut moved = entry.clone();
        moved.revision = "b".repeat(40);
        assert!(!store.is_installed(&moved));

        store.remove(&entry).unwrap();
        assert!(!store.is_installed(&entry));
        assert!(!dir.exists());
        // Nothing left to remove is not an error.
        store.remove(&entry).unwrap();
    }

    #[test]
    fn a_download_that_broke_off_goes_on_from_its_bytes() {
        let (entry, files) = model();
        let host = host(files, Mode::CutFirst(100_000));
        let data = tempfile::tempdir().unwrap();
        let store = Store::new(data.path(), Some(&host.url));

        let error = install(&store, &entry).unwrap_err().to_string();
        assert!(
            error.contains("broke off") || error.contains("ended after"),
            "{error}"
        );
        assert!(!store.is_installed(&entry));
        let part = data
            .path()
            .join("models/.partial/test-model/model.safetensors.part");
        assert_eq!(fs::metadata(&part).unwrap().len(), 100_000);

        let mut first = None;
        let outcome = store
            .install(
                &entry,
                &mut |done, _| {
                    first.get_or_insert(done);
                },
                &AtomicBool::new(false),
            )
            .unwrap();
        assert_eq!(outcome, Outcome::Installed);
        assert!(store.is_installed(&entry));
        // The second try starts from what the first left: the small file
        // and the first 100,000 bytes.
        assert_eq!(first, Some(entry.files[0].size + 100_000));
        let requests = host.requests.lock().unwrap();
        let weights: Vec<_> = requests
            .iter()
            .filter(|(path, _)| path.ends_with("model.safetensors"))
            .collect();
        assert_eq!(weights.len(), 2);
        assert_eq!(weights[0].1, None);
        assert_eq!(weights[1].1.as_deref(), Some("bytes=100000-"));
        // The small file was not fetched again.
        assert_eq!(
            requests
                .iter()
                .filter(|(path, _)| path.ends_with("config.json"))
                .count(),
            1
        );
    }

    #[test]
    fn a_cancelled_download_keeps_its_bytes_and_a_host_without_ranges_starts_over() {
        let (entry, files) = model();
        let host = host(files, Mode::IgnoreRange);
        let data = tempfile::tempdir().unwrap();
        let store = Store::new(data.path(), Some(&host.url));

        let cancel = AtomicBool::new(false);
        let outcome = store
            .install(
                &entry,
                &mut |done, _| {
                    // After the first chunk of the large file.
                    if done > entry.files[0].size {
                        cancel.store(true, Ordering::Relaxed);
                    }
                },
                &cancel,
            )
            .unwrap();
        assert_eq!(outcome, Outcome::Cancelled);
        assert!(!store.is_installed(&entry));
        let part = data
            .path()
            .join("models/.partial/test-model/model.safetensors.part");
        let kept = fs::metadata(&part).unwrap().len();
        assert!(kept > 0 && kept < entry.files[1].size, "{kept}");

        // The host answers the Range request with the whole file; the
        // download starts over and still ends with the right bytes.
        let mut lowest = u64::MAX;
        let outcome = store
            .install(
                &entry,
                &mut |done, _| lowest = lowest.min(done),
                &AtomicBool::new(false),
            )
            .unwrap();
        assert_eq!(outcome, Outcome::Installed);
        assert!(store.is_installed(&entry));
        assert_eq!(lowest, entry.files[0].size);
    }

    #[test]
    fn a_file_that_does_not_match_its_checksum_is_refused_and_deleted() {
        let (mut entry, files) = model();
        entry.files[1].sha256 = "0".repeat(64);
        let host = host(files, Mode::Normal);
        let data = tempfile::tempdir().unwrap();
        let store = Store::new(data.path(), Some(&host.url));

        let error = install(&store, &entry).unwrap_err().to_string();
        assert!(error.contains("does not match its checksum"), "{error}");
        assert!(!store.is_installed(&entry));
        assert!(!data.path().join("models").join("test-model").exists());
        assert!(
            !data
                .path()
                .join("models/.partial/test-model/model.safetensors.part")
                .exists()
        );
    }

    #[test]
    fn a_file_larger_than_the_catalog_says_is_refused() {
        let (mut entry, files) = model();
        entry.files[1].size = 1000;
        let host = host(files, Mode::Normal);
        let data = tempfile::tempdir().unwrap();
        let store = Store::new(data.path(), Some(&host.url));
        let error = install(&store, &entry).unwrap_err().to_string();
        assert!(error.contains("larger than the 1000 bytes"), "{error}");
    }

    #[test]
    fn a_missing_file_is_reported_with_its_address() {
        let (entry, mut files) = model();
        files.retain(|path, _| !path.ends_with("model.safetensors"));
        let host = host(files, Mode::Normal);
        let data = tempfile::tempdir().unwrap();
        let store = Store::new(data.path(), Some(&host.url));
        let error = install(&store, &entry).unwrap_err().to_string();
        assert!(error.contains("model.safetensors"), "{error}");
        assert!(!store.is_installed(&entry));
    }

    #[test]
    fn only_one_program_installs_a_model_at_a_time() {
        let (entry, files) = model();
        let host = host(files, Mode::Normal);
        let data = tempfile::tempdir().unwrap();
        let store = Store::new(data.path(), Some(&host.url));
        let partial = data.path().join("models").join(".partial");
        fs::create_dir_all(&partial).unwrap();
        let held = File::create(partial.join("test-model.lock")).unwrap();
        held.lock().unwrap();

        let error = install(&store, &entry).unwrap_err().to_string();
        assert!(
            error.ends_with("another program is installing Test Model"),
            "{error}"
        );
        drop(held);
        assert_eq!(install(&store, &entry).unwrap(), Outcome::Installed);
    }

    /// The real host, by hand: `cargo test -p cw-models -- --ignored`. It
    /// downloads 99 MB, stops once on the way and goes on, which is how a
    /// redirect to the host's storage and a `Range` request are checked.
    #[test]
    #[ignore = "downloads a model from Hugging Face"]
    fn the_small_model_installs_from_hugging_face() {
        let entry = catalog::find("granite-small-english-r2").unwrap();
        let data = tempfile::tempdir().unwrap();
        let store = Store::new(data.path(), None);
        let cancel = AtomicBool::new(false);
        let outcome = store
            .install(
                entry,
                &mut |done, _| {
                    if done > 20_000_000 {
                        cancel.store(true, Ordering::Relaxed);
                    }
                },
                &cancel,
            )
            .unwrap();
        assert_eq!(outcome, Outcome::Cancelled);
        let mut first = None;
        let outcome = store
            .install(
                entry,
                &mut |done, _| {
                    first.get_or_insert(done);
                },
                &AtomicBool::new(false),
            )
            .unwrap();
        assert_eq!(outcome, Outcome::Installed);
        assert!(first.is_some_and(|done| done > 20_000_000), "{first:?}");
        assert!(store.is_installed(entry));
    }

    #[test]
    fn the_catalogs_models_are_found_when_installed() {
        let data = tempfile::tempdir().unwrap();
        let store = Store::new(data.path(), None);
        assert!(store.installed().is_empty());
        // Put a model of the catalog in place by hand, as an install does.
        let entry = catalog::find("granite-small-english-r2").unwrap();
        let dir = store.dir(entry);
        fs::create_dir_all(&dir).unwrap();
        for file in &entry.files {
            let handle = File::create(dir.join(&file.path)).unwrap();
            handle.set_len(file.size).unwrap();
        }
        assert!(store.installed().is_empty(), "no manifest yet");
        fs::write(
            dir.join("manifest.json"),
            json!({ "revision": entry.revision }).to_string(),
        )
        .unwrap();
        let installed: Vec<&str> = store.installed().iter().map(|e| e.id.as_str()).collect();
        assert_eq!(installed, ["granite-small-english-r2"]);
    }
}
