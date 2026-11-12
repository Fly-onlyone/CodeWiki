//! Parser and static evaluator for OpenUI Lang.
//!
//! OpenUI Lang (<https://github.com/thesysdev/openui>, MIT) is a compact
//! language in which a model describes a user interface as a list of
//! `name = Component(args)` statements. This crate is an independent Rust
//! implementation of the language's syntax, written against the published 1.0
//! specification and the behavior of the reference parser. It has no UI
//! dependencies: rendering the resulting tree is the caller's job.
//!
//! ```
//! use openui_lang::{Kind, Library, Node, evaluate, parse};
//!
//! let library = Library::new(Some("Card"))
//!     .component("Card", &[("children", Kind::List, true)])
//!     .component("TextContent", &[("text", Kind::Text, true)]);
//! let document = evaluate(&parse("root = Card([t])\nt = TextContent(\"Hi\")"), &library);
//! assert!(document.is_valid());
//! assert!(matches!(document.root, Some(Node::Element { .. })));
//! ```

pub mod eval;
pub mod lexer;
pub mod parser;

pub use eval::{Component, Document, Kind, LangError, Library, Node, Param, Severity, evaluate};
pub use parser::{Expr, Program, Statement, parse};
