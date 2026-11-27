//! C and C++ symbol and fact extraction, one walker for both languages.
//!
//! Written against the node kinds and field names published in the
//! grammars' own `node-types.json` (tree-sitter-c 0.24 and tree-sitter-cpp
//! 0.23). No query files are used.
//!
//! One walker serves both languages because the C++ grammar is the C one
//! plus more node kinds: every C construct has the same shape in both, and
//! the walk simply never meets the C++ kinds in a C file. The grammar is
//! chosen by `CInput::cpp`.
//!
//! # What a module is
//!
//! There are no packages. A module is the header/source pair of one name:
//! `src/net/socket.h` and `src/net/socket.c` (or `.cpp`) are the module
//! `net::socket`, the declaration in the header and the definition in the
//! source one symbol (the scan merges them; see `scan.rs`). A header with no
//! source, or a source with no header, is a module of its own. The scan
//! names the module and tells the parser, because pairing needs the other
//! files. Namespaces are not modules: a namespace spans many files, and in
//! most projects there is one. A symbol records the namespace it was
//! declared in as the attribute `namespace:a::b`, and a qualified name such
//! as `a::b::f()` is matched by the resolver against it.
//!
//! # Names
//!
//! A symbol is `<module>::<Name>`; a member is `<module>::<Class>::<name>`
//! and an enumerator `<module>::<Enum>::<NAME>`. An out-of-line definition
//! `void Foo::bar() {}` is a method of `Foo` in the module of the file that
//! defines it; the scan fixes the class chain once it knows the classes.
//!
//! # What is kept out
//!
//! Forward declarations of types (`struct Foo;`) name something another
//! header owns. A specialization is not a symbol: its body is walked for the
//! calls it makes, on behalf of the primary template. Include guards and
//! empty `#define`s are no symbols; an object-like macro with a value is a
//! constant and a function-like macro is a macro. Code under `#if` and
//! `#ifdef` is parsed in every branch, and a name declared in several is
//! kept once by the scan. Conditions are never evaluated.

use std::collections::HashSet;

use cw_core::{Error, Result, SymbolKind, Visibility};
use tree_sitter::{Node, Parser};

use crate::lang::{Sink, block_doc, collapse, field_all, named};
use crate::rust::ParsedFile;

/// What an `#include` names, as the scan decides it from the files of the
/// repository and what its build files declare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Include {
    /// A header of the repository: the module that header belongs to.
    Repo(Vec<String>),
    /// A library the repository declares (CMake `find_package`, vcpkg,
    /// Conan): its name, and what the include takes from it.
    Library { package: String, name: String },
    /// The platform, or something the repository does not say anything about.
    Unknown,
}

/// What the scan tells the parser about one file.
pub struct CInput<'a> {
    /// The module the file belongs to (the header/source pair).
    pub module: &'a str,
    /// Parse with the C++ grammar.
    pub cpp: bool,
    /// Decides an `#include "x.h"` (`quoted`) or `#include <x.h>`.
    pub include: &'a dyn Fn(&str, bool) -> Include,
}

/// Attribute of a symbol that is only declared where it stands: a prototype,
/// an `extern` variable, an opaque typedef. The scan prefers the part that
/// defines it.
pub const DECLARATION_ATTR: &str = "declaration";
/// Attribute prefix: the namespace a symbol was declared in.
pub const NAMESPACE_ATTR: &str = "namespace:";
/// Attribute prefix, set by the parser and removed by the scan: the class an
/// out-of-line definition names before its name (`Foo` of `Foo::bar`).
pub const OUTLINE_ATTR: &str = "outline:";
/// Attribute prefix: the file that defines a symbol a header declares.
pub const IMPL_ATTR: &str = "impl:";
/// Attribute of a test: `test:<framework>`, with the test's own name in
/// `test-name:<name>` and the group it belongs to in `test-suite:<suite>`.
pub const TEST_ATTR: &str = "test";
pub const TEST_FRAMEWORK_ATTR: &str = "test-framework:";
pub const TEST_NAME_ATTR: &str = "test-name:";
pub const TEST_SUITE_ATTR: &str = "test-suite:";

/// Macros of the test frameworks that the grammar reads as a function
/// definition: `TEST(Suite, Name) { ... }`. The framework and whether the
/// first argument is a group (fixture, suite) or the name itself.
const FUNCTION_TESTS: [(&str, &str); 7] = [
    ("TEST", "gtest"),
    ("TEST_F", "gtest"),
    ("TEST_P", "gtest"),
    ("TYPED_TEST", "gtest"),
    ("TYPED_TEST_P", "gtest"),
    ("BOOST_AUTO_TEST_CASE", "boost"),
    ("BOOST_FIXTURE_TEST_CASE", "boost"),
];

/// Macros the grammar reads as a call followed by a block: `TEST_CASE("x")
/// { ... }`. They stand in the statement position, outside any function.
const BLOCK_TESTS: [&str; 3] = ["TEST_CASE", "SCENARIO", "TEST_CASE_METHOD"];

/// The entry points a program starts at.
pub const ENTRY_NAMES: [&str; 4] = ["main", "wmain", "WinMain", "wWinMain"];

/// Parse one C or C++ source file.
pub fn parse_c(file: &str, source: &str, input: &CInput) -> Result<ParsedFile> {
    let mut parser = Parser::new();
    let (language, label) = if input.cpp {
        (tree_sitter_cpp::LANGUAGE.into(), "C++")
    } else {
        (tree_sitter_c::LANGUAGE.into(), "C")
    };
    parser
        .set_language(&language)
        .map_err(|e| Error::external(format!("loading the {label} grammar"), e))?;
    // A macro that stands alone on its line (`FT_BEGIN_HEADER`,
    // `BSSL_NAMESPACE_BEGIN`) has no semicolon, and the grammar then reads
    // the declaration after it as part of it. Those lines are blanked in a
    // copy of the source, byte for byte, so that every position still stands.
    let blanked = blank_statement_macros(source);
    let source = blanked.as_deref().unwrap_or(source);
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| Error::invalid(format!("parser produced no tree for {file}")))?;
    let root = tree.root_node();
    let mut ex = Extractor {
        sink: Sink::new(file, source, input.module, root.has_error()),
        input,
    };
    let doc = file_doc(source);
    let name = file.rsplit('/').next().unwrap_or(file);
    ex.sink
        .module_symbol(name.to_string(), doc, 1, source.lines().count() as u32);
    let ctx = Ctx {
        scope: input.module.to_string(),
        class: None,
        namespace: Vec::new(),
        anonymous: false,
        access: Access::Public,
        in_class: false,
        generics: HashSet::new(),
    };
    ex.items(root, &ctx);
    Ok(ex.sink.out)
}

/// The source with the macros that the grammar cannot read set aside, byte
/// for byte, or `None` when there is none.
///
/// - A macro that stands alone on its line (`FT_BEGIN_HEADER`,
///   `BSSL_NAMESPACE_BEGIN`) has no semicolon, and the grammar then reads the
///   declaration after it as part of it: the line is blanked.
/// - A macro that wraps a return type (`FT_EXPORT( FT_Error ) name( ... )`)
///   is read as a statement followed by junk: its name and parentheses are
///   blanked, and the type stays.
///
/// Only a line that stands between declarations is touched: what comes
/// before it ends a statement, a block or a preprocessor line, and what
/// comes after it does not close or continue a list.
fn blank_statement_macros(source: &str) -> Option<String> {
    fn macro_name(line: &str) -> Option<&str> {
        let name_len = line
            .chars()
            .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
            .count();
        let name = &line[..name_len];
        // The whole word, not the capitals at the start of `FT_Int`.
        if line[name_len..].starts_with(|c: char| c.is_alphanumeric() || c == '_') {
            return None;
        }
        (name.len() >= 4 && name.contains('_') && !name.starts_with(|c: char| c.is_ascii_digit()))
            .then_some(name)
    }
    /// The text inside a pair of parentheses that opens `text` and closes
    /// before it ends, and what follows the closing one.
    fn arguments(text: &str) -> Option<(&str, &str)> {
        let inner_start = text.strip_prefix('(')?;
        let mut depth = 1i32;
        for (i, c) in inner_start.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((&inner_start[..i], &inner_start[i + 1..]));
                    }
                }
                '{' | '}' | ';' => return None,
                _ => {}
            }
        }
        None
    }
    fn is_comment(line: &str) -> bool {
        line.starts_with("//")
            || line.starts_with("/*")
            || line.starts_with('*')
            || line.ends_with("*/")
    }
    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    let mut edits: Vec<(usize, String)> = Vec::new();
    // The lines already blanked: for what comes after them, they are not there.
    let mut gone = vec![false; lines.len()];
    for (i, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        let Some(name) = macro_name(line) else {
            continue;
        };
        let rest = line[name.len()..].trim_start();
        let before = lines[..i]
            .iter()
            .enumerate()
            .rev()
            .filter(|(at, _)| !gone[*at])
            .map(|(_, l)| l.trim())
            .find(|l| !l.is_empty() && !is_comment(l));
        let before_ok = before.is_none_or(|l| {
            l.starts_with('#') || l.ends_with(';') || l.ends_with('}') || l.ends_with('{')
        });
        let after = lines[i + 1..]
            .iter()
            .map(|l| l.trim())
            .find(|l| !l.is_empty() && !is_comment(l));
        if !before_ok {
            continue;
        }
        let indent = raw.len() - raw.trim_start().len();
        let width = raw.trim_end_matches(['\r', '\n']).len();
        // A lone macro. One with an argument list, whose next line is a name
        // followed at once by its parameters, wraps a type: `FT_LOCAL( int )`.
        let names_a_function = |l: &str| {
            let word = l
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .count();
            word > 0
                && !l.starts_with(|c: char| c.is_ascii_digit())
                && l[word..].trim_start().starts_with('(')
        };
        let alone = rest.is_empty()
            || (arguments(rest).is_some_and(|(_, tail)| tail.trim().is_empty())
                && !after.is_some_and(names_a_function));
        if alone {
            let continues = after.is_some_and(|l| {
                l.starts_with(['}', ')', ',', '.', '=', ']', '|', '&', '+', '-', '?', ':'])
            });
            if !continues {
                let mut blank = " ".repeat(width);
                blank.push_str(&raw[width..]);
                edits.push((i, blank));
                gone[i] = true;
            }
            continue;
        }
        // A macro around a return type, with the name of the function after
        // it on the line or the next.
        let Some((inner, tail)) = arguments(rest) else {
            continue;
        };
        if inner.contains(',') || inner.trim().is_empty() {
            continue;
        }
        let tail = tail.trim_start();
        let starts_name = |t: &str| t.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_');
        let next_is_name = if tail.is_empty() {
            after.is_some_and(starts_name)
        } else {
            starts_name(tail)
        };
        if !next_is_name {
            continue;
        }
        // `NAME` and its parentheses become spaces; the type stays.
        let open_at =
            indent + name.len() + (rest.as_ptr() as usize - line[name.len()..].as_ptr() as usize);
        let close_at = open_at + 1 + inner.len();
        let mut bytes = raw.as_bytes().to_vec();
        for byte in bytes.iter_mut().take(open_at + 1).skip(indent) {
            *byte = b' ';
        }
        bytes[close_at] = b' ';
        if let Ok(text) = String::from_utf8(bytes) {
            edits.push((i, text));
        }
    }
    if edits.is_empty() {
        return None;
    }
    let mut text = String::with_capacity(source.len());
    let mut next = edits.iter().peekable();
    for (i, line) in lines.iter().enumerate() {
        match next.peek() {
            Some((at, replaced)) if *at == i => {
                text.push_str(replaced);
                next.next();
            }
            _ => text.push_str(line),
        }
    }
    Some(text)
}

/// A leading `//!` block or a `/*! */` or `/** @file */` comment: the
/// documentation of the file. Other comments at the top are mostly licenses,
/// and a `/** */` comment is the documentation of the declaration below it.
fn file_doc(source: &str) -> String {
    let head = source.trim_start_matches('\u{feff}').trim_start();
    if head.starts_with("//!") {
        return head
            .lines()
            .take_while(|l| l.trim_start().starts_with("//!"))
            .map(|l| {
                let l = l.trim_start()[3..].to_string();
                l.strip_prefix(' ').map(str::to_string).unwrap_or(l)
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();
    }
    if let Some(rest) = head
        .strip_prefix("/*!")
        .or_else(|| head.strip_prefix("/**"))
        && let Some(end) = rest.find("*/")
    {
        let text = &rest[..end];
        let wanted = head.starts_with("/*!") || text.contains("@file") || text.contains("\\file");
        if wanted {
            return clean_block(text);
        }
    }
    String::new()
}

fn clean_block(inner: &str) -> String {
    inner
        .lines()
        .map(|l| {
            let l = l.trim();
            let l = l.strip_prefix('*').unwrap_or(l);
            l.strip_prefix(' ').unwrap_or(l).trim_end()
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Access {
    Public,
    Protected,
    Private,
}

#[derive(Clone)]
struct Ctx {
    /// Qualified name new symbols nest under: the module or the class.
    scope: String,
    /// Qualified name of the enclosing class, for `this->`.
    class: Option<String>,
    namespace: Vec<String>,
    /// Inside an anonymous namespace: private to the file.
    anonymous: bool,
    /// Access of the members now being read.
    access: Access,
    in_class: bool,
    /// Names of template parameters in reach: not types of the repository.
    generics: HashSet<String>,
}

struct Extractor<'a, 'i> {
    sink: Sink<'a>,
    input: &'i CInput<'i>,
}

/// What a declarator names.
struct Declared<'t> {
    name: String,
    /// Scope written before the name: `Foo` and `ns` of `ns::Foo::bar`.
    qualifier: Vec<String>,
    /// The `function_declarator`, when the name is a function's.
    function: Option<Node<'t>>,
    /// `f<int>` or `Foo<T*>`: the name of a specialization.
    specialization: bool,
    /// A destructor, a constructor-like name, a conversion or an operator.
    special: bool,
    _node: std::marker::PhantomData<Node<'t>>,
}

fn is_macro_name(name: &str) -> bool {
    name.len() >= 2
        && name.chars().any(|c| c.is_ascii_uppercase())
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// A name that by convention is a constant: `MAX_SIZE`, `kDefault`.
fn is_constant_name(name: &str) -> bool {
    is_macro_name(name)
        || (name.len() > 1
            && name.starts_with('k')
            && name[1..].chars().next().is_some_and(char::is_uppercase))
}

impl<'a> Extractor<'a, '_> {
    fn text(&self, node: Node) -> &'a str {
        self.sink.text(node)
    }

    // ----- comments -----

    /// The comment lines directly above `node`, markers stripped. Plain `//`
    /// and `/* */` comments count: C code seldom has any other kind.
    fn doc(&self, node: Node) -> String {
        let mut parts: Vec<String> = Vec::new();
        let mut next_row = node.start_position().row;
        let mut current = node.prev_sibling();
        while let Some(prev) = current {
            if prev.kind() != "comment" || prev.end_position().row + 1 < next_row {
                break;
            }
            // A comment on the line of the previous declaration belongs to it.
            if let Some(before) = prev.prev_sibling()
                && before.kind() != "comment"
                && before.end_position().row == prev.start_position().row
            {
                break;
            }
            parts.push(comment_text(self.text(prev)));
            next_row = prev.start_position().row;
            current = prev.prev_sibling();
        }
        parts.reverse();
        let text = parts.join("\n").trim().to_string();
        let lower = text.to_ascii_lowercase();
        // A line of dashes, a license, a banner: not a description.
        if !text.chars().any(char::is_alphanumeric)
            || lower.contains("copyright")
            || lower.contains("license")
            || lower.contains("spdx-")
        {
            return String::new();
        }
        text
    }

    // ----- includes -----

    fn include(&mut self, node: Node) {
        let Some(path) = node.child_by_field_name("path") else {
            return;
        };
        let (spec, quoted) = match path.kind() {
            "string_literal" => (self.text(path).trim_matches('"').to_string(), true),
            "system_lib_string" => (
                self.text(path)
                    .trim_start_matches('<')
                    .trim_end_matches('>')
                    .to_string(),
                false,
            ),
            _ => return,
        };
        match (self.input.include)(&spec, quoted) {
            Include::Repo(module) => self.sink.import(module, "", true, false),
            Include::Library { package, name } => self.sink.import_external(&package, &name),
            Include::Unknown => {}
        }
    }

    // ----- macros -----

    fn define(&mut self, node: Node, ctx: &Ctx) {
        let Some(name_node) = node.child_by_field_name("name") else {
            return;
        };
        let name = self.text(name_node);
        let function = node.kind() == "preproc_function_def";
        let value = node
            .child_by_field_name("value")
            .map(|v| collapse(&self.text(v).replace("\\\r\n", " ").replace("\\\n", " ")))
            .unwrap_or_default();
        // An include guard or a flag: `#define FOO_H`, `#define HAVE_X`.
        if !function && (value.is_empty() || value == name) {
            return;
        }
        let params = node
            .child_by_field_name("parameters")
            .map(|p| self.text(p))
            .unwrap_or("");
        let signature = collapse(&format!("#define {name}{params} {value}"));
        let doc = self.doc(node);
        let kind = if function {
            SymbolKind::Macro
        } else {
            SymbolKind::Const
        };
        let symbol = self.sink.symbol(&ctx.scope, name, kind, node);
        symbol.signature = signature;
        symbol.doc = doc;
        symbol.attrs.push("macro".to_string());
        if !ctx.namespace.is_empty() {
            symbol.attrs.push(namespace_attr(&ctx.namespace));
        }
        let qualified = symbol.qualified.clone();
        // A macro's value mentions names too: `#define SIZE (sizeof(struct foo))`.
        if let Some(value) = node.child_by_field_name("value") {
            self.names_in_text(self.text(value), &qualified);
        }
    }

    /// The upper-case names in a macro's text: constants and other macros.
    fn names_in_text(&mut self, text: &str, owner: &str) {
        let mut word = String::new();
        let mut words: Vec<String> = Vec::new();
        for c in text.chars().chain(std::iter::once(' ')) {
            if c.is_alphanumeric() || c == '_' {
                word.push(c);
            } else if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        }
        for word in words {
            if is_macro_name(&word) {
                self.sink.value(owner, vec![word]);
            }
        }
    }

    // ----- items -----

    /// The declarations that stand directly inside `parent`: a file, a
    /// namespace, a linkage block or a branch of the preprocessor.
    fn items(&mut self, parent: Node, ctx: &Ctx) {
        let children = named(parent);
        let mut i = 0;
        while i < children.len() {
            let node = children[i];
            i += 1;
            // `TEST_CASE("x") { ... }` is a call and a block side by side.
            if node.kind() == "expression_statement"
                && let Some(call) = node
                    .named_child(0)
                    .filter(|c| c.kind() == "call_expression")
                && let Some(next) = children.get(i).filter(|n| n.kind() == "compound_statement")
                && self.block_test(node, call, *next, ctx)
            {
                i += 1;
                continue;
            }
            self.item(node, ctx);
        }
    }

    fn item(&mut self, node: Node, ctx: &Ctx) {
        match node.kind() {
            "preproc_include" => self.include(node),
            "preproc_def" | "preproc_function_def" => self.define(node, ctx),
            "preproc_if" | "preproc_ifdef" | "preproc_elif" | "preproc_elifdef"
            | "preproc_else" => {
                // Every branch is read; no condition is evaluated.
                let condition = node.child_by_field_name("condition").map(|c| c.id());
                let children: Vec<Node> = named(node)
                    .into_iter()
                    .filter(|c| Some(c.id()) != condition)
                    .collect();
                let name = node.child_by_field_name("name").map(|c| c.id());
                let alternative = node.child_by_field_name("alternative").map(|c| c.id());
                for child in children {
                    if Some(child.id()) == name {
                        continue;
                    }
                    if Some(child.id()) == alternative {
                        self.item(child, ctx);
                    } else {
                        self.items_of(child, ctx);
                    }
                }
            }
            "namespace_definition" => self.namespace(node, ctx),
            "linkage_specification" => {
                if let Some(body) = node.child_by_field_name("body") {
                    if body.kind() == "declaration_list" {
                        self.items(body, ctx);
                    } else {
                        self.item(body, ctx);
                    }
                }
            }
            "template_declaration" => self.template(node, ctx),
            "function_definition" => self.function(node, node, ctx),
            "declaration" => self.declaration(node, node, ctx),
            "type_definition" => self.typedef(node, ctx),
            "alias_declaration" => self.alias(node, node, ctx),
            "class_specifier" | "struct_specifier" | "union_specifier" | "enum_specifier" => {
                self.type_spec(node, node, ctx, None);
            }
            "ERROR" => {
                // A construct the grammar lost: read what still stands.
                for child in named(node) {
                    self.item(child, ctx);
                }
            }
            _ => {}
        }
    }

    /// One child of a preprocessor branch: a single declaration.
    fn items_of(&mut self, node: Node, ctx: &Ctx) {
        // Consecutive children of a branch may be a test macro and its block.
        if let Some(parent) = node.parent() {
            let siblings = named(parent);
            let at = siblings.iter().position(|s| s.id() == node.id());
            if node.kind() == "expression_statement"
                && let Some(at) = at
                && let Some(call) = node
                    .named_child(0)
                    .filter(|c| c.kind() == "call_expression")
                && let Some(next) = siblings
                    .get(at + 1)
                    .filter(|n| n.kind() == "compound_statement")
                && self.block_test(node, call, *next, ctx)
            {
                return;
            }
            if node.kind() == "compound_statement"
                && let Some(at) = at
                && at > 0
                && siblings[at - 1].kind() == "expression_statement"
                && siblings[at - 1]
                    .named_child(0)
                    .filter(|c| c.kind() == "call_expression")
                    .and_then(|c| c.child_by_field_name("function"))
                    .is_some_and(|f| {
                        BLOCK_TESTS.contains(&self.text(f))
                            || FUNCTION_TESTS.iter().any(|(m, _)| *m == self.text(f))
                    })
            {
                return;
            }
        }
        self.item(node, ctx);
    }

    fn namespace(&mut self, node: Node, ctx: &Ctx) {
        let Some(body) = node.child_by_field_name("body") else {
            return;
        };
        let mut inner = ctx.clone();
        match node.child_by_field_name("name") {
            Some(name) => {
                for part in self.text(name).split("::") {
                    inner.namespace.push(part.trim().to_string());
                }
            }
            None => inner.anonymous = true,
        }
        self.items(body, &inner);
    }

    /// `template <...> declaration`: the declaration inside, as a template.
    fn template(&mut self, node: Node, ctx: &Ctx) {
        let mut inner = ctx.clone();
        let parameters = node.child_by_field_name("parameters");
        let mut specialization = false;
        if let Some(list) = parameters {
            specialization = list.named_child_count() == 0;
            for parameter in named(list) {
                if let Some(name) = self.template_parameter_name(parameter) {
                    inner.generics.insert(name);
                }
            }
        }
        for child in named(node) {
            if Some(child.id()) == parameters.map(|p| p.id()) {
                continue;
            }
            let template = Template {
                node,
                specialization,
            };
            match child.kind() {
                "function_definition" => self.function_in(child, node, &inner, Some(&template)),
                "declaration" => self.declaration_in(child, node, &inner, Some(&template)),
                "alias_declaration" => self.alias(child, node, &inner),
                "class_specifier" | "struct_specifier" | "union_specifier" | "enum_specifier" => {
                    self.type_spec(child, node, &inner, Some(&template));
                }
                "template_declaration" => self.template(child, &inner),
                _ => {}
            }
        }
    }

    fn template_parameter_name(&self, parameter: Node) -> Option<String> {
        for child in named(parameter) {
            if matches!(child.kind(), "type_identifier" | "identifier") {
                return Some(self.text(child).to_string());
            }
        }
        parameter
            .child_by_field_name("declarator")
            .map(|d| self.text(d).to_string())
    }

    // ----- declarators -----

    fn declared<'t>(&self, node: Node<'t>) -> Option<Declared<'t>> {
        match node.kind() {
            "function_declarator" => {
                let inner = node.child_by_field_name("declarator")?;
                let mut declared = self.declared(inner)?;
                // `int (*callback)(int)` is a variable of function-pointer
                // type; `int (*get(void))(int)` is a function returning one.
                if inner.kind() != "parenthesized_declarator" && declared.function.is_none() {
                    declared.function = Some(node);
                }
                Some(declared)
            }
            "pointer_declarator"
            | "reference_declarator"
            | "array_declarator"
            | "parenthesized_declarator"
            | "attributed_declarator"
            | "init_declarator"
            | "abstract_function_declarator" => {
                if let Some(inner) = node.child_by_field_name("declarator") {
                    return self.declared(inner);
                }
                named(node).into_iter().find_map(|c| self.declared(c))
            }
            "identifier" | "field_identifier" | "type_identifier" | "namespace_identifier" => {
                Some(Declared {
                    name: self.text(node).to_string(),
                    qualifier: Vec::new(),
                    function: None,
                    specialization: false,
                    special: false,
                    _node: std::marker::PhantomData,
                })
            }
            "destructor_name" => Some(Declared {
                name: format!("~{}", collapse(&self.text(node)[1..])),
                qualifier: Vec::new(),
                function: None,
                specialization: false,
                special: true,
                _node: std::marker::PhantomData,
            }),
            "operator_name" | "operator_cast" => Some(Declared {
                name: collapse(self.text(node)),
                qualifier: Vec::new(),
                function: None,
                specialization: false,
                special: true,
                _node: std::marker::PhantomData,
            }),
            "template_function" | "template_method" | "template_type" => {
                let mut declared = self.declared(node.child_by_field_name("name")?)?;
                declared.specialization = true;
                Some(declared)
            }
            "qualified_identifier" => {
                let mut qualifier = Vec::new();
                if let Some(scope) = node.child_by_field_name("scope") {
                    qualifier.extend(self.scope_names(scope));
                }
                let mut declared = self.declared(node.child_by_field_name("name")?)?;
                qualifier.append(&mut declared.qualifier);
                declared.qualifier = qualifier;
                // `Foo<T>::bar` names a member of the primary template.
                Some(declared)
            }
            _ => None,
        }
    }

    fn scope_names(&self, scope: Node) -> Vec<String> {
        match scope.kind() {
            "template_type" => scope
                .child_by_field_name("name")
                .map(|n| vec![self.text(n).to_string()])
                .unwrap_or_default(),
            "namespace_identifier" | "type_identifier" | "identifier" => {
                vec![self.text(scope).to_string()]
            }
            _ => Vec::new(),
        }
    }

    // ----- visibility, signatures -----

    fn is_static(&self, node: Node) -> bool {
        named(node)
            .into_iter()
            .any(|c| c.kind() == "storage_class_specifier" && self.text(c) == "static")
    }

    fn has_qualifier(&self, node: Node, word: &str) -> bool {
        named(node).into_iter().any(|c| {
            matches!(c.kind(), "type_qualifier" | "storage_class_specifier") && self.text(c) == word
        })
    }

    fn visibility(&self, node: Node, ctx: &Ctx) -> Visibility {
        if ctx.in_class {
            return match ctx.access {
                Access::Public => Visibility::Public,
                Access::Protected => Visibility::Crate,
                Access::Private => Visibility::Private,
            };
        }
        if ctx.anonymous || self.is_static(node) {
            Visibility::Private
        } else {
            Visibility::Public
        }
    }

    fn finish(&self, symbol: &mut cw_core::Symbol, ctx: &Ctx, template: Option<&Template>) {
        if !ctx.namespace.is_empty() {
            symbol.attrs.push(namespace_attr(&ctx.namespace));
        }
        if template.is_some() {
            symbol.attrs.push("template".to_string());
        }
    }

    // ----- functions -----

    fn function(&mut self, node: Node, outer: Node, ctx: &Ctx) {
        self.function_in(node, outer, ctx, None);
    }

    /// A function definition, a member's or a free one's.
    fn function_in(&mut self, node: Node, outer: Node, ctx: &Ctx, template: Option<&Template>) {
        let Some(declarator) = node.child_by_field_name("declarator") else {
            return;
        };
        let Some(declared) = self.declared(declarator) else {
            return;
        };
        let body = node.child_by_field_name("body");
        let has_type = node.child_by_field_name("type").is_some();
        // `TEST(Suite, Name) { ... }`, a macro the grammar reads as a function.
        if !ctx.in_class
            && !has_type
            && let Some((_, framework)) = FUNCTION_TESTS.iter().find(|(m, _)| *m == declared.name)
            && let Some(function) = declared.function
        {
            return self.function_test(node, function, framework, &declared.name, ctx);
        }
        let Some(function) = declared.function.or_else(|| {
            // `operator bool() const { ... }` has no function declarator.
            (declared.special && declarator.kind() == "operator_cast").then_some(declarator)
        }) else {
            return;
        };
        // `DECLARE_THING(x) { ... }`: a macro, not a function.
        if !has_type
            && !declared.special
            && declared.qualifier.is_empty()
            && !ctx.in_class
            && is_macro_name(&declared.name)
        {
            if let Some(body) = body {
                self.walk(body, &ctx.scope, ctx);
            }
            return;
        }
        let end = [
            body.map(|b| b.start_byte()),
            named(node)
                .into_iter()
                .find(|c| c.kind() == "field_initializer_list")
                .map(|c| c.start_byte()),
        ]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or_else(|| node.end_byte());
        let signature = self.signature(outer, end);
        let doc = self.doc(outer);
        let definition = body.is_some()
            || named(node)
                .iter()
                .any(|c| c.kind() == "default_method_clause");
        self.callable(
            node,
            &declared,
            function,
            signature,
            doc,
            !definition,
            ctx,
            template,
        );
        let owner = self.owner_of(&declared, ctx);
        if declared.specialization {
            // A specialization names nothing new; its calls belong to the
            // primary.
            if let Some(body) = body {
                self.walk(body, &owner, ctx);
            }
            return;
        }
        let mut inner = ctx.clone();
        inner.class = self.class_of(&declared, ctx);
        for child in named(node) {
            if Some(child.id()) == body.map(|b| b.id()) || child.kind() == "comment" {
                continue;
            }
            self.walk(child, &owner, &inner);
        }
        if let Some(body) = body {
            self.walk(body, &owner, &inner);
        }
    }

    /// The qualified name of the symbol a function declarator makes.
    fn owner_of(&self, declared: &Declared, ctx: &Ctx) -> String {
        match declared.qualifier.last() {
            Some(class) if !ctx.in_class => {
                format!("{}::{class}::{}", ctx.scope, declared.name)
            }
            _ => format!("{}::{}", ctx.scope, declared.name),
        }
    }

    fn class_of(&self, declared: &Declared, ctx: &Ctx) -> Option<String> {
        match declared.qualifier.last() {
            Some(class) if !ctx.in_class => Some(format!("{}::{class}", ctx.scope)),
            _ => ctx.class.clone(),
        }
    }

    /// Record a function or method symbol.
    #[allow(clippy::too_many_arguments)]
    fn callable(
        &mut self,
        node: Node,
        declared: &Declared,
        function: Node,
        signature: String,
        doc: String,
        only_declared: bool,
        ctx: &Ctx,
        template: Option<&Template>,
    ) {
        if declared.specialization || declared.name.is_empty() {
            return;
        }
        let _ = function;
        let outline = declared.qualifier.last().filter(|_| !ctx.in_class);
        let method = ctx.in_class || outline.is_some();
        let scope = match outline {
            Some(class) => format!("{}::{class}", ctx.scope),
            None => ctx.scope.clone(),
        };
        let kind = if method {
            SymbolKind::Method
        } else {
            SymbolKind::Function
        };
        let mut visibility = self.visibility(node, ctx);
        if outline.is_some() {
            // Whatever the class says about it; the scan reads the access
            // from the declaration in the header.
            visibility = Visibility::Public;
        }
        // `virtual` is a keyword the grammar leaves unnamed.
        let virtual_ = (0..node.child_count())
            .filter_map(|i| node.child(i))
            .any(|c| c.kind() == "virtual");
        let mut attrs = Vec::new();
        if only_declared {
            attrs.push(DECLARATION_ATTR.to_string());
        }
        if declared.special {
            attrs.push(if declared.name.starts_with('~') {
                "destructor".to_string()
            } else if declared.name.starts_with("operator") {
                "operator".to_string()
            } else {
                "special".to_string()
            });
        }
        if virtual_ {
            attrs.push("virtual".to_string());
        }
        if let Some(class) = outline {
            attrs.push(format!("{OUTLINE_ATTR}{class}"));
        }
        let name = declared.name.clone();
        let symbol = self.sink.symbol(&scope, &name, kind, node);
        symbol.visibility = visibility;
        symbol.signature = signature;
        symbol.doc = doc;
        symbol.attrs = attrs;
        // The span of a member written inside a template covers the template.
        if let Some(template) = template {
            symbol.start_line = template.node.start_position().row as u32 + 1;
        }
        let mut copy = symbol.clone();
        self.finish(&mut copy, ctx, template);
        let last = self.sink.out.symbols.len() - 1;
        self.sink.out.symbols[last] = copy;
    }

    fn signature(&self, outer: Node, end: usize) -> String {
        collapse(self.sink.between(outer.start_byte(), end))
    }

    fn function_test(
        &mut self,
        node: Node,
        function: Node,
        framework: &str,
        macro_name: &str,
        ctx: &Ctx,
    ) {
        let params: Vec<String> = function
            .child_by_field_name("parameters")
            .map(|p| {
                named(p)
                    .into_iter()
                    .map(|c| collapse(self.text(c)))
                    .collect()
            })
            .unwrap_or_default();
        let body = node.child_by_field_name("body");
        let end = body.map_or(node.end_byte(), |b| b.start_byte());
        let signature = collapse(self.sink.between(node.start_byte(), end));
        self.macro_test(node, macro_name, framework, &params, signature, body, ctx);
    }

    /// A test macro written with plain arguments: `TEST(Suite, Name)`.
    #[allow(clippy::too_many_arguments)]
    fn macro_test(
        &mut self,
        node: Node,
        macro_name: &str,
        framework: &str,
        params: &[String],
        signature: String,
        body: Option<Node>,
        ctx: &Ctx,
    ) {
        let (suite, name) = match (macro_name, params) {
            ("BOOST_AUTO_TEST_CASE", [name]) => (String::new(), name.clone()),
            ("BOOST_FIXTURE_TEST_CASE", [name, fixture, ..]) => (fixture.clone(), name.clone()),
            (_, [first, second, ..]) => (first.clone(), second.clone()),
            _ => return,
        };
        let full = if suite.is_empty() || framework == "boost" {
            name.clone()
        } else {
            format!("{suite}.{name}")
        };
        let symbol_name = if suite.is_empty() || framework == "boost" {
            name.clone()
        } else {
            format!("{suite}_{name}")
        };
        self.test_symbol(
            node,
            &symbol_name,
            signature,
            framework,
            &full,
            &suite,
            body,
            ctx,
        );
    }

    /// `TEST_CASE("name", "[tags]") { ... }`: the call is `call`, the body
    /// is `block`. Returns false when the call is no test macro.
    fn block_test(&mut self, node: Node, call: Node, block: Node, ctx: &Ctx) -> bool {
        let Some(function) = call.child_by_field_name("function") else {
            return false;
        };
        let macro_name = self.text(function);
        let Some(arguments) = call.child_by_field_name("arguments") else {
            return false;
        };
        // The C grammar reads `TEST(Suite, Name) { ... }` as a call and a
        // block, as it does `TEST_CASE`.
        if let Some((_, framework)) = FUNCTION_TESTS.iter().find(|(m, _)| *m == macro_name) {
            let params: Vec<String> = named(arguments)
                .into_iter()
                .map(|a| collapse(self.text(a)))
                .collect();
            let signature = collapse(self.sink.between(node.start_byte(), call.end_byte()));
            let before = self.sink.out.symbols.len();
            self.macro_test(
                node,
                macro_name,
                framework,
                &params,
                signature,
                Some(block),
                ctx,
            );
            return self.sink.out.symbols.len() > before;
        }
        if !BLOCK_TESTS.contains(&macro_name) {
            return false;
        }
        let args = named(arguments);
        // `TEST_CASE_METHOD(Fixture, "name")` names the case second.
        let (suite, name_node) = if macro_name == "TEST_CASE_METHOD" {
            (
                args.first().map(|a| self.text(*a).to_string()),
                args.get(1).copied(),
            )
        } else {
            (None, args.first().copied())
        };
        let Some(name_node) = name_node.filter(|n| n.kind() == "string_literal") else {
            return false;
        };
        let name = self.text(name_node).trim_matches('"').to_string();
        let symbol_name = sanitize_test_name(&name);
        if symbol_name.is_empty() {
            return false;
        }
        let framework = "catch";
        let signature = collapse(self.sink.between(node.start_byte(), call.end_byte()));
        let suite = suite.unwrap_or_default();
        self.test_symbol(
            node,
            &symbol_name,
            signature,
            framework,
            &name,
            &suite,
            Some(block),
            ctx,
        );
        true
    }

    #[allow(clippy::too_many_arguments)]
    fn test_symbol(
        &mut self,
        node: Node,
        symbol_name: &str,
        signature: String,
        framework: &str,
        full: &str,
        suite: &str,
        body: Option<Node>,
        ctx: &Ctx,
    ) {
        let end = body.map_or(node.end_position().row, |b| b.end_position().row);
        let symbol = self
            .sink
            .symbol(&ctx.scope, symbol_name, SymbolKind::Function, node);
        symbol.end_line = end as u32 + 1;
        symbol.signature = signature;
        symbol.attrs = vec![
            TEST_ATTR.to_string(),
            format!("{TEST_FRAMEWORK_ATTR}{framework}"),
            format!("{TEST_NAME_ATTR}{full}"),
        ];
        if !suite.is_empty() {
            symbol.attrs.push(format!("{TEST_SUITE_ATTR}{suite}"));
        }
        symbol.visibility = Visibility::Private;
        let owner = symbol.qualified.clone();
        if let Some(body) = body {
            self.walk(body, &owner, ctx);
        }
    }

    // ----- declarations -----

    fn declaration(&mut self, node: Node, outer: Node, ctx: &Ctx) {
        self.declaration_in(node, outer, ctx, None);
    }

    /// A declaration: prototypes, variables, constants, and the types they
    /// declare along the way.
    fn declaration_in(&mut self, node: Node, outer: Node, ctx: &Ctx, template: Option<&Template>) {
        let type_node = node.child_by_field_name("type");
        let constant = self.has_qualifier(node, "const") || self.has_qualifier(node, "constexpr");
        let declarators = field_all(node, "declarator");
        // `struct Foo { ... } foo;` declares the type as well.
        if let Some(ty) = type_node
            && matches!(
                ty.kind(),
                "class_specifier" | "struct_specifier" | "union_specifier" | "enum_specifier"
            )
        {
            self.type_spec(ty, outer, ctx, template);
        } else if let Some(ty) = type_node {
            self.walk(ty, &ctx.scope, ctx);
        }
        let mut any = false;
        for declarator in declarators {
            let Some(declared) = self.declared(declarator) else {
                continue;
            };
            any = true;
            if let Some(function) = declared.function {
                let signature = self.signature(outer, node.end_byte());
                let doc = self.doc(outer);
                self.callable(
                    node, &declared, function, signature, doc, true, ctx, template,
                );
                let owner = self.owner_of(&declared, ctx);
                // Types in the parameters and the return type.
                if let Some(parameters) = function.child_by_field_name("parameters") {
                    self.walk(parameters, &owner, ctx);
                }
                if let Some(ty) = type_node {
                    self.walk(ty, &owner, ctx);
                }
                continue;
            }
            if declared.specialization || !declared.qualifier.is_empty() && !ctx.in_class {
                // A static member defined outside its class: `int Foo::n = 1;`.
                if let Some(class) = declared.qualifier.last()
                    && !declared.specialization
                {
                    let scope = format!("{}::{class}", ctx.scope);
                    let kind = if constant {
                        SymbolKind::Const
                    } else {
                        SymbolKind::Static
                    };
                    let signature = self.signature(outer, node.end_byte());
                    let symbol = self.sink.symbol(&scope, &declared.name, kind, node);
                    symbol.signature = signature;
                    symbol.attrs.push(format!("{OUTLINE_ATTR}{class}"));
                }
                continue;
            }
            let kind = if constant {
                SymbolKind::Const
            } else {
                SymbolKind::Static
            };
            let extern_only =
                self.has_qualifier(node, "extern") && declarator.kind() != "init_declarator";
            let signature = self.signature(outer, node.end_byte());
            let doc = self.doc(outer);
            let visibility = self.visibility(node, ctx);
            let symbol = self.sink.symbol(&ctx.scope, &declared.name, kind, node);
            symbol.visibility = visibility;
            symbol.signature = signature;
            symbol.doc = doc;
            if extern_only {
                symbol.attrs.push(DECLARATION_ATTR.to_string());
            }
            let owner = symbol.qualified.clone();
            let mut copy = symbol.clone();
            self.finish(&mut copy, ctx, template);
            let last = self.sink.out.symbols.len() - 1;
            self.sink.out.symbols[last] = copy;
            if let Some(ty) = type_node {
                self.walk(ty, &owner, ctx);
            }
            if declarator.kind() == "init_declarator"
                && let Some(value) = declarator.child_by_field_name("value")
            {
                self.walk(value, &owner, ctx);
            }
        }
        if !any && type_node.is_none() {
            // Nothing declared that we know; read it for what it mentions.
            self.walk(node, &ctx.scope, ctx);
        }
    }

    /// `typedef <type> name;`.
    fn typedef(&mut self, node: Node, ctx: &Ctx) {
        let Some(ty) = node.child_by_field_name("type") else {
            return;
        };
        let names: Vec<(String, Node)> = field_all(node, "declarator")
            .into_iter()
            .filter_map(|d| self.declared(d).map(|x| (x.name, d)))
            .collect();
        let is_spec = matches!(
            ty.kind(),
            "class_specifier" | "struct_specifier" | "union_specifier" | "enum_specifier"
        );
        let has_body = is_spec && ty.child_by_field_name("body").is_some();
        let tag = if is_spec {
            ty.child_by_field_name("name")
                .map(|n| self.text(n).to_string())
        } else {
            None
        };
        let mut done_type = false;
        if has_body {
            // `typedef struct { ... } Name;` is the struct `Name`.
            // `typedef struct tag { ... } Name;` is the struct `tag` and an
            // alias, unless the two names are one in all but spelling.
            let same = tag
                .as_deref()
                .zip(names.first())
                .is_some_and(|(tag, (name, _))| same_type_name(tag, name));
            if tag.is_none() || same {
                if let Some((name, _)) = names.first() {
                    let signature = match &tag {
                        Some(tag) => {
                            format!("typedef {} {tag} {{ ... }} {name}", self.keyword_of(ty))
                        }
                        None => format!("typedef {} {{ ... }} {name}", self.keyword_of(ty)),
                    };
                    self.type_spec_named(ty, node, ctx, None, Some(name), signature);
                    done_type = true;
                }
            } else {
                self.type_spec(ty, node, ctx, None);
                done_type = true;
            }
        }
        for (name, declarator) in names {
            if done_type && tag.is_none() {
                continue;
            }
            if done_type && same_type_name(tag.as_deref().unwrap_or(""), &name) {
                continue;
            }
            let signature = collapse(self.text(node));
            let doc = self.doc(node);
            let visibility = if ctx.anonymous {
                Visibility::Private
            } else {
                Visibility::Public
            };
            let symbol = self
                .sink
                .symbol(&ctx.scope, &name, SymbolKind::TypeAlias, node);
            symbol.signature = signature;
            symbol.doc = doc;
            symbol.visibility = visibility;
            if !has_body && is_spec {
                symbol.attrs.push(DECLARATION_ATTR.to_string());
            }
            let owner = symbol.qualified.clone();
            let mut copy = symbol.clone();
            self.finish(&mut copy, ctx, None);
            let last = self.sink.out.symbols.len() - 1;
            self.sink.out.symbols[last] = copy;
            if !has_body {
                self.walk(ty, &owner, ctx);
            }
            if let Some(function) = self.declared(declarator).and_then(|d| d.function)
                && let Some(parameters) = function.child_by_field_name("parameters")
            {
                self.walk(parameters, &owner, ctx);
            }
        }
    }

    fn keyword_of(&self, spec: Node) -> &'static str {
        match spec.kind() {
            "class_specifier" => "class",
            "union_specifier" => "union",
            "enum_specifier" => "enum",
            _ => "struct",
        }
    }

    /// `using Name = type;`
    fn alias(&mut self, node: Node, outer: Node, ctx: &Ctx) {
        let Some(name) = node.child_by_field_name("name") else {
            return;
        };
        let name = self.text(name).to_string();
        let signature = self.signature(outer, node.end_byte());
        let doc = self.doc(outer);
        let visibility = self.visibility(node, ctx);
        let symbol = self
            .sink
            .symbol(&ctx.scope, &name, SymbolKind::TypeAlias, node);
        symbol.signature = signature;
        symbol.doc = doc;
        symbol.visibility = visibility;
        let owner = symbol.qualified.clone();
        let mut copy = symbol.clone();
        self.finish(&mut copy, ctx, None);
        let last = self.sink.out.symbols.len() - 1;
        self.sink.out.symbols[last] = copy;
        if let Some(ty) = node.child_by_field_name("type") {
            self.walk(ty, &owner, ctx);
        }
    }

    // ----- types -----

    fn type_spec(&mut self, node: Node, outer: Node, ctx: &Ctx, template: Option<&Template>) {
        let signature = String::new();
        self.type_spec_named(node, outer, ctx, template, None, signature);
    }

    /// A struct, union, class or enum with a body. `name` replaces the tag
    /// when a typedef gives the type its name.
    fn type_spec_named(
        &mut self,
        node: Node,
        outer: Node,
        ctx: &Ctx,
        template: Option<&Template>,
        name: Option<&str>,
        typedef_signature: String,
    ) {
        let Some(body) = node.child_by_field_name("body") else {
            return;
        };
        let tag_node = node.child_by_field_name("name");
        let is_enum = node.kind() == "enum_specifier";
        let specialization = tag_node.is_some_and(|t| t.kind() == "template_type");
        let written: Option<String> = match (name, tag_node) {
            (Some(name), _) => Some(name.to_string()),
            (None, Some(tag)) => Some(match tag.kind() {
                "template_type" => tag
                    .child_by_field_name("name")
                    .map(|n| self.text(n).to_string())
                    .unwrap_or_default(),
                "qualified_identifier" => self.declared(tag).map(|d| d.name).unwrap_or_default(),
                _ => self.text(tag).to_string(),
            }),
            (None, None) => None,
        };
        // `struct Outer::Inner { ... }` defines a nested type out of line.
        let qualifier = tag_node
            .filter(|t| t.kind() == "qualified_identifier")
            .and_then(|t| self.declared(t))
            .map(|d| d.qualifier)
            .unwrap_or_default();
        let Some(written) = written.filter(|w| !w.is_empty()) else {
            // An anonymous type: its members belong to whatever holds it.
            if is_enum {
                self.enumerators(body, &ctx.scope, ctx);
            } else {
                self.members(body, &ctx.scope, ctx, node, &ctx.scope.clone());
            }
            return;
        };
        let keyword = self.keyword_of(node);
        let kind = match keyword {
            "class" if is_enum => SymbolKind::Enum,
            _ if is_enum => SymbolKind::Enum,
            "union" => SymbolKind::Union,
            "class" => SymbolKind::Class,
            _ => SymbolKind::Struct,
        };
        let is_class_keyword = keyword == "class" && !is_enum;
        let end = body.start_byte();
        let signature = if typedef_signature.is_empty() {
            self.signature(outer, end)
        } else {
            typedef_signature
        };
        let doc = self.doc(outer);
        let scope = match qualifier.last() {
            Some(outer_class) => format!("{}::{outer_class}", ctx.scope),
            None => ctx.scope.clone(),
        };
        if specialization {
            // Not a symbol; read it for the calls it makes.
            let owner = format!("{scope}::{written}");
            if !is_enum {
                let mut inner = ctx.clone();
                inner.class = Some(owner.clone());
                inner.in_class = true;
                self.members(body, &owner, &inner, node, &owner);
            }
            return;
        }
        let mut visibility = if ctx.in_class {
            match ctx.access {
                Access::Public => Visibility::Public,
                Access::Protected => Visibility::Crate,
                Access::Private => Visibility::Private,
            }
        } else if ctx.anonymous {
            Visibility::Private
        } else {
            Visibility::Public
        };
        if matches!(kind, SymbolKind::Class | SymbolKind::Struct) && ctx.in_class {
            // A nested type keeps its own access.
        } else if self.is_static_parent(outer) {
            visibility = Visibility::Private;
        }
        let (pure, concrete) = if is_enum {
            (0, 0)
        } else {
            self.interface_shape(body)
        };
        let interface = !is_enum && pure > 0 && concrete == 0;
        let mut attrs = vec![];
        if !is_enum && pure > 0 && !interface {
            attrs.push("abstract".to_string());
        }
        if !qualifier.is_empty() {
            attrs.push(format!("{OUTLINE_ATTR}{}", qualifier.join("::")));
        }
        let symbol_kind = if interface {
            SymbolKind::Interface
        } else {
            kind
        };
        let symbol = self.sink.symbol(&scope, &written, symbol_kind, node);
        symbol.visibility = visibility;
        symbol.signature = signature;
        symbol.doc = doc;
        symbol.attrs = attrs;
        if let Some(template) = template {
            symbol.start_line = template.node.start_position().row as u32 + 1;
        }
        let mut copy = symbol.clone();
        self.finish(&mut copy, ctx, template);
        let last = self.sink.out.symbols.len() - 1;
        self.sink.out.symbols[last] = copy;
        let qualified = format!("{scope}::{written}");
        for child in named(node) {
            if child.kind() == "base_class_clause" {
                self.bases(child, &qualified, ctx);
            }
        }
        if is_enum {
            if let Some(base) = node.child_by_field_name("base") {
                self.walk(base, &qualified, ctx);
            }
            self.enumerators(body, &qualified, ctx);
            return;
        }
        let mut inner = ctx.clone();
        inner.in_class = true;
        inner.class = Some(qualified.clone());
        inner.access = if is_class_keyword {
            Access::Private
        } else {
            Access::Public
        };
        self.members(body, &qualified, &inner, node, &qualified);
    }

    fn is_static_parent(&self, outer: Node) -> bool {
        outer.kind() == "declaration" && self.is_static(outer)
    }

    /// Pure virtual members, and the members that keep a class from being a
    /// pure interface: data, and functions with a body or no `virtual`.
    fn interface_shape(&self, body: Node) -> (usize, usize) {
        let mut pure = 0;
        let mut concrete = 0;
        let mut stack = named(body);
        while let Some(member) = stack.pop() {
            match member.kind() {
                "preproc_if" | "preproc_ifdef" | "preproc_else" | "preproc_elif" => {
                    stack.extend(named(member));
                }
                "field_declaration" | "declaration" => {
                    let declarators = field_all(member, "declarator");
                    for declarator in declarators {
                        let Some(declared) = self.declared(declarator) else {
                            continue;
                        };
                        if declared.function.is_none() {
                            if !self.is_static(member) {
                                concrete += 1;
                            }
                            continue;
                        }
                        let is_pure = member
                            .child_by_field_name("default_value")
                            .is_some_and(|v| self.text(v) == "0");
                        if is_pure {
                            pure += 1;
                        } else if (!declared.special || declared.name.starts_with("operator"))
                            && !self.is_static(member)
                        {
                            concrete += 1;
                        }
                    }
                }
                "function_definition" => {
                    let special = member
                        .child_by_field_name("declarator")
                        .and_then(|d| self.declared(d))
                        .is_some_and(|d| d.special && d.name.starts_with('~'));
                    let defaulted = named(member)
                        .iter()
                        .any(|c| c.kind() == "default_method_clause");
                    if !special && !defaulted && !self.is_static(member) {
                        concrete += 1;
                    }
                }
                "template_declaration" => concrete += 1,
                _ => {}
            }
        }
        (pure, concrete)
    }

    fn bases(&mut self, clause: Node, class: &str, ctx: &Ctx) {
        for base in named(clause) {
            match base.kind() {
                "type_identifier" | "qualified_identifier" | "template_type" => {
                    let path = self.type_path(base);
                    if !path.is_empty() {
                        self.sink.inherits(class, path);
                    }
                }
                _ => {}
            }
        }
        let _ = ctx;
    }

    fn enumerators(&mut self, body: Node, scope: &str, ctx: &Ctx) {
        for child in named(body) {
            match child.kind() {
                "enumerator" => {
                    let Some(name) = child.child_by_field_name("name") else {
                        continue;
                    };
                    let name = self.text(name).to_string();
                    let doc = self.doc(child);
                    let visibility = if ctx.anonymous {
                        Visibility::Private
                    } else {
                        Visibility::Public
                    };
                    let symbol = self.sink.symbol(scope, &name, SymbolKind::Const, child);
                    symbol.signature = name.clone();
                    symbol.doc = doc;
                    symbol.visibility = visibility;
                    let owner = symbol.qualified.clone();
                    let mut copy = symbol.clone();
                    self.finish(&mut copy, ctx, None);
                    let last = self.sink.out.symbols.len() - 1;
                    self.sink.out.symbols[last] = copy;
                    if let Some(value) = child.child_by_field_name("value") {
                        self.walk(value, &owner, ctx);
                    }
                }
                "preproc_if" | "preproc_ifdef" | "preproc_else" | "preproc_elif" => {
                    self.enumerators(child, scope, ctx);
                }
                _ => {}
            }
        }
    }

    /// The members of a class body.
    fn members(&mut self, body: Node, scope: &str, ctx: &Ctx, class: Node, owner: &str) {
        let _ = class;
        let mut inner = ctx.clone();
        inner.scope = scope.to_string();
        for member in named(body) {
            self.member(member, owner, &mut inner);
        }
    }

    fn member(&mut self, member: Node, owner: &str, ctx: &mut Ctx) {
        match member.kind() {
            "access_specifier" => {
                ctx.access = match self.text(member) {
                    "private" => Access::Private,
                    "protected" => Access::Protected,
                    _ => Access::Public,
                };
            }
            "function_definition" => self.function(member, member, ctx),
            "field_declaration" | "declaration" => self.field(member, owner, ctx),
            "template_declaration" => self.template(member, ctx),
            "alias_declaration" => self.alias(member, member, ctx),
            "type_definition" => self.typedef(member, ctx),
            "class_specifier" | "struct_specifier" | "union_specifier" | "enum_specifier" => {
                self.type_spec(member, member, ctx, None);
            }
            "friend_declaration" => {
                // A friend function's body still makes calls.
                for child in named(member) {
                    if child.kind() == "function_definition"
                        && let Some(body) = child.child_by_field_name("body")
                    {
                        self.walk(body, owner, ctx);
                    }
                }
            }
            "preproc_def" | "preproc_function_def" => self.define(member, ctx),
            "preproc_if" | "preproc_ifdef" | "preproc_else" | "preproc_elif" => {
                let condition = member.child_by_field_name("condition").map(|c| c.id());
                for child in named(member) {
                    if Some(child.id()) != condition {
                        self.member(child, owner, ctx);
                    }
                }
            }
            "preproc_include" => self.include(member),
            "ERROR" => {
                for child in named(member) {
                    self.member(child, owner, ctx);
                }
            }
            _ => {}
        }
    }

    /// A data member, a member function declaration or a static member.
    fn field(&mut self, node: Node, owner: &str, ctx: &Ctx) {
        let static_member = self.is_static(node);
        let constant = self.has_qualifier(node, "const") || self.has_qualifier(node, "constexpr");
        let type_node = node.child_by_field_name("type");
        if let Some(ty) = type_node
            && matches!(
                ty.kind(),
                "class_specifier" | "struct_specifier" | "union_specifier" | "enum_specifier"
            )
        {
            self.type_spec(ty, node, ctx, None);
        } else if let Some(ty) = type_node {
            self.walk(ty, owner, ctx);
        }
        for declarator in field_all(node, "declarator") {
            let Some(declared) = self.declared(declarator) else {
                continue;
            };
            if let Some(function) = declared.function {
                let signature = self.signature(node, node.end_byte());
                let doc = self.doc(node);
                self.callable(node, &declared, function, signature, doc, true, ctx, None);
                let me = format!("{}::{}", ctx.scope, declared.name);
                if let Some(parameters) = function.child_by_field_name("parameters") {
                    self.walk(parameters, &me, ctx);
                }
                continue;
            }
            if declarator.kind() == "init_declarator" || static_member {
                // `static const int N = 3;`
            }
            if static_member {
                let kind = if constant {
                    SymbolKind::Const
                } else {
                    SymbolKind::Static
                };
                let signature = self.signature(node, node.end_byte());
                let doc = self.doc(node);
                let visibility = self.visibility(node, ctx);
                let symbol = self.sink.symbol(&ctx.scope, &declared.name, kind, node);
                symbol.signature = signature;
                symbol.doc = doc;
                symbol.visibility = visibility;
                symbol.attrs.push(DECLARATION_ATTR.to_string());
            }
        }
        // The values of data members mention names too.
        for value in field_all(node, "default_value") {
            self.walk(value, owner, ctx);
        }
    }

    // ----- references -----

    /// Path of a type with its template arguments removed.
    fn type_path(&self, node: Node) -> Vec<String> {
        match node.kind() {
            "type_identifier" | "namespace_identifier" | "identifier" => {
                vec![self.text(node).to_string()]
            }
            "qualified_identifier" => {
                let mut path = node
                    .child_by_field_name("scope")
                    .map(|s| self.type_path(s))
                    .unwrap_or_default();
                if let Some(name) = node.child_by_field_name("name") {
                    path.extend(self.type_path(name));
                }
                path
            }
            "template_type" | "template_function" => node
                .child_by_field_name("name")
                .map(|n| self.type_path(n))
                .unwrap_or_default(),
            "struct_specifier" | "class_specifier" | "union_specifier" | "enum_specifier" => node
                .child_by_field_name("name")
                .filter(|_| node.child_by_field_name("body").is_none())
                .map(|n| self.type_path(n))
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// A call's callee as a path, when it is a name.
    fn callee_path(&self, node: Node) -> Vec<String> {
        match node.kind() {
            "identifier" | "namespace_identifier" => vec![self.text(node).to_string()],
            "qualified_identifier" => {
                let mut path = node
                    .child_by_field_name("scope")
                    .map(|s| self.type_path(s))
                    .unwrap_or_default();
                if let Some(name) = node.child_by_field_name("name") {
                    path.extend(self.callee_path(name));
                }
                path
            }
            "template_function" | "template_method" => node
                .child_by_field_name("name")
                .map(|n| self.callee_path(n))
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    fn walk_children(&mut self, node: Node, owner: &str, ctx: &Ctx) {
        for child in named(node) {
            self.walk(child, owner, ctx);
        }
    }

    /// Record what an expression, statement or type mentions.
    fn walk(&mut self, node: Node, owner: &str, ctx: &Ctx) {
        match node.kind() {
            "comment" | "string_literal" | "char_literal" | "number_literal" => {}
            "type_identifier" => {
                let name = self.text(node);
                if !ctx.generics.contains(name) {
                    self.sink.type_ref(owner, vec![name.to_string()]);
                }
            }
            "qualified_identifier" => {
                let path = self.type_path(node);
                let leaf_is_type = node
                    .child_by_field_name("name")
                    .is_some_and(|n| matches!(n.kind(), "type_identifier" | "template_type"));
                if leaf_is_type {
                    self.sink.type_ref(owner, path);
                } else if path.first().is_some_and(|p| {
                    p.chars().next().is_some_and(char::is_uppercase) || is_constant_name(p)
                }) || path.last().is_some_and(|l| is_constant_name(l))
                {
                    self.sink.value(owner, path);
                }
                if let Some(scope) = node.child_by_field_name("scope")
                    && scope.kind() == "template_type"
                {
                    self.walk_children(scope, owner, ctx);
                }
            }
            "struct_specifier" | "class_specifier" | "union_specifier" | "enum_specifier" => {
                let path = self.type_path(node);
                if !path.is_empty() {
                    self.sink.type_ref(owner, path);
                }
            }
            "call_expression" => self.call(node, owner, ctx),
            "new_expression" => {
                if let Some(ty) = node.child_by_field_name("type") {
                    self.construct(ty, owner);
                    self.walk(ty, owner, ctx);
                }
                for field in ["arguments", "placement", "declarator"] {
                    if let Some(part) = node.child_by_field_name(field) {
                        self.walk(part, owner, ctx);
                    }
                }
            }
            "compound_literal_expression" => {
                if let Some(ty) = node.child_by_field_name("type") {
                    if self.input.cpp {
                        self.construct(ty, owner);
                    }
                    self.walk(ty, owner, ctx);
                }
                if let Some(value) = node.child_by_field_name("value") {
                    self.walk(value, owner, ctx);
                }
            }
            "declaration" => {
                // `Foo foo(1);` constructs a `Foo`.
                if self.input.cpp
                    && let Some(ty) = node.child_by_field_name("type")
                    && field_all(node, "declarator").iter().any(|d| {
                        d.kind() == "init_declarator"
                            && d.child_by_field_name("value").is_some_and(|v| {
                                matches!(v.kind(), "argument_list" | "initializer_list")
                            })
                    })
                {
                    self.construct(ty, owner);
                }
                self.walk_children(node, owner, ctx);
            }
            "field_initializer" => {
                // `: Base(x), member_(y)`: a base class is a constructor call.
                if let Some(first) = node.named_child(0)
                    && first.kind() == "field_identifier"
                {
                    let name = self.text(first);
                    if name.chars().next().is_some_and(char::is_uppercase) {
                        self.sink.type_ref(owner, vec![name.to_string()]);
                    }
                }
                for child in named(node).into_iter().skip(1) {
                    self.walk(child, owner, ctx);
                }
            }
            "field_expression" => {
                if let Some(argument) = node.child_by_field_name("argument") {
                    self.walk(argument, owner, ctx);
                }
            }
            "identifier" => {
                let name = self.text(node);
                if is_constant_name(name) {
                    self.sink.value(owner, vec![name.to_string()]);
                }
            }
            "preproc_def" | "preproc_function_def" => {}
            "lambda_expression" | "ERROR" | "compound_statement" => {
                self.walk_children(node, owner, ctx);
            }
            _ => self.walk_children(node, owner, ctx),
        }
    }

    /// `Type(args)`, `new Type(args)`, `Type name(args);`: a call of the
    /// constructor, which is the method named like its class.
    fn construct(&mut self, ty: Node, owner: &str) {
        let path = self.type_path(ty);
        if let Some(last) = path.last().cloned() {
            let mut constructor = path;
            constructor.push(last);
            self.sink.call(owner, constructor, false, None, ty);
        }
    }

    fn call(&mut self, node: Node, owner: &str, ctx: &Ctx) {
        let function = node.child_by_field_name("function");
        if let Some(function) = function {
            match function.kind() {
                "identifier" | "qualified_identifier" | "template_function" => {
                    let path = self.callee_path(function);
                    if !path.is_empty() {
                        // `std::` and the like are never the repository's;
                        // the resolver says so by finding nothing.
                        self.sink.call(owner, path, false, None, function);
                    }
                    if function.kind() == "qualified_identifier" {
                        self.walk(function, owner, ctx);
                    }
                }
                "field_expression" => {
                    let field = function.child_by_field_name("field");
                    let argument = function.child_by_field_name("argument");
                    if let Some(field) = field {
                        let path = self.callee_path(field);
                        let name = if path.is_empty() {
                            vec![self.text(field).to_string()]
                        } else {
                            path
                        };
                        let name = vec![name.last().cloned().unwrap_or_default()];
                        if argument.is_some_and(|a| a.kind() == "this") {
                            self.sink
                                .call(owner, name, true, ctx.class.as_deref(), field);
                        } else if !name[0].is_empty() {
                            self.sink.call(owner, name, true, None, field);
                        }
                    }
                    if let Some(argument) = argument {
                        self.walk(argument, owner, ctx);
                    }
                }
                _ => self.walk(function, owner, ctx),
            }
        }
        if let Some(arguments) = node.child_by_field_name("arguments") {
            self.walk(arguments, owner, ctx);
        }
    }
}

struct Template<'t> {
    node: Node<'t>,
    #[allow(dead_code)]
    specialization: bool,
}

fn namespace_attr(namespace: &[String]) -> String {
    format!("{NAMESPACE_ATTR}{}", namespace.join("::"))
}

/// `Foo`, `_Foo`, `foo_t`, `FOO` and `foo` are one name for the purpose of
/// `typedef struct _foo { ... } foo_t;`.
pub(crate) fn same_type_name(a: &str, b: &str) -> bool {
    let norm = |s: &str| {
        s.trim_matches('_')
            .trim_end_matches("_t")
            .trim_end_matches("_s")
            .trim_end_matches("_st")
            .replace('_', "")
            .to_ascii_lowercase()
    };
    !a.is_empty() && norm(a) == norm(b)
}

/// A test's name made usable as a name segment.
fn sanitize_test_name(name: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in name.chars() {
        if c.is_alphanumeric() || c == '_' {
            if gap && !out.is_empty() {
                out.push('_');
            }
            gap = false;
            out.push(c);
        } else {
            gap = true;
        }
    }
    out
}

/// A comment's text without its markers.
fn comment_text(raw: &str) -> String {
    if let Some(doc) = block_doc(raw) {
        return doc;
    }
    if let Some(rest) = raw.strip_prefix("/*") {
        let inner = rest.strip_suffix("*/").unwrap_or(rest);
        let inner = inner.strip_prefix('!').unwrap_or(inner);
        return clean_block(inner);
    }
    let rest = raw
        .strip_prefix("///")
        .or_else(|| raw.strip_prefix("//!"))
        .or_else(|| raw.strip_prefix("//"))
        .unwrap_or(raw);
    rest.strip_prefix(' ')
        .unwrap_or(rest)
        .trim_end()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cw_core::Symbol;

    fn repo_include(spec: &str, quoted: bool) -> Include {
        match (spec, quoted) {
            ("util.h", true) => Include::Repo(vec!["util".into()]),
            ("fmt/core.h", false) => Include::Library {
                package: "fmt".into(),
                name: "core.h".into(),
            },
            _ => Include::Unknown,
        }
    }

    fn parse(file: &str, source: &str) -> ParsedFile {
        let cpp = !file.ends_with(".c");
        let module = file
            .rsplit_once('.')
            .map_or(file, |(m, _)| m)
            .replace('/', "::");
        let input = CInput {
            module: &module,
            cpp,
            include: &repo_include,
        };
        parse_c(file, source, &input).unwrap()
    }

    fn sym<'a>(parsed: &'a ParsedFile, qualified: &str) -> &'a Symbol {
        parsed
            .symbols
            .iter()
            .find(|s| s.qualified == qualified)
            .unwrap_or_else(|| {
                let all: Vec<_> = parsed
                    .symbols
                    .iter()
                    .map(|s| format!("{} {:?}", s.qualified, s.kind))
                    .collect();
                panic!("no {qualified} in {all:#?}")
            })
    }

    fn kinds(parsed: &ParsedFile) -> Vec<String> {
        parsed
            .symbols
            .iter()
            .map(|s| format!("{} {:?}", s.qualified, s.kind))
            .collect()
    }

    fn has_attr(symbol: &Symbol, attr: &str) -> bool {
        symbol.attrs.iter().any(|a| a == attr)
    }

    fn calls(parsed: &ParsedFile, caller: &str) -> Vec<(Vec<String>, bool)> {
        let mut found: Vec<_> = parsed
            .calls
            .iter()
            .filter(|c| c.caller == caller)
            .map(|c| (c.path.clone(), c.method))
            .collect();
        found.sort();
        found
    }

    fn path(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|p| p.to_string()).collect()
    }

    #[test]
    fn maps_the_declarations_of_c() {
        let p = parse(
            "src/buf.c",
            r#"#include <stdio.h>
#define BUF_MAX 64
#define BUF_SQ(x) ((x) * (x))
#define BUF_H
#define BUF_FLAG

/* A buffer. */
struct buf { int len; char *data; };
union value { int i; float f; };
enum color { RED, GREEN = 2 };
typedef struct { int x; } point_t;
typedef struct node { struct node *next; } node;
typedef struct opaque opaque_t;
typedef int (*callback_t)(int);
typedef unsigned long ulong_t;

int counter;
static int hidden = 1;
const int LIMIT = 10;
extern int shared;
int (*handler)(int) = 0;

int add(int a, int b);
/** Add two numbers. */
int add(int a, int b) { return a + b; }
static void local(void) {}
struct buf *buf_new(void);
"#,
        );
        assert!(!p.has_errors);
        let k = kinds(&p);
        for expected in [
            "src::buf Module",
            "src::buf::BUF_MAX Const",
            "src::buf::BUF_SQ Macro",
            "src::buf::buf Struct",
            "src::buf::value Union",
            "src::buf::color Enum",
            "src::buf::color::RED Const",
            "src::buf::color::GREEN Const",
            // An anonymous struct takes the name of its typedef.
            "src::buf::point_t Struct",
            // `typedef struct node {} node;` is one name.
            "src::buf::node Struct",
            "src::buf::opaque_t TypeAlias",
            "src::buf::callback_t TypeAlias",
            "src::buf::ulong_t TypeAlias",
            "src::buf::counter Static",
            "src::buf::hidden Static",
            "src::buf::LIMIT Const",
            "src::buf::shared Static",
            "src::buf::handler Static",
            "src::buf::add Function",
            "src::buf::local Function",
            "src::buf::buf_new Function",
        ] {
            assert!(k.contains(&expected.to_string()), "{expected} in {k:#?}");
        }
        // An include guard and a flag are no symbols.
        assert!(
            !k.iter()
                .any(|s| s.contains("BUF_H") || s.contains("BUF_FLAG"))
        );
        assert_eq!(sym(&p, "src::buf::hidden").visibility, Visibility::Private);
        assert_eq!(sym(&p, "src::buf::counter").visibility, Visibility::Public);
        assert!(has_attr(sym(&p, "src::buf::shared"), DECLARATION_ATTR));
        assert!(!has_attr(sym(&p, "src::buf::counter"), DECLARATION_ATTR));
        assert!(has_attr(sym(&p, "src::buf::opaque_t"), DECLARATION_ATTR));
        // A function prototype and its definition: both are parsed, the
        // scan makes one of them.
        let adds: Vec<_> = p
            .symbols
            .iter()
            .filter(|s| s.qualified == "src::buf::add")
            .collect();
        assert_eq!(adds.len(), 2);
        assert!(has_attr(adds[0], DECLARATION_ATTR) && !has_attr(adds[1], DECLARATION_ATTR));
        assert_eq!(adds[1].doc, "Add two numbers.");
        assert_eq!(adds[1].signature, "int add(int a, int b)");
        assert_eq!(sym(&p, "src::buf::buf").doc, "A buffer.");
        assert_eq!(sym(&p, "src::buf::BUF_MAX").signature, "#define BUF_MAX 64");
        assert_eq!(
            sym(&p, "src::buf::BUF_SQ").signature,
            "#define BUF_SQ(x) ((x) * (x))"
        );
        assert_eq!(sym(&p, "src::buf::local").visibility, Visibility::Private);
    }

    #[test]
    fn maps_the_declarations_of_cpp() {
        let p = parse(
            "src/shape.hpp",
            r#"#pragma once
#include <vector>

namespace geo::detail {

/// Anything with an area.
class Shape : public Base, protected Other<int> {
public:
    Shape(int id);
    virtual ~Shape();
    virtual double area() const = 0;
    bool operator==(const Shape& other) const;
    static const int kMax = 3;
    int id() const { return id_; }
protected:
    void touch();
private:
    int id_;
};

struct Point { int x, y; void move(); };
enum class Mode { On, Off };
using Points = std::vector<Point>;
typedef Point P2;

template <typename T>
class Box {
public:
    T get() const;
};

template <typename T>
T clamp(T v, T lo, T hi);

template <>
class Box<int> { public: int get() const { return 1; } };

namespace { int hidden = 3; void helper() {} }
inline namespace v2 { void api(); }

}  // namespace geo::detail
"#,
        );
        assert!(!p.has_errors, "{:?}", kinds(&p));
        let k = kinds(&p);
        for expected in [
            "src::shape::Shape Class",
            "src::shape::Shape::Shape Method",
            "src::shape::Shape::~Shape Method",
            "src::shape::Shape::area Method",
            "src::shape::Shape::operator== Method",
            "src::shape::Shape::kMax Const",
            "src::shape::Shape::id Method",
            "src::shape::Shape::touch Method",
            "src::shape::Point Struct",
            "src::shape::Point::move Method",
            "src::shape::Mode Enum",
            "src::shape::Mode::On Const",
            "src::shape::Points TypeAlias",
            "src::shape::P2 TypeAlias",
            "src::shape::Box Class",
            "src::shape::Box::get Method",
            "src::shape::clamp Function",
            "src::shape::hidden Static",
            "src::shape::helper Function",
            "src::shape::api Function",
        ] {
            assert!(k.contains(&expected.to_string()), "{expected} in {k:#?}");
        }
        // A specialization is no new symbol.
        assert_eq!(
            p.symbols.iter().filter(|s| s.name == "Box").count(),
            1,
            "{k:#?}"
        );
        // The namespace is an attribute; the anonymous one is private.
        let shape = sym(&p, "src::shape::Shape");
        assert!(has_attr(shape, "namespace:geo::detail"));
        assert_eq!(shape.doc, "Anything with an area.");
        assert_eq!(
            shape.signature,
            "class Shape : public Base, protected Other<int>"
        );
        assert_eq!(
            sym(&p, "src::shape::hidden").visibility,
            Visibility::Private
        );
        assert_eq!(
            sym(&p, "src::shape::helper").visibility,
            Visibility::Private
        );
        assert_eq!(sym(&p, "src::shape::api").visibility, Visibility::Public);
        // Access specifiers: a class is private by default, a struct public.
        assert_eq!(
            sym(&p, "src::shape::Shape::area").visibility,
            Visibility::Public
        );
        assert_eq!(
            sym(&p, "src::shape::Shape::touch").visibility,
            Visibility::Crate
        );
        assert_eq!(
            sym(&p, "src::shape::Point::move").visibility,
            Visibility::Public
        );
        assert!(has_attr(sym(&p, "src::shape::Shape::~Shape"), "destructor"));
        assert!(has_attr(sym(&p, "src::shape::Shape::area"), "virtual"));
        assert!(has_attr(sym(&p, "src::shape::clamp"), "template"));
        // Base classes are the edge inheritance is.
        let bases: Vec<_> = p
            .impls
            .iter()
            .filter(|i| i.type_path == ["Shape"])
            .map(|i| i.trait_path.clone().unwrap())
            .collect();
        assert_eq!(bases, [path(&["Base"]), path(&["Other"])]);
    }

    #[test]
    fn a_class_of_pure_virtual_members_is_an_interface() {
        let p = parse(
            "src/api.hpp",
            "class Reader { public: virtual ~Reader() = default; virtual int read() = 0; virtual void close() = 0; };\n\
             class Partial { public: virtual int a() = 0; int b() { return 1; } };\n\
             class Plain { public: int a(); int n; };\n\
             struct Stateful { virtual void f() = 0; int count; };\n",
        );
        assert_eq!(sym(&p, "src::api::Reader").kind, SymbolKind::Interface);
        // Abstract, but with a body: still a class, marked abstract.
        let partial = sym(&p, "src::api::Partial");
        assert_eq!(partial.kind, SymbolKind::Class);
        assert!(has_attr(partial, "abstract"));
        assert_eq!(sym(&p, "src::api::Plain").kind, SymbolKind::Class);
        assert_eq!(sym(&p, "src::api::Stateful").kind, SymbolKind::Struct);
    }

    #[test]
    fn out_of_line_definitions_name_their_class() {
        let p = parse(
            "src/shape.cpp",
            r#"#include "util.h"
namespace geo {
Shape::Shape(int id) : id_(id) { util_init(); }
Shape::~Shape() {}
double Shape::area() const { return helper(); }
bool Shape::operator==(const Shape& o) const { return id_ == o.id_; }
template <typename T>
T Box<T>::get() const { return value_; }
Outer::Inner::Inner() {}
void ns::free_fn() {}
int Shape::count = 0;
}
"#,
        );
        for (name, class) in [
            ("Shape", "Shape"),
            ("~Shape", "Shape"),
            ("area", "Shape"),
            ("operator==", "Shape"),
            ("get", "Box"),
            ("Inner", "Inner"),
            ("free_fn", "ns"),
        ] {
            let q = format!("src::shape::{class}::{name}");
            let symbol = sym(&p, &q);
            assert_eq!(symbol.kind, SymbolKind::Method, "{q}");
            assert!(has_attr(symbol, &format!("{OUTLINE_ATTR}{class}")), "{q}");
            assert!(!has_attr(symbol, DECLARATION_ATTR), "{q}");
        }
        let count = sym(&p, "src::shape::Shape::count");
        assert_eq!(count.kind, SymbolKind::Static);
        // What the bodies call belongs to the method.
        assert_eq!(
            calls(&p, "src::shape::Shape::Shape"),
            [(path(&["util_init"]), false)]
        );
        assert_eq!(
            calls(&p, "src::shape::Shape::area"),
            [(path(&["helper"]), false)]
        );
    }

    #[test]
    fn reads_includes_as_modules_and_libraries() {
        let p = parse(
            "lib/net.c",
            "#include \"util.h\"\n#include <fmt/core.h>\n#include <vector>\n#include \"missing.h\"\n#include MACRO_HEADER\n",
        );
        assert_eq!(p.uses.len(), 2);
        assert_eq!(p.uses[0].path, ["util"]);
        assert!(p.uses[0].glob);
        assert_eq!(p.uses[0].scope, "lib::net");
        assert_eq!(p.uses[1].path, ["fmt"]);
        assert_eq!(p.uses[1].external, "core.h");
    }

    #[test]
    fn reads_calls_in_c_and_cpp() {
        let p = parse(
            "src/run.cpp",
            r#"
struct Job { int n; };
void Job::go() {
    helper(1);
    ns::util(2);
    Job::make();
    this->step();
    other.size();
    ptr->push_back(3);
    Job j(4);
    Job k{5};
    auto* q = new Job(6);
    Widget::create<int>(7);
    LOG("x", MAX_SIZE);
    auto f = [](int x) { return inner(x); };
}
"#,
        );
        let owner = "src::run::Job::go";
        let found = calls(&p, owner);
        for expected in [
            (path(&["helper"]), false),
            (path(&["ns", "util"]), false),
            (path(&["Job", "make"]), false),
            (path(&["step"]), true),
            (path(&["size"]), true),
            (path(&["push_back"]), true),
            (path(&["Job", "Job"]), false),
            (path(&["Widget", "create"]), false),
            (path(&["LOG"]), false),
            (path(&["inner"]), false),
        ] {
            assert!(found.contains(&expected), "{expected:?} in {found:#?}");
        }
        // `this->step()` knows its class.
        let step = p.calls.iter().find(|c| c.path == ["step"]).unwrap();
        assert_eq!(step.self_type.as_deref(), Some("src::run::Job"));
        // A constant used in a call's arguments is a value reference.
        assert!(
            p.value_refs
                .iter()
                .any(|v| v.from == owner && v.path == ["MAX_SIZE"])
        );
        assert!(
            p.type_refs
                .iter()
                .any(|t| t.from == owner && t.path == ["Job"])
        );
    }

    #[test]
    fn reads_every_branch_of_the_preprocessor() {
        let p = parse(
            "src/plat.c",
            "#ifdef _WIN32\nint plat(void) { return 1; }\n#define NAME \"win\"\n#elif defined(__linux__)\nint plat(void) { return 2; }\n#else\nint plat(void) { return 3; }\nstatic int other;\n#endif\n#if 0\nint dead_branch(void);\n#endif\n",
        );
        let plats = p
            .symbols
            .iter()
            .filter(|s| s.qualified == "src::plat::plat")
            .count();
        // Every branch is parsed; the scan keeps one.
        assert_eq!(plats, 3);
        assert_eq!(sym(&p, "src::plat::NAME").kind, SymbolKind::Const);
        assert_eq!(sym(&p, "src::plat::other").kind, SymbolKind::Static);
        assert_eq!(sym(&p, "src::plat::dead_branch").kind, SymbolKind::Function);
    }

    #[test]
    fn finds_the_test_macros_of_each_framework() {
        let gtest = parse(
            "tests/list_test.cc",
            "TEST(List, Push) { List l; l.push(1); EXPECT_EQ(l.size(), 1); }\n\
             TEST_F(Fixture, Reads) { run(); }\n\
             TEST_P(Params, Cases) {}\n\
             TYPED_TEST(Typed, Works) {}\n",
        );
        let push = sym(&gtest, "tests::list_test::List_Push");
        assert_eq!(push.kind, SymbolKind::Function);
        assert!(has_attr(push, TEST_ATTR));
        assert!(has_attr(push, "test-framework:gtest"));
        assert!(has_attr(push, "test-name:List.Push"));
        assert!(has_attr(push, "test-suite:List"));
        // What the body calls belongs to the test.
        assert!(
            calls(&gtest, "tests::list_test::List_Push")
                .iter()
                .any(|(path, _)| path == &["EXPECT_EQ"])
        );
        assert!(has_attr(
            sym(&gtest, "tests::list_test::Fixture_Reads"),
            "test-name:Fixture.Reads"
        ));
        assert!(has_attr(
            sym(&gtest, "tests::list_test::Params_Cases"),
            "test-name:Params.Cases"
        ));
        assert!(has_attr(
            sym(&gtest, "tests::list_test::Typed_Works"),
            "test-name:Typed.Works"
        ));
        // The names of the test are no types the code uses.
        for name in [
            "Suite", "Fixture", "Params", "Cases", "Typed", "Works", "Reads",
        ] {
            assert!(
                !gtest.type_refs.iter().any(|t| t.path == [name]),
                "{:?}",
                gtest.type_refs
            );
        }

        let catch = parse(
            "tests/map_test.cpp",
            "TEST_CASE(\"map inserts\", \"[map]\") { insert(); }\n\
             SCENARIO(\"a map is read\") { GIVEN(\"a map\") { } }\n\
             TEST_CASE_METHOD(Fixture, \"fixture case\") { run(); }\n",
        );
        let inserts = sym(&catch, "tests::map_test::map_inserts");
        assert!(has_attr(inserts, "test-framework:catch"));
        assert!(has_attr(inserts, "test-name:map inserts"));
        assert!(
            calls(&catch, "tests::map_test::map_inserts").contains(&(path(&["insert"]), false))
        );
        assert!(has_attr(
            sym(&catch, "tests::map_test::a_map_is_read"),
            "test-name:a map is read"
        ));
        assert!(has_attr(
            sym(&catch, "tests::map_test::fixture_case"),
            "test-suite:Fixture"
        ));

        let boost = parse(
            "test/b.cpp",
            "BOOST_AUTO_TEST_CASE(adds_up) { BOOST_CHECK(1); }\nBOOST_FIXTURE_TEST_CASE(with_fixture, Fix) {}\n",
        );
        let adds = sym(&boost, "test::b::adds_up");
        assert!(has_attr(adds, "test-framework:boost"));
        assert!(has_attr(adds, "test-name:adds_up"));
        assert!(has_attr(sym(&boost, "test::b::with_fixture"), TEST_ATTR));

        // The C grammar reads the same macros.
        let c = parse("tests/t.c", "TEST(Suite, Case) { check(); }\n");
        assert!(has_attr(
            sym(&c, "tests::t::Suite_Case"),
            "test-name:Suite.Case"
        ));
    }

    #[test]
    fn reads_the_comment_above_a_declaration() {
        let p = parse(
            "src/doc.c",
            "// First line.\n// Second line.\nint a(void);\n\n// Detached.\n\nint b(void);\n/** Block doc. */\nint c(void);\n/// Triple.\nint d(void);\nint e(void); // trailing\nint f(void);\n/* Copyright 2020 Someone. License: MIT. */\nint g(void);\n//////////\nint h(void);\n",
        );
        assert_eq!(sym(&p, "src::doc::a").doc, "First line.\nSecond line.");
        assert_eq!(sym(&p, "src::doc::b").doc, "");
        assert_eq!(sym(&p, "src::doc::c").doc, "Block doc.");
        assert_eq!(sym(&p, "src::doc::d").doc, "Triple.");
        // A comment on the line of the declaration before belongs to that one.
        assert_eq!(sym(&p, "src::doc::f").doc, "");
        // A license and a line of slashes describe nothing.
        assert_eq!(sym(&p, "src::doc::g").doc, "");
        assert_eq!(sym(&p, "src::doc::h").doc, "");
        // The documentation of a file is what says so.
        let file = parse("src/f.h", "//! The f module.\n//! More.\nint a(void);\n");
        assert_eq!(sym(&file, "src::f").doc, "The f module.\nMore.");
        let license = parse("src/g.h", "/* Copyright (c) 2020 */\nint a(void);\n");
        assert_eq!(sym(&license, "src::g").doc, "");
        let tagged = parse(
            "src/h.h",
            "/**\n * @file h.h\n * Helpers.\n */\nint a(void);\n",
        );
        assert!(sym(&tagged, "src::h").doc.contains("Helpers."));
    }

    #[test]
    fn code_the_grammar_cannot_read_still_gives_what_stands() {
        // Macros that wrap a declaration are no C: the grammar recovers.
        let p = parse(
            "src/macro.h",
            "FT_BEGIN_HEADER\nFT_EXPORT( int ) api_call( int a );\nstatic inline int ok(void) { return 0; }\nFT_END_HEADER\n",
        );
        // The lone macros are set aside and the one around the type is
        // unwrapped, so that the grammar reads what stands.
        assert!(!p.has_errors);
        assert_eq!(sym(&p, "src::macro::api_call").kind, SymbolKind::Function);
        assert_eq!(
            sym(&p, "src::macro::api_call").signature,
            "int api_call( int a )"
        );
        assert_eq!(sym(&p, "src::macro::ok").kind, SymbolKind::Function);
        // A name in a list is no macro on a line of its own.
        let list = parse(
            "src/list.h",
            "enum e {\n    FIRST_ONE,\n    LAST_ONE\n};\nint x[] = {\n    ONE_CONST,\n    TWO_CONST\n};\n",
        );
        assert!(!list.has_errors);
        assert!(
            list.symbols
                .iter()
                .any(|s| s.qualified == "src::list::e::LAST_ONE")
        );
        // Namespace macros around declarations of a class.
        let namespaced = parse(
            "src/ns.cc",
            "#include \"ns.h\"\n\nBSSL_NAMESPACE_BEGIN\n\nCERT::CERT(int a) : m(a) {}\n\nBSSL_NAMESPACE_END\n",
        );
        assert_eq!(
            sym(&namespaced, "src::ns::CERT::CERT").kind,
            SymbolKind::Method
        );
        assert!(!namespaced.symbols.iter().any(|s| s.name == "m"));
    }
}
