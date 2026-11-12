//! Data model shared between the parser, index, graph and API layers.

use serde::{Deserialize, Serialize};

/// Source language of a file. Languages without their own variant are indexed
/// at file level as `Other`; see [`Language::has_symbol_support`] for the ones
/// whose symbols are extracted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    Python,
    TypeScript,
    JavaScript,
    Java,
    Markdown,
    Toml,
    Json,
    Yaml,
    Other,
}

impl Language {
    /// Detect the language from a repo-relative path.
    pub fn from_path(path: &str) -> Self {
        let ext = path.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
        match ext.to_ascii_lowercase().as_str() {
            "rs" => Self::Rust,
            "py" | "pyi" => Self::Python,
            "ts" | "tsx" | "mts" | "cts" => Self::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => Self::JavaScript,
            "java" => Self::Java,
            "md" | "markdown" => Self::Markdown,
            "toml" => Self::Toml,
            "json" => Self::Json,
            "yml" | "yaml" => Self::Yaml,
            _ => Self::Other,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Java => "java",
            Self::Markdown => "markdown",
            Self::Toml => "toml",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Other => "other",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "rust" => Self::Rust,
            "python" => Self::Python,
            "typescript" => Self::TypeScript,
            "javascript" => Self::JavaScript,
            "java" => Self::Java,
            "markdown" => Self::Markdown,
            "toml" => Self::Toml,
            "json" => Self::Json,
            "yaml" => Self::Yaml,
            _ => Self::Other,
        }
    }

    /// True when symbols and edges are extracted for this language.
    pub fn has_symbol_support(self) -> bool {
        matches!(
            self,
            Self::Rust | Self::Python | Self::TypeScript | Self::JavaScript | Self::Java
        )
    }
}

/// One indexed source file. `path` is repo-relative with forward slashes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRecord {
    pub path: String,
    pub language: Language,
    pub hash: String,
    pub bytes: u64,
    pub lines: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Module,
    Function,
    Method,
    Struct,
    Enum,
    Union,
    Trait,
    Impl,
    Const,
    Static,
    TypeAlias,
    Macro,
    /// A class in Python, TypeScript or Java; also a Java record.
    Class,
    /// An interface in TypeScript or Java; also a Java annotation type.
    Interface,
}

impl SymbolKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Function => "function",
            Self::Method => "method",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Union => "union",
            Self::Trait => "trait",
            Self::Impl => "impl",
            Self::Const => "const",
            Self::Static => "static",
            Self::TypeAlias => "type_alias",
            Self::Macro => "macro",
            Self::Class => "class",
            Self::Interface => "interface",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "module" => Self::Module,
            "function" => Self::Function,
            "method" => Self::Method,
            "struct" => Self::Struct,
            "enum" => Self::Enum,
            "union" => Self::Union,
            "trait" => Self::Trait,
            "impl" => Self::Impl,
            "const" => Self::Const,
            "static" => Self::Static,
            "type_alias" => Self::TypeAlias,
            "macro" => Self::Macro,
            "class" => Self::Class,
            "interface" => Self::Interface,
            _ => return None,
        })
    }

    /// Types are the symbols that can earn their own concept note.
    pub fn is_type(self) -> bool {
        matches!(
            self,
            Self::Struct | Self::Enum | Self::Union | Self::Trait | Self::Class | Self::Interface
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    Crate,
    Private,
}

impl Visibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Crate => "crate",
            Self::Private => "private",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "public" => Self::Public,
            "crate" => Self::Crate,
            _ => Self::Private,
        }
    }
}

/// A named code entity. `qualified` is unique within a repository, for example
/// `cw_core::hash::sha256_hex` or `cw_core::model::Language::from_path`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Symbol {
    pub qualified: String,
    pub name: String,
    pub kind: SymbolKind,
    pub visibility: Visibility,
    pub file: String,
    /// 1-based inclusive line range in `file`.
    pub start_line: u32,
    pub end_line: u32,
    /// Declaration text up to the body, whitespace-collapsed.
    pub signature: String,
    /// Leading `///` documentation, markers stripped.
    pub doc: String,
    /// Qualified name of the enclosing symbol (module, impl or trait), if any.
    pub parent: Option<String>,
    /// Names of the attributes on the item, for example `derive`, `test`,
    /// `tokio::main`. Attribute macros can make an item reachable without a
    /// visible caller.
    #[serde(default)]
    pub attrs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// File or symbol imports another (`use`).
    Imports,
    /// Function or method calls another.
    Calls,
    /// Type implements a trait.
    Implements,
    /// Symbol mentions a type in its signature or body.
    References,
    /// Module declares a child module (`mod x;`).
    Declares,
}

impl EdgeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Imports => "imports",
            Self::Calls => "calls",
            Self::Implements => "implements",
            Self::References => "references",
            Self::Declares => "declares",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "imports" => Self::Imports,
            "calls" => Self::Calls,
            "implements" => Self::Implements,
            "references" => Self::References,
            "declares" => Self::Declares,
            _ => return None,
        })
    }
}

/// How sure the resolver is that an edge is real. Name resolution runs without
/// a compiler, so every edge carries its confidence and consumers weight by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// Matched by bare name only, several candidates existed.
    Low,
    /// Matched by bare name, single candidate in scope.
    Medium,
    /// Resolved through an explicit path or import.
    High,
}

impl Confidence {
    pub fn weight(self) -> f64 {
        match self {
            Self::Low => 0.25,
            Self::Medium => 0.6,
            Self::High => 1.0,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "high" => Self::High,
            "medium" => Self::Medium,
            _ => Self::Low,
        }
    }
}

/// A directed relation between two nodes. A node is a symbol's qualified name,
/// or a repo-relative file path prefixed with `file:` for file-level nodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Edge {
    pub src: String,
    pub dst: String,
    pub kind: EdgeKind,
    pub confidence: Confidence,
}

/// Node id for a file-level node.
pub fn file_node(path: &str) -> String {
    format!("file:{path}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_detection() {
        assert_eq!(Language::from_path("src/lib.rs"), Language::Rust);
        assert_eq!(Language::from_path("web/App.TSX"), Language::TypeScript);
        assert_eq!(Language::from_path("Makefile"), Language::Other);
        assert!(Language::Rust.has_symbol_support());
        assert!(Language::Python.has_symbol_support());
        assert!(Language::from_path("web/App.TSX").has_symbol_support());
        assert!(Language::Java.has_symbol_support());
        assert!(Language::JavaScript.has_symbol_support());
    }

    #[test]
    fn enum_text_round_trips() {
        for kind in [
            SymbolKind::Module,
            SymbolKind::TypeAlias,
            SymbolKind::Macro,
            SymbolKind::Class,
            SymbolKind::Interface,
        ] {
            assert_eq!(SymbolKind::parse(kind.as_str()), Some(kind));
        }
        assert!(SymbolKind::Class.is_type() && SymbolKind::Interface.is_type());
        for kind in [EdgeKind::Imports, EdgeKind::Declares] {
            assert_eq!(EdgeKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(
            Confidence::parse(Confidence::Medium.as_str()),
            Confidence::Medium
        );
        assert!(Confidence::High > Confidence::Low);
    }
}
