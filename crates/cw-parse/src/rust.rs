//! Rust symbol and fact extraction.
//!
//! Written against the node kinds and field names published in the grammar's
//! own `node-types.json` (tree-sitter-rust 0.24). No query files are used.

use cw_core::{Error, Result, Symbol, SymbolKind, Visibility};
use tree_sitter::{Node, Parser};

/// One name brought into scope by a `use` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseImport {
    /// Module the `use` appears in.
    pub scope: String,
    /// Path segments as written, for example `["crate", "hash", "sha256_hex"]`.
    pub path: Vec<String>,
    /// Name bound in scope; empty for glob imports.
    pub alias: String,
    pub glob: bool,
    pub public: bool,
}

/// A call found in a function or method body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallSite {
    /// Qualified name of the calling function or method.
    pub caller: String,
    /// Module the caller lives in; name lookup starts here.
    pub module: String,
    /// Type named by `Self` at the call site, if the caller is a method.
    pub self_type: Option<String>,
    /// Path segments of the callee; one segment for a bare or method call.
    pub path: Vec<String>,
    /// True for `receiver.name(...)`.
    pub method: bool,
}

/// `impl Trait for Type` or an inherent `impl Type`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplFact {
    pub module: String,
    pub type_path: Vec<String>,
    pub trait_path: Option<Vec<String>>,
}

/// `mod name;` (file module) or `mod name { .. }` (inline).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModDecl {
    pub parent: String,
    pub name: String,
    pub inline: bool,
}

/// A type mentioned inside a symbol's declaration or body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRef {
    pub from: String,
    pub module: String,
    pub path: Vec<String>,
}

/// A name used as a value, or called inside a macro's arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueRef {
    pub from: String,
    pub module: String,
    pub self_type: Option<String>,
    pub path: Vec<String>,
    /// True when the name is followed by an argument list.
    pub call: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedFile {
    pub symbols: Vec<Symbol>,
    pub uses: Vec<UseImport>,
    pub calls: Vec<CallSite>,
    pub impls: Vec<ImplFact>,
    pub mods: Vec<ModDecl>,
    pub type_refs: Vec<TypeRef>,
    pub value_refs: Vec<ValueRef>,
    /// True when the parser reported syntax errors; facts may be incomplete.
    pub has_errors: bool,
}

/// Parse one Rust source file. `module_path` is the qualified module the file
/// defines, for example `cw_core::hash`.
pub fn parse_rust(file: &str, module_path: &str, source: &str) -> Result<ParsedFile> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .map_err(|e| Error::external("loading the Rust grammar", e))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| Error::invalid(format!("parser produced no tree for {file}")))?;
    let root = tree.root_node();
    let mut ex = Extractor {
        file,
        src: source.as_bytes(),
        out: ParsedFile {
            has_errors: root.has_error(),
            ..ParsedFile::default()
        },
        value_seen: std::collections::HashSet::new(),
    };
    let ctx = Ctx {
        scope: module_path.to_string(),
        module: module_path.to_string(),
        self_type: None,
        in_trait_or_impl: false,
        inherited_public: false,
    };
    ex.visit_children(root, &ctx);
    Ok(ex.out)
}

#[derive(Clone)]
struct Ctx {
    /// Qualified name that new symbols are nested under.
    scope: String,
    /// Nearest enclosing module.
    module: String,
    self_type: Option<String>,
    in_trait_or_impl: bool,
    /// Trait items carry no visibility modifier; they take the trait's.
    inherited_public: bool,
}

struct Extractor<'a> {
    file: &'a str,
    src: &'a [u8],
    out: ParsedFile,
    /// Value references already recorded, to keep one per owner and path.
    value_seen: std::collections::HashSet<(String, Vec<String>, bool)>,
}

impl<'a> Extractor<'a> {
    fn text(&self, node: Node) -> &'a str {
        node.utf8_text(self.src).unwrap_or("")
    }

    fn visit_children(&mut self, node: Node, ctx: &Ctx) {
        let mut cursor = node.walk();
        let children: Vec<Node> = node.named_children(&mut cursor).collect();
        for child in children {
            self.visit_item(child, ctx);
        }
    }

    fn visit_item(&mut self, node: Node, ctx: &Ctx) {
        match node.kind() {
            "function_item" | "function_signature_item" => self.function(node, ctx),
            "struct_item" => self.type_item(node, ctx, SymbolKind::Struct),
            "enum_item" => self.type_item(node, ctx, SymbolKind::Enum),
            "union_item" => self.type_item(node, ctx, SymbolKind::Union),
            "trait_item" => self.trait_item(node, ctx),
            "impl_item" => self.impl_item(node, ctx),
            "mod_item" => self.mod_item(node, ctx),
            "use_declaration" => self.use_declaration(node, ctx),
            "const_item" => self.simple_item(node, ctx, SymbolKind::Const),
            "static_item" => self.simple_item(node, ctx, SymbolKind::Static),
            "type_item" => self.simple_item(node, ctx, SymbolKind::TypeAlias),
            "macro_definition" => self.simple_item(node, ctx, SymbolKind::Macro),
            _ => {}
        }
    }

    fn name_of(&self, node: Node) -> Option<&'a str> {
        node.child_by_field_name("name").map(|n| self.text(n))
    }

    fn visibility(&self, node: Node, ctx: &Ctx) -> Visibility {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "visibility_modifier" {
                return if self.text(child).trim() == "pub" {
                    Visibility::Public
                } else {
                    Visibility::Crate
                };
            }
        }
        if ctx.inherited_public {
            Visibility::Public
        } else {
            Visibility::Private
        }
    }

    /// Leading `///` lines directly above the item, attributes skipped.
    fn doc(&self, node: Node) -> String {
        let mut lines: Vec<&str> = Vec::new();
        let mut prev = node.prev_sibling();
        while let Some(p) = prev {
            match p.kind() {
                "line_comment" => {
                    let t = self.text(p).trim_end();
                    match t.strip_prefix("///") {
                        Some(rest) if !rest.starts_with('/') => {
                            lines.push(rest.strip_prefix(' ').unwrap_or(rest));
                        }
                        _ => break,
                    }
                }
                "attribute_item" => {}
                _ => break,
            }
            prev = p.prev_sibling();
        }
        lines.reverse();
        lines.join("\n").trim().to_string()
    }

    /// Names of the attributes directly above the item, in source order.
    fn attrs(&self, node: Node) -> Vec<String> {
        let mut names = Vec::new();
        let mut prev = node.prev_sibling();
        while let Some(p) = prev {
            match p.kind() {
                "attribute_item" => {
                    let text = self.text(p);
                    let inner = text.trim_start_matches("#[").trim_end_matches(']');
                    let name: String = inner
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | ':'))
                        .collect();
                    if !name.is_empty() {
                        names.push(name);
                    }
                }
                "line_comment" | "block_comment" => {}
                _ => break,
            }
            prev = p.prev_sibling();
        }
        names.reverse();
        names
    }

    /// Declaration text up to the body, with whitespace collapsed.
    fn signature(&self, node: Node) -> String {
        let end = node
            .child_by_field_name("body")
            .map(|b| b.start_byte())
            .unwrap_or_else(|| node.end_byte());
        let raw = std::str::from_utf8(&self.src[node.start_byte()..end]).unwrap_or("");
        let mut sig = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        let sig_trimmed = sig.trim_end_matches([';', '{']).trim_end().len();
        sig.truncate(sig_trimmed);
        if sig.chars().count() > 300 {
            sig = sig.chars().take(297).collect::<String>() + "...";
        }
        sig
    }

    fn push_symbol(&mut self, node: Node, ctx: &Ctx, name: &str, kind: SymbolKind) -> String {
        let qualified = format!("{}::{}", ctx.scope, name);
        self.out.symbols.push(Symbol {
            qualified: qualified.clone(),
            name: name.to_string(),
            kind,
            visibility: self.visibility(node, ctx),
            file: self.file.to_string(),
            start_line: node.start_position().row as u32 + 1,
            end_line: node.end_position().row as u32 + 1,
            signature: self.signature(node),
            doc: self.doc(node),
            parent: Some(ctx.scope.clone()),
            attrs: self.attrs(node),
        });
        qualified
    }

    fn function(&mut self, node: Node, ctx: &Ctx) {
        let Some(name) = self.name_of(node) else {
            return;
        };
        let kind = if ctx.in_trait_or_impl {
            SymbolKind::Method
        } else {
            SymbolKind::Function
        };
        let qualified = self.push_symbol(node, ctx, name, kind);
        let inner = Ctx {
            scope: qualified.clone(),
            module: ctx.module.clone(),
            self_type: ctx.self_type.clone(),
            in_trait_or_impl: false,
            inherited_public: false,
        };
        let mut seen = Vec::new();
        for field in ["parameters", "return_type"] {
            if let Some(part) = node.child_by_field_name(field) {
                self.collect_refs(part, &qualified, &inner, &mut seen);
            }
        }
        if let Some(body) = node.child_by_field_name("body") {
            self.walk_body(body, &qualified, &inner, &mut seen);
        }
    }

    /// Walk a function body: record calls and type mentions, and extract
    /// items declared inside it under the function's own scope.
    fn walk_body(&mut self, node: Node, owner: &str, ctx: &Ctx, seen: &mut Vec<Vec<String>>) {
        let mut cursor = node.walk();
        let children: Vec<Node> = node.named_children(&mut cursor).collect();
        for child in children {
            match child.kind() {
                "function_item" | "struct_item" | "enum_item" | "union_item" | "trait_item"
                | "impl_item" | "mod_item" | "use_declaration" | "const_item" | "static_item"
                | "type_item" | "macro_definition" => {
                    self.visit_item(child, ctx);
                    continue;
                }
                "call_expression" => {
                    if let Some(f) = child.child_by_field_name("function") {
                        self.record_call(f, owner, ctx);
                    }
                }
                "type_identifier" | "scoped_type_identifier" => {
                    self.record_type_ref(child, owner, ctx, seen);
                    continue;
                }
                // A name used as a value: a function passed as a callback, a
                // constant, a static. Locals are recorded too and simply fail
                // to resolve.
                "identifier" | "scoped_identifier" => {
                    self.record_value(split_path(self.text(child)), false, owner, ctx);
                    continue;
                }
                "macro_invocation" => {
                    let mut cursor = child.walk();
                    let trees: Vec<Node> = child
                        .children(&mut cursor)
                        .filter(|n| n.kind() == "token_tree")
                        .collect();
                    for tree in trees {
                        self.scan_tokens(tree, owner, ctx);
                    }
                    continue;
                }
                _ => {}
            }
            self.walk_body(child, owner, ctx, seen);
        }
    }

    fn record_value(&mut self, path: Vec<String>, call: bool, owner: &str, ctx: &Ctx) {
        if path.is_empty()
            || !self
                .value_seen
                .insert((owner.to_string(), path.clone(), call))
        {
            return;
        }
        self.out.value_refs.push(ValueRef {
            from: owner.to_string(),
            module: ctx.module.clone(),
            self_type: ctx.self_type.clone(),
            path,
            call,
        });
    }

    /// Macro arguments are an unparsed token tree, so calls such as
    /// `format!("{}", helper(x))` are invisible to the expression walk.
    /// Rebuild paths from the tokens: `a :: b` followed by `( .. )` is a call.
    fn scan_tokens(&mut self, tree: Node, owner: &str, ctx: &Ctx) {
        let mut cursor = tree.walk();
        let tokens: Vec<Node> = tree.children(&mut cursor).collect();
        let mut path: Vec<String> = Vec::new();
        let mut joining = false;
        for token in tokens {
            match token.kind() {
                "identifier" | "crate" | "self" | "super" => {
                    if !joining && !path.is_empty() {
                        self.record_value(std::mem::take(&mut path), false, owner, ctx);
                    }
                    path.push(self.text(token).to_string());
                    joining = false;
                }
                "string_literal" | "raw_string_literal" => {
                    if !path.is_empty() {
                        self.record_value(std::mem::take(&mut path), false, owner, ctx);
                    }
                    joining = false;
                    for name in format_captures(self.text(token)) {
                        self.record_value(vec![name], false, owner, ctx);
                    }
                }
                "::" => joining = !path.is_empty(),
                "token_tree" => {
                    let call = self.text(token).starts_with('(');
                    if !path.is_empty() {
                        self.record_value(std::mem::take(&mut path), call, owner, ctx);
                    }
                    joining = false;
                    self.scan_tokens(token, owner, ctx);
                }
                _ => {
                    if !path.is_empty() {
                        self.record_value(std::mem::take(&mut path), false, owner, ctx);
                    }
                    joining = false;
                }
            }
        }
        if !path.is_empty() {
            self.record_value(path, false, owner, ctx);
        }
    }

    fn record_call(&mut self, function: Node, owner: &str, ctx: &Ctx) {
        let (path, method) = match function.kind() {
            "identifier" => (vec![self.text(function).to_string()], false),
            "scoped_identifier" => (split_path(self.text(function)), false),
            "field_expression" => match function.child_by_field_name("field") {
                Some(f) if f.kind() == "field_identifier" => (vec![self.text(f).to_string()], true),
                _ => return,
            },
            "generic_function" => {
                if let Some(inner) = function.child_by_field_name("function") {
                    self.record_call(inner, owner, ctx);
                }
                return;
            }
            _ => return,
        };
        if path.is_empty() {
            return;
        }
        self.out.calls.push(CallSite {
            caller: owner.to_string(),
            module: ctx.module.clone(),
            self_type: ctx.self_type.clone(),
            path,
            method,
        });
    }

    fn record_type_ref(&mut self, node: Node, owner: &str, ctx: &Ctx, seen: &mut Vec<Vec<String>>) {
        let path = split_path(self.text(node));
        let Some(last) = path.last() else {
            return;
        };
        // Single uppercase letters are almost always generic parameters.
        if last == "Self" || (last.len() == 1 && last.chars().all(|c| c.is_ascii_uppercase())) {
            return;
        }
        if seen.contains(&path) {
            return;
        }
        seen.push(path.clone());
        self.out.type_refs.push(TypeRef {
            from: owner.to_string(),
            module: ctx.module.clone(),
            path,
        });
    }

    /// Record every type mentioned under `node`.
    fn collect_refs(&mut self, node: Node, owner: &str, ctx: &Ctx, seen: &mut Vec<Vec<String>>) {
        if node.kind() == "attribute_item" {
            // `#[serde(default = "default_limit")]` names a function in a string.
            for name in quoted_paths(self.text(node)) {
                self.record_value(name, false, owner, ctx);
            }
            return;
        }
        if matches!(node.kind(), "type_identifier" | "scoped_type_identifier") {
            self.record_type_ref(node, owner, ctx, seen);
            return;
        }
        let mut cursor = node.walk();
        let children: Vec<Node> = node.named_children(&mut cursor).collect();
        for child in children {
            self.collect_refs(child, owner, ctx, seen);
        }
    }

    fn type_item(&mut self, node: Node, ctx: &Ctx, kind: SymbolKind) {
        let Some(name) = self.name_of(node) else {
            return;
        };
        let qualified = self.push_symbol(node, ctx, name, kind);
        if let Some(body) = node.child_by_field_name("body") {
            let mut seen = vec![vec![name.to_string()]];
            self.collect_refs(body, &qualified, ctx, &mut seen);
        }
    }

    fn simple_item(&mut self, node: Node, ctx: &Ctx, kind: SymbolKind) {
        let Some(name) = self.name_of(node) else {
            return;
        };
        let kind = if ctx.in_trait_or_impl && kind == SymbolKind::Const {
            SymbolKind::Const
        } else {
            kind
        };
        let qualified = self.push_symbol(node, ctx, name, kind);
        if let Some(ty) = node.child_by_field_name("type") {
            let mut seen = vec![vec![name.to_string()]];
            self.collect_refs(ty, &qualified, ctx, &mut seen);
        }
    }

    fn trait_item(&mut self, node: Node, ctx: &Ctx) {
        let Some(name) = self.name_of(node) else {
            return;
        };
        let public = self.visibility(node, ctx) == Visibility::Public;
        let qualified = self.push_symbol(node, ctx, name, SymbolKind::Trait);
        if let Some(body) = node.child_by_field_name("body") {
            let inner = Ctx {
                scope: qualified.clone(),
                module: ctx.module.clone(),
                self_type: Some(qualified),
                in_trait_or_impl: true,
                inherited_public: public,
            };
            self.visit_children(body, &inner);
        }
    }

    fn impl_item(&mut self, node: Node, ctx: &Ctx) {
        let Some(ty) = node.child_by_field_name("type") else {
            return;
        };
        let type_path = self.type_path(ty);
        let Some(type_name) = type_path.last().cloned() else {
            return;
        };
        let trait_path = node
            .child_by_field_name("trait")
            .map(|t| self.type_path(t))
            .filter(|p| !p.is_empty());
        let is_trait_impl = trait_path.is_some();
        self.out.impls.push(ImplFact {
            module: ctx.module.clone(),
            type_path,
            trait_path,
        });
        if let Some(body) = node.child_by_field_name("body") {
            // Methods are named under the type, as callers write them.
            let scope = format!("{}::{}", ctx.module, type_name);
            let inner = Ctx {
                scope: scope.clone(),
                module: ctx.module.clone(),
                self_type: Some(scope),
                in_trait_or_impl: true,
                // Trait impl methods have no modifier but are reachable
                // wherever the trait is.
                inherited_public: is_trait_impl,
            };
            self.visit_children(body, &inner);
        }
    }

    /// Path of a type expression with generic arguments and references removed.
    fn type_path(&self, node: Node) -> Vec<String> {
        match node.kind() {
            "type_identifier" | "identifier" | "scoped_type_identifier" | "scoped_identifier" => {
                split_path(self.text(node))
            }
            "generic_type" => node
                .child_by_field_name("type")
                .map(|t| self.type_path(t))
                .unwrap_or_default(),
            "reference_type" | "pointer_type" => node
                .child_by_field_name("type")
                .map(|t| self.type_path(t))
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    fn mod_item(&mut self, node: Node, ctx: &Ctx) {
        let Some(name) = self.name_of(node) else {
            return;
        };
        // Unit-test modules are not part of the documented design, and their
        // helper functions would otherwise attract unrelated call edges.
        if matches!(name, "tests" | "test") {
            return;
        }
        let body = node.child_by_field_name("body");
        self.out.mods.push(ModDecl {
            parent: ctx.scope.clone(),
            name: name.to_string(),
            inline: body.is_some(),
        });
        if let Some(body) = body {
            let qualified = self.push_symbol(node, ctx, name, SymbolKind::Module);
            let inner = Ctx {
                scope: qualified.clone(),
                module: qualified,
                self_type: None,
                in_trait_or_impl: false,
                inherited_public: false,
            };
            self.visit_children(body, &inner);
        }
    }

    fn use_declaration(&mut self, node: Node, ctx: &Ctx) {
        let public = self.visibility(node, ctx) != Visibility::Private;
        if let Some(arg) = node.child_by_field_name("argument") {
            self.flatten_use(arg, &[], &ctx.module, public);
        }
    }

    fn flatten_use(&mut self, node: Node, prefix: &[String], scope: &str, public: bool) {
        match node.kind() {
            "identifier" | "crate" | "self" | "super" | "scoped_identifier" => {
                let mut path = prefix.to_vec();
                path.extend(split_path(self.text(node)));
                // `use a::b::{self}` binds `b`.
                if path.last().is_some_and(|s| s == "self") && path.len() > 1 {
                    path.pop();
                }
                let alias = path.last().cloned().unwrap_or_default();
                self.out.uses.push(UseImport {
                    scope: scope.to_string(),
                    path,
                    alias,
                    glob: false,
                    public,
                });
            }
            "use_as_clause" => {
                let Some(p) = node.child_by_field_name("path") else {
                    return;
                };
                let mut path = prefix.to_vec();
                path.extend(split_path(self.text(p)));
                let alias = node
                    .child_by_field_name("alias")
                    .map(|a| self.text(a).to_string())
                    .unwrap_or_default();
                self.out.uses.push(UseImport {
                    scope: scope.to_string(),
                    path,
                    alias,
                    glob: false,
                    public,
                });
            }
            "use_wildcard" => {
                let mut path = prefix.to_vec();
                let text = self.text(node);
                path.extend(split_path(
                    text.trim_end_matches('*').trim_end_matches("::"),
                ));
                self.out.uses.push(UseImport {
                    scope: scope.to_string(),
                    path,
                    alias: String::new(),
                    glob: true,
                    public,
                });
            }
            "scoped_use_list" => {
                let mut path = prefix.to_vec();
                if let Some(p) = node.child_by_field_name("path") {
                    path.extend(split_path(self.text(p)));
                }
                if let Some(list) = node.child_by_field_name("list") {
                    self.flatten_use(list, &path, scope, public);
                }
            }
            "use_list" => {
                let mut cursor = node.walk();
                let children: Vec<Node> = node.named_children(&mut cursor).collect();
                for child in children {
                    self.flatten_use(child, prefix, scope, public);
                }
            }
            _ => {}
        }
    }
}

/// Identifiers captured inline by a format string: `{name}`, `{name:>5}`.
fn format_captures(literal: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = literal;
    while let Some(open) = rest.find('{') {
        rest = &rest[open + 1..];
        if rest.starts_with('{') {
            rest = &rest[1..];
            continue;
        }
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let after = rest[name.len()..].chars().next();
        if !name.is_empty()
            && !name.chars().next().is_some_and(|c| c.is_ascii_digit())
            && matches!(after, Some('}') | Some(':'))
        {
            out.push(name);
        }
    }
    out
}

/// Quoted strings in an attribute that look like paths: `"a::b"`.
fn quoted_paths(attribute: &str) -> Vec<Vec<String>> {
    attribute
        .split('"')
        .skip(1)
        .step_by(2)
        .filter(|s| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, '_' | ':'))
                && !s.chars().next().is_some_and(|c| c.is_ascii_digit())
        })
        .map(split_path)
        .collect()
}

/// Split `a::b::<T>::c` style text into plain segments.
fn split_path(text: &str) -> Vec<String> {
    let mut depth = 0usize;
    let mut cleaned = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            _ if depth == 0 && !c.is_whitespace() => cleaned.push(c),
            _ => {}
        }
    }
    cleaned
        .split("::")
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
use crate::hash::{sha256_hex, normalize_text as norm};
use std::collections::*;
pub use self::inner::Thing;

/// A widget.
/// Second line.
#[derive(Debug)]
pub struct Widget {
    size: Size,
    items: Vec<inner::Thing>,
}

pub(crate) enum Size { Small, Large }

pub trait Draw {
    fn draw(&self) -> String;
}

impl Draw for Widget {
    fn draw(&self) -> String {
        let h = sha256_hex("x");
        self.helper();
        crate::hash::normalize_text(&h)
    }
}

impl Widget {
    pub fn new() -> Self {
        Self::build::<u8>(Size::Small)
    }
    fn helper(&self) {}
    fn build<T>(size: Size) -> Self { todo!() }
}

mod inner {
    pub struct Thing;
    pub fn make() -> Thing { Thing }
}

mod sibling;

const LIMIT: usize = 3;
fn free(w: &Widget) -> usize { LIMIT }
"#;

    fn parsed() -> ParsedFile {
        parse_rust("src/lib.rs", "demo", SAMPLE).unwrap()
    }

    fn sym<'a>(p: &'a ParsedFile, q: &str) -> &'a Symbol {
        p.symbols
            .iter()
            .find(|s| s.qualified == q)
            .unwrap_or_else(|| panic!("missing symbol {q}"))
    }

    #[test]
    fn extracts_symbols_with_kinds_and_visibility() {
        let p = parsed();
        assert!(!p.has_errors);
        let w = sym(&p, "demo::Widget");
        assert_eq!(w.kind, SymbolKind::Struct);
        assert_eq!(w.visibility, Visibility::Public);
        assert_eq!(w.doc, "A widget.\nSecond line.");
        assert_eq!(w.signature, "pub struct Widget");
        assert_eq!(w.attrs, ["derive"]);
        assert!(sym(&p, "demo::Size").attrs.is_empty());
        assert_eq!(sym(&p, "demo::Size").visibility, Visibility::Crate);
        assert_eq!(sym(&p, "demo::Draw::draw").kind, SymbolKind::Method);
        assert_eq!(sym(&p, "demo::Draw::draw").visibility, Visibility::Public);
        assert_eq!(sym(&p, "demo::Widget::new").kind, SymbolKind::Method);
        assert_eq!(
            sym(&p, "demo::Widget::helper").visibility,
            Visibility::Private
        );
        assert_eq!(sym(&p, "demo::inner").kind, SymbolKind::Module);
        assert_eq!(sym(&p, "demo::inner::make").kind, SymbolKind::Function);
        assert_eq!(sym(&p, "demo::LIMIT").kind, SymbolKind::Const);
        assert_eq!(
            sym(&p, "demo::free").signature,
            "fn free(w: &Widget) -> usize"
        );
    }

    #[test]
    fn flattens_use_declarations() {
        let p = parsed();
        let find = |alias: &str| p.uses.iter().find(|u| u.alias == alias).unwrap();
        assert_eq!(find("sha256_hex").path, ["crate", "hash", "sha256_hex"]);
        assert_eq!(find("norm").path, ["crate", "hash", "normalize_text"]);
        assert!(find("Thing").public);
        let glob = p.uses.iter().find(|u| u.glob).unwrap();
        assert_eq!(glob.path, ["std", "collections"]);
    }

    #[test]
    fn records_calls_impls_and_mods() {
        let p = parsed();
        let calls_from = |caller: &str| -> Vec<(Vec<String>, bool)> {
            p.calls
                .iter()
                .filter(|c| c.caller == caller)
                .map(|c| (c.path.clone(), c.method))
                .collect()
        };
        let draw = calls_from("demo::Widget::draw");
        assert!(draw.contains(&(vec!["sha256_hex".into()], false)));
        assert!(draw.contains(&(vec!["helper".into()], true)));
        assert!(draw.contains(&(
            vec!["crate".into(), "hash".into(), "normalize_text".into()],
            false
        )));
        let new = calls_from("demo::Widget::new");
        assert_eq!(
            new,
            vec![(vec!["Self".to_string(), "build".to_string()], false)]
        );
        assert_eq!(
            p.calls
                .iter()
                .find(|c| c.caller == "demo::Widget::new")
                .unwrap()
                .self_type
                .as_deref(),
            Some("demo::Widget")
        );

        assert!(p.impls.iter().any(|i| i.type_path == ["Widget"]
            && i.trait_path.as_deref() == Some(&["Draw".to_string()][..])));
        assert!(
            p.impls
                .iter()
                .any(|i| i.type_path == ["Widget"] && i.trait_path.is_none())
        );

        assert!(p.mods.contains(&ModDecl {
            parent: "demo".into(),
            name: "inner".into(),
            inline: true
        }));
        assert!(p.mods.contains(&ModDecl {
            parent: "demo".into(),
            name: "sibling".into(),
            inline: false
        }));
    }

    #[test]
    fn records_type_references() {
        let p = parsed();
        let refs: Vec<&TypeRef> = p
            .type_refs
            .iter()
            .filter(|r| r.from == "demo::Widget")
            .collect();
        assert!(refs.iter().any(|r| r.path == ["Size"]));
        assert!(refs.iter().any(|r| r.path == ["inner", "Thing"]));
        assert!(
            p.type_refs
                .iter()
                .any(|r| r.from == "demo::free" && r.path == ["Widget"])
        );
    }

    #[test]
    fn records_value_uses_and_calls_inside_macros() {
        let src = "const LIMIT: u8 = 3;\nfn helper(x: u8) -> u8 { x }\nfn cb() {}\nfn user(list: Vec<u8>) -> String {\n    register(cb);\n    let n = LIMIT;\n    format!(\"{} {}\", helper(n), crate::other::name(1))\n}\n";
        let p = parse_rust("src/lib.rs", "demo", src).unwrap();
        let has = |path: &[&str], call: bool| {
            p.value_refs.iter().any(|r| {
                r.from == "demo::user"
                    && r.call == call
                    && r.path.iter().map(String::as_str).eq(path.iter().copied())
            })
        };
        assert!(has(&["cb"], false), "{:?}", p.value_refs);
        assert!(has(&["LIMIT"], false));
        assert!(has(&["helper"], true), "call inside format!");
        assert!(
            has(&["crate", "other", "name"], true),
            "scoped call inside format!"
        );
    }

    #[test]
    fn tolerates_broken_source() {
        let p = parse_rust("src/bad.rs", "demo::bad", "pub fn ok() {}\nfn broken( {").unwrap();
        assert!(p.has_errors);
        assert!(p.symbols.iter().any(|s| s.qualified == "demo::bad::ok"));
    }
}
