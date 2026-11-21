//! Opening the index of a repository without building one.

mod support;

use std::fs;
use std::path::Path;

use cw_wiki::service::{open_existing_index_in, open_index_in};

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "Cargo.toml",
        "[package]\nname = \"engine\"\nversion = \"0.1.0\"\n",
    );
    write(
        dir.path(),
        "src/lib.rs",
        "//! Order matching.\n\n/// A price in ticks.\npub struct Price(pub u64);\n\n/// The best price.\npub fn best() -> Price { Price(0) }\n",
    );
    dir
}

#[test]
fn an_index_that_was_never_built_is_not_opened() {
    let repo = repository();
    let data = tempfile::tempdir().unwrap();
    assert!(
        open_existing_index_in(data.path(), repo.path())
            .unwrap()
            .is_none()
    );
    // Asking did not build one.
    assert!(
        open_existing_index_in(data.path(), repo.path())
            .unwrap()
            .is_none()
    );

    support::no_model_server();
    open_index_in(data.path(), repo.path()).unwrap();
    assert!(
        open_existing_index_in(data.path(), repo.path())
            .unwrap()
            .is_some()
    );
}
