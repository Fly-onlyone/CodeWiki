//! Reads and writes of structural facts.

use std::path::Path;

use cw_core::{
    Confidence, Edge, EdgeKind, Error, FileRecord, Language, Result, Symbol, SymbolKind, Visibility,
};
use cw_parse::{CrateInfo, ExternalUse, RepoFacts};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::schema::{DROP_ALL, SCHEMA, SCHEMA_VERSION};

pub(crate) fn db(context: &'static str) -> impl Fn(rusqlite::Error) -> Error {
    move |e| Error::external(context, e)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub pagerank: f64,
    pub betweenness: f64,
    pub scc: usize,
    pub community: usize,
    pub x: f32,
    pub y: f32,
}

pub struct Index {
    pub(crate) conn: Connection,
}

const SYMBOL_COLUMNS: &str =
    "qualified, name, kind, visibility, file, start_line, end_line, signature, doc, parent, attrs";

fn symbol_from_row(row: &Row) -> rusqlite::Result<Symbol> {
    let kind: String = row.get(2)?;
    let visibility: String = row.get(3)?;
    Ok(Symbol {
        qualified: row.get(0)?,
        name: row.get(1)?,
        kind: SymbolKind::parse(&kind).unwrap_or(SymbolKind::Function),
        visibility: Visibility::parse(&visibility),
        file: row.get(4)?,
        start_line: row.get(5)?,
        end_line: row.get(6)?,
        signature: row.get(7)?,
        doc: row.get(8)?,
        parent: row.get(9)?,
        attrs: row
            .get::<_, String>(10)?
            .split(',')
            .filter(|a| !a.is_empty())
            .map(str::to_string)
            .collect(),
    })
}

fn edge_from_row(row: &Row) -> rusqlite::Result<Edge> {
    let kind: String = row.get(2)?;
    let confidence: String = row.get(3)?;
    Ok(Edge {
        src: row.get(0)?,
        dst: row.get(1)?,
        kind: EdgeKind::parse(&kind).unwrap_or(EdgeKind::References),
        confidence: Confidence::parse(&confidence),
    })
}

impl Index {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let conn = Connection::open(path).map_err(db("opening the index"))?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")
            .map_err(db("configuring the index"))?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory().map_err(db("opening the index"))?)
    }

    fn init(conn: Connection) -> Result<Self> {
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(db("reading the schema version"))?;
        if version != SCHEMA_VERSION {
            conn.execute_batch(DROP_ALL)
                .map_err(db("dropping an outdated index"))?;
        }
        conn.execute_batch(SCHEMA)
            .map_err(db("creating the schema"))?;
        conn.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION}"))
            .map_err(db("writing the schema version"))?;
        Ok(Self { conn })
    }

    /// Replace every structural fact in one transaction.
    pub fn replace_facts(&mut self, facts: &RepoFacts) -> Result<()> {
        let err = db("writing facts");
        let tx = self.conn.transaction().map_err(&err)?;
        tx.execute_batch(
            "DELETE FROM files; DELETE FROM symbols; DELETE FROM edges; \
             DELETE FROM crates; DELETE FROM external_uses;",
        )
        .map_err(&err)?;
        {
            let mut stmt = tx
                .prepare("INSERT INTO files VALUES (?1, ?2, ?3, ?4, ?5)")
                .map_err(&err)?;
            for f in &facts.files {
                stmt.execute(params![
                    f.path,
                    f.language.as_str(),
                    f.hash,
                    f.bytes as i64,
                    f.lines
                ])
                .map_err(&err)?;
            }
            let mut stmt = tx
                .prepare("INSERT OR IGNORE INTO symbols VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)")
                .map_err(&err)?;
            for s in &facts.symbols {
                stmt.execute(params![
                    s.qualified,
                    s.name,
                    s.kind.as_str(),
                    s.visibility.as_str(),
                    s.file,
                    s.start_line,
                    s.end_line,
                    s.signature,
                    s.doc,
                    s.parent,
                    s.attrs.join(",")
                ])
                .map_err(&err)?;
            }
            let mut stmt = tx
                .prepare("INSERT OR REPLACE INTO edges VALUES (?1, ?2, ?3, ?4)")
                .map_err(&err)?;
            for e in &facts.edges {
                stmt.execute(params![
                    e.src,
                    e.dst,
                    e.kind.as_str(),
                    e.confidence.as_str()
                ])
                .map_err(&err)?;
            }
            let mut stmt = tx
                .prepare("INSERT OR REPLACE INTO crates VALUES (?1, ?2)")
                .map_err(&err)?;
            for c in &facts.crates {
                let json =
                    serde_json::to_string(c).map_err(|e| Error::external("encoding a crate", e))?;
                stmt.execute(params![c.name, json]).map_err(&err)?;
            }
            let mut stmt = tx
                .prepare("INSERT OR IGNORE INTO external_uses VALUES (?1, ?2)")
                .map_err(&err)?;
            for u in &facts.external_uses {
                stmt.execute(params![u.file, u.krate]).map_err(&err)?;
            }
        }
        tx.commit().map_err(&err)
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO meta VALUES (?1, ?2)",
                params![key, value],
            )
            .map(|_| ())
            .map_err(db("writing metadata"))
    }

    pub fn meta(&self, key: &str) -> Result<Option<String>> {
        self.conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
            .optional()
            .map_err(db("reading metadata"))
    }

    fn collect<T>(
        &self,
        sql: &str,
        args: impl rusqlite::Params,
        map: impl FnMut(&Row) -> rusqlite::Result<T>,
    ) -> Result<Vec<T>> {
        let err = db("querying the index");
        let mut stmt = self.conn.prepare(sql).map_err(&err)?;
        let rows = stmt.query_map(args, map).map_err(&err)?;
        rows.collect::<rusqlite::Result<Vec<T>>>().map_err(&err)
    }

    pub fn files(&self) -> Result<Vec<FileRecord>> {
        self.collect(
            "SELECT path, language, hash, bytes, lines FROM files ORDER BY path",
            [],
            |r| {
                let language: String = r.get(1)?;
                let bytes: i64 = r.get(3)?;
                Ok(FileRecord {
                    path: r.get(0)?,
                    language: Language::parse(&language),
                    hash: r.get(2)?,
                    bytes: bytes as u64,
                    lines: r.get(4)?,
                })
            },
        )
    }

    pub fn symbols(&self) -> Result<Vec<Symbol>> {
        self.collect(
            &format!("SELECT {SYMBOL_COLUMNS} FROM symbols ORDER BY qualified"),
            [],
            symbol_from_row,
        )
    }

    pub fn symbol(&self, qualified: &str) -> Result<Option<Symbol>> {
        self.conn
            .query_row(
                &format!("SELECT {SYMBOL_COLUMNS} FROM symbols WHERE qualified = ?1"),
                [qualified],
                symbol_from_row,
            )
            .optional()
            .map_err(db("reading a symbol"))
    }

    pub fn symbols_in_file(&self, file: &str) -> Result<Vec<Symbol>> {
        self.collect(
            &format!("SELECT {SYMBOL_COLUMNS} FROM symbols WHERE file = ?1 ORDER BY start_line, qualified"),
            [file],
            symbol_from_row,
        )
    }

    /// Symbols whose bare or qualified name contains `needle`, exact bare-name
    /// matches first.
    pub fn find_symbols(&self, needle: &str, limit: usize) -> Result<Vec<Symbol>> {
        let pattern = format!("%{}%", needle.replace('%', "").replace('_', "\\_"));
        self.collect(
            &format!(
                "SELECT {SYMBOL_COLUMNS} FROM symbols \
                 WHERE qualified LIKE ?1 ESCAPE '\\' \
                 ORDER BY (name = ?2) DESC, (qualified = ?2) DESC, length(qualified), qualified \
                 LIMIT ?3"
            ),
            params![pattern, needle, limit as i64],
            symbol_from_row,
        )
    }

    pub fn edges(&self) -> Result<Vec<Edge>> {
        self.collect(
            "SELECT src, dst, kind, confidence FROM edges ORDER BY src, dst, kind",
            [],
            edge_from_row,
        )
    }

    pub fn edges_from(&self, node: &str) -> Result<Vec<Edge>> {
        self.collect(
            "SELECT src, dst, kind, confidence FROM edges WHERE src = ?1 ORDER BY dst, kind",
            [node],
            edge_from_row,
        )
    }

    pub fn edges_to(&self, node: &str) -> Result<Vec<Edge>> {
        self.collect(
            "SELECT src, dst, kind, confidence FROM edges WHERE dst = ?1 ORDER BY src, kind",
            [node],
            edge_from_row,
        )
    }

    pub fn crates(&self) -> Result<Vec<CrateInfo>> {
        let rows = self.collect("SELECT json FROM crates ORDER BY name", [], |r| {
            r.get::<_, String>(0)
        })?;
        rows.iter()
            .map(|j| serde_json::from_str(j).map_err(|e| Error::external("decoding a crate", e)))
            .collect()
    }

    pub fn external_uses(&self) -> Result<Vec<ExternalUse>> {
        self.collect(
            "SELECT file, krate FROM external_uses ORDER BY krate, file",
            [],
            |r| {
                Ok(ExternalUse {
                    file: r.get(0)?,
                    krate: r.get(1)?,
                })
            },
        )
    }

    pub fn replace_metrics(&mut self, metrics: &[(String, NodeMetrics)]) -> Result<()> {
        let err = db("writing metrics");
        let tx = self.conn.transaction().map_err(&err)?;
        tx.execute("DELETE FROM metrics", []).map_err(&err)?;
        {
            let mut stmt = tx
                .prepare("INSERT OR REPLACE INTO metrics VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
                .map_err(&err)?;
            for (node, m) in metrics {
                stmt.execute(params![
                    node,
                    m.pagerank,
                    m.betweenness,
                    m.scc as i64,
                    m.community as i64,
                    m.x as f64,
                    m.y as f64
                ])
                .map_err(&err)?;
            }
        }
        tx.commit().map_err(&err)
    }

    pub fn metrics(&self) -> Result<Vec<(String, NodeMetrics)>> {
        self.collect(
            "SELECT node, pagerank, betweenness, scc, community, x, y FROM metrics ORDER BY node",
            [],
            |r| {
                let scc: i64 = r.get(3)?;
                let community: i64 = r.get(4)?;
                let x: f64 = r.get(5)?;
                let y: f64 = r.get(6)?;
                Ok((
                    r.get(0)?,
                    NodeMetrics {
                        pagerank: r.get(1)?,
                        betweenness: r.get(2)?,
                        scc: scc as usize,
                        community: community as usize,
                        x: x as f32,
                        y: y as f32,
                    },
                ))
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> RepoFacts {
        let symbol = |q: &str, kind, file: &str| Symbol {
            qualified: q.into(),
            name: q.rsplit("::").next().unwrap().into(),
            kind,
            visibility: Visibility::Public,
            file: file.into(),
            start_line: 1,
            end_line: 3,
            signature: format!("sig {q}"),
            doc: "Docs.".into(),
            parent: q.rsplit_once("::").map(|(p, _)| p.to_string()),
            attrs: vec!["derive".into()],
        };
        RepoFacts {
            crates: vec![CrateInfo {
                name: "demo".into(),
                package: "demo".into(),
                dir: String::new(),
                description: "A demo".into(),
                deps: vec!["serde".into()],
                has_lib: true,
                has_bin: false,
            }],
            files: vec![FileRecord {
                path: "src/lib.rs".into(),
                language: Language::Rust,
                hash: "abc".into(),
                bytes: 10,
                lines: 3,
            }],
            symbols: vec![
                symbol("demo", SymbolKind::Module, "src/lib.rs"),
                symbol("demo::run", SymbolKind::Function, "src/lib.rs"),
                symbol("demo::run_fast", SymbolKind::Function, "src/lib.rs"),
            ],
            edges: vec![Edge {
                src: "demo::run_fast".into(),
                dst: "demo::run".into(),
                kind: EdgeKind::Calls,
                confidence: Confidence::High,
            }],
            external_uses: vec![ExternalUse {
                file: "src/lib.rs".into(),
                krate: "serde".into(),
            }],
            files_with_errors: vec![],
        }
    }

    #[test]
    fn round_trips_facts() {
        let mut index = Index::open_in_memory().unwrap();
        let facts = facts();
        index.replace_facts(&facts).unwrap();
        // Writing twice must replace, not duplicate.
        index.replace_facts(&facts).unwrap();
        assert_eq!(index.files().unwrap(), facts.files);
        assert_eq!(index.symbols().unwrap(), facts.symbols);
        assert_eq!(index.edges().unwrap(), facts.edges);
        assert_eq!(index.crates().unwrap(), facts.crates);
        assert_eq!(index.external_uses().unwrap(), facts.external_uses);
        assert_eq!(index.symbol("demo::run").unwrap().unwrap().name, "run");
        assert!(index.symbol("demo::nope").unwrap().is_none());
        assert_eq!(index.symbols_in_file("src/lib.rs").unwrap().len(), 3);
        assert_eq!(index.edges_to("demo::run").unwrap().len(), 1);
        assert_eq!(index.edges_from("demo::run").unwrap().len(), 0);
    }

    #[test]
    fn finds_symbols_exact_name_first() {
        let mut index = Index::open_in_memory().unwrap();
        index.replace_facts(&facts()).unwrap();
        let hits = index.find_symbols("run", 10).unwrap();
        assert_eq!(hits[0].qualified, "demo::run");
        assert_eq!(hits.len(), 2);
        // `_` is a literal, not a wildcard.
        assert_eq!(index.find_symbols("run_", 10).unwrap().len(), 1);
    }

    #[test]
    fn stores_meta_and_metrics() {
        let mut index = Index::open_in_memory().unwrap();
        assert_eq!(index.meta("commit").unwrap(), None);
        index.set_meta("commit", "abc").unwrap();
        index.set_meta("commit", "def").unwrap();
        assert_eq!(index.meta("commit").unwrap().as_deref(), Some("def"));
        let m = NodeMetrics {
            pagerank: 0.5,
            betweenness: 2.0,
            scc: 1,
            community: 3,
            x: 1.5,
            y: -2.5,
        };
        index.replace_metrics(&[("demo::run".into(), m)]).unwrap();
        assert_eq!(index.metrics().unwrap(), vec![("demo::run".to_string(), m)]);
    }

    #[test]
    fn reopening_a_file_keeps_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("index.db");
        {
            let mut index = Index::open(&path).unwrap();
            index.replace_facts(&facts()).unwrap();
        }
        let index = Index::open(&path).unwrap();
        assert_eq!(index.symbols().unwrap().len(), 3);
    }
}
