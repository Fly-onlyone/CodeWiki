//! Embedding vectors for semantic search.
//!
//! The index stores vectors and compares them; it does not know how they
//! were made. Each row carries a hash of the text and embedder it came from,
//! so only changed documents are embedded again.

use std::collections::{HashMap, HashSet};

use cw_core::Result;
use rusqlite::params;

use crate::search::Hit;
use crate::store::{Index, db};

/// One stored vector. `kind` and `key` match the search document it embeds.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorRow {
    pub kind: String,
    pub key: String,
    pub title: String,
    /// Hash of the embedded text and the embedder's name.
    pub hash: String,
    pub vector: Vec<f32>,
}

fn to_blob(vector: &[f32]) -> Vec<u8> {
    vector.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn from_blob(blob: &[u8]) -> Vec<f32> {
    blob.as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect()
}

fn delete(tx: &rusqlite::Transaction<'_>, ids: &[(String, String)]) -> Result<()> {
    let err = db("writing vectors");
    let mut stmt = tx
        .prepare("DELETE FROM vectors WHERE kind = ?1 AND key = ?2")
        .map_err(&err)?;
    for (kind, key) in ids {
        stmt.execute(params![kind, key]).map_err(&err)?;
    }
    Ok(())
}

fn upsert(tx: &rusqlite::Transaction<'_>, rows: &[VectorRow]) -> Result<()> {
    let err = db("writing vectors");
    let mut stmt = tx
        .prepare(
            "INSERT OR REPLACE INTO vectors (kind, key, title, hash, data) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .map_err(&err)?;
    for row in rows {
        stmt.execute(params![
            row.kind,
            row.key,
            row.title,
            row.hash,
            to_blob(&row.vector)
        ])
        .map_err(&err)?;
    }
    Ok(())
}

impl Index {
    /// The stored hash of every vector of the given kinds, by `(kind, key)`.
    pub fn vector_hashes(&self, kinds: &[&str]) -> Result<HashMap<(String, String), String>> {
        let err = db("reading vector hashes");
        let mut stmt = self
            .conn
            .prepare("SELECT kind, key, hash FROM vectors")
            .map_err(&err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    (r.get::<_, String>(0)?, r.get::<_, String>(1)?),
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(&err)?;
        let mut out = HashMap::new();
        for row in rows {
            let (id, hash) = row.map_err(&err)?;
            if kinds.contains(&id.0.as_str()) {
                out.insert(id, hash);
            }
        }
        Ok(out)
    }

    /// The kinds of document that have a stored vector.
    pub fn vector_kinds(&self) -> Result<Vec<String>> {
        let err = db("reading vector kinds");
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT kind FROM vectors ORDER BY kind")
            .map_err(&err)?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(&err)?;
        rows.collect::<std::result::Result<_, _>>().map_err(&err)
    }

    /// Insert or replace vectors, and delete every vector of `kinds` whose
    /// `(kind, key)` is not in `keep`, in one step.
    pub fn store_vectors(
        &mut self,
        kinds: &[&str],
        keep: &HashSet<(String, String)>,
        rows: &[VectorRow],
    ) -> Result<()> {
        let stale = self.stale_vectors(kinds, keep)?;
        let err = db("writing vectors");
        let tx = self.conn.transaction().map_err(&err)?;
        delete(&tx, &stale)?;
        upsert(&tx, rows)?;
        tx.commit().map_err(&err)
    }

    /// Insert or replace vectors and touch no other. An embedding that runs
    /// batch by batch stores each batch with this, so that a run cut short
    /// keeps what it has done.
    pub fn upsert_vectors(&mut self, rows: &[VectorRow]) -> Result<()> {
        let err = db("writing vectors");
        let tx = self.conn.transaction().map_err(&err)?;
        upsert(&tx, rows)?;
        tx.commit().map_err(&err)
    }

    /// Delete every vector of `kinds` whose `(kind, key)` is not in `keep`.
    /// Returns how many were deleted.
    pub fn prune_vectors(
        &mut self,
        kinds: &[&str],
        keep: &HashSet<(String, String)>,
    ) -> Result<usize> {
        let stale = self.stale_vectors(kinds, keep)?;
        let err = db("writing vectors");
        let tx = self.conn.transaction().map_err(&err)?;
        delete(&tx, &stale)?;
        tx.commit().map_err(&err)?;
        Ok(stale.len())
    }

    /// The vectors of `kinds` that `keep` does not name.
    fn stale_vectors(
        &self,
        kinds: &[&str],
        keep: &HashSet<(String, String)>,
    ) -> Result<Vec<(String, String)>> {
        Ok(self
            .vector_hashes(kinds)?
            .into_keys()
            .filter(|id| !keep.contains(id))
            .collect())
    }

    /// The stored vectors closest to `query`, best first. Scores are dot
    /// products, so both sides must be unit vectors. Vectors of another
    /// length (made by a different embedder) are skipped.
    pub fn nearest(&self, query: &[f32], kind: Option<&str>, limit: usize) -> Result<Vec<Hit>> {
        let err = db("reading vectors");
        let mut stmt = self
            .conn
            .prepare("SELECT kind, key, title, data FROM vectors WHERE ?1 IS NULL OR kind = ?1")
            .map_err(&err)?;
        let rows = stmt
            .query_map(params![kind], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Vec<u8>>(3)?,
                ))
            })
            .map_err(&err)?;
        let mut hits = Vec::new();
        for row in rows {
            let (kind, key, title, blob) = row.map_err(&err)?;
            if blob.len() != query.len() * 4 {
                continue;
            }
            let score: f32 = from_blob(&blob).iter().zip(query).map(|(a, b)| a * b).sum();
            hits.push(Hit {
                kind,
                key,
                title,
                snippet: String::new(),
                score: f64::from(score),
            });
        }
        hits.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.key.cmp(&b.key)));
        hits.truncate(limit);
        Ok(hits)
    }

    /// The opening of a search document's body, for hits that full-text
    /// search did not return a snippet for.
    pub fn document_opening(&self, kind: &str, key: &str, chars: usize) -> Result<String> {
        let err = db("reading a search document");
        let mut stmt = self
            .conn
            .prepare("SELECT body FROM documents WHERE kind = ?1 AND key = ?2 LIMIT 1")
            .map_err(&err)?;
        let mut rows = stmt.query(params![kind, key]).map_err(&err)?;
        let body: String = match rows.next().map_err(&err)? {
            Some(row) => row.get(0).map_err(&err)?,
            None => return Ok(String::new()),
        };
        let flat = body.split_whitespace().collect::<Vec<_>>().join(" ");
        Ok(flat.chars().take(chars).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::Document;

    fn row(kind: &str, key: &str, hash: &str, vector: &[f32]) -> VectorRow {
        VectorRow {
            kind: kind.into(),
            key: key.into(),
            title: key.to_uppercase(),
            hash: hash.into(),
            vector: vector.to_vec(),
        }
    }

    fn id(kind: &str, key: &str) -> (String, String) {
        (kind.to_string(), key.to_string())
    }

    #[test]
    fn stores_ranks_and_prunes_vectors() {
        let mut index = Index::open_in_memory().unwrap();
        let keep: HashSet<_> = [id("note", "a"), id("note", "b"), id("symbol", "s")].into();
        index
            .store_vectors(
                &["note", "symbol"],
                &keep,
                &[
                    row("note", "a", "h1", &[1.0, 0.0]),
                    row("note", "b", "h2", &[0.6, 0.8]),
                    row("symbol", "s", "h3", &[0.0, 1.0]),
                ],
            )
            .unwrap();
        let hits = index.nearest(&[1.0, 0.0], None, 10).unwrap();
        let keys: Vec<&str> = hits.iter().map(|h| h.key.as_str()).collect();
        assert_eq!(keys, ["a", "b", "s"]);
        assert!((hits[1].score - 0.6).abs() < 1e-6);
        assert_eq!(hits[0].title, "A");
        assert_eq!(index.vector_kinds().unwrap(), ["note", "symbol"]);

        let only_symbols = index.nearest(&[1.0, 0.0], Some("symbol"), 10).unwrap();
        assert_eq!(only_symbols.len(), 1);
        assert_eq!(index.nearest(&[1.0, 0.0], None, 1).unwrap().len(), 1);
        // A query of another length matches nothing rather than garbage.
        assert!(
            index
                .nearest(&[1.0, 0.0, 0.0], None, 10)
                .unwrap()
                .is_empty()
        );

        // Dropping `b` from the notes leaves symbols alone.
        let keep: HashSet<_> = [id("note", "a")].into();
        index.store_vectors(&["note"], &keep, &[]).unwrap();
        let hashes = index.vector_hashes(&["note", "symbol"]).unwrap();
        assert_eq!(hashes.len(), 2);
        assert_eq!(hashes[&id("note", "a")], "h1");
        assert!(hashes.contains_key(&id("symbol", "s")));
        assert_eq!(index.vector_hashes(&["note"]).unwrap().len(), 1);
    }

    #[test]
    fn a_batch_is_stored_without_touching_the_other_vectors() {
        let mut index = Index::open_in_memory().unwrap();
        index
            .upsert_vectors(&[row("note", "a", "h1", &[1.0, 0.0])])
            .unwrap();
        // A second batch leaves the first alone, though it does not name it.
        index
            .upsert_vectors(&[
                row("note", "b", "h2", &[0.0, 1.0]),
                row("symbol", "s", "h3", &[0.6, 0.8]),
            ])
            .unwrap();
        assert_eq!(index.vector_hashes(&["note", "symbol"]).unwrap().len(), 3);
        // The same key again replaces its vector.
        index
            .upsert_vectors(&[row("note", "a", "h4", &[0.0, 1.0])])
            .unwrap();
        assert_eq!(
            index.vector_hashes(&["note"]).unwrap()[&id("note", "a")],
            "h4"
        );
        assert_eq!(index.vector_hashes(&["note", "symbol"]).unwrap().len(), 3);
    }

    #[test]
    fn pruning_deletes_only_what_is_not_kept_of_the_named_kinds() {
        let mut index = Index::open_in_memory().unwrap();
        index
            .upsert_vectors(&[
                row("note", "a", "h1", &[1.0, 0.0]),
                row("note", "b", "h2", &[0.0, 1.0]),
                row("symbol", "s", "h3", &[0.6, 0.8]),
            ])
            .unwrap();
        let keep: HashSet<_> = [id("note", "a")].into();
        // `s` is not kept either, but symbols were not named.
        assert_eq!(index.prune_vectors(&["note"], &keep).unwrap(), 1);
        let hashes = index.vector_hashes(&["note", "symbol"]).unwrap();
        assert_eq!(hashes.len(), 2);
        assert!(hashes.contains_key(&id("note", "a")));
        assert!(hashes.contains_key(&id("symbol", "s")));
        assert_eq!(index.prune_vectors(&["note"], &keep).unwrap(), 0);
    }

    #[test]
    fn replaces_a_vector_with_the_same_key() {
        let mut index = Index::open_in_memory().unwrap();
        let keep: HashSet<_> = [id("note", "a")].into();
        for (hash, vector) in [("h1", [1.0, 0.0]), ("h2", [0.0, 1.0])] {
            index
                .store_vectors(&["note"], &keep, &[row("note", "a", hash, &vector)])
                .unwrap();
        }
        assert_eq!(
            index.vector_hashes(&["note"]).unwrap()[&id("note", "a")],
            "h2"
        );
        assert!((index.nearest(&[0.0, 1.0], None, 1).unwrap()[0].score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn document_opening_flattens_whitespace() {
        let mut index = Index::open_in_memory().unwrap();
        index
            .replace_documents(
                &["note"],
                &[Document {
                    kind: "note".into(),
                    key: "a.md".into(),
                    title: "A".into(),
                    body: "first line\n\n  second   line".into(),
                }],
            )
            .unwrap();
        assert_eq!(
            index.document_opening("note", "a.md", 17).unwrap(),
            "first line second"
        );
        assert_eq!(index.document_opening("note", "missing", 10).unwrap(), "");
    }
}
