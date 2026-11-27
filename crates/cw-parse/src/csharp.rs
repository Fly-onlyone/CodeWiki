//! C# symbol and fact extraction.
//!
//! Written against the node kinds and field names published in the grammar's
//! own `node-types.json` (tree-sitter-c-sharp 0.24). No query files are used.
//!
//! Mapping, in short. A namespace is a module, so the files of one namespace
//! share one module, as the files of a Java package do. Symbols: a class and
//! a record are `Class`, a struct and a record struct `Struct`, an interface
//! `Interface`, an enum `Enum`, a delegate `TypeAlias`, a method, a
//! constructor and an operator-free member function `Method`, a constant, a
//! `static readonly` field and an enum member `Const`. Properties, events,
//! indexers, operators and plain fields are no symbols: the types and values
//! they mention count for their type, as a Java field does. Top-level
//! statements are the function `Main` of their namespace.

use std::collections::HashSet;

use cw_core::{Error, Result, SymbolKind, Visibility};
use tree_sitter::{Node, Parser};

use crate::lang::{
    Sink, block_doc, collapse, csharp_default_namespace, dotted, marker_name, split_qualified,
    starts_uppercase,
};
use crate::rust::ParsedFile;

/// Parse one C# source file. The module is the file's namespace, for example
/// `Acme::Text` for `namespace Acme.Text`; a file without one is named by its
/// directory.
///
/// A `global using` is recorded with an empty scope: the scanner applies it
/// to every namespace. Other imports are recorded as written; the scanner
/// decides afterwards which of them name repository namespaces.
pub fn parse_csharp(file: &str, source: &str) -> Result<ParsedFile> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
        .map_err(|e| Error::external("loading the C# grammar", e))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| Error::invalid(format!("parser produced no tree for {file}")))?;
    let root = tree.root_node();
    let items = flat(root);
    let first_namespace = items.iter().find(|n| is_namespace(n.kind())).copied();
    let src = source.as_bytes();
    let declared = first_namespace.map(|n| namespace_segments(n, src));
    let module = match &declared {
        Some(segments) if !segments.is_empty() => segments.join("::"),
        _ => csharp_default_namespace(file),
    };
    let mut ex = Extractor {
        sink: Sink::new(file, source, &module, root.has_error()),
        imported: HashSet::new(),
    };
    if first_namespace.is_none() {
        ex.module_symbol(&module, 1, 1);
    }
    let ctx = Ctx {
        scope: module.clone(),
        namespace: Vec::new(),
        classes: Vec::new(),
        member_default: Visibility::Crate,
    };
    ex.container(&items, &ctx);
    Ok(ex.sink.out)
}

fn is_namespace(kind: &str) -> bool {
    matches!(
        kind,
        "namespace_declaration" | "file_scoped_namespace_declaration"
    )
}

/// The named children of `node` with the name of the field each one is in.
fn fielded<'t>(node: Node<'t>) -> Vec<(Option<&'t str>, Node<'t>)> {
    (0..node.child_count())
        .filter_map(|i| {
            let child = node.child(i)?;
            child
                .is_named()
                .then(|| (node.field_name_for_child(i), child))
        })
        .collect()
}

/// The children of `node` that count as its items: the branches of a
/// conditional compilation block stand where the block stands, and the other
/// directives are left out.
fn flat(node: Node) -> Vec<Node> {
    let condition = node.child_by_field_name("condition").map(|c| c.id());
    let mut out = Vec::new();
    for child in crate::lang::named(node) {
        let kind = child.kind();
        if Some(child.id()) == condition {
            continue;
        }
        match kind {
            "preproc_if" | "preproc_elif" | "preproc_else" => out.extend(flat(child)),
            _ if kind.starts_with("preproc_") => {}
            _ => out.push(child),
        }
    }
    out
}

/// Segments of a namespace's name.
fn namespace_segments(node: Node, src: &[u8]) -> Vec<String> {
    node.child_by_field_name("name")
        .map(|n| dotted(n.utf8_text(src).unwrap_or("")))
        .unwrap_or_default()
}

#[derive(Clone)]
struct ClassScope {
    qualified: String,
    /// Members that are symbols, so that a bare name can be made absolute.
    members: HashSet<String>,
    /// Nested types: the only members a type name can mean.
    types: HashSet<String>,
}

#[derive(Clone)]
struct Ctx {
    /// Qualified name new symbols nest under: the namespace or enclosing type.
    scope: String,
    /// Segments of the namespace being read; empty without one.
    namespace: Vec<String>,
    /// Enclosing types, outermost first.
    classes: Vec<ClassScope>,
    /// What a declaration without an access modifier is.
    member_default: Visibility,
}

#[derive(Default)]
struct Modifiers {
    keywords: Vec<String>,
    attributes: Vec<String>,
    /// Byte offset just past the leading attributes and modifiers.
    end: Option<usize>,
}

impl Modifiers {
    fn has(&self, keyword: &str) -> bool {
        self.keywords.iter().any(|k| k == keyword)
    }

    fn visibility(&self, default: Visibility) -> Visibility {
        if self.has("public") {
            Visibility::Public
        } else if self.has("private") || self.has("file") {
            Visibility::Private
        } else if self.has("internal") || self.has("protected") {
            Visibility::Crate
        } else {
            default
        }
    }

    /// A `const` field, or a `static readonly` one, which acts as one.
    fn constant(&self) -> bool {
        self.has("const") || (self.has("static") && self.has("readonly"))
    }
}

struct Extractor<'a> {
    sink: Sink<'a>,
    /// Names bound by alias usings.
    imported: HashSet<String>,
}

impl<'a> Extractor<'a> {
    fn name_of(&self, node: Node) -> Option<&'a str> {
        node.child_by_field_name("name").map(|n| self.sink.text(n))
    }

    fn module_symbol(&mut self, module: &str, start: u32, end: u32) {
        let previous = std::mem::replace(&mut self.sink.module, module.to_string());
        self.sink.module_symbol(
            format!("namespace {}", module.replace("::", ".")),
            String::new(),
            start,
            end,
        );
        self.sink.module = previous;
    }

    /// The items of a compilation unit or of a namespace body.
    fn container(&mut self, items: &[Node], ctx: &Ctx) {
        let mut ctx = ctx.clone();
        let mut statements: Vec<Node> = Vec::new();
        for &item in items {
            match item.kind() {
                "using_directive" => self.using(item),
                "namespace_declaration" => {
                    let mut inner = ctx.clone();
                    inner
                        .namespace
                        .extend(namespace_segments(item, self.sink.src));
                    let module = inner.namespace.join("::");
                    let line = item.start_position().row as u32 + 1;
                    self.module_symbol(&module, line, line);
                    inner.scope = module.clone();
                    let outer = std::mem::replace(&mut self.sink.module, module);
                    if let Some(body) = item.child_by_field_name("body") {
                        self.container(&flat(body), &inner);
                    }
                    self.sink.module = outer;
                }
                "file_scoped_namespace_declaration" => {
                    ctx.namespace
                        .extend(namespace_segments(item, self.sink.src));
                    let module = ctx.namespace.join("::");
                    let line = item.start_position().row as u32 + 1;
                    self.module_symbol(&module, line, line);
                    ctx.scope = module.clone();
                    self.sink.module = module;
                }
                "global_statement" => statements.push(item),
                kind if is_type_decl(kind) => self.type_decl(item, &ctx),
                _ => {}
            }
        }
        if let (Some(first), Some(last)) = (statements.first(), statements.last()) {
            let symbol = self
                .sink
                .symbol(&ctx.scope, "Main", SymbolKind::Function, *first);
            symbol.end_line = last.end_position().row as u32 + 1;
            symbol.visibility = Visibility::Crate;
            symbol.signature = "top-level statements".to_string();
            let owner = symbol.qualified.clone();
            for statement in statements {
                self.walk(statement, &owner, &ctx);
            }
        }
    }

    /// `using Ns;`, `using static Ns.Type;`, `using Alias = Ns.Type;` and
    /// `global` forms of each.
    fn using(&mut self, node: Node) {
        let mut cursor = node.walk();
        let tokens: Vec<&str> = node.children(&mut cursor).map(|c| c.kind()).collect();
        let global = tokens.contains(&"global");
        let alias = node.child_by_field_name("name");
        let target = crate::lang::named(node)
            .into_iter()
            .find(|n| Some(n.id()) != alias.map(|a| a.id()) && n.kind() != "comment");
        let Some(path) = target.map(|t| self.type_path(t)).filter(|p| !p.is_empty()) else {
            return;
        };
        match alias {
            Some(alias) => {
                let alias = self.sink.text(alias).to_string();
                self.imported.insert(alias.clone());
                self.sink.import(path, &alias, false, false);
            }
            None => self.sink.import(path, "", true, false),
        }
        if global && let Some(last) = self.sink.out.uses.last_mut() {
            last.scope = String::new();
        }
    }

    fn modifiers(&self, node: Node) -> Modifiers {
        let mut out = Modifiers::default();
        for (_, child) in fielded(node) {
            match child.kind() {
                "modifier" => out.keywords.push(self.sink.text(child).to_string()),
                "attribute_list" => {
                    for attribute in crate::lang::named(child) {
                        if attribute.kind() == "attribute"
                            && let Some(name) = attribute.child_by_field_name("name")
                        {
                            out.attributes.push(marker_name(self.sink.text(name)));
                        }
                    }
                }
                "comment" | "preproc_if_in_attribute_list" => continue,
                _ => break,
            }
            out.end = Some(child.end_byte());
        }
        out
    }

    /// Declaration text up to `end`, with attributes left out.
    fn signature(&self, node: Node, modifiers: &Modifiers, end: usize) -> String {
        let rest = self
            .sink
            .between(modifiers.end.unwrap_or_else(|| node.start_byte()), end);
        collapse(&format!("{} {rest}", modifiers.keywords.join(" ")))
    }

    /// The documentation comment lines directly above `node`.
    fn doc(&self, node: Node) -> String {
        let mut lines: Vec<String> = Vec::new();
        let mut next_row = node.start_position().row;
        let mut at = node.prev_sibling();
        while let Some(comment) = at.filter(|c| c.kind() == "comment") {
            // A comment that does not end on the line above is not about it.
            if comment.end_position().row + 1 < next_row {
                break;
            }
            let text = self.sink.text(comment).trim();
            if let Some(line) = text.strip_prefix("///") {
                lines.push(
                    line.strip_prefix(' ')
                        .unwrap_or(line)
                        .trim_end()
                        .to_string(),
                );
            } else if let Some(block) = block_doc(text) {
                lines.push(block);
                break;
            } else {
                break;
            }
            next_row = comment.start_position().row;
            at = comment.prev_sibling();
        }
        lines.reverse();
        xml_doc(&lines.join("\n"))
    }

    /// Path of a type with its type arguments removed.
    fn type_path(&self, node: Node) -> Vec<String> {
        match node.kind() {
            "identifier" => vec![self.sink.text(node).to_string()],
            "generic_name" => crate::lang::named(node)
                .into_iter()
                .find(|n| n.kind() == "identifier")
                .map(|n| vec![self.sink.text(n).to_string()])
                .unwrap_or_default(),
            "qualified_name" => {
                let mut path = node
                    .child_by_field_name("qualifier")
                    .map(|q| self.type_path(q))
                    .unwrap_or_default();
                if let Some(name) = node.child_by_field_name("name") {
                    path.extend(self.type_path(name));
                }
                path
            }
            // `global::System.Text`: the alias names the root, not a segment.
            "alias_qualified_name" => node
                .child_by_field_name("name")
                .map(|n| self.type_path(n))
                .unwrap_or_default(),
            "nullable_type" => node
                .child_by_field_name("type")
                .map(|n| self.type_path(n))
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// Segments of `A.b.c` when every part is a plain name.
    fn chain(&self, node: Node) -> Option<Vec<String>> {
        match node.kind() {
            "identifier" => Some(vec![self.sink.text(node).to_string()]),
            "generic_name" => {
                let path = self.type_path(node);
                (!path.is_empty()).then_some(path)
            }
            "member_access_expression" => {
                let mut path = self.chain(node.child_by_field_name("expression")?)?;
                path.extend(self.chain(node.child_by_field_name("name")?)?);
                Some(path)
            }
            _ => None,
        }
    }

    /// Make `path` absolute when its first segment is a member of an
    /// enclosing type, innermost first.
    fn qualify(&self, ctx: &Ctx, path: Vec<String>) -> Vec<String> {
        self.qualify_in(ctx, path, |c| &c.members)
    }

    /// The same for a type name, which only a nested type can be.
    fn qualify_type(&self, ctx: &Ctx, path: Vec<String>) -> Vec<String> {
        self.qualify_in(ctx, path, |c| &c.types)
    }

    fn qualify_in(
        &self,
        ctx: &Ctx,
        path: Vec<String>,
        names: impl Fn(&ClassScope) -> &HashSet<String>,
    ) -> Vec<String> {
        let Some(first) = path.first() else {
            return path;
        };
        match ctx.classes.iter().rev().find(|c| names(c).contains(first)) {
            Some(class) => {
                let mut full = split_qualified(&class.qualified);
                full.extend(path);
                full
            }
            None => path,
        }
    }

    /// Names declared directly in a type body that are symbols.
    fn member_names(&self, body: Node, members: &mut HashSet<String>, types: &mut HashSet<String>) {
        for member in flat(body) {
            match member.kind() {
                "method_declaration" | "enum_member_declaration" => {
                    members.extend(self.name_of(member).map(str::to_string));
                }
                kind if is_type_decl(kind) => {
                    let name = self.name_of(member).map(str::to_string);
                    types.extend(name.clone());
                    members.extend(name);
                }
                "field_declaration" if self.modifiers(member).constant() => {
                    for declarator in declarators(member) {
                        members.extend(self.name_of(declarator).map(str::to_string));
                    }
                }
                _ => {}
            }
        }
    }

    fn type_decl(&mut self, node: Node, ctx: &Ctx) {
        let kind = match node.kind() {
            "class_declaration" => SymbolKind::Class,
            "struct_declaration" => SymbolKind::Struct,
            "record_declaration" => {
                let mut cursor = node.walk();
                if node.children(&mut cursor).any(|c| c.kind() == "struct") {
                    SymbolKind::Struct
                } else {
                    SymbolKind::Class
                }
            }
            "interface_declaration" => SymbolKind::Interface,
            "enum_declaration" => SymbolKind::Enum,
            "delegate_declaration" => return self.delegate(node, ctx),
            _ => return,
        };
        let Some(name_node) = node.child_by_field_name("name") else {
            return;
        };
        let name = self.sink.text(name_node);
        let body = node.child_by_field_name("body");
        let modifiers = self.modifiers(node);
        let end = body.map_or(node.end_byte(), |b| b.start_byte());
        let signature = self.signature(node, &modifiers, end);
        let doc = self.doc(node);
        let symbol = self.sink.symbol(&ctx.scope, name, kind, node);
        symbol.visibility = modifiers.visibility(ctx.member_default);
        symbol.signature = signature;
        symbol.doc = doc;
        symbol.attrs = modifiers.attributes.clone();
        let qualified = symbol.qualified.clone();

        let mut scope = ClassScope {
            qualified: qualified.clone(),
            members: HashSet::new(),
            types: HashSet::new(),
        };
        if let Some(body) = body {
            self.member_names(body, &mut scope.members, &mut scope.types);
        }
        let mut inner = ctx.clone();
        inner.scope = qualified.clone();
        inner.classes.push(scope);
        inner.member_default = match kind {
            SymbolKind::Interface | SymbolKind::Enum => Visibility::Public,
            _ => Visibility::Private,
        };

        for (_, child) in fielded(node) {
            if child.id() == name_node.id() || Some(child.id()) == body.map(|b| b.id()) {
                continue;
            }
            match child.kind() {
                "base_list" => self.base_list(child, &qualified, ctx, &inner),
                "modifier" | "type_parameter_list" => {}
                // Attributes, primary constructor parameters, constraints.
                _ => self.walk(child, &qualified, &inner),
            }
        }
        if let Some(body) = body {
            self.members(&flat(body), &inner);
        }
    }

    /// The base type and interfaces a type lists.
    fn base_list(&mut self, list: Node, qualified: &str, outer: &Ctx, inner: &Ctx) {
        for item in crate::lang::named(list) {
            let ty = match item.kind() {
                "primary_constructor_base_type" => {
                    for part in crate::lang::named(item) {
                        if part.kind() == "argument_list" {
                            self.walk(part, qualified, inner);
                        }
                    }
                    item.child_by_field_name("type")
                }
                "argument_list" => {
                    self.walk(item, qualified, inner);
                    None
                }
                "comment" => None,
                _ => Some(item),
            };
            let Some(ty) = ty else {
                continue;
            };
            let path = self.qualify_type(outer, self.type_path(ty));
            self.sink.inherits(qualified, path);
            self.walk_type(ty, qualified, inner);
        }
    }

    fn delegate(&mut self, node: Node, ctx: &Ctx) {
        let Some(name_node) = node.child_by_field_name("name") else {
            return;
        };
        let name = self.sink.text(name_node);
        let modifiers = self.modifiers(node);
        let signature = self.signature(node, &modifiers, node.end_byte());
        let doc = self.doc(node);
        let symbol = self
            .sink
            .symbol(&ctx.scope, name, SymbolKind::TypeAlias, node);
        symbol.visibility = modifiers.visibility(ctx.member_default);
        symbol.signature = signature;
        symbol.doc = doc;
        symbol.attrs = modifiers.attributes;
        let qualified = symbol.qualified.clone();
        for (_, child) in fielded(node) {
            if child.id() != name_node.id() && child.kind() != "modifier" {
                self.walk(child, &qualified, ctx);
            }
        }
    }

    fn members(&mut self, items: &[Node], ctx: &Ctx) {
        for &member in items {
            match member.kind() {
                "comment" => {}
                "method_declaration" | "constructor_declaration" => self.method(member, ctx),
                "field_declaration" => self.field(member, ctx),
                "enum_member_declaration" => self.enum_member(member, ctx),
                "extension_declaration" => {
                    // C# 14 extension blocks: their members belong to the
                    // enclosing static class.
                    for part in crate::lang::named(member) {
                        if part.kind() == "extension_body" {
                            self.members(&flat(part), ctx);
                        }
                    }
                }
                kind if is_type_decl(kind) => self.type_decl(member, ctx),
                // Properties, events, indexers, operators, finalizers.
                _ => {
                    let owner = ctx.scope.clone();
                    self.walk(member, &owner, ctx);
                }
            }
        }
    }

    fn method(&mut self, node: Node, ctx: &Ctx) {
        let Some(name_node) = node.child_by_field_name("name") else {
            return;
        };
        let name = self.sink.text(name_node);
        let modifiers = self.modifiers(node);
        let end = node
            .child_by_field_name("body")
            .map_or(node.end_byte(), |b| b.start_byte());
        let signature = self.signature(node, &modifiers, end);
        let doc = self.doc(node);
        let explicit = crate::lang::named(node)
            .iter()
            .any(|n| n.kind() == "explicit_interface_specifier");
        let symbol = self.sink.symbol(&ctx.scope, name, SymbolKind::Method, node);
        symbol.visibility = if explicit {
            Visibility::Public
        } else {
            modifiers.visibility(ctx.member_default)
        };
        symbol.signature = signature;
        symbol.doc = doc;
        symbol.attrs = modifiers.attributes;
        let qualified = symbol.qualified.clone();
        for (field, child) in fielded(node) {
            match (field, child.kind()) {
                (Some("name"), _) | (_, "modifier" | "type_parameter_list") => {}
                (Some("returns"), _) => self.walk_type(child, &qualified, ctx),
                _ => self.walk(child, &qualified, ctx),
            }
        }
    }

    /// A `const` or `static readonly` field is a constant symbol; other
    /// fields only contribute the types and values they mention.
    fn field(&mut self, node: Node, ctx: &Ctx) {
        let modifiers = self.modifiers(node);
        let constant = modifiers.constant();
        let doc = self.doc(node);
        let owner = ctx.scope.clone();
        let mut declaration = None;
        for (_, child) in fielded(node) {
            match child.kind() {
                "attribute_list" => self.walk(child, &owner, ctx),
                "variable_declaration" => declaration = Some(child),
                _ => {}
            }
        }
        let Some(declaration) = declaration else {
            return;
        };
        let ty = declaration.child_by_field_name("type");
        let type_text = ty.map(|t| self.sink.text(t)).unwrap_or("");
        for declarator in declarators(node) {
            let Some(name) = self.name_of(declarator) else {
                continue;
            };
            let owner = if constant {
                let signature = collapse(&format!(
                    "{} {type_text} {name}",
                    modifiers.keywords.join(" ")
                ));
                let symbol = self.sink.symbol(&ctx.scope, name, SymbolKind::Const, node);
                symbol.visibility = modifiers.visibility(ctx.member_default);
                symbol.signature = signature;
                symbol.doc = doc.clone();
                symbol.attrs = modifiers.attributes.clone();
                symbol.qualified.clone()
            } else {
                ctx.scope.clone()
            };
            if let Some(ty) = ty {
                self.walk_type(ty, &owner, ctx);
            }
            self.children(declarator, &owner, ctx);
        }
    }

    fn enum_member(&mut self, node: Node, ctx: &Ctx) {
        let Some(name_node) = node.child_by_field_name("name") else {
            return;
        };
        let name = self.sink.text(name_node);
        let doc = self.doc(node);
        let modifiers = self.modifiers(node);
        let symbol = self.sink.symbol(&ctx.scope, name, SymbolKind::Const, node);
        symbol.signature = name.to_string();
        symbol.doc = doc;
        symbol.attrs = modifiers.attributes;
        let qualified = symbol.qualified.clone();
        for (field, child) in fielded(node) {
            if field != Some("name") {
                self.walk(child, &qualified, ctx);
            }
        }
    }

    /// Walk the children of `node`. A part in a `type` or `returns` field is
    /// a type; a `name` is a declared name and mentions nothing.
    fn children(&mut self, node: Node, owner: &str, ctx: &Ctx) {
        for (field, child) in fielded(node) {
            match field {
                Some("type" | "returns") => self.walk_type(child, owner, ctx),
                Some("name") => {}
                _ => self.walk(child, owner, ctx),
            }
        }
    }

    /// Record what a type written at `node` mentions.
    fn walk_type(&mut self, node: Node, owner: &str, ctx: &Ctx) {
        match node.kind() {
            "identifier" => {
                let path = self.qualify_type(ctx, vec![self.sink.text(node).to_string()]);
                self.sink.type_ref(owner, path);
            }
            "generic_name" => {
                let path = self.qualify_type(ctx, self.type_path(node));
                self.sink.type_ref(owner, path);
                for part in crate::lang::named(node) {
                    if part.kind() == "type_argument_list" {
                        for argument in crate::lang::named(part) {
                            self.walk_type(argument, owner, ctx);
                        }
                    }
                }
            }
            "qualified_name" => {
                let path = self.qualify_type(ctx, self.type_path(node));
                self.sink.type_ref(owner, path);
                // `Ns.List<Item>`: the arguments of any segment.
                let mut at = Some(node);
                while let Some(current) = at {
                    let name = current.child_by_field_name("name");
                    if let Some(name) = name.filter(|n| n.kind() == "generic_name") {
                        for part in crate::lang::named(name) {
                            if part.kind() == "type_argument_list" {
                                for argument in crate::lang::named(part) {
                                    self.walk_type(argument, owner, ctx);
                                }
                            }
                        }
                    }
                    at = current
                        .child_by_field_name("qualifier")
                        .filter(|q| q.kind() == "qualified_name");
                }
            }
            "alias_qualified_name" => {
                if let Some(name) = node.child_by_field_name("name") {
                    self.walk_type(name, owner, ctx);
                }
            }
            "nullable_type" | "array_type" | "ref_type" | "scoped_type" | "pointer_type" => {
                if let Some(inner) = node.child_by_field_name("type") {
                    self.walk_type(inner, owner, ctx);
                }
            }
            "tuple_type" => {
                for element in crate::lang::named(node) {
                    if let Some(inner) = element.child_by_field_name("type") {
                        self.walk_type(inner, owner, ctx);
                    }
                }
            }
            "predefined_type" | "implicit_type" | "function_pointer_type" | "comment" => {}
            _ => self.walk(node, owner, ctx),
        }
    }

    /// Record what an expression, statement or declaration mentions.
    ///
    /// A bare lowercase name in an expression is a variable, so only
    /// capitalized names (types, constants, members) are recorded as value
    /// references.
    fn walk(&mut self, node: Node, owner: &str, ctx: &Ctx) {
        match node.kind() {
            "comment" | "predefined_type" | "implicit_type" => {}
            "attribute" => {
                if let Some(name) = node.child_by_field_name("name") {
                    let path = self.qualify_type(ctx, self.type_path(name));
                    if let Some(last) = path.last().cloned() {
                        self.sink.type_ref(owner, path.clone());
                        let mut full = path;
                        full.pop();
                        full.push(format!("{last}Attribute"));
                        self.sink.type_ref(owner, full);
                    }
                }
                for part in crate::lang::named(node) {
                    if part.kind() == "attribute_argument_list" {
                        self.children(part, owner, ctx);
                    }
                }
            }
            "generic_name" | "qualified_name" | "alias_qualified_name" => {
                self.walk_type(node, owner, ctx);
            }
            "invocation_expression" => self.invocation(node, owner, ctx),
            "object_creation_expression" => {
                let class = node.child_by_field_name("type");
                let path = class
                    .map(|t| self.qualify_type(ctx, self.type_path(t)))
                    .unwrap_or_default();
                if let Some(class) = class {
                    self.walk_type(class, owner, ctx);
                    // The constructor is the method named after its class.
                    if let Some(last) = path.last().cloned() {
                        let mut constructor = path;
                        constructor.push(last);
                        self.sink.call(owner, constructor, false, None, class);
                    }
                }
                for (field, child) in fielded(node) {
                    if field != Some("type") {
                        self.walk(child, owner, ctx);
                    }
                }
            }
            "member_access_expression" => match self.chain(node) {
                Some(chain) => {
                    if starts_uppercase(&chain[0]) {
                        let root = self.qualify(ctx, vec![chain[0].clone()]);
                        self.sink.value(owner, root);
                        let full = self.qualify(ctx, chain);
                        self.sink.value(owner, full);
                    }
                }
                None => {
                    if let Some(object) = node.child_by_field_name("expression") {
                        self.walk(object, owner, ctx);
                    }
                }
            },
            "identifier" => {
                let name = self.sink.text(node);
                if starts_uppercase(name) {
                    let path = self.qualify(ctx, vec![name.to_string()]);
                    self.sink.value(owner, path);
                }
            }
            _ => self.children(node, owner, ctx),
        }
    }

    fn invocation(&mut self, node: Node, owner: &str, ctx: &Ctx) {
        let Some(function) = node.child_by_field_name("function") else {
            return;
        };
        match function.kind() {
            "identifier" | "generic_name" => {
                let name = self.type_path(function);
                // `nameof(X)` is no call; its argument is still read.
                if name != ["nameof"] {
                    let path = self.qualify(ctx, name);
                    self.sink.call(owner, path, false, None, function);
                }
                self.type_arguments(function, owner, ctx);
            }
            "member_access_expression" => self.member_call(function, owner, ctx),
            "conditional_access_expression" => {
                // `a?.B()` is the call of `.B` on `a`.
                if let Some(condition) = function.child_by_field_name("condition") {
                    self.walk(condition, owner, ctx);
                }
                for part in crate::lang::named(function) {
                    if part.kind() == "member_binding_expression"
                        && let Some(name) = part.child_by_field_name("name")
                        && let Some(path) = self.chain(name)
                    {
                        self.sink.call(owner, path, true, None, name);
                    }
                }
            }
            _ => self.walk(function, owner, ctx),
        }
        if let Some(arguments) = node.child_by_field_name("arguments") {
            self.walk(arguments, owner, ctx);
        }
    }

    /// The type arguments of a generic name used as a call.
    fn type_arguments(&mut self, name: Node, owner: &str, ctx: &Ctx) {
        if name.kind() != "generic_name" {
            return;
        }
        for part in crate::lang::named(name) {
            if part.kind() == "type_argument_list" {
                for argument in crate::lang::named(part) {
                    self.walk_type(argument, owner, ctx);
                }
            }
        }
    }

    /// `receiver.Name(...)`.
    fn member_call(&mut self, function: Node, owner: &str, ctx: &Ctx) {
        let (Some(receiver), Some(name_node)) = (
            function.child_by_field_name("expression"),
            function.child_by_field_name("name"),
        ) else {
            return;
        };
        let Some(name) = self.type_path(name_node).pop() else {
            return;
        };
        self.type_arguments(name_node, owner, ctx);
        match receiver.kind() {
            "this" | "this_expression" => {
                let class = ctx.classes.last().map(|c| c.qualified.as_str());
                self.sink.call(owner, vec![name], true, class, name_node);
            }
            // `string.Join`, `int.Parse`: the platform's own.
            "predefined_type" => {}
            _ => {
                match self.chain(receiver) {
                    // `Type.Name(...)`: a path through a type.
                    Some(chain)
                        if starts_uppercase(&chain[0]) || self.imported.contains(&chain[0]) =>
                    {
                        let mut path = self.qualify(ctx, chain);
                        path.push(name);
                        self.sink.call(owner, path, false, None, name_node);
                    }
                    // A receiver of unknown type, `base` included.
                    _ => self.sink.call(owner, vec![name], true, None, name_node),
                }
                self.walk(receiver, owner, ctx);
            }
        }
    }
}

fn is_type_decl(kind: &str) -> bool {
    matches!(
        kind,
        "class_declaration"
            | "struct_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration"
            | "delegate_declaration"
    )
}

/// The declarators of a field.
fn declarators(field: Node) -> Vec<Node> {
    crate::lang::named(field)
        .into_iter()
        .filter(|n| n.kind() == "variable_declaration")
        .flat_map(crate::lang::named)
        .filter(|n| n.kind() == "variable_declarator")
        .collect()
}

/// The text of an XML documentation comment as plain paragraphs, with the
/// `<param>`, `<returns>` and `<exception>` tags as the tag lines a Javadoc
/// has, which the note writer already turns into list items.
fn xml_doc(text: &str) -> String {
    let text = text.trim();
    if !text.contains('<') {
        return text.to_string();
    }
    let mut paragraphs: Vec<String> = Vec::new();
    let mut tags: Vec<String> = Vec::new();
    for (tag, attributes, inner) in xml_elements(text) {
        let body = inline_xml(&inner);
        if body.is_empty() {
            continue;
        }
        match tag.as_str() {
            "summary" | "remarks" => paragraphs.push(body),
            "param" => tags.push(format!("@param {} {body}", attribute(&attributes, "name"))),
            "returns" => tags.push(format!("@returns {body}")),
            "exception" => tags.push(format!(
                "@throws {} {body}",
                short_cref(&attribute(&attributes, "cref"))
            )),
            _ => {}
        }
    }
    if paragraphs.is_empty() && tags.is_empty() && !text.contains("<summary") {
        // Text with inline markup and no element of its own.
        return inline_xml(text);
    }
    let mut out = paragraphs.join("\n\n");
    // Tags with no prose over them say too little to list.
    if !tags.is_empty() && !out.is_empty() {
        out.push_str("\n\n");
        out.push_str(&tags.join("\n"));
    }
    out
}

/// The top-level elements of `text`: tag, attribute text and inner text.
fn xml_elements(text: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('>') else {
            break;
        };
        let head = &after[..close];
        let name: String = head
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect();
        if name.is_empty() || head.starts_with('/') {
            rest = &after[close + 1..];
            continue;
        }
        let attributes = head[name.len()..].trim().trim_end_matches('/').to_string();
        if head.ends_with('/') {
            rest = &after[close + 1..];
            continue;
        }
        let body_start = &after[close + 1..];
        let end_tag = format!("</{name}>");
        match body_start.find(&end_tag) {
            Some(end) => {
                out.push((name, attributes, body_start[..end].to_string()));
                rest = &body_start[end + end_tag.len()..];
            }
            None => rest = body_start,
        }
    }
    out
}

/// The value of attribute `name` in the text of a tag.
fn attribute(attributes: &str, name: &str) -> String {
    let key = format!("{name}=\"");
    attributes
        .find(&key)
        .and_then(|at| {
            let value = &attributes[at + key.len()..];
            value.find('"').map(|end| value[..end].to_string())
        })
        .unwrap_or_default()
}

/// A `cref` as the name a reader knows: `T:Acme.Oops` becomes `Oops`.
fn short_cref(cref: &str) -> String {
    let name = cref.split_once(':').map_or(cref, |(_, rest)| rest);
    let name = name.split('(').next().unwrap_or(name);
    name.rsplit('.')
        .next()
        .unwrap_or(name)
        .replace(['{', '}'], "")
}

/// Inline XML of a doc comment as Markdown text.
fn inline_xml(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('>') else {
            out.push_str(&rest[open..]);
            rest = "";
            break;
        };
        let head = &after[..close];
        let closing = head.starts_with('/');
        let name: String = head
            .trim_start_matches('/')
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect();
        let attributes = &head[head.find(&name).map_or(0, |at| at + name.len())..];
        match (name.as_str(), closing) {
            ("see" | "seealso", false) => {
                let target = ["cref", "langword", "href"]
                    .iter()
                    .map(|key| attribute(attributes, key))
                    .find(|v| !v.is_empty())
                    .unwrap_or_default();
                if head.ends_with('/') {
                    if !target.is_empty() {
                        out.push('`');
                        out.push_str(&short_cref(&target));
                        out.push('`');
                    }
                } else if let Some(end) = after[close + 1..].find("</see") {
                    let label = &after[close + 1..close + 1 + end];
                    out.push_str(label);
                    let skip = after[close + 1 + end..].find('>').unwrap_or(0);
                    rest = &after[close + 1 + end + skip + 1..];
                    continue;
                }
            }
            ("paramref" | "typeparamref", false) => {
                out.push('`');
                out.push_str(&attribute(attributes, "name"));
                out.push('`');
            }
            ("c", _) => out.push('`'),
            ("code", false) => out.push_str("\n```csharp\n"),
            ("code", true) => out.push_str("\n```\n"),
            ("para" | "br", _) if !closing => out.push_str("\n\n"),
            _ => {}
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    let out = out
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&");
    // One line per source line, blank lines between paragraphs.
    let mut lines: Vec<String> = Vec::new();
    for line in out.lines() {
        let line = line.trim();
        if line.is_empty() && lines.last().is_none_or(String::is_empty) {
            continue;
        }
        lines.push(line.to_string());
    }
    lines.join("\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = r#"using System;
using System.Collections.Generic;
using Acme.Text;
using static Acme.Text.Words;
using Pad = Acme.Text.Padding;
global using Xunit;

namespace Acme.App;

/// <summary>
/// The application.
/// </summary>
/// <remarks>Second paragraph.</remarks>
[Obsolete("old")]
public sealed class App : Base, IRunnable, INamed<string>
{
    public const int Limit = 3;
    private static readonly string Name = "app";
    private Label _label;

    public Label Label { get; set; }

    /// <summary>Creates the app.</summary>
    /// <param name="label">The label.</param>
    public App(Label label)
    {
        _label = label;
    }

    /// <summary>Runs <see cref="Helper"/> once.</summary>
    /// <returns>Nothing useful.</returns>
    /// <exception cref="T:System.InvalidOperationException">When it is running.</exception>
    public void Run()
    {
        var text = Shout(Describe());
        var made = new Label(text);
        this.Helper(made, Limit);
        made.Show();
        Words.Pad(text, Color.Red);
        Inner.Make();
        Console.WriteLine(string.Join(",", text));
    }

    protected string Describe() => "app";

    internal void Helper(Label l, int n) { }

    private class Inner : Label
    {
        public static Inner Make() => new Inner();
    }

    public static string Shorten(this string value, int n) => value;
}

interface INamed<T> : IShape
{
    int Max { get; }
    T Name();
}

public enum Color { Red, Green }

public readonly record struct Point(int X, int Y);

public record Person(string Name, Label Label) : Base(Name);

public struct Pair { public int A; }

public delegate void Handler(object sender);
"#;

    fn parsed() -> ParsedFile {
        parse_csharp("src/App/App.cs", SOURCE).unwrap()
    }

    fn symbol<'a>(p: &'a ParsedFile, q: &str) -> &'a cw_core::Symbol {
        p.symbols
            .iter()
            .find(|s| s.qualified == q)
            .unwrap_or_else(|| panic!("missing symbol {q}"))
    }

    #[test]
    fn extracts_symbols() {
        let p = parsed();
        assert!(!p.has_errors);
        let a = "Acme::App";

        let namespace = symbol(&p, a);
        assert_eq!(namespace.kind, SymbolKind::Module);
        assert_eq!(namespace.signature, "namespace Acme.App");
        assert_eq!((namespace.start_line, namespace.end_line), (8, 8));

        let app = symbol(&p, &format!("{a}::App"));
        assert_eq!(app.kind, SymbolKind::Class);
        assert_eq!(app.visibility, Visibility::Public);
        assert_eq!(app.doc, "The application.\n\nSecond paragraph.");
        assert_eq!(app.attrs, ["Obsolete"]);
        assert_eq!(
            app.signature,
            "public sealed class App : Base, IRunnable, INamed<string>"
        );
        assert_eq!(app.parent.as_deref(), Some(a));

        let limit = symbol(&p, &format!("{a}::App::Limit"));
        assert_eq!(limit.kind, SymbolKind::Const);
        assert_eq!(limit.signature, "public const int Limit");
        assert_eq!(
            symbol(&p, &format!("{a}::App::Name")).signature,
            "private static readonly string Name"
        );
        // Plain fields and properties are no symbols.
        assert!(p.symbols.iter().all(
            |s| s.name != "_label" && s.name != "Label" || s.qualified == format!("{a}::Label")
        ));

        let ctor = symbol(&p, &format!("{a}::App::App"));
        assert_eq!(ctor.kind, SymbolKind::Method);
        assert_eq!(ctor.doc, "Creates the app.\n\n@param label The label.");
        assert_eq!(ctor.signature, "public App(Label label)");

        let run = symbol(&p, &format!("{a}::App::Run"));
        assert_eq!(run.signature, "public void Run()");
        assert_eq!(
            run.doc,
            "Runs `Helper` once.\n\n@returns Nothing useful.\n@throws InvalidOperationException When it is running."
        );
        assert_eq!(run.end_line - run.start_line, 9);
        assert_eq!(
            symbol(&p, &format!("{a}::App::Describe")).visibility,
            Visibility::Crate
        );
        assert_eq!(
            symbol(&p, &format!("{a}::App::Helper")).visibility,
            Visibility::Crate
        );
        assert_eq!(
            symbol(&p, &format!("{a}::App::Shorten")).signature,
            "public static string Shorten(this string value, int n)"
        );

        let inner = symbol(&p, &format!("{a}::App::Inner"));
        assert_eq!(inner.visibility, Visibility::Private);
        assert_eq!(
            symbol(&p, &format!("{a}::App::Inner::Make")).kind,
            SymbolKind::Method
        );

        let named_iface = symbol(&p, &format!("{a}::INamed"));
        assert_eq!(named_iface.kind, SymbolKind::Interface);
        assert_eq!(named_iface.visibility, Visibility::Crate);
        assert_eq!(named_iface.signature, "interface INamed<T> : IShape");
        // Interface members are public without saying so.
        let method = symbol(&p, &format!("{a}::INamed::Name"));
        assert_eq!(method.visibility, Visibility::Public);
        assert_eq!(method.signature, "T Name()");

        assert_eq!(symbol(&p, &format!("{a}::Color")).kind, SymbolKind::Enum);
        assert_eq!(
            symbol(&p, &format!("{a}::Color::Red")).kind,
            SymbolKind::Const
        );
        let point = symbol(&p, &format!("{a}::Point"));
        assert_eq!(point.kind, SymbolKind::Struct);
        assert_eq!(
            point.signature,
            "public readonly record struct Point(int X, int Y)"
        );
        let person = symbol(&p, &format!("{a}::Person"));
        assert_eq!(person.kind, SymbolKind::Class);
        assert_eq!(
            person.signature,
            "public record Person(string Name, Label Label) : Base(Name)"
        );
        assert_eq!(symbol(&p, &format!("{a}::Pair")).kind, SymbolKind::Struct);
        let handler = symbol(&p, &format!("{a}::Handler"));
        assert_eq!(handler.kind, SymbolKind::TypeAlias);
        assert_eq!(
            handler.signature,
            "public delegate void Handler(object sender)"
        );
    }

    #[test]
    fn extracts_usings() {
        let p = parsed();
        let has = |path: &[&str], alias: &str, glob: bool, scope: &str| {
            p.uses
                .iter()
                .any(|u| u.path == path && u.alias == alias && u.glob == glob && u.scope == scope)
        };
        let ns = "Acme::App";
        assert!(has(&["System"], "", true, ns));
        assert!(has(&["System", "Collections", "Generic"], "", true, ns));
        assert!(has(&["Acme", "Text"], "", true, ns));
        // `using static` opens the members of a type.
        assert!(has(&["Acme", "Text", "Words"], "", true, ns));
        assert!(has(&["Acme", "Text", "Padding"], "Pad", false, ns));
        // A global using belongs to no namespace in particular.
        assert!(has(&["Xunit"], "", true, ""));
    }

    /// The names `caller` calls, in the order they stand in `source`.
    fn call_order(p: &ParsedFile, caller: &str, source: &str) -> Vec<String> {
        let mut calls: Vec<(u32, &str)> = p
            .calls
            .iter()
            .filter(|c| c.caller == caller)
            .map(|c| (c.at, c.path[c.path.len() - 1].as_str()))
            .collect();
        calls.sort();
        for (at, name) in &calls {
            assert!(source[*at as usize..].starts_with(name), "{name} at {at}");
        }
        calls
            .into_iter()
            .map(|(_, name)| name.to_string())
            .collect()
    }

    #[test]
    fn a_call_knows_where_it_stands() {
        let source = "namespace P;
class A {
  void Run(B x) {
    Third(Second(First()));
    x.Alpha().Beta();
    x?.Gamma();
  }
}
";
        let p = parse_csharp("src/P/A.cs", source).unwrap();
        // The outer call of a chain is recorded first and stands last.
        assert_eq!(
            call_order(&p, "P::A::Run", source),
            ["Third", "Second", "First", "Alpha", "Beta", "Gamma"]
        );
    }

    #[test]
    fn extracts_calls_and_inheritance() {
        let p = parsed();
        let run = "Acme::App::App::Run";
        let call = |caller: &str, path: &[&str]| {
            p.calls
                .iter()
                .find(|c| c.caller == caller && c.path == path)
                .unwrap_or_else(|| panic!("missing call {caller} -> {path:?}"))
        };
        // A bare call names a member of the enclosing class.
        assert!(!call(run, &["Acme", "App", "App", "Describe"]).method);
        // A name from `using static` stays a bare name.
        assert!(!call(run, &["Shout"]).method);
        assert!(!call(run, &["Label", "Label"]).method);
        let on_this = call(run, &["Helper"]);
        assert!(on_this.method);
        assert_eq!(on_this.self_type.as_deref(), Some("Acme::App::App"));
        let on_local = call(run, &["Show"]);
        assert!(on_local.method && on_local.self_type.is_none());
        assert!(!call(run, &["Words", "Pad"]).method);
        // A nested type is a member too.
        assert!(!call(run, &["Acme", "App", "App", "Inner", "Make"]).method);
        assert!(
            !call(
                "Acme::App::App::Inner::Make",
                &["Acme", "App", "App", "Inner", "Inner"]
            )
            .method
        );
        // `string.Join` belongs to the platform.
        assert!(
            !p.calls
                .iter()
                .any(|c| c.path.last().is_some_and(|n| n == "Join"))
        );
        assert!(!call(run, &["Console", "WriteLine"]).method);

        let value = |path: &[&str]| p.value_refs.iter().any(|v| v.from == run && v.path == path);
        assert!(value(&["Color", "Red"]));
        assert!(value(&["Acme", "App", "App", "Limit"]));
        assert!(!value(&["text"]) && !value(&["made"]));

        let supers = |ty: &[&str]| -> Vec<Vec<String>> {
            p.impls
                .iter()
                .filter(|i| i.type_path == ty)
                .filter_map(|i| i.trait_path.clone())
                .collect()
        };
        assert_eq!(
            supers(&["App"]),
            [vec!["Base"], vec!["IRunnable"], vec!["INamed"]]
        );
        assert_eq!(supers(&["App", "Inner"]), [vec!["Label"]]);
        assert_eq!(supers(&["INamed"]), [vec!["IShape"]]);
        assert_eq!(supers(&["Person"]), [vec!["Base"]]);

        let type_ref = |from: &str, path: &[&str]| {
            p.type_refs.iter().any(|t| t.from == from && t.path == path)
        };
        assert!(type_ref(run, &["Label"]));
        // A primary constructor's parameters and a property's type.
        assert!(type_ref("Acme::App::Person", &["Label"]));
        assert!(type_ref("Acme::App::App", &["Label"]));
        assert!(type_ref("Acme::App::App", &["ObsoleteAttribute"]));
    }

    #[test]
    fn names_a_file_without_a_namespace_by_its_directory() {
        let p = parse_csharp("tools/Gen.cs", "class Gen { void Run() {} }").unwrap();
        assert_eq!(symbol(&p, "tools").kind, SymbolKind::Module);
        assert_eq!(symbol(&p, "tools::Gen").kind, SymbolKind::Class);
        assert_eq!(symbol(&p, "tools::Gen::Run").kind, SymbolKind::Method);
        assert_eq!(
            symbol(&p, "tools::Gen").visibility,
            Visibility::Crate,
            "a top-level type is internal"
        );
    }

    #[test]
    fn reads_block_and_nested_namespaces() {
        let source = "using System;
namespace Acme.Core
{
    namespace Models
    {
        public class Order { }
    }
    public class Engine { public void Go() { Models.Order.Create(); } }
}
namespace Other { class Thing { } }
";
        let p = parse_csharp("src/Multi.cs", source).unwrap();
        assert_eq!(symbol(&p, "Acme::Core").kind, SymbolKind::Module);
        assert_eq!(symbol(&p, "Acme::Core::Models").kind, SymbolKind::Module);
        assert_eq!(
            symbol(&p, "Acme::Core::Models::Order").kind,
            SymbolKind::Class
        );
        assert_eq!(
            symbol(&p, "Acme::Core::Engine::Go").kind,
            SymbolKind::Method
        );
        assert_eq!(symbol(&p, "Other::Thing").kind, SymbolKind::Class);
        // A call keeps the module of the namespace it is written in.
        let call = p
            .calls
            .iter()
            .find(|c| c.caller == "Acme::Core::Engine::Go")
            .unwrap();
        assert_eq!(call.module, "Acme::Core");
        assert_eq!(call.path, ["Models", "Order", "Create"]);
        // A using before the namespaces belongs to the first of them.
        assert!(p.uses.iter().all(|u| u.scope == "Acme::Core"));
    }

    #[test]
    fn top_level_statements_are_the_function_main() {
        let source = "using Acme;
var app = new App();
app.Run();
Helper();
void Helper() { Log(); }
";
        let p = parse_csharp("src/App/Program.cs", source).unwrap();
        let main = symbol(&p, "src::App::Main");
        assert_eq!(main.kind, SymbolKind::Function);
        assert_eq!(main.signature, "top-level statements");
        assert_eq!((main.start_line, main.end_line), (2, 5));
        assert!(
            p.calls
                .iter()
                .any(|c| c.caller == "src::App::Main" && c.path == ["App", "App"])
        );
        assert!(
            p.calls
                .iter()
                .any(|c| c.caller == "src::App::Main" && c.path == ["Run"] && c.method)
        );
    }

    #[test]
    fn keeps_every_branch_of_conditional_compilation() {
        let source = "namespace P;
public class A {
#if DEBUG
    public void Debug() { }
#else
    public void Release() { }
#endif
}
";
        let p = parse_csharp("src/A.cs", source).unwrap();
        assert!(!p.has_errors);
        symbol(&p, "P::A::Debug");
        symbol(&p, "P::A::Release");
    }

    #[test]
    fn reads_extension_blocks_and_explicit_interface_members() {
        let source = "namespace P;
public static class Ext {
    extension(string s) {
        public int Twice() => s.Length * 2;
    }
}
class C : IDisposable {
    void IDisposable.Dispose() { }
}
";
        let p = parse_csharp("src/E.cs", source).unwrap();
        assert!(!p.has_errors);
        symbol(&p, "P::Ext::Twice");
        let dispose = symbol(&p, "P::C::Dispose");
        assert_eq!(dispose.visibility, Visibility::Public);
    }

    #[test]
    fn reads_doc_comments_of_every_form() {
        let source = "namespace P;
/** Block doc. */
class A { }
/// Plain text doc.
class B { }
// not documentation
class C { }
/// <summary>Detached.</summary>

class D { }
/// <summary>
/// Uses <c>x</c> and <paramref name=\"y\"/> &amp; <see langword=\"null\"/>.
/// </summary>
class E { }
";
        let p = parse_csharp("src/D.cs", source).unwrap();
        assert_eq!(symbol(&p, "P::A").doc, "Block doc.");
        assert_eq!(symbol(&p, "P::B").doc, "Plain text doc.");
        assert_eq!(symbol(&p, "P::C").doc, "");
        assert_eq!(symbol(&p, "P::D").doc, "");
        assert_eq!(symbol(&p, "P::E").doc, "Uses `x` and `y` & `null`.");
    }

    #[test]
    fn a_property_named_like_its_type_still_mentions_the_type() {
        let source = "namespace P;
public class Holder {
    public Order Order { get; set; }
    public void Use() { var o = Order.Create(); }
}
";
        let p = parse_csharp("src/H.cs", source).unwrap();
        assert!(
            p.type_refs
                .iter()
                .any(|t| t.from == "P::Holder" && t.path == ["Order"])
        );
        // The property is not a symbol, so `Order.Create` stays `Order`.
        assert!(p.calls.iter().any(|c| c.path == ["Order", "Create"]));
    }

    #[test]
    fn reads_generic_and_attribute_syntax() {
        let source = "namespace P;
public class Repo<T> where T : Entity, new() {
    [Fact]
    [Theory(Skip = \"x\")]
    public async Task<List<Item>> Load<TKey>(Dictionary<TKey, Item> by) {
        return await Fetch<Item>(by);
    }
    private Item Fetch<TItem>(object o) => null;
}
";
        let p = parse_csharp("src/R.cs", source).unwrap();
        assert!(!p.has_errors);
        let load = symbol(&p, "P::Repo::Load");
        assert_eq!(load.attrs, ["Fact", "Theory"]);
        assert_eq!(
            load.signature,
            "public async Task<List<Item>> Load<TKey>(Dictionary<TKey, Item> by)"
        );
        let refs = |path: &[&str]| {
            p.type_refs
                .iter()
                .any(|t| t.from == "P::Repo::Load" && t.path == path)
        };
        assert!(refs(&["Item"]) && refs(&["Task"]) && refs(&["List"]) && refs(&["Dictionary"]));
        assert!(
            p.type_refs
                .iter()
                .any(|t| t.from == "P::Repo" && t.path == ["Entity"])
        );
        assert!(
            p.calls
                .iter()
                .any(|c| c.caller == "P::Repo::Load" && c.path == ["P", "Repo", "Fetch"])
        );
    }
}
