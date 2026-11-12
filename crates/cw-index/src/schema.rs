//! Database schema. Bump `SCHEMA_VERSION` on any change; an index with a
//! different version is dropped and rebuilt rather than migrated.

pub const SCHEMA_VERSION: i64 = 3;

pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS files (
    path     TEXT PRIMARY KEY,
    language TEXT NOT NULL,
    hash     TEXT NOT NULL,
    bytes    INTEGER NOT NULL,
    lines    INTEGER NOT NULL
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS symbols (
    qualified  TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    kind       TEXT NOT NULL,
    visibility TEXT NOT NULL,
    file       TEXT NOT NULL,
    start_line INTEGER NOT NULL,
    end_line   INTEGER NOT NULL,
    signature  TEXT NOT NULL,
    doc        TEXT NOT NULL,
    parent     TEXT,
    attrs      TEXT NOT NULL
) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS symbols_file ON symbols(file);
CREATE INDEX IF NOT EXISTS symbols_name ON symbols(name);
CREATE INDEX IF NOT EXISTS symbols_parent ON symbols(parent);

CREATE TABLE IF NOT EXISTS edges (
    src        TEXT NOT NULL,
    dst        TEXT NOT NULL,
    kind       TEXT NOT NULL,
    confidence TEXT NOT NULL,
    PRIMARY KEY (src, dst, kind)
) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS edges_dst ON edges(dst);

CREATE TABLE IF NOT EXISTS crates (
    name TEXT PRIMARY KEY,
    json TEXT NOT NULL
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS external_uses (
    file  TEXT NOT NULL,
    krate TEXT NOT NULL,
    PRIMARY KEY (file, krate)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS metrics (
    node        TEXT PRIMARY KEY,
    pagerank    REAL NOT NULL,
    betweenness REAL NOT NULL,
    scc         INTEGER NOT NULL,
    community   INTEGER NOT NULL,
    x           REAL NOT NULL,
    y           REAL NOT NULL
) WITHOUT ROWID;

CREATE VIRTUAL TABLE IF NOT EXISTS documents USING fts5(
    kind UNINDEXED,
    key UNINDEXED,
    title,
    body,
    tokenize = "unicode61 tokenchars '_'"
);

CREATE TABLE IF NOT EXISTS vectors (
    kind  TEXT NOT NULL,
    key   TEXT NOT NULL,
    title TEXT NOT NULL,
    hash  TEXT NOT NULL,
    data  BLOB NOT NULL,
    PRIMARY KEY (kind, key)
) WITHOUT ROWID;
"#;

pub const DROP_ALL: &str = r#"
DROP TABLE IF EXISTS meta;
DROP TABLE IF EXISTS files;
DROP TABLE IF EXISTS symbols;
DROP TABLE IF EXISTS edges;
DROP TABLE IF EXISTS crates;
DROP TABLE IF EXISTS external_uses;
DROP TABLE IF EXISTS metrics;
DROP TABLE IF EXISTS documents;
DROP TABLE IF EXISTS vectors;
"#;
