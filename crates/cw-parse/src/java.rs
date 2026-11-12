//! Java symbol and fact extraction.
//!
//! Written against the node kinds and field names published in the grammar's
//! own `node-types.json` (tree-sitter-java 0.23). No query files are used.

use std::collections::HashSet;

use cw_core::{Error, Result, SymbolKind, Visibility};
use tree_sitter::{Node, Parser};

use crate::lang::{
    Sink, block_doc, collapse, dotted, field_all, java_default_package, marker_name, named,
    split_qualified, starts_uppercase,
};
use crate::rust::ParsedFile;

/// Parse one Java source file. The module is the file's `package`, for
/// example `com::acme::text`; a file without one is named by its directory.
///
/// Imports are recorded as written; the scanner decides afterwards which of
/// them name repository packages.
pub fn parse_java(file: &str, source: &str) -> Result<ParsedFile> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .map_err(|e| Error::external("loading the Java grammar", e))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| Error::invalid(format!("parser produced no tree for {file}")))?;
    let root = tree.root_node();
    let src = source.as_bytes();
    let package = named(root)
        .into_iter()
        .find(|n| n.kind() == "package_declaration");
    let declared = package.and_then(|p| {
        named(p)
            .into_iter()
            .find(|n| matches!(n.kind(), "scoped_identifier" | "identifier"))
            .map(|n| dotted(n.utf8_text(src).unwrap_or("")))
    });
    let module = match &declared {
        Some(segments) if !segments.is_empty() => segments.join("::"),
        _ => java_default_package(file),
    };
    let mut ex = Extractor {
        sink: Sink::new(file, source, &module, root.has_error()),
        imported: HashSet::new(),
    };
    let (start, end) = package.map_or((1, 1), |p| {
        (
            p.start_position().row as u32 + 1,
            p.end_position().row as u32 + 1,
        )
    });
    ex.sink.module_symbol(
        format!("package {}", module.replace("::", ".")),
        String::new(),
        start,
        end,
    );
    let ctx = Ctx {
        scope: module,
        classes: Vec::new(),
        interface: false,
    };
    for child in named(root) {
        match child.kind() {
            "import_declaration" => ex.import(child),
            _ => ex.type_decl(child, &ctx),
        }
    }
    Ok(ex.sink.out)
}

#[derive(Clone)]
struct Ctx {
    /// Qualified name new symbols nest under: the package or enclosing type.
    scope: String,
    /// Enclosing types, outermost first, each with the names of its members.
    /// A bare name that is a member of one of them is resolved right here,
    /// which is exactly Java's own rule.
    classes: Vec<(String, HashSet<String>)>,
    /// Members of an interface are public unless marked otherwise.
    interface: bool,
}

#[derive(Default)]
struct Modifiers {
    keywords: Vec<String>,
    annotations: Vec<String>,
    /// Byte offset just past the modifiers.
    end: Option<usize>,
}

impl Modifiers {
    fn has(&self, keyword: &str) -> bool {
        self.keywords.iter().any(|k| k == keyword)
    }

    fn visibility(&self, interface: bool) -> Visibility {
        if self.has("public") {
            Visibility::Public
        } else if self.has("private") {
            Visibility::Private
        } else if interface && !self.has("protected") {
            Visibility::Public
        } else {
            // `protected` and package-private: visible inside the package.
            Visibility::Crate
        }
    }
}

struct Extractor<'a> {
    sink: Sink<'a>,
    /// Simple names bound by single-type and static imports.
    imported: HashSet<String>,
}

fn is_type_decl(kind: &str) -> bool {
    matches!(
        kind,
        "class_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration"
            | "annotation_type_declaration"
    )
}

impl<'a> Extractor<'a> {
    fn name_of(&self, node: Node) -> Option<&'a str> {
        node.child_by_field_name("name").map(|n| self.sink.text(n))
    }

    fn import(&mut self, node: Node) {
        let mut cursor = node.walk();
        let is_static = node.children(&mut cursor).any(|c| c.kind() == "static");
        let parts = named(node);
        let glob = parts.iter().any(|n| n.kind() == "asterisk");
        let Some(path) = parts
            .iter()
            .find(|n| matches!(n.kind(), "scoped_identifier" | "identifier"))
            .map(|n| dotted(self.sink.text(*n)))
        else {
            return;
        };
        if glob {
            return self.sink.import(path, "", true, false);
        }
        let Some(alias) = path.last().cloned() else {
            return;
        };
        if is_static && path.len() > 1 {
            // The file depends on the class even when the member it imports
            // is not an indexed symbol.
            self.sink
                .import(path[..path.len() - 1].to_vec(), "", false, false);
        }
        self.imported.insert(alias.clone());
        self.sink.import(path, &alias, false, false);
    }

    fn modifiers(&self, node: Node) -> Modifiers {
        let Some(list) = named(node).into_iter().find(|n| n.kind() == "modifiers") else {
            return Modifiers::default();
        };
        let mut out = Modifiers {
            end: Some(list.end_byte()),
            ..Modifiers::default()
        };
        let mut cursor = list.walk();
        for child in list.children(&mut cursor) {
            match child.kind() {
                "annotation" | "marker_annotation" => {
                    out.annotations.push(marker_name(self.sink.text(child)));
                }
                "line_comment" | "block_comment" => {}
                _ => out.keywords.push(self.sink.text(child).to_string()),
            }
        }
        out
    }

    /// Declaration text up to `end`, with annotations left out.
    fn signature(&self, node: Node, modifiers: &Modifiers, end: usize) -> String {
        let start = modifiers.end.unwrap_or_else(|| node.start_byte());
        let rest = self.sink.between(start, end);
        collapse(&format!("{} {rest}", modifiers.keywords.join(" ")))
    }

    /// The Javadoc comment directly above `node`.
    fn doc(&self, node: Node) -> String {
        node.prev_sibling()
            .filter(|p| p.kind() == "block_comment")
            .and_then(|p| block_doc(self.sink.text(p)))
            .unwrap_or_default()
    }

    /// Path of a type with its type arguments and annotations removed.
    fn type_path(&self, node: Node) -> Vec<String> {
        match node.kind() {
            "type_identifier" => vec![self.sink.text(node).to_string()],
            "scoped_type_identifier" => named(node)
                .into_iter()
                .flat_map(|part| self.type_path(part))
                .collect(),
            "generic_type" => named(node)
                .into_iter()
                .find(|n| matches!(n.kind(), "type_identifier" | "scoped_type_identifier"))
                .map(|n| self.type_path(n))
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// Segments of `A.b.c` when every part is a plain name.
    fn chain(&self, node: Node) -> Option<Vec<String>> {
        match node.kind() {
            "identifier" => Some(vec![self.sink.text(node).to_string()]),
            "field_access" => {
                let mut path = self.chain(node.child_by_field_name("object")?)?;
                let field = node
                    .child_by_field_name("field")
                    .filter(|f| f.kind() == "identifier")?;
                path.push(self.sink.text(field).to_string());
                Some(path)
            }
            _ => None,
        }
    }

    /// Make `path` absolute when its first segment is a member of an
    /// enclosing type, innermost first.
    fn qualify(&self, ctx: &Ctx, path: Vec<String>) -> Vec<String> {
        let Some(first) = path.first() else {
            return path;
        };
        match ctx.classes.iter().rev().find(|(_, m)| m.contains(first)) {
            Some((class, _)) => {
                let mut full = split_qualified(class);
                full.extend(path);
                full
            }
            None => path,
        }
    }

    /// Names declared directly in a type body.
    fn member_names(&self, body: Node, out: &mut HashSet<String>) {
        for member in named(body) {
            match member.kind() {
                "enum_body_declarations" => self.member_names(member, out),
                "field_declaration" | "constant_declaration" => {
                    for declarator in field_all(member, "declarator") {
                        out.extend(self.name_of(declarator).map(str::to_string));
                    }
                }
                "line_comment" | "block_comment" | "static_initializer" | "block" => {}
                _ => out.extend(self.name_of(member).map(str::to_string)),
            }
        }
    }

    fn type_decl(&mut self, node: Node, ctx: &Ctx) {
        let kind = match node.kind() {
            "class_declaration" | "record_declaration" => SymbolKind::Class,
            "interface_declaration" | "annotation_type_declaration" => SymbolKind::Interface,
            "enum_declaration" => SymbolKind::Enum,
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
        symbol.visibility = modifiers.visibility(ctx.interface);
        symbol.signature = signature;
        symbol.doc = doc;
        symbol.attrs = modifiers.annotations;
        let qualified = symbol.qualified.clone();

        let mut members = HashSet::new();
        if let Some(body) = body {
            self.member_names(body, &mut members);
        }
        let mut inner = ctx.clone();
        inner.scope = qualified.clone();
        inner.classes.push((qualified.clone(), members));
        inner.interface = kind == SymbolKind::Interface;

        for child in named(node) {
            if child.id() == name_node.id() || Some(child.id()) == body.map(|b| b.id()) {
                continue;
            }
            if matches!(
                child.kind(),
                "superclass" | "super_interfaces" | "extends_interfaces"
            ) {
                let mut types = named(child);
                if let Some(list) = types.iter().find(|n| n.kind() == "type_list") {
                    types = named(*list);
                }
                for ty in types {
                    let path = self.qualify(ctx, self.type_path(ty));
                    self.sink.inherits(&qualified, path);
                }
            }
            // Annotations, type parameters, supertypes, record components.
            self.walk(child, &qualified, &inner);
        }
        if let Some(body) = body {
            self.members(body, &inner);
        }
    }

    fn members(&mut self, body: Node, ctx: &Ctx) {
        for member in named(body) {
            match member.kind() {
                "line_comment" | "block_comment" => {}
                "method_declaration"
                | "constructor_declaration"
                | "compact_constructor_declaration"
                | "annotation_type_element_declaration" => self.method(member, ctx),
                "field_declaration" | "constant_declaration" => self.field(member, ctx),
                "enum_constant" => self.enum_constant(member, ctx),
                "enum_body_declarations" => self.members(member, ctx),
                kind if is_type_decl(kind) => self.type_decl(member, ctx),
                // Initializer blocks.
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
        let symbol = self.sink.symbol(&ctx.scope, name, SymbolKind::Method, node);
        symbol.visibility = modifiers.visibility(ctx.interface);
        symbol.signature = signature;
        symbol.doc = doc;
        symbol.attrs = modifiers.annotations;
        let qualified = symbol.qualified.clone();
        for child in named(node) {
            if child.id() != name_node.id() {
                self.walk(child, &qualified, ctx);
            }
        }
    }

    /// A `static final` field or an interface constant is a constant symbol;
    /// other fields only contribute the types and values they mention.
    fn field(&mut self, node: Node, ctx: &Ctx) {
        let modifiers = self.modifiers(node);
        let constant = node.kind() == "constant_declaration"
            || (modifiers.has("static") && modifiers.has("final"));
        let ty = node.child_by_field_name("type");
        let type_text = ty.map(|t| self.sink.text(t)).unwrap_or("");
        let doc = self.doc(node);
        for child in named(node) {
            if child.kind() == "modifiers" {
                let owner = ctx.scope.clone();
                self.walk(child, &owner, ctx);
            }
        }
        for declarator in field_all(node, "declarator") {
            let Some(name) = self.name_of(declarator) else {
                continue;
            };
            let owner = if constant {
                let signature = collapse(&format!(
                    "{} {type_text} {name}",
                    modifiers.keywords.join(" ")
                ));
                let symbol = self.sink.symbol(&ctx.scope, name, SymbolKind::Const, node);
                symbol.visibility = modifiers.visibility(ctx.interface);
                symbol.signature = signature;
                symbol.doc = doc.clone();
                symbol.attrs = modifiers.annotations.clone();
                symbol.qualified.clone()
            } else {
                ctx.scope.clone()
            };
            if let Some(ty) = ty {
                self.walk(ty, &owner, ctx);
            }
            if let Some(value) = declarator.child_by_field_name("value") {
                self.walk(value, &owner, ctx);
            }
        }
    }

    fn enum_constant(&mut self, node: Node, ctx: &Ctx) {
        let Some(name_node) = node.child_by_field_name("name") else {
            return;
        };
        let name = self.sink.text(name_node);
        let doc = self.doc(node);
        let symbol = self.sink.symbol(&ctx.scope, name, SymbolKind::Const, node);
        symbol.signature = name.to_string();
        symbol.doc = doc;
        let qualified = symbol.qualified.clone();
        for child in named(node) {
            if child.id() != name_node.id() {
                self.walk(child, &qualified, ctx);
            }
        }
    }

    fn children(&mut self, node: Node, owner: &str, ctx: &Ctx) {
        for child in named(node) {
            self.walk(child, owner, ctx);
        }
    }

    /// Record what an expression, statement or type mentions.
    ///
    /// Methods and variables live in separate namespaces in Java, so a bare
    /// lowercase name in an expression is a variable and never a method. Only
    /// capitalized names (types and constants, by universal convention) are
    /// recorded as value references.
    fn walk(&mut self, node: Node, owner: &str, ctx: &Ctx) {
        match node.kind() {
            "line_comment" | "block_comment" => {}
            "type_identifier" => {
                let path = self.qualify(ctx, vec![self.sink.text(node).to_string()]);
                self.sink.type_ref(owner, path);
            }
            "scoped_type_identifier" => {
                let path = self.qualify(ctx, self.type_path(node));
                self.sink.type_ref(owner, path);
            }
            "method_invocation" => self.invocation(node, owner, ctx),
            "object_creation_expression" => {
                let path = node
                    .child_by_field_name("type")
                    .map(|t| self.qualify(ctx, self.type_path(t)))
                    .unwrap_or_default();
                // The constructor is the method named after its class.
                if let Some(last) = path.last().cloned() {
                    let mut constructor = path;
                    constructor.push(last);
                    self.sink.call(owner, constructor, false, None);
                }
                self.children(node, owner, ctx);
            }
            "field_access" => match self.chain(node) {
                Some(chain) => {
                    if starts_uppercase(&chain[0]) {
                        let root = self.qualify(ctx, vec![chain[0].clone()]);
                        self.sink.value(owner, root);
                        let full = self.qualify(ctx, chain);
                        self.sink.value(owner, full);
                    }
                }
                None => {
                    if let Some(object) = node.child_by_field_name("object") {
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
        let Some(name) = self.name_of(node).map(str::to_string) else {
            return;
        };
        match node.child_by_field_name("object") {
            // A bare call names a member of an enclosing type or a static
            // import. Inherited methods are not followed.
            None => {
                let path = self.qualify(ctx, vec![name]);
                self.sink.call(owner, path, false, None);
            }
            Some(object) if object.kind() == "this" => {
                let class = ctx.classes.last().map(|(q, _)| q.as_str());
                self.sink.call(owner, vec![name], true, class);
            }
            Some(object) => {
                match self.chain(object) {
                    // `Type.name(...)`: a path through a type.
                    Some(chain)
                        if starts_uppercase(&chain[0]) || self.imported.contains(&chain[0]) =>
                    {
                        let mut path = self.qualify(ctx, chain);
                        path.push(name);
                        self.sink.call(owner, path, false, None);
                    }
                    // A receiver of unknown type, `super` included.
                    _ => self.sink.call(owner, vec![name], true, None),
                }
                self.walk(object, owner, ctx);
            }
        }
        for field in ["type_arguments", "arguments"] {
            if let Some(part) = node.child_by_field_name(field) {
                self.walk(part, owner, ctx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = r#"package com.acme.app;

import com.acme.text.Label;
import com.acme.text.*;
import static com.acme.text.Words.shout;
import java.util.List;
import org.junit.Test;

/**
 * The application.
 *
 * Second paragraph.
 */
@Deprecated
public final class App extends Base implements Runnable, Named<String> {
    public static final int LIMIT = 3;
    private Label label;

    /** Creates the app. */
    public App(Label label) {
        this.label = label;
    }

    @Override
    public void run() {
        String text = shout(name());
        Label made = new Label(text);
        this.helper(made, LIMIT);
        made.show();
        Words.pad(text, Color.RED);
        Inner.make();
    }

    protected String name() {
        return "app";
    }

    void helper(Label l, int n) {}

    private static class Inner extends Label {
        static Inner make() {
            return new Inner();
        }
    }
}

interface Named<T> extends Shape {
    int MAX = 10;
    T name();
}

enum Color implements Named<String> {
    RED, GREEN;

    public String name() { return "c"; }
}

record Point(int x, Label label) {}
"#;

    fn parsed() -> ParsedFile {
        parse_java("src/main/java/com/acme/app/App.java", SOURCE).unwrap()
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
        let a = "com::acme::app";

        let package = symbol(&p, a);
        assert_eq!(package.kind, SymbolKind::Module);
        assert_eq!(package.signature, "package com.acme.app");
        assert_eq!((package.start_line, package.end_line), (1, 1));

        let app = symbol(&p, &format!("{a}::App"));
        assert_eq!(app.kind, SymbolKind::Class);
        assert_eq!(app.visibility, Visibility::Public);
        assert_eq!(app.doc, "The application.\n\nSecond paragraph.");
        assert_eq!(app.attrs, ["Deprecated"]);
        assert_eq!(
            app.signature,
            "public final class App extends Base implements Runnable, Named<String>"
        );
        assert_eq!(app.parent.as_deref(), Some(a));

        let limit = symbol(&p, &format!("{a}::App::LIMIT"));
        assert_eq!(limit.kind, SymbolKind::Const);
        assert_eq!(limit.signature, "public static final int LIMIT");
        assert!(p.symbols.iter().all(|s| s.name != "label"));

        let ctor = symbol(&p, &format!("{a}::App::App"));
        assert_eq!(ctor.kind, SymbolKind::Method);
        assert_eq!(ctor.doc, "Creates the app.");
        assert_eq!(ctor.signature, "public App(Label label)");

        let run = symbol(&p, &format!("{a}::App::run"));
        assert_eq!(run.attrs, ["Override"]);
        assert_eq!(run.signature, "public void run()");
        assert_eq!(run.end_line - run.start_line, 8);
        assert_eq!(
            symbol(&p, &format!("{a}::App::name")).visibility,
            Visibility::Crate
        );
        assert_eq!(
            symbol(&p, &format!("{a}::App::helper")).visibility,
            Visibility::Crate
        );

        let inner = symbol(&p, &format!("{a}::App::Inner"));
        assert_eq!(inner.visibility, Visibility::Private);
        assert_eq!(
            symbol(&p, &format!("{a}::App::Inner::make")).kind,
            SymbolKind::Method
        );

        let named_iface = symbol(&p, &format!("{a}::Named"));
        assert_eq!(named_iface.kind, SymbolKind::Interface);
        assert_eq!(named_iface.visibility, Visibility::Crate);
        assert_eq!(named_iface.signature, "interface Named<T> extends Shape");
        // Interface members are public without saying so.
        let method = symbol(&p, &format!("{a}::Named::name"));
        assert_eq!(method.visibility, Visibility::Public);
        assert_eq!(method.signature, "T name()");
        assert_eq!(
            symbol(&p, &format!("{a}::Named::MAX")).kind,
            SymbolKind::Const
        );

        assert_eq!(symbol(&p, &format!("{a}::Color")).kind, SymbolKind::Enum);
        assert_eq!(
            symbol(&p, &format!("{a}::Color::RED")).kind,
            SymbolKind::Const
        );
        assert_eq!(
            symbol(&p, &format!("{a}::Color::name")).kind,
            SymbolKind::Method
        );
        let point = symbol(&p, &format!("{a}::Point"));
        assert_eq!(point.kind, SymbolKind::Class);
        assert_eq!(point.signature, "record Point(int x, Label label)");
    }

    #[test]
    fn extracts_imports() {
        let p = parsed();
        let has = |path: &[&str], alias: &str, glob: bool| {
            p.uses
                .iter()
                .any(|u| u.path == path && u.alias == alias && u.glob == glob)
        };
        assert!(has(&["com", "acme", "text", "Label"], "Label", false));
        assert!(has(&["com", "acme", "text"], "", true));
        assert!(has(
            &["com", "acme", "text", "Words", "shout"],
            "shout",
            false
        ));
        assert!(has(&["com", "acme", "text", "Words"], "", false));
        assert!(has(&["java", "util", "List"], "List", false));
        assert!(has(&["org", "junit", "Test"], "Test", false));
        assert!(p.uses.iter().all(|u| u.scope == "com::acme::app"));
    }

    #[test]
    fn extracts_calls_and_inheritance() {
        let p = parsed();
        let run = "com::acme::app::App::run";
        let call = |caller: &str, path: &[&str]| {
            p.calls
                .iter()
                .find(|c| c.caller == caller && c.path == path)
                .unwrap_or_else(|| panic!("missing call {caller} -> {path:?}"))
        };
        // A statically imported method stays a bare name.
        assert!(!call(run, &["shout"]).method);
        // A member of the enclosing class is named in full.
        assert!(!call(run, &["com", "acme", "app", "App", "name"]).method);
        assert!(!call(run, &["Label", "Label"]).method);
        let on_this = call(run, &["helper"]);
        assert!(on_this.method);
        assert_eq!(on_this.self_type.as_deref(), Some("com::acme::app::App"));
        let on_local = call(run, &["show"]);
        assert!(on_local.method && on_local.self_type.is_none());
        assert!(!call(run, &["Words", "pad"]).method);
        // A nested type is a member too.
        assert!(!call(run, &["com", "acme", "app", "App", "Inner", "make"]).method);
        assert!(
            !call(
                "com::acme::app::App::Inner::make",
                &["com", "acme", "app", "App", "Inner", "Inner"]
            )
            .method
        );

        let value = |path: &[&str]| p.value_refs.iter().any(|v| v.from == run && v.path == path);
        assert!(value(&["com", "acme", "app", "App", "LIMIT"]));
        assert!(value(&["Color", "RED"]));
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
            [vec!["Base"], vec!["Runnable"], vec!["Named"]]
        );
        assert_eq!(supers(&["App", "Inner"]), [vec!["Label"]]);
        assert_eq!(supers(&["Named"]), [vec!["Shape"]]);
        assert_eq!(supers(&["Color"]), [vec!["Named"]]);

        assert!(
            p.type_refs
                .iter()
                .any(|t| t.from == run && t.path == ["Label"])
        );
        assert!(
            p.type_refs
                .iter()
                .any(|t| t.from == "com::acme::app::Point" && t.path == ["Label"])
        );
    }

    #[test]
    fn names_a_file_without_a_package_by_its_directory() {
        let p = parse_java("tools/Gen.java", "class Gen { void run() {} }").unwrap();
        assert_eq!(symbol(&p, "tools::Gen").kind, SymbolKind::Class);
        assert_eq!(symbol(&p, "tools::Gen::run").kind, SymbolKind::Method);
    }
}
