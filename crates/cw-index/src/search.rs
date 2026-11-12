//! Full-text search over notes, symbols and files (SQLite FTS5, BM25).

use cw_core::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::store::{Index, db};

/// One searchable unit. `kind` is `note`, `symbol` or `file`; `key` is the
/// note path, qualified symbol name or file path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub kind: String,
    pub key: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hit {
    pub kind: String,
    pub key: String,
    pub title: String,
    pub snippet: String,
    /// Higher is better.
    pub score: f64,
}

/// Turn free text into an FTS5 query: each word becomes a quoted prefix term,
/// joined with OR so partial matches still rank. Quoting keeps user input from
/// being read as FTS5 syntax.
fn fts_query(text: &str) -> Option<String> {
    let terms: Vec<String> = text
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{t}\"*"))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" OR "))
}

impl Index {
    /// Replace all documents of the given kinds.
    pub fn replace_documents(&mut self, kinds: &[&str], docs: &[Document]) -> Result<()> {
        let err = db("writing search documents");
        let tx = self.conn.transaction().map_err(&err)?;
        for kind in kinds {
            tx.execute("DELETE FROM documents WHERE kind = ?1", [kind])
                .map_err(&err)?;
        }
        {
            let mut stmt = tx
                .prepare("INSERT INTO documents (kind, key, title, body) VALUES (?1, ?2, ?3, ?4)")
                .map_err(&err)?;
            for d in docs {
                stmt.execute(params![d.kind, d.key, d.title, d.body])
                    .map_err(&err)?;
            }
        }
        tx.commit().map_err(&err)
    }

    /// Search documents. `kind` restricts results when given. Title matches
    /// weigh five times body matches.
    pub fn search(&self, text: &str, kind: Option<&str>, limit: usize) -> Result<Vec<Hit>> {
        let Some(query) = fts_query(text) else {
            return Ok(Vec::new());
        };
        let err = db("searching");
        let mut stmt = self
            .conn
            .prepare(
                "SELECT kind, key, title, \
                        snippet(documents, 3, '', '', ' … ', 24), \
                        -bm25(documents, 0.0, 0.0, 5.0, 1.0) AS score \
                 FROM documents \
                 WHERE documents MATCH ?1 AND (?2 IS NULL OR kind = ?2) \
                 ORDER BY score DESC, key \
                 LIMIT ?3",
            )
            .map_err(&err)?;
        let rows = stmt
            .query_map(params![query, kind, limit as i64], |r| {
                Ok(Hit {
                    kind: r.get(0)?,
                    key: r.get(1)?,
                    title: r.get(2)?,
                    snippet: r.get(3)?,
                    score: r.get(4)?,
                })
            })
            .map_err(&err)?;
        rows.collect::<rusqlite::Result<Vec<Hit>>>().map_err(&err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(kind: &str, key: &str, title: &str, body: &str) -> Document {
        Document {
            kind: kind.into(),
            key: key.into(),
            title: title.into(),
            body: body.into(),
        }
    }

    fn index() -> Index {
        let mut index = Index::open_in_memory().unwrap();
        index
            .replace_documents(
                &["note", "symbol"],
                &[
                    doc(
                        "note",
                        "01-Core/Hashing.md",
                        "Hashing",
                        "Content hashing detects edited notes.",
                    ),
                    doc(
                        "note",
                        "01-Core/Vault.md",
                        "Vault",
                        "The vault stores notes. Hashing is mentioned once.",
                    ),
                    doc(
                        "symbol",
                        "cw_core::hash::sha256_hex",
                        "sha256_hex",
                        "Lowercase hex SHA-256 of bytes.",
                    ),
                ],
            )
            .unwrap();
        index
    }

    #[test]
    fn ranks_title_matches_first() {
        let hits = index().search("hashing", None, 10).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].key, "01-Core/Hashing.md");
        assert!(hits[0].score > hits[1].score);
    }

    #[test]
    fn filters_by_kind_and_matches_prefixes_and_underscores() {
        let index = index();
        let hits = index.search("sha256", Some("symbol"), 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].key, "cw_core::hash::sha256_hex");
        assert_eq!(index.search("sha256_hex", None, 10).unwrap().len(), 1);
        assert!(index.search("sha256", Some("note"), 10).unwrap().is_empty());
    }

    #[test]
    fn hostile_input_is_not_parsed_as_syntax() {
        let index = index();
        assert!(index.search("\"", None, 10).unwrap().is_empty());
        assert!(index.search("", None, 10).unwrap().is_empty());
        assert_eq!(
            index.search("vault AND (", None, 10).unwrap()[0].key,
            "01-Core/Vault.md"
        );
    }

    #[test]
    fn replacing_one_kind_keeps_the_others() {
        let mut index = index();
        index.replace_documents(&["note"], &[]).unwrap();
        assert!(index.search("hashing", None, 10).unwrap().is_empty());
        assert_eq!(index.search("sha256", None, 10).unwrap().len(), 1);
    }
}
