//! Kotlin symbol and fact extraction.
//!
//! Written against the node kinds and field names published in the grammar's
//! own `node-types.json` (tree-sitter-kotlin-ng 1.1.0). No query files are
//! used. Kotlin is mapped at the level Java is, and the two share their
//! packages.
//!
//! How Kotlin maps onto the shared model:
//!
//! - **A package is a module**, written `package a.b.c` in the file, whatever
//!   folder the file stands in. A Java file of the same package names the
//!   same module, so a Java class and a Kotlin class of `com.acme.store` are
//!   siblings and resolve each other. A file with no `package` is in the
//!   default package of its folder, exactly as a Java file is.
//! - **Declarations.** A class, a data class, a sealed class and an inner
//!   class are [`SymbolKind::Class`]; an interface, a `fun interface` and an
//!   annotation class are [`SymbolKind::Interface`]; an `enum class` is a
//!   [`SymbolKind::Enum`] whose entries are [`SymbolKind::Const`]; a
//!   `typealias` is a [`SymbolKind::TypeAlias`]. An `object` is a class (its
//!   members are what Java calls static members), and a `companion object`
//!   is no symbol: its members belong to the class around it, so
//!   `Foo.create()` is `pkg::Foo::create` wherever `create` is written.
//! - **Top level.** A function at the top of a file is a
//!   [`SymbolKind::Function`] of its package; one inside a type is a
//!   [`SymbolKind::Method`]. A top-level property, and a property of an
//!   `object` or a companion, is a symbol too: [`SymbolKind::Const`] for a
//!   `val` and a `const val`, [`SymbolKind::Static`] for a `var`. A property
//!   of a class, the ones its primary constructor declares with `val` or
//!   `var` included, is no symbol, as a Java field is none: the type it
//!   names counts as a reference of its class.
//! - **Extension functions** (`fun String.shout()`) are functions or
//!   methods of the place they are written in, never of the receiver. They
//!   carry the attribute [`EXTENSION_ATTR`], which lets a call on a receiver
//!   of unknown type (`name.shout()`) find them as it finds methods.
//! - **Visibility.** `public` (the default) is [`Visibility::Public`],
//!   `internal` and `protected` are [`Visibility::Crate`], `private` is
//!   [`Visibility::Private`]. Annotations, without their arguments, are the
//!   symbol's `attrs`. The documentation is the KDoc comment (`/** */`)
//!   directly above.
//! - **Inheritance.** The supertype list after `:`, a class with its
//!   constructor call and an interface alike, is the edge Java gives to
//!   `extends` and `implements`. A class that extends one of the Kotest
//!   spec styles carries [`KOTEST_ATTR`]: such a class is a test without an
//!   annotation.
//! - **Imports.** `import a.b.C`, `import a.b.*` and `import a.b.C as D`. A
//!   `Companion` segment of an import path is left out, since the companion
//!   is no symbol. The platform (`java.*`, `javax.*`, `kotlin.*`) is not an
//!   integration; `kotlinx.*` and every other outside root is a library.
//! - **Calls.** `foo()`, `Type.foo()`, `x.foo()` (a receiver of unknown
//!   type), `this.foo()`, a call through an import alias, a call with named
//!   arguments, a call with a trailing lambda (the lambda does not change
//!   the callee) and `Foo(1)`, a call whose name is a type: that is a
//!   construction, a reference to `Foo` and a call of its constructor. A
//!   bare name that is a member of an enclosing type is that member. The
//!   default imports (`kotlin.*`, `kotlin.collections.*`, `kotlin.io.*`...)
//!   are not imports of the file, so `listOf` or `println` is a bare name
//!   that no repository function answers to unless the package or an import
//!   of the file has one: that is Kotlin's own rule.
//! - **Not followed.** Names bound inside a function (a lowercase name in an
//!   expression is a variable, as in Java), inherited members, a call
//!   through a typed receiver, and a top-level function called from Java as
//!   `UtilsKt.shout()`.
//! - **A grammar limit.** `fun interface I { ... }` makes the grammar lose
//!   the rest of the file, so the parser hands the grammar a copy of the
//!   source in which that `fun` is blanked out (same length, so every
//!   position still stands). Other constructs the grammar cannot read make
//!   the file report syntax errors; what could be read is still extracted,
//!   and the members of an error node are taken as members of the scope
//!   they stand in.

use std::borrow::Cow;
use std::collections::HashSet;

use cw_core::{Error, Result, SymbolKind, Visibility};
use tree_sitter::{Node, Parser};

use crate::lang::{
    Sink, block_doc, collapse, dotted, java_default_package, named, sanitize, starts_uppercase,
};
use crate::rust::ParsedFile;

/// The attribute of an extension function or property.
pub const EXTENSION_ATTR: &str = "extension";

/// The attribute of a class that extends one of the Kotest spec styles.
pub const KOTEST_ATTR: &str = "kotest";

/// The spec styles of Kotest. A class that extends one is a test, though
/// nothing on it says so.
const KOTEST_SPECS: [&str; 10] = [
    "AnnotationSpec",
    "BehaviorSpec",
    "DescribeSpec",
    "ExpectSpec",
    "FeatureSpec",
    "FreeSpec",
    "FunSpec",
    "ShouldSpec",
    "StringSpec",
    "WordSpec",
];

/// Parse one Kotlin source file (`.kt`, or a `.kts` script that is no Gradle
/// build file). The module is the file's `package`, for example
/// `com::acme::text`; a file without one is named by its directory.
///
/// Imports are recorded as written; the scanner decides afterwards which of
/// them name repository packages.
pub fn parse_kotlin(file: &str, source: &str) -> Result<ParsedFile> {
    let (text, blanked) = blank_fun_interfaces(source);
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_kotlin_ng::LANGUAGE.into())
        .map_err(|e| Error::external("loading the Kotlin grammar", e))?;
    let tree = parser
        .parse(text.as_ref(), None)
        .ok_or_else(|| Error::invalid(format!("parser produced no tree for {file}")))?;
    let root = tree.root_node();
    let package = named(root)
        .into_iter()
        .find(|n| n.kind() == "package_header");
    let declared: Vec<String> = package
        .and_then(|p| {
            named(p)
                .into_iter()
                .find(|n| n.kind() == "qualified_identifier")
        })
        .map(|q| {
            dotted(
                &q.utf8_text(source.as_bytes())
                    .unwrap_or("")
                    .replace('`', ""),
            )
            .iter()
            .map(|s| sanitize(s))
            .collect()
        })
        .unwrap_or_default();
    let module = if declared.is_empty() {
        java_default_package(file)
    } else {
        declared.join("::")
    };
    let mut ex = Extractor {
        sink: Sink::new(file, source, &module, root.has_error()),
        imported: HashSet::new(),
        blanked,
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
        level: Level::File,
    };
    for child in named(root) {
        match child.kind() {
            "package_header" | "file_annotation" | "shebang" => {}
            "import" => ex.import(child),
            _ => ex.declaration(child, &ctx),
        }
    }
    Ok(ex.sink.out)
}

/// The source with the `fun` of every `fun interface` blanked out, and where
/// those keywords stand. The grammar reads a `fun interface` with a body as
/// a function and loses the rest of the file; without the keyword it reads
/// an ordinary interface. Blank bytes keep every position.
fn blank_fun_interfaces(source: &str) -> (Cow<'_, str>, Vec<usize>) {
    let mut at: Vec<usize> = Vec::new();
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        let mut rest = line.trim_start();
        let mut pos = offset + (line.len() - rest.len());
        loop {
            let word = rest.split_whitespace().next().unwrap_or("");
            if matches!(
                word,
                "public" | "internal" | "private" | "protected" | "expect" | "actual"
            ) {
                let after = rest[word.len()..].trim_start();
                pos += rest.len() - after.len();
                rest = after;
            } else {
                break;
            }
        }
        if let Some(after) = rest.strip_prefix("fun")
            && after.starts_with(char::is_whitespace)
            && let Some(name) = after.trim_start().strip_prefix("interface")
            && name.starts_with(char::is_whitespace)
        {
            at.push(pos);
        }
        offset += line.len();
    }
    if at.is_empty() {
        return (Cow::Borrowed(source), at);
    }
    let mut bytes = source.as_bytes().to_vec();
    for pos in &at {
        bytes[*pos..*pos + 3].fill(b' ');
    }
    // Only ASCII bytes were replaced by ASCII bytes.
    let text = String::from_utf8(bytes).unwrap_or_else(|_| source.to_string());
    (Cow::Owned(text), at)
}

/// Where a declaration stands, which decides what a function and a
/// property in it are.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Level {
    /// The top of a file: functions are functions, properties are symbols.
    File,
    /// A class, an interface or an enum: functions are methods, properties
    /// are no symbols.
    Class,
    /// An `object` or a companion: methods, and properties are symbols.
    Object,
}

#[derive(Clone)]
struct Ctx {
    /// Qualified name new symbols nest under: the package or enclosing type.
    scope: String,
    /// Enclosing types, outermost first, each with the names of its members.
    /// A bare name that is a member of one of them is resolved right here.
    classes: Vec<(String, HashSet<String>)>,
    level: Level,
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

    fn visibility(&self) -> Visibility {
        if self.has("private") {
            Visibility::Private
        } else if self.has("internal") || self.has("protected") {
            Visibility::Crate
        } else {
            Visibility::Public
        }
    }
}

struct Extractor<'a> {
    sink: Sink<'a>,
    /// Simple names bound by imports, aliases included.
    imported: HashSet<String>,
    /// Where the blanked `fun` of a `fun interface` stood.
    blanked: Vec<usize>,
}

impl<'a> Extractor<'a> {
    /// The name an identifier node gives: no backticks, and a name written
    /// in them (a test's sentence) made safe for a qualified name.
    fn clean(&self, node: Node) -> String {
        sanitize(self.sink.text(node).trim_matches('`'))
    }

    fn name_of(&self, node: Node) -> Option<String> {
        node.child_by_field_name("name").map(|n| self.clean(n))
    }

    fn import(&mut self, node: Node) {
        let parts = named(node);
        let Some(qualified) = parts.iter().find(|n| n.kind() == "qualified_identifier") else {
            return;
        };
        let mut cursor = node.walk();
        let glob = node.children(&mut cursor).any(|c| c.kind() == "*");
        let alias = parts.iter().find(|n| n.kind() == "identifier");
        let mut path: Vec<String> = named(*qualified)
            .into_iter()
            .filter(|n| n.kind() == "identifier")
            .map(|n| self.sink.text(n).trim_matches('`').to_string())
            .collect();
        // The companion is no symbol: its members are the class's.
        let last = path.len().saturating_sub(1);
        let mut at = 0;
        path.retain(|s| {
            at += 1;
            at - 1 == last || s != "Companion"
        });
        if glob {
            return self.sink.import(path, "", true, false);
        }
        let Some(name) = path.last().cloned() else {
            return;
        };
        let alias = alias.map_or(name, |a| self.sink.text(*a).trim_matches('`').to_string());
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
        for child in named(list) {
            match child.kind() {
                "annotation" => out.annotations.push(self.annotation_name(child)),
                "line_comment" | "block_comment" => {}
                _ => out.keywords.push(self.sink.text(child).to_string()),
            }
        }
        out
    }

    /// The name of an annotation as written, without its arguments and its
    /// use-site target: `Test`, `org.junit.Test`.
    fn annotation_name(&self, node: Node) -> String {
        for child in named(node) {
            match child.kind() {
                "user_type" => return self.type_path(child).join("."),
                "constructor_invocation" => {
                    if let Some(ty) = named(child).into_iter().find(|n| n.kind() == "user_type") {
                        return self.type_path(ty).join(".");
                    }
                }
                _ => {}
            }
        }
        crate::lang::marker_name(self.sink.text(node))
    }

    /// Where the text of a declaration without modifiers starts: its node,
    /// or the blanked `fun` of a `fun interface` before it.
    fn start_of(&self, node: Node) -> usize {
        let start = node.start_byte();
        self.blanked
            .iter()
            .copied()
            .find(|b| {
                b + 3 <= start
                    && self
                        .sink
                        .between(b + 3, start)
                        .chars()
                        .all(char::is_whitespace)
            })
            .unwrap_or(start)
    }

    /// Declaration text up to `end`, with annotations left out.
    fn signature(&self, node: Node, modifiers: &Modifiers, end: usize) -> String {
        let start = modifiers.end.unwrap_or_else(|| self.start_of(node));
        let rest = self.sink.between(start, end);
        // A parameter list written one to a line, with a trailing comma, is
        // one line of text: `class A(val x: Int)`.
        collapse(&format!("{} {rest}", modifiers.keywords.join(" ")))
            .replace("( ", "(")
            .replace(", )", ")")
            .replace(" )", ")")
    }

    /// The KDoc comment directly above `node`.
    fn doc(&self, node: Node) -> String {
        node.prev_sibling()
            .filter(|p| p.kind() == "block_comment")
            .and_then(|p| block_doc(self.sink.text(p)))
            .unwrap_or_default()
    }

    /// Path of a type with its type arguments left out.
    fn type_path(&self, node: Node) -> Vec<String> {
        named(node)
            .into_iter()
            .filter(|n| n.kind() == "identifier")
            .map(|n| self.sink.text(n).trim_matches('`').to_string())
            .collect()
    }

    /// Segments of `A.b.c` when every part is a plain name.
    fn chain(&self, node: Node) -> Option<Vec<String>> {
        match node.kind() {
            "identifier" => Some(vec![self.sink.text(node).trim_matches('`').to_string()]),
            "navigation_expression" => {
                let parts = named(node);
                let [receiver, name] = parts.as_slice() else {
                    return None;
                };
                if name.kind() != "identifier" {
                    return None;
                }
                let mut path = self.chain(*receiver)?;
                path.push(self.sink.text(*name).trim_matches('`').to_string());
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
                let mut full = crate::lang::split_qualified(class);
                full.extend(path);
                full
            }
            None => path,
        }
    }

    /// Names declared directly in a type body, those of a companion object
    /// included: they are members of the type around it.
    fn member_names(&self, body: Node, out: &mut HashSet<String>) {
        for member in named(body) {
            match member.kind() {
                "function_declaration" | "class_declaration" | "object_declaration" => {
                    out.extend(self.name_of(member));
                }
                "type_alias" => {
                    out.extend(member.child_by_field_name("type").map(|n| self.clean(n)));
                }
                "property_declaration" => {
                    for decl in variables(member) {
                        out.extend(first_identifier(decl).map(|n| self.clean(n)));
                    }
                }
                "enum_entry" => out.extend(first_identifier(member).map(|n| self.clean(n))),
                "companion_object" => {
                    for part in named(member) {
                        if part.kind() == "class_body" {
                            self.member_names(part, out);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// A declaration at the top of a file or in a type body.
    fn declaration(&mut self, node: Node, ctx: &Ctx) {
        match node.kind() {
            "class_declaration" | "object_declaration" => self.type_decl(node, ctx),
            "function_declaration" => self.function(node, ctx),
            "property_declaration" => self.property(node, ctx),
            "type_alias" => self.type_alias(node, ctx),
            "secondary_constructor" => self.constructor(node, ctx),
            "enum_entry" => self.enum_entry(node, ctx),
            "companion_object" => {
                let mut inner = ctx.clone();
                inner.level = Level::Object;
                let owner = ctx.scope.clone();
                for child in named(node) {
                    match child.kind() {
                        "class_body" => {
                            for member in named(child) {
                                self.declaration(member, &inner);
                            }
                        }
                        "modifiers" | "delegation_specifiers" => self.walk(child, &owner, ctx),
                        _ => {}
                    }
                }
            }
            "line_comment" | "block_comment" => {}
            // What the grammar could not place: its parts still stand where
            // the error does.
            "ERROR" => {
                for child in named(node) {
                    self.declaration(child, ctx);
                }
            }
            // Initializer blocks, and the statements of a script.
            _ => {
                let owner = ctx.scope.clone();
                self.walk(node, &owner, ctx);
            }
        }
    }

    fn type_decl(&mut self, node: Node, ctx: &Ctx) {
        let object = node.kind() == "object_declaration";
        let Some(name_node) = node.child_by_field_name("name") else {
            return;
        };
        let name = self.clean(name_node);
        let modifiers = self.modifiers(node);
        let mut cursor = node.walk();
        let interface = node.children(&mut cursor).any(|c| c.kind() == "interface");
        let kind = if object {
            SymbolKind::Class
        } else if interface || modifiers.has("annotation") {
            SymbolKind::Interface
        } else if modifiers.has("enum") {
            SymbolKind::Enum
        } else {
            SymbolKind::Class
        };
        let body = named(node)
            .into_iter()
            .find(|n| matches!(n.kind(), "class_body" | "enum_class_body"));
        let end = match body {
            Some(body) => body.start_byte(),
            // `class Spec : StringSpec({ ... })` has no body and ends in a
            // lambda that is no part of its signature.
            None => self.before_lambda(node),
        };
        let signature = self.signature(node, &modifiers, end);
        let doc = self.doc(node);
        let visibility = modifiers.visibility();
        let symbol = self.sink.symbol(&ctx.scope, &name, kind, node);
        symbol.visibility = visibility;
        symbol.signature = signature;
        symbol.doc = doc;
        symbol.attrs = modifiers.annotations;
        let qualified = symbol.qualified.clone();
        let at = self.sink.out.symbols.len() - 1;

        let mut members = HashSet::new();
        if let Some(body) = body {
            self.member_names(body, &mut members);
        }
        let mut inner = ctx.clone();
        inner.scope = qualified.clone();
        inner.classes.push((qualified.clone(), members));
        inner.level = if object { Level::Object } else { Level::Class };

        for child in named(node) {
            if child.id() == name_node.id() || Some(child.id()) == body.map(|b| b.id()) {
                continue;
            }
            if child.kind() == "delegation_specifiers" {
                for specifier in named(child) {
                    let Some(ty) = self.supertype(specifier) else {
                        continue;
                    };
                    let path = self.qualify(ctx, self.type_path(ty));
                    if path
                        .last()
                        .is_some_and(|last| KOTEST_SPECS.contains(&last.as_str()))
                        && !self.sink.out.symbols[at]
                            .attrs
                            .iter()
                            .any(|a| a == KOTEST_ATTR)
                    {
                        self.sink.out.symbols[at]
                            .attrs
                            .push(KOTEST_ATTR.to_string());
                    }
                    self.sink.inherits(&qualified, path);
                }
            }
            // Annotations, type parameters, the primary constructor, the
            // supertypes' arguments.
            self.walk(child, &qualified, &inner);
        }
        if let Some(body) = body {
            for member in named(body) {
                self.declaration(member, &inner);
            }
        }
    }

    /// The type a supertype entry names: `Base(args)`, `Named<T>`, and
    /// `Base by delegate`.
    fn supertype<'t>(&self, specifier: Node<'t>) -> Option<Node<'t>> {
        let part = named(specifier).into_iter().find(|n| {
            matches!(
                n.kind(),
                "user_type" | "constructor_invocation" | "explicit_delegation"
            )
        })?;
        match part.kind() {
            "user_type" => Some(part),
            _ => named(part).into_iter().find(|n| n.kind() == "user_type"),
        }
    }

    /// Where the text of a bodiless class ends: before the first lambda of
    /// its supertype arguments, else at the node's end.
    fn before_lambda(&self, node: Node) -> usize {
        for specifiers in named(node)
            .into_iter()
            .filter(|n| n.kind() == "delegation_specifiers")
        {
            for specifier in named(specifiers) {
                for part in named(specifier) {
                    for args in named(part)
                        .into_iter()
                        .filter(|n| n.kind() == "value_arguments")
                    {
                        if self.sink.text(args).contains('{') {
                            return args.start_byte();
                        }
                    }
                }
            }
        }
        node.end_byte()
    }

    fn type_alias(&mut self, node: Node, ctx: &Ctx) {
        let Some(name_node) = node.child_by_field_name("type") else {
            return;
        };
        let name = self.clean(name_node);
        let modifiers = self.modifiers(node);
        let signature = self.signature(node, &modifiers, node.end_byte());
        let doc = self.doc(node);
        let visibility = modifiers.visibility();
        let symbol = self
            .sink
            .symbol(&ctx.scope, &name, SymbolKind::TypeAlias, node);
        symbol.visibility = visibility;
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

    fn function(&mut self, node: Node, ctx: &Ctx) {
        let Some(name_node) = node.child_by_field_name("name") else {
            return;
        };
        let name = self.clean(name_node);
        let modifiers = self.modifiers(node);
        let end = named(node)
            .into_iter()
            .find(|n| n.kind() == "function_body")
            .map_or(node.end_byte(), |b| b.start_byte());
        let signature = self.signature(node, &modifiers, end);
        let doc = self.doc(node);
        // A `.` before the name is the receiver's: `fun String.shout()`.
        let mut cursor = node.walk();
        let extension = node
            .children(&mut cursor)
            .take_while(|c| c.id() != name_node.id())
            .any(|c| c.kind() == ".");
        let kind = if ctx.level == Level::File {
            SymbolKind::Function
        } else {
            SymbolKind::Method
        };
        let visibility = modifiers.visibility();
        let symbol = self.sink.symbol(&ctx.scope, &name, kind, node);
        symbol.visibility = visibility;
        symbol.signature = signature;
        symbol.doc = doc;
        symbol.attrs = modifiers.annotations;
        if extension {
            symbol.attrs.push(EXTENSION_ATTR.to_string());
        }
        let qualified = symbol.qualified.clone();
        for child in named(node) {
            if child.id() != name_node.id() {
                self.walk(child, &qualified, ctx);
            }
        }
    }

    /// `constructor(...)` of a class: a method named like the class.
    fn constructor(&mut self, node: Node, ctx: &Ctx) {
        let Some(class) = ctx.scope.rsplit("::").next().map(str::to_string) else {
            return;
        };
        let modifiers = self.modifiers(node);
        let end = named(node)
            .into_iter()
            .find(|n| matches!(n.kind(), "constructor_delegation_call" | "block"))
            .map_or(node.end_byte(), |n| n.start_byte());
        let signature = self.signature(node, &modifiers, end);
        let doc = self.doc(node);
        let visibility = modifiers.visibility();
        let symbol = self
            .sink
            .symbol(&ctx.scope, &class, SymbolKind::Method, node);
        symbol.visibility = visibility;
        symbol.signature = signature;
        symbol.doc = doc;
        symbol.attrs = modifiers.annotations;
        let qualified = symbol.qualified.clone();
        for child in named(node) {
            self.walk(child, &qualified, ctx);
        }
    }

    fn property(&mut self, node: Node, ctx: &Ctx) {
        let declared = variables(node);
        if ctx.level == Level::Class || declared.len() != 1 {
            // A property of a class is no symbol: only what it mentions
            // counts, for the class.
            let owner = ctx.scope.clone();
            for child in named(node) {
                self.walk(child, &owner, ctx);
            }
            return;
        }
        let Some(name_node) = first_identifier(declared[0]) else {
            return;
        };
        let name = self.clean(name_node);
        let modifiers = self.modifiers(node);
        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();
        let var = children.iter().any(|c| c.kind() == "var");
        let end = children
            .iter()
            .find(|c| matches!(c.kind(), "=" | "property_delegate" | "getter" | "setter"))
            .map_or(node.end_byte(), |c| c.start_byte());
        let signature = self.signature(node, &modifiers, end);
        let doc = self.doc(node);
        let extension = children
            .iter()
            .take_while(|c| c.id() != declared[0].id())
            .any(|c| c.kind() == ".");
        let kind = if var {
            SymbolKind::Static
        } else {
            SymbolKind::Const
        };
        let visibility = modifiers.visibility();
        let symbol = self.sink.symbol(&ctx.scope, &name, kind, node);
        symbol.visibility = visibility;
        symbol.signature = signature;
        symbol.doc = doc;
        symbol.attrs = modifiers.annotations;
        if extension {
            symbol.attrs.push(EXTENSION_ATTR.to_string());
        }
        let qualified = symbol.qualified.clone();
        for child in named(node) {
            self.walk(child, &qualified, ctx);
        }
    }

    fn enum_entry(&mut self, node: Node, ctx: &Ctx) {
        let Some(name_node) = first_identifier(node) else {
            return;
        };
        let name = self.clean(name_node);
        let doc = self.doc(node);
        let symbol = self.sink.symbol(&ctx.scope, &name, SymbolKind::Const, node);
        symbol.signature = name.clone();
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

    /// Walk the children of `node`, leaving out the first identifier among
    /// them: the name a declaration binds.
    fn children_without_binding(&mut self, node: Node, owner: &str, ctx: &Ctx) {
        let mut skipped = false;
        for child in named(node) {
            if !skipped && child.kind() == "identifier" {
                skipped = true;
                continue;
            }
            self.walk(child, owner, ctx);
        }
    }

    /// Record what an expression, statement or type mentions.
    ///
    /// Functions and variables live in other namespaces than types, so a
    /// lowercase name in an expression is a variable and never a repository
    /// function. Only capitalized names (types and constants, by convention)
    /// are recorded as value references.
    fn walk(&mut self, node: Node, owner: &str, ctx: &Ctx) {
        match node.kind() {
            "line_comment" | "block_comment" | "this_expression" => {}
            "user_type" => {
                let path = self.qualify(ctx, self.type_path(node));
                self.sink.type_ref(owner, path);
                for child in named(node) {
                    if child.kind() != "identifier" {
                        self.walk(child, owner, ctx);
                    }
                }
            }
            "call_expression" => self.call(node, owner, ctx),
            "binary_expression" => self.binary(node, owner, ctx),
            "infix_expression" => {
                let parts = named(node);
                if let [left, name, right] = parts.as_slice()
                    && name.kind() == "identifier"
                {
                    self.walk(*left, owner, ctx);
                    let called = self.sink.text(*name).trim_matches('`').to_string();
                    self.sink.call(owner, vec![called], true, None, *name);
                    self.walk(*right, owner, ctx);
                } else {
                    self.children(node, owner, ctx);
                }
            }
            "navigation_expression" => match self.chain(node) {
                Some(chain) => {
                    if starts_uppercase(&chain[0]) {
                        let root = self.qualify(ctx, vec![chain[0].clone()]);
                        self.sink.value(owner, root);
                        let full = self.qualify(ctx, chain);
                        self.sink.value(owner, full);
                    }
                }
                None => {
                    if let Some(receiver) = named(node).first() {
                        self.walk(*receiver, owner, ctx);
                    }
                }
            },
            "identifier" => {
                let name = self.sink.text(node).trim_matches('`');
                if starts_uppercase(name) {
                    let path = self.qualify(ctx, vec![name.to_string()]);
                    self.sink.value(owner, path);
                }
            }
            "callable_reference" => {
                for child in named(node) {
                    if child.kind() == "identifier" {
                        let name = self.sink.text(child).trim_matches('`').to_string();
                        let path = self.qualify(ctx, vec![name]);
                        self.sink.value(owner, path);
                    } else {
                        self.walk(child, owner, ctx);
                    }
                }
            }
            // A supertype's constructor call or an annotation: the type is
            // referred to, nothing is called.
            "constructor_invocation" => self.children(node, owner, ctx),
            "value_argument" => {
                let mut cursor = node.walk();
                let has_name = node.children(&mut cursor).any(|c| c.kind() == "=");
                if has_name {
                    // `name = value`: the name is the parameter's.
                    for child in named(node).into_iter().skip(1) {
                        self.walk(child, owner, ctx);
                    }
                } else {
                    self.children(node, owner, ctx);
                }
            }
            "parameter"
            | "class_parameter"
            | "variable_declaration"
            | "type_parameter"
            | "catch_block" => self.children_without_binding(node, owner, ctx),
            "return_expression" => {
                let label = node.child_by_field_name("label").map(|l| l.id());
                for child in named(node) {
                    if Some(child.id()) != label {
                        self.walk(child, owner, ctx);
                    }
                }
            }
            "super_expression" => {
                for child in named(node) {
                    if child.kind() != "identifier" {
                        self.walk(child, owner, ctx);
                    }
                }
            }
            // Declarations inside a body are local: no symbols, only what
            // they mention.
            "function_declaration" | "class_declaration" | "object_declaration" => {
                let name = node.child_by_field_name("name").map(|n| n.id());
                for child in named(node) {
                    if Some(child.id()) != name {
                        self.walk(child, owner, ctx);
                    }
                }
            }
            _ => self.children(node, owner, ctx),
        }
    }

    /// `Foo<Int>(1)` reads as two comparisons in the grammar when it has one
    /// type argument and arguments; that shape is a call.
    fn binary(&mut self, node: Node, owner: &str, ctx: &Ctx) {
        let operator = |n: Node| {
            n.child_by_field_name("operator")
                .map(|o| self.sink.text(o))
                .unwrap_or("")
        };
        if operator(node) == ">"
            && let (Some(left), Some(right)) = (
                node.child_by_field_name("left"),
                node.child_by_field_name("right"),
            )
            && left.kind() == "binary_expression"
            && operator(left) == "<"
            && right.kind() == "parenthesized_expression"
            && let (Some(callee), Some(argument)) = (
                left.child_by_field_name("left")
                    .filter(|n| n.kind() == "identifier"),
                left.child_by_field_name("right"),
            )
        {
            self.call_by_name(callee, owner, ctx);
            self.walk(argument, owner, ctx);
            self.children(right, owner, ctx);
            return;
        }
        self.children(node, owner, ctx);
    }

    /// A call of a bare name: a function, or a construction when the name
    /// is a type's.
    fn call_by_name(&mut self, callee: Node, owner: &str, ctx: &Ctx) {
        let name = self.sink.text(callee).trim_matches('`').to_string();
        let path = self.qualify(ctx, vec![name.clone()]);
        if starts_uppercase(&name) {
            self.sink.type_ref(owner, path.clone());
            let mut constructor = path.clone();
            constructor.push(name);
            self.sink.call(owner, constructor, false, None, callee);
        }
        self.sink.call(owner, path, false, None, callee);
    }

    fn call(&mut self, node: Node, owner: &str, ctx: &Ctx) {
        let parts = named(node);
        let Some(callee) = parts.first().copied() else {
            return;
        };
        match callee.kind() {
            "identifier" => self.call_by_name(callee, owner, ctx),
            "navigation_expression" => self.member_call(callee, owner, ctx),
            // `f(1) { ... }` and `f(1)(2)`: the callee is the call inside.
            _ => self.walk(callee, owner, ctx),
        }
        for part in &parts[1..] {
            self.walk(*part, owner, ctx);
        }
    }

    /// `receiver.name(...)`, `receiver?.name(...)`.
    fn member_call(&mut self, callee: Node, owner: &str, ctx: &Ctx) {
        let parts = named(callee);
        let (Some(receiver), Some(name_node)) = (
            parts.first().copied(),
            parts.last().copied().filter(|n| n.kind() == "identifier"),
        ) else {
            return self.walk(callee, owner, ctx);
        };
        if parts.len() != 2 {
            return self.walk(callee, owner, ctx);
        }
        let name = self.sink.text(name_node).trim_matches('`').to_string();
        match receiver.kind() {
            "this_expression" => {
                let class = ctx.classes.last().map(|(q, _)| q.as_str());
                self.sink.call(owner, vec![name], true, class, name_node);
            }
            _ => {
                match self.chain(receiver) {
                    // `Type.name(...)`: a path through a type, a package
                    // alias or an object.
                    Some(chain)
                        if starts_uppercase(&chain[0]) || self.imported.contains(&chain[0]) =>
                    {
                        let mut path = self.qualify(ctx, chain);
                        path.push(name);
                        self.sink.call(owner, path, false, None, name_node);
                    }
                    // An extension function the file imports by name.
                    _ if self.imported.contains(&name) => {
                        self.sink.call(owner, vec![name], false, None, name_node);
                    }
                    // A receiver of unknown type, `super` included.
                    _ => self.sink.call(owner, vec![name], true, None, name_node),
                }
                self.walk(receiver, owner, ctx);
            }
        }
    }
}

/// The first identifier among the children of `node`: the name a
/// declaration binds.
fn first_identifier(node: Node) -> Option<Node> {
    named(node).into_iter().find(|n| n.kind() == "identifier")
}

/// The `variable_declaration` nodes of a property: one, or the several of a
/// destructuring declaration.
fn variables(property: Node) -> Vec<Node> {
    let mut out = Vec::new();
    for child in named(property) {
        match child.kind() {
            "variable_declaration" => out.push(child),
            "multi_variable_declaration" => out.extend(
                named(child)
                    .into_iter()
                    .filter(|n| n.kind() == "variable_declaration"),
            ),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = r#"@file:JvmName("AppKt")
package com.acme.app

import com.acme.text.Label
import com.acme.text.*
import com.acme.text.Words as W
import com.acme.text.Words.Companion.pad
import com.acme.util.shout
import kotlinx.coroutines.launch
import java.util.Locale

/**
 * The application.
 *
 * Second paragraph.
 */
@Deprecated("old")
data class App(val label: Label, private var count: Int = LIMIT) : Base(label), Named<String>, Runnable {
    companion object Factory {
        const val MAX = 3
        val names = listOf("a")

        /** Makes one. */
        fun create(): App = App(Label("x"))
    }

    private val inner: Inner = Inner()

    constructor(count: Int) : this(Label("n"), count)

    override fun run() {
        val text = shout(name())
        val made = Label(text)
        this.helper(made, MAX)
        made.show()
        W.pad(text, count = 2)
        create()
        App.create()
        items.map { it.trim() }.forEach { launch { show2(it) } }
        text.loud()
        ArrayList<String>(10)
        val f = ::helper
    }

    protected fun name(): String = "app"

    internal fun helper(l: Label, n: Int) {}

    private class Inner : Label("i") {
        fun make(): Inner = Inner()
    }
}

sealed interface Shape

fun interface Handler {
    fun handle(x: Int): Int
}

enum class Color(val rgb: Int) : Named<String> {
    RED(1),
    GREEN(2) {
        override fun name() = "g"
    };

    override fun name() = "c"
}

object Registry : Base() {
    val all = mutableListOf<Label>()
    var last: Label? = null

    @JvmStatic
    fun main(args: Array<String>) {
        Color.RED.name()
    }
}

typealias Names = List<String>

/** Shouts. */
fun String.loud(): String = this.uppercase(Locale.ROOT)

fun <T : Comparable<T>> List<T>.second(): T = this[1]

val LIMIT = 5
private var counter: Int = 0
const val GREETING = "hi"

fun main() {
    println("hi")
}

annotation class Marker
"#;

    fn parsed() -> ParsedFile {
        parse_kotlin("src/main/kotlin/com/acme/app/App.kt", SOURCE).unwrap()
    }

    fn symbol<'a>(p: &'a ParsedFile, q: &str) -> &'a cw_core::Symbol {
        p.symbols
            .iter()
            .find(|s| s.qualified == q)
            .unwrap_or_else(|| panic!("missing symbol {q}"))
    }

    #[test]
    fn extracts_declarations() {
        let p = parsed();
        assert!(!p.has_errors);
        let a = "com::acme::app";

        let package = symbol(&p, a);
        assert_eq!(package.kind, SymbolKind::Module);
        assert_eq!(package.signature, "package com.acme.app");
        assert_eq!((package.start_line, package.end_line), (2, 2));

        let app = symbol(&p, &format!("{a}::App"));
        assert_eq!(app.kind, SymbolKind::Class);
        assert_eq!(app.visibility, Visibility::Public);
        assert_eq!(app.doc, "The application.\n\nSecond paragraph.");
        assert_eq!(app.attrs, ["Deprecated"]);
        assert_eq!(
            app.signature,
            "data class App(val label: Label, private var count: Int = LIMIT) : Base(label), Named<String>, Runnable"
        );
        assert_eq!(app.parent.as_deref(), Some(a));
        // A property of a class is no symbol, the constructor's included.
        assert!(
            p.symbols
                .iter()
                .all(|s| s.name != "label" && s.name != "count" && s.name != "inner")
        );

        // The companion's members belong to the class around it.
        let max = symbol(&p, &format!("{a}::App::MAX"));
        assert_eq!(max.kind, SymbolKind::Const);
        assert_eq!(max.signature, "const val MAX");
        assert_eq!(
            symbol(&p, &format!("{a}::App::names")).kind,
            SymbolKind::Const
        );
        let create = symbol(&p, &format!("{a}::App::create"));
        assert_eq!(create.kind, SymbolKind::Method);
        assert_eq!(create.doc, "Makes one.");
        assert_eq!(create.signature, "fun create(): App");
        assert!(p.symbols.iter().all(|s| s.name != "Factory"));

        let secondary = symbol(&p, &format!("{a}::App::App"));
        assert_eq!(secondary.kind, SymbolKind::Method);
        assert_eq!(secondary.signature, "constructor(count: Int)");

        let run = symbol(&p, &format!("{a}::App::run"));
        assert_eq!(run.signature, "override fun run()");
        assert_eq!(
            symbol(&p, &format!("{a}::App::name")).visibility,
            Visibility::Crate
        );
        assert_eq!(
            symbol(&p, &format!("{a}::App::helper")).visibility,
            Visibility::Crate
        );
        let inner = symbol(&p, &format!("{a}::App::Inner"));
        assert_eq!(
            (inner.kind, inner.visibility),
            (SymbolKind::Class, Visibility::Private)
        );
        assert_eq!(
            symbol(&p, &format!("{a}::App::Inner::make")).kind,
            SymbolKind::Method
        );

        assert_eq!(
            symbol(&p, &format!("{a}::Shape")).kind,
            SymbolKind::Interface
        );
        // `fun interface` is an interface, and the file stays whole.
        let handler = symbol(&p, &format!("{a}::Handler"));
        assert_eq!(handler.kind, SymbolKind::Interface);
        assert_eq!(handler.signature, "fun interface Handler");
        assert_eq!(
            symbol(&p, &format!("{a}::Handler::handle")).kind,
            SymbolKind::Method
        );

        assert_eq!(symbol(&p, &format!("{a}::Color")).kind, SymbolKind::Enum);
        assert_eq!(
            symbol(&p, &format!("{a}::Color::RED")).kind,
            SymbolKind::Const
        );
        assert_eq!(
            symbol(&p, &format!("{a}::Color::GREEN")).kind,
            SymbolKind::Const
        );
        assert_eq!(
            symbol(&p, &format!("{a}::Color::name")).kind,
            SymbolKind::Method
        );

        // An object is a class; its properties are symbols, `var` a static.
        let registry = symbol(&p, &format!("{a}::Registry"));
        assert_eq!(registry.kind, SymbolKind::Class);
        assert_eq!(registry.signature, "object Registry : Base()");
        assert_eq!(
            symbol(&p, &format!("{a}::Registry::all")).kind,
            SymbolKind::Const
        );
        let last = symbol(&p, &format!("{a}::Registry::last"));
        assert_eq!(last.kind, SymbolKind::Static);
        assert_eq!(last.signature, "var last: Label?");
        let main = symbol(&p, &format!("{a}::Registry::main"));
        assert_eq!(main.kind, SymbolKind::Method);
        assert_eq!(main.attrs, ["JvmStatic"]);

        let alias = symbol(&p, &format!("{a}::Names"));
        assert_eq!(alias.kind, SymbolKind::TypeAlias);
        assert_eq!(alias.signature, "typealias Names = List<String>");

        // Top-level functions and properties belong to the package.
        let loud = symbol(&p, &format!("{a}::loud"));
        assert_eq!(loud.kind, SymbolKind::Function);
        assert_eq!(loud.doc, "Shouts.");
        assert_eq!(loud.signature, "fun String.loud(): String");
        assert_eq!(loud.attrs, [EXTENSION_ATTR]);
        let second = symbol(&p, &format!("{a}::second"));
        assert_eq!(
            second.signature,
            "fun <T : Comparable<T>> List<T>.second(): T"
        );
        assert_eq!(second.attrs, [EXTENSION_ATTR]);
        assert_eq!(symbol(&p, &format!("{a}::LIMIT")).kind, SymbolKind::Const);
        let counter = symbol(&p, &format!("{a}::counter"));
        assert_eq!(
            (counter.kind, counter.visibility),
            (SymbolKind::Static, Visibility::Private)
        );
        assert_eq!(
            symbol(&p, &format!("{a}::GREETING")).signature,
            "const val GREETING"
        );
        let entry = symbol(&p, &format!("{a}::main"));
        assert_eq!(entry.kind, SymbolKind::Function);
        assert_eq!(entry.signature, "fun main()");
        let marker = symbol(&p, &format!("{a}::Marker"));
        assert_eq!(marker.kind, SymbolKind::Interface);
        assert_eq!(marker.signature, "annotation class Marker");
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
        assert!(has(&["com", "acme", "text", "Words"], "W", false));
        // The companion is no symbol: `pad` is a member of `Words`.
        assert!(has(&["com", "acme", "text", "Words", "pad"], "pad", false));
        assert!(has(&["com", "acme", "util", "shout"], "shout", false));
        assert!(has(&["kotlinx", "coroutines", "launch"], "launch", false));
        assert!(has(&["java", "util", "Locale"], "Locale", false));
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
        // A function the file imports stays a bare name.
        assert!(!call(run, &["shout"]).method);
        // A member of the enclosing class, or of its companion, is in full.
        assert!(!call(run, &["com", "acme", "app", "App", "name"]).method);
        assert!(!call(run, &["com", "acme", "app", "App", "create"]).method);
        // A name that is a type is a construction.
        assert!(!call(run, &["Label", "Label"]).method);
        let on_this = call(run, &["helper"]);
        assert!(on_this.method);
        assert_eq!(on_this.self_type.as_deref(), Some("com::acme::app::App"));
        let on_local = call(run, &["show"]);
        assert!(on_local.method && on_local.self_type.is_none());
        // Through an import alias, with a named argument.
        assert!(!call(run, &["W", "pad"]).method);
        assert!(!call(run, &["App", "create"]).method);
        // A trailing lambda does not change the callee; its body is walked.
        assert!(call(run, &["map"]).method);
        assert!(call(run, &["forEach"]).method);
        assert!(!call(run, &["launch"]).method);
        assert!(call(run, &["trim"]).method);
        // An extension function on a receiver of unknown type.
        assert!(call(run, &["loud"]).method);
        // A generic constructor call the grammar reads as comparisons.
        assert!(!call(run, &["ArrayList", "ArrayList"]).method);
        // Default imports are bare names a repository does not answer to.
        assert!(!call("com::acme::app::main", &["println"]).method);
        // A nested type's constructor.
        assert!(
            !call(
                "com::acme::app::App::Inner::make",
                &["com", "acme", "app", "App", "Inner", "Inner"]
            )
            .method
        );

        let value = |path: &[&str]| p.value_refs.iter().any(|v| v.from == run && v.path == path);
        assert!(value(&["com", "acme", "app", "App", "MAX"]));
        assert!(value(&["com", "acme", "app", "App", "helper"]));
        assert!(!value(&["text"]) && !value(&["made"]) && !value(&["it"]));
        assert!(
            p.value_refs
                .iter()
                .any(|v| v.from == "com::acme::app::Registry::main" && v.path == ["Color", "RED"])
        );

        let supers = |ty: &[&str]| -> Vec<Vec<String>> {
            p.impls
                .iter()
                .filter(|i| i.type_path == ty)
                .filter_map(|i| i.trait_path.clone())
                .collect()
        };
        assert_eq!(
            supers(&["App"]),
            [vec!["Base"], vec!["Named"], vec!["Runnable"]]
        );
        assert_eq!(supers(&["App", "Inner"]), [vec!["Label"]]);
        assert_eq!(supers(&["Color"]), [vec!["Named"]]);
        assert_eq!(supers(&["Registry"]), [vec!["Base"]]);

        let type_ref = |from: &str, path: &[&str]| {
            p.type_refs.iter().any(|t| t.from == from && t.path == path)
        };
        assert!(type_ref(run, &["Label"]));
        assert!(type_ref("com::acme::app::App", &["Label"]));
        assert!(type_ref(
            "com::acme::app::App",
            &["com", "acme", "app", "App", "Inner"]
        ));
        // A construction refers to its type.
        assert!(type_ref(run, &["ArrayList"]));
    }

    #[test]
    fn a_signature_is_one_line() {
        let source = "package p

class Many(
    private val a: Int,
    val b: String,
) : Base()

fun call(
    x: Int,
    y: Int,
): Int = x + y
";
        let p = parse_kotlin("p/Many.kt", source).unwrap();
        assert_eq!(
            symbol(&p, "p::Many").signature,
            "class Many(private val a: Int, val b: String) : Base()"
        );
        assert_eq!(
            symbol(&p, "p::call").signature,
            "fun call(x: Int, y: Int): Int"
        );
    }

    #[test]
    fn what_the_grammar_reads_as_comparisons_is_still_a_call() {
        let source = "package p

fun run() {
    val a = ArrayList<String>(10)
    emit<Foo>(x)
    val b = mutableMapOf<String, Int>(\"k\" to 1)
}
";
        let p = parse_kotlin("p/Run.kt", source).unwrap();
        let called = |path: &[&str]| {
            p.calls
                .iter()
                .any(|c| c.caller == "p::run" && c.path == path)
        };
        assert!(called(&["ArrayList", "ArrayList"]));
        assert!(called(&["emit"]));
        assert!(called(&["mutableMapOf"]));
        assert!(p.type_refs.iter().any(|t| t.path == ["ArrayList"]));
    }

    #[test]
    fn a_call_knows_where_it_stands() {
        let source = "package p

class A {
    fun run(x: B) {
        third(second(first()))
        x.alpha().beta()
    }
}
";
        let p = parse_kotlin("src/main/kotlin/p/A.kt", source).unwrap();
        let mut calls: Vec<(u32, &str)> = p
            .calls
            .iter()
            .filter(|c| c.caller == "p::A::run")
            .map(|c| (c.at, c.path[c.path.len() - 1].as_str()))
            .collect();
        calls.sort();
        for (at, name) in &calls {
            assert!(source[*at as usize..].starts_with(name), "{name} at {at}");
        }
        let names: Vec<&str> = calls.into_iter().map(|(_, name)| name).collect();
        assert_eq!(names, ["third", "second", "first", "alpha", "beta"]);
    }

    #[test]
    fn names_a_file_without_a_package_by_its_directory() {
        let p = parse_kotlin("tools/Gen.kt", "class Gen { fun run() {} }\nfun top() {}").unwrap();
        assert_eq!(symbol(&p, "tools::Gen").kind, SymbolKind::Class);
        assert_eq!(symbol(&p, "tools::Gen::run").kind, SymbolKind::Method);
        assert_eq!(symbol(&p, "tools::top").kind, SymbolKind::Function);
        let root = parse_kotlin("Main.kt", "fun main() {}").unwrap();
        assert_eq!(
            symbol(&root, "default_package::main").kind,
            SymbolKind::Function
        );
        // The folder says nothing when the file names its package.
        let elsewhere = parse_kotlin("a/b/C.kt", "package x.y\nclass C").unwrap();
        assert_eq!(symbol(&elsewhere, "x::y::C").kind, SymbolKind::Class);
    }

    #[test]
    fn tests_are_annotations_and_kotest_specs() {
        let source = "package t

import org.junit.jupiter.api.Test
import io.kotest.core.spec.style.StringSpec

class UserTest {
    @Test
    fun `finds a user by name`() {}

    @org.junit.Test fun plain() {}
}

class UserSpec : StringSpec({
    \"a user\" { check() }
})
";
        let p = parse_kotlin("src/test/kotlin/t/UserTest.kt", source).unwrap();
        assert!(!p.has_errors);
        let named_test = symbol(&p, "t::UserTest::finds_a_user_by_name");
        assert_eq!(named_test.kind, SymbolKind::Method);
        assert_eq!(named_test.attrs, ["Test"]);
        assert_eq!(symbol(&p, "t::UserTest::plain").attrs, ["org.junit.Test"]);
        let spec = symbol(&p, "t::UserSpec");
        assert_eq!(spec.attrs, [KOTEST_ATTR]);
        assert_eq!(spec.signature, "class UserSpec : StringSpec");
        assert!(symbol(&p, "t::UserTest").attrs.is_empty());
    }

    #[test]
    fn a_script_is_parsed_and_error_nodes_keep_their_parts() {
        let script = parse_kotlin("tools/gen.kts", "println(\"x\")\nfun helper() = 1\n").unwrap();
        assert_eq!(symbol(&script, "tools::helper").kind, SymbolKind::Function);
        // Whatever the grammar cannot read is flagged, and what it could is kept.
        let bad = parse_kotlin(
            "a/Bad.kt",
            "package a\nfun ok() {}\nfun broken( {\nclass After\n",
        )
        .unwrap();
        assert!(bad.has_errors);
        assert_eq!(symbol(&bad, "a::ok").kind, SymbolKind::Function);
    }
}
