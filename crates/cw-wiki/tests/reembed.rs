//! Embedding the documents of an index again, with another embedder, without
//! a scan of the repository.

use std::fs;
use std::ops::ControlFlow;
use std::path::Path;

use cw_core::Result;
use cw_embed::{Embedder, LocalEmbedder};
use cw_wiki::semantic::{Progress, behind};
use cw_wiki::service::{DOCUMENT_KINDS, open_existing_index_in, open_index_in, reembed};

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

/// Another embedder as far as the index can tell, which takes two documents
/// at a time.
struct Other(LocalEmbedder);

impl Embedder for Other {
    fn name(&self) -> String {
        "other".into()
    }

    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        assert!(texts.len() <= 2, "more than its batch size");
        self.0.embed(texts)
    }

    fn batch_size(&self) -> usize {
        2
    }
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

    open_index_in(data.path(), repo.path()).unwrap();
    assert!(
        open_existing_index_in(data.path(), repo.path())
            .unwrap()
            .is_some()
    );
}

#[test]
fn the_documents_of_an_index_are_embedded_again_in_steps() {
    let repo = repository();
    let data = tempfile::tempdir().unwrap();
    open_index_in(data.path(), repo.path()).unwrap();
    let mut index = open_existing_index_in(data.path(), repo.path())
        .unwrap()
        .unwrap();
    let documents = index.documents(&DOCUMENT_KINDS).unwrap().len();
    assert!(documents >= 4, "{documents} documents");

    let other = Other(LocalEmbedder::default());
    assert!(behind(&index, &other.name(), &DOCUMENT_KINDS).unwrap());

    // Stopped after the first batch.
    let first = reembed(&mut index, &other, &mut |_| ControlFlow::Break(())).unwrap();
    assert_eq!(
        first,
        Progress {
            done: 2,
            total: documents
        }
    );
    assert!(behind(&index, &other.name(), &DOCUMENT_KINDS).unwrap());

    // The next call makes the rest, and hears of every batch.
    let mut heard = Vec::new();
    let second = reembed(&mut index, &other, &mut |progress| {
        heard.push(progress.done);
        ControlFlow::Continue(())
    })
    .unwrap();
    assert_eq!(
        second,
        Progress {
            done: documents - 2,
            total: documents - 2
        }
    );
    assert_eq!(heard.last(), Some(&(documents - 2)));
    assert!(heard.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(!behind(&index, &other.name(), &DOCUMENT_KINDS).unwrap());
    assert_eq!(
        index.vector_hashes(&DOCUMENT_KINDS).unwrap().len(),
        documents
    );

    // With nothing changed there is nothing to do.
    let third = reembed(&mut index, &other, &mut |_| ControlFlow::Continue(())).unwrap();
    assert_eq!(third, Progress { done: 0, total: 0 });
}
