//! TypeScript and TSX symbol and fact extraction.
//!
//! Written against the node kinds and field names published in the grammar's
//! own `node-types.json` (tree-sitter-typescript 0.23, both dialects). No
//! query files are used.

use std::collections::HashSet;

use cw_core::{Error, Result, SymbolKind, Visibility};
use tree_sitter::{Node, Parser};

use crate::lang::{
    Sink, block_doc, collapse, dotted, field_all, marker_name, named, split_qualified, ts_package,
};
use crate::rust::ParsedFile;

/// Parse one TypeScript or TSX source file. `module` is the qualified module
/// the file defines, for example `src::util::text`. `resolve` maps a relative
/// import specifier written in this file to the module it names, or `None`
/// when no such file is indexed.
pub fn parse_typescript(
    file: &str,
    module: &str,
    source: &str,
    resolve: &dyn Fn(&str) -> Option<String>,
) -> Result<ParsedFile> {
    let lower = file.to_ascii_lowercase();
    // The TSX grammar also parses JavaScript and JSX. Plain `.ts` needs its
    // own grammar because `<T>value` casts are not valid TSX.
    let tsx_like = [".tsx", ".js", ".jsx", ".mjs", ".cjs"]
        .iter()
        .any(|ext| lower.ends_with(ext));
    let language = if tsx_like {
        tree_sitter_typescript::LANGUAGE_TSX
    } else {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT
    };
    let mut parser = Parser::new();
    parser
        .set_language(&language.into())
        .map_err(|e| Error::external("loading the TypeScript grammar", e))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| Error::invalid(format!("parser produced no tree for {file}")))?;
    let root = tree.root_node();
    let mut ex = Extractor {
        sink: Sink::new(file, source, module, root.has_error()),
        resolve,
        roots: HashSet::new(),
        exported: HashSet::new(),
    };
    ex.collect_roots(root);
    ex.sink.module_symbol(
        format!("module {file}"),
        String::new(),
        1,
        source.lines().count() as u32,
    );
    let ctx = Ctx {
        owner: module.to_string(),
        self_type: None,
        locals: HashSet::new(),
        public: true,
        nested: false,
    };
    for child in named(root) {
        ex.item(child, child, false, &ctx);
    }
    // `export { a, b }` exports declarations made elsewhere in the file.
    for symbol in &mut ex.sink.out.symbols {
        if ex.exported.contains(&symbol.qualified) {
            symbol.visibility = Visibility::Public;
        }
    }
    Ok(ex.sink.out)
}

#[derive(Clone)]
struct Ctx {
    /// Qualified name new symbols nest under and references are charged to.
    owner: String,
    /// Class that `this` refers to.
    self_type: Option<String>,
    /// Names bound in the enclosing function scopes.
    locals: HashSet<String>,
    /// Whether an exported or unmarked member of `owner` is visible outside
    /// the file.
    public: bool,
    /// True inside a function body.
    nested: bool,
}

/// What an import specifier names.
enum Target {
    /// A module of the repository.
    Module(Vec<String>),
    /// A package from outside the repository.
    Package(String),
    /// A builtin, an unresolved relative path, or an unknown path alias.
    Unknown,
}

struct Extractor<'a> {
    sink: Sink<'a>,
    resolve: &'a dyn Fn(&str) -> Option<String>,
    /// Names bound by imports and module-level declarations. Only a dotted
    /// call that starts at one of these is resolved as a path.
    roots: HashSet<String>,
    /// Qualified names exported through an `export { .. }` clause.
    exported: HashSet<String>,
}

fn text<'a>(node: Node, src: &'a [u8]) -> &'a str {
    node.utf8_text(src).unwrap_or("")
}

/// Names bound by a declaration or parameter pattern.
fn pattern_names(node: Node, src: &[u8], out: &mut HashSet<String>) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            out.insert(text(node, src).to_string());
        }
        "pair_pattern" => {
            if let Some(value) = node.child_by_field_name("value") {
                pattern_names(value, src, out);
            }
        }
        "assignment_pattern" | "object_assignment_pattern" => {
            if let Some(left) = node.child_by_field_name("left") {
                pattern_names(left, src, out);
            }
        }
        "object_pattern" | "array_pattern" | "rest_pattern" => {
            for child in named(node) {
                pattern_names(child, src, out);
            }
        }
        _ => {}
    }
}

fn is_function_value(kind: &str) -> bool {
    matches!(
        kind,
        "arrow_function" | "function_expression" | "generator_function"
    )
}

/// Names bound anywhere in a function scope, not descending into nested
/// functions and classes.
fn bindings(node: Node, src: &[u8], out: &mut HashSet<String>) {
    for child in named(node) {
        let field = match child.kind() {
            "function_declaration"
            | "generator_function_declaration"
            | "class_declaration"
            | "abstract_class_declaration"
            | "class"
            | "method_definition" => continue,
            kind if is_function_value(kind) => continue,
            "variable_declarator" => Some("name"),
            "required_parameter" | "optional_parameter" => Some("pattern"),
            "catch_clause" => Some("parameter"),
            "for_in_statement" => Some("left"),
            _ => None,
        };
        if let Some(pattern) = field.and_then(|f| child.child_by_field_name(f)) {
            pattern_names(pattern, src, out);
        }
        bindings(child, src, out);
    }
}

impl<'a> Extractor<'a> {
    fn name_of(&self, node: Node) -> Option<&'a str> {
        node.child_by_field_name("name").map(|n| self.sink.text(n))
    }

    /// Contents of a string literal.
    fn string(&self, node: Node) -> String {
        match named(node)
            .into_iter()
            .find(|n| n.kind() == "string_fragment")
        {
            Some(fragment) => self.sink.text(fragment).to_string(),
            None => self
                .sink
                .text(node)
                .trim_matches(['"', '\'', '`'])
                .to_string(),
        }
    }

    fn target(&self, spec: &str) -> Target {
        if let Some(module) = (self.resolve)(spec) {
            return Target::Module(split_qualified(&module));
        }
        match ts_package(spec) {
            Some(package) => Target::Package(package),
            None => Target::Unknown,
        }
    }

    /// Names an import or module-level declaration binds in this file.
    fn collect_roots(&mut self, root: Node) {
        let mut pending = named(root);
        while let Some(node) = pending.pop() {
            match node.kind() {
                "export_statement" | "ambient_declaration" | "import_clause" | "named_imports" => {
                    pending.extend(named(node));
                }
                "import_statement" => pending.extend(
                    named(node)
                        .into_iter()
                        .filter(|n| n.kind() != "string" && n.kind() != "import_attribute"),
                ),
                "import_specifier" => {
                    let bound = node
                        .child_by_field_name("alias")
                        .or_else(|| node.child_by_field_name("name"));
                    if let Some(bound) = bound {
                        self.roots.insert(self.sink.text(bound).to_string());
                    }
                }
                "namespace_import" | "import_require_clause" => {
                    if let Some(id) = named(node).into_iter().find(|n| n.kind() == "identifier") {
                        self.roots.insert(self.sink.text(id).to_string());
                    }
                }
                "identifier" => {
                    self.roots.insert(self.sink.text(node).to_string());
                }
                "function_declaration"
                | "generator_function_declaration"
                | "function_signature"
                | "class_declaration"
                | "abstract_class_declaration"
                | "enum_declaration"
                | "internal_module" => {
                    if let Some(name) = self.name_of(node) {
                        self.roots.insert(name.to_string());
                    }
                }
                "lexical_declaration" | "variable_declaration" => {
                    for declarator in named(node) {
                        if let Some(name) = declarator
                            .child_by_field_name("name")
                            .filter(|n| n.kind() == "identifier")
                        {
                            self.roots.insert(self.sink.text(name).to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn visibility(&self, exported: bool, ctx: &Ctx) -> Visibility {
        if exported && ctx.public && !ctx.nested {
            Visibility::Public
        } else {
            Visibility::Private
        }
    }

    /// The `/** ... */` comment directly above `anchor`, or failing that the
    /// run of `//` lines directly above it.
    fn doc(&self, anchor: Node) -> String {
        let mut lines: Vec<&str> = Vec::new();
        let mut next_row = anchor.start_position().row;
        let mut prev = anchor.prev_sibling();
        while let Some(node) = prev {
            if node.kind() == "decorator" {
                next_row = node.start_position().row;
                prev = node.prev_sibling();
                continue;
            }
            // A comment that trails code on its own line documents that code.
            let own_line = node
                .prev_sibling()
                .is_none_or(|p| p.end_position().row < node.start_position().row);
            if node.kind() != "comment" || node.end_position().row + 1 < next_row || !own_line {
                break;
            }
            let comment = self.sink.text(node);
            if let Some(doc) = block_doc(comment) {
                if lines.is_empty() {
                    return doc;
                }
                break;
            }
            let Some(rest) = comment.strip_prefix("//") else {
                break;
            };
            lines.push(rest.strip_prefix(' ').unwrap_or(rest).trim_end());
            next_row = node.start_position().row;
            prev = node.prev_sibling();
        }
        lines.reverse();
        lines.join("\n").trim().to_string()
    }

    /// Declaration text from the start of `node` up to its body.
    fn signature(&self, node: Node) -> String {
        let end = node
            .child_by_field_name("body")
            .map_or(node.end_byte(), |b| b.start_byte());
        collapse(self.sink.between(node.start_byte(), end))
    }

    /// Segments of `a.b.c` when every part is a plain name.
    fn chain(&self, node: Node) -> Option<Vec<String>> {
        match node.kind() {
            "identifier" => Some(vec![self.sink.text(node).to_string()]),
            "member_expression" => {
                let mut path = self.chain(node.child_by_field_name("object")?)?;
                let property = node
                    .child_by_field_name("property")
                    .filter(|p| p.kind() == "property_identifier")?;
                path.push(self.sink.text(property).to_string());
                Some(path)
            }
            _ => None,
        }
    }

    /// Path of a type name with its type arguments removed.
    fn type_path(&self, node: Node) -> Option<Vec<String>> {
        match node.kind() {
            "type_identifier" => Some(vec![self.sink.text(node).to_string()]),
            "nested_type_identifier" => Some(dotted(self.sink.text(node))),
            "generic_type" => self.type_path(node.child_by_field_name("name")?),
            _ => None,
        }
    }

    /// A statement at module or namespace level. `anchor` is the statement
    /// including a wrapping `export`, which is where its comment sits.
    fn item(&mut self, node: Node, anchor: Node, exported: bool, ctx: &Ctx) {
        match node.kind() {
            "comment" => {}
            "import_statement" => self.import(node),
            "export_statement" => self.export(node, ctx),
            "ambient_declaration" => {
                for child in named(node) {
                    self.item(child, anchor, exported, ctx);
                }
            }
            "function_declaration" | "generator_function_declaration" | "function_signature" => {
                self.function(node, anchor, exported, ctx);
            }
            "class_declaration" | "abstract_class_declaration" => {
                self.class(node, anchor, exported, ctx);
            }
            "interface_declaration" => self.interface(node, anchor, exported, ctx),
            "enum_declaration" => self.enumeration(node, anchor, exported, ctx),
            "type_alias_declaration" => self.type_alias(node, anchor, exported, ctx),
            "lexical_declaration" | "variable_declaration" => {
                self.variables(node, anchor, exported, ctx);
            }
            "internal_module" | "module" => self.namespace(node, anchor, exported, ctx),
            // A bare `namespace X { .. }` is parsed as an expression statement.
            "expression_statement"
                if node
                    .named_child(0)
                    .is_some_and(|n| n.kind() == "internal_module") =>
            {
                for child in named(node) {
                    self.item(child, anchor, exported, ctx);
                }
            }
            _ => self.walk(node, ctx),
        }
    }

    fn import(&mut self, node: Node) {
        let require = named(node)
            .into_iter()
            .find(|n| n.kind() == "import_require_clause");
        let Some(source) = node
            .child_by_field_name("source")
            .or_else(|| require.and_then(|r| r.child_by_field_name("source")))
        else {
            return;
        };
        let target = match self.target(&self.string(source)) {
            Target::Module(target) => target,
            Target::Package(package) => return self.sink.import(vec![package], "", false, false),
            Target::Unknown => return,
        };
        // The file depends on the module even when a name it imports is not
        // an indexed symbol.
        self.sink.import(target.clone(), "", false, false);
        let join = |name: &str| {
            let mut path = target.clone();
            path.push(name.to_string());
            path
        };
        for clause in named(node) {
            match clause.kind() {
                "import_clause" => {
                    for part in named(clause) {
                        match part.kind() {
                            "identifier" => {
                                let alias = self.sink.text(part);
                                self.sink.import(join("default"), alias, false, false);
                            }
                            "namespace_import" => {
                                if let Some(id) = part.named_child(0) {
                                    let alias = self.sink.text(id);
                                    self.sink.import(target.clone(), alias, false, false);
                                }
                            }
                            "named_imports" => {
                                for spec in named(part) {
                                    let Some(name) = spec.child_by_field_name("name") else {
                                        continue;
                                    };
                                    let name = self.specifier_name(name);
                                    let alias = spec
                                        .child_by_field_name("alias")
                                        .map_or(name.clone(), |a| self.sink.text(a).to_string());
                                    self.sink.import(join(&name), &alias, false, false);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                "import_require_clause" => {
                    if let Some(id) = named(clause).into_iter().find(|n| n.kind() == "identifier") {
                        let alias = self.sink.text(id);
                        self.sink.import(target.clone(), alias, false, false);
                    }
                }
                _ => {}
            }
        }
    }

    /// An import or export specifier's name, which may be a string literal.
    fn specifier_name(&self, node: Node) -> String {
        if node.kind() == "string" {
            self.string(node)
        } else {
            self.sink.text(node).to_string()
        }
    }

    fn export(&mut self, node: Node, ctx: &Ctx) {
        if let Some(source) = node.child_by_field_name("source") {
            return self.reexport(node, source);
        }
        let mut cursor = node.walk();
        let is_default = node.children(&mut cursor).any(|c| c.kind() == "default");
        let at_module = ctx.owner == self.sink.module;
        let default_alias = |sink: &mut Sink, name: &str| {
            // `import x from "./m"` asks for the name `default`.
            let mut path = split_qualified(&sink.module);
            path.push(name.to_string());
            sink.import(path, "default", false, true);
        };
        if let Some(declaration) = node.child_by_field_name("declaration") {
            self.item(declaration, node, true, ctx);
            if is_default
                && at_module
                && let Some(name) = self.name_of(declaration)
            {
                default_alias(&mut self.sink, name);
            }
            return;
        }
        for child in named(node) {
            match child.kind() {
                "comment" | "decorator" => {}
                "export_clause" => {
                    for spec in named(child) {
                        let Some(name) = spec.child_by_field_name("name") else {
                            continue;
                        };
                        let name = self.specifier_name(name);
                        self.exported.insert(format!("{}::{name}", ctx.owner));
                        let alias = spec
                            .child_by_field_name("alias")
                            .map(|a| self.specifier_name(a));
                        if let Some(alias) = alias.filter(|a| *a != name && at_module) {
                            let mut path = split_qualified(&ctx.owner);
                            path.push(name);
                            self.sink.import(path, &alias, false, true);
                        }
                    }
                }
                // `export default <expression>`.
                _ => {
                    if is_default && at_module && child.kind() == "identifier" {
                        let name = self.sink.text(child);
                        self.exported.insert(format!("{}::{name}", ctx.owner));
                        default_alias(&mut self.sink, name);
                    }
                    self.walk(child, ctx);
                }
            }
        }
    }

    /// `export ... from "./m"`.
    fn reexport(&mut self, node: Node, source: Node) {
        let target = match self.target(&self.string(source)) {
            Target::Module(target) => target,
            Target::Package(package) => return self.sink.import(vec![package], "", false, false),
            Target::Unknown => return,
        };
        self.sink.import(target.clone(), "", false, true);
        let mut selective = false;
        for child in named(node) {
            match child.kind() {
                "export_clause" => {
                    selective = true;
                    for spec in named(child) {
                        let Some(name) = spec.child_by_field_name("name") else {
                            continue;
                        };
                        let name = self.specifier_name(name);
                        let alias = spec
                            .child_by_field_name("alias")
                            .map_or(name.clone(), |a| self.specifier_name(a));
                        let mut path = target.clone();
                        path.push(name);
                        self.sink.import(path, &alias, false, true);
                    }
                }
                "namespace_export" => {
                    selective = true;
                    if let Some(id) = child.named_child(0) {
                        let alias = self.specifier_name(id);
                        self.sink.import(target.clone(), &alias, false, true);
                    }
                }
                _ => {}
            }
        }
        if !selective {
            self.sink.import(target, "", true, true);
        }
    }

    fn function(&mut self, node: Node, anchor: Node, exported: bool, ctx: &Ctx) {
        let Some(name) = self.name_of(node) else {
            return;
        };
        let signature = self.signature(node);
        let doc = self.doc(anchor);
        let visibility = self.visibility(exported, ctx);
        let symbol = self
            .sink
            .symbol(&ctx.owner, name, SymbolKind::Function, anchor);
        symbol.visibility = visibility;
        symbol.signature = signature;
        symbol.doc = doc;
        let qualified = symbol.qualified.clone();
        self.callable(node, &qualified, None, ctx);
    }

    /// Walk the parameters, return type and body of a function-like node,
    /// charging what it mentions to `owner`.
    fn callable(&mut self, node: Node, owner: &str, self_type: Option<String>, ctx: &Ctx) {
        let mut inner = Ctx {
            owner: owner.to_string(),
            self_type,
            locals: ctx.locals.clone(),
            public: false,
            nested: true,
        };
        if let Some(params) = node.child_by_field_name("parameters") {
            bindings(params, self.sink.src, &mut inner.locals);
        }
        // `x => x + 1` has a bare parameter.
        if let Some(param) = node.child_by_field_name("parameter") {
            pattern_names(param, self.sink.src, &mut inner.locals);
        }
        if let Some(body) = node.child_by_field_name("body") {
            bindings(body, self.sink.src, &mut inner.locals);
        }
        for field in ["type_parameters", "parameters", "return_type", "body"] {
            if let Some(part) = node.child_by_field_name(field) {
                self.walk(part, &inner);
            }
        }
    }

    fn class(&mut self, node: Node, anchor: Node, exported: bool, ctx: &Ctx) {
        let Some(name) = self.name_of(node) else {
            return;
        };
        let signature = self.signature(node);
        let doc = self.doc(anchor);
        let visibility = self.visibility(exported, ctx);
        let mut decorators = field_all(node, "decorator");
        if anchor.id() != node.id() {
            decorators.extend(field_all(anchor, "decorator"));
        }
        let attrs = decorators
            .iter()
            .map(|d| marker_name(self.sink.text(*d)))
            .filter(|n| !n.is_empty())
            .collect();
        let symbol = self
            .sink
            .symbol(&ctx.owner, name, SymbolKind::Class, anchor);
        symbol.visibility = visibility;
        symbol.signature = signature;
        symbol.doc = doc;
        symbol.attrs = attrs;
        let qualified = symbol.qualified.clone();

        let inner = Ctx {
            owner: qualified.clone(),
            self_type: Some(qualified.clone()),
            locals: ctx.locals.clone(),
            public: visibility == Visibility::Public,
            nested: ctx.nested,
        };
        for decorator in decorators {
            self.walk(decorator, &inner);
        }
        for child in named(node) {
            match child.kind() {
                "class_heritage" => {
                    for clause in named(child) {
                        match clause.kind() {
                            "extends_clause" => {
                                for value in field_all(clause, "value") {
                                    if let Some(path) = self.chain(value) {
                                        self.sink.inherits(&qualified, path);
                                    }
                                }
                            }
                            "implements_clause" => {
                                for ty in named(clause) {
                                    if let Some(path) = self.type_path(ty) {
                                        self.sink.inherits(&qualified, path);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    self.walk(child, &inner);
                }
                "type_parameters" => self.walk(child, &inner),
                _ => {}
            }
        }
        if let Some(body) = node.child_by_field_name("body") {
            self.class_body(body, &inner);
        }
    }

    fn class_body(&mut self, body: Node, ctx: &Ctx) {
        for member in named(body) {
            match member.kind() {
                "comment" => {}
                "method_definition" | "method_signature" | "abstract_method_signature" => {
                    self.method(member, ctx);
                }
                "public_field_definition" => self.field(member, ctx),
                _ => self.walk(member, ctx),
            }
        }
    }

    /// Name of a class member when it is a plain or `#private` identifier.
    fn member_name(&self, member: Node) -> Option<&'a str> {
        member
            .child_by_field_name("name")
            .filter(|n| {
                matches!(
                    n.kind(),
                    "property_identifier" | "private_property_identifier"
                )
            })
            .map(|n| self.sink.text(n))
    }

    fn member_visibility(&self, member: Node, name: &str, ctx: &Ctx) -> Visibility {
        let restricted = named(member).into_iter().any(|n| {
            n.kind() == "accessibility_modifier"
                && matches!(self.sink.text(n), "private" | "protected")
        });
        if restricted || name.starts_with('#') || !ctx.public {
            Visibility::Private
        } else {
            Visibility::Public
        }
    }

    /// Decorators written directly above a class member.
    fn member_attrs(&self, member: Node) -> Vec<String> {
        let mut attrs = Vec::new();
        let mut prev = member.prev_sibling();
        while let Some(node) = prev.filter(|n| n.kind() == "decorator") {
            attrs.push(marker_name(self.sink.text(node)));
            prev = node.prev_sibling();
        }
        attrs.reverse();
        attrs
    }

    fn method(&mut self, member: Node, ctx: &Ctx) {
        let Some(name) = self.member_name(member) else {
            // `[Symbol.iterator]() { .. }` has no nameable symbol.
            return self.callable(member, &ctx.owner, ctx.self_type.clone(), ctx);
        };
        let signature = self.signature(member);
        let doc = self.doc(member);
        let visibility = self.member_visibility(member, name, ctx);
        let attrs = self.member_attrs(member);
        let symbol = self
            .sink
            .symbol(&ctx.owner, name, SymbolKind::Method, member);
        symbol.visibility = visibility;
        symbol.signature = signature;
        symbol.doc = doc;
        symbol.attrs = attrs;
        let qualified = symbol.qualified.clone();
        self.callable(member, &qualified, ctx.self_type.clone(), ctx);
    }

    fn field(&mut self, member: Node, ctx: &Ctx) {
        let value = member.child_by_field_name("value");
        let function = value.filter(|v| is_function_value(v.kind()));
        if let (Some(name), Some(function)) = (self.member_name(member), function) {
            // `handle = () => { .. }` is a method in all but syntax.
            let end = function
                .child_by_field_name("body")
                .map_or(function.end_byte(), |b| b.start_byte());
            let signature = collapse(self.sink.between(member.start_byte(), end));
            let doc = self.doc(member);
            let visibility = self.member_visibility(member, name, ctx);
            let attrs = self.member_attrs(member);
            let symbol = self
                .sink
                .symbol(&ctx.owner, name, SymbolKind::Method, member);
            symbol.visibility = visibility;
            symbol.signature = signature;
            symbol.doc = doc;
            symbol.attrs = attrs;
            let qualified = symbol.qualified.clone();
            let this = (function.kind() == "arrow_function")
                .then(|| ctx.self_type.clone())
                .flatten();
            self.callable(function, &qualified, this, ctx);
            return;
        }
        for field in ["type", "value"] {
            if let Some(part) = member.child_by_field_name(field) {
                self.walk(part, ctx);
            }
        }
    }

    fn interface(&mut self, node: Node, anchor: Node, exported: bool, ctx: &Ctx) {
        let Some(name) = self.name_of(node) else {
            return;
        };
        let signature = self.signature(node);
        let doc = self.doc(anchor);
        let visibility = self.visibility(exported, ctx);
        let symbol = self
            .sink
            .symbol(&ctx.owner, name, SymbolKind::Interface, anchor);
        symbol.visibility = visibility;
        symbol.signature = signature;
        symbol.doc = doc;
        let qualified = symbol.qualified.clone();
        let inner = Ctx {
            owner: qualified.clone(),
            self_type: None,
            locals: ctx.locals.clone(),
            public: visibility == Visibility::Public,
            nested: ctx.nested,
        };
        for child in named(node) {
            match child.kind() {
                "extends_type_clause" => {
                    for ty in field_all(child, "type") {
                        if let Some(path) = self.type_path(ty) {
                            self.sink.inherits(&qualified, path);
                        }
                    }
                    self.walk(child, &inner);
                }
                "type_parameters" => self.walk(child, &inner),
                _ => {}
            }
        }
        let Some(body) = node.child_by_field_name("body") else {
            return;
        };
        for member in named(body) {
            match member.kind() {
                "comment" => {}
                "method_signature" => self.method(member, &inner),
                _ => self.walk(member, &inner),
            }
        }
    }

    fn enumeration(&mut self, node: Node, anchor: Node, exported: bool, ctx: &Ctx) {
        let Some(name) = self.name_of(node) else {
            return;
        };
        let signature = self.signature(node);
        let doc = self.doc(anchor);
        let visibility = self.visibility(exported, ctx);
        let symbol = self.sink.symbol(&ctx.owner, name, SymbolKind::Enum, anchor);
        symbol.visibility = visibility;
        symbol.signature = signature;
        symbol.doc = doc;
        let qualified = symbol.qualified.clone();
        let Some(body) = node.child_by_field_name("body") else {
            return;
        };
        for member in named(body) {
            let name_node = match member.kind() {
                "property_identifier" => member,
                "enum_assignment" => match member.child_by_field_name("name") {
                    Some(n) if n.kind() == "property_identifier" => n,
                    _ => continue,
                },
                _ => continue,
            };
            let member_name = self.sink.text(name_node);
            let signature = collapse(self.sink.text(member));
            let doc = self.doc(member);
            let symbol = self
                .sink
                .symbol(&qualified, member_name, SymbolKind::Const, member);
            symbol.visibility = visibility;
            symbol.signature = signature;
            symbol.doc = doc;
            let owner = symbol.qualified.clone();
            if let Some(value) = member.child_by_field_name("value") {
                let inner = Ctx {
                    owner,
                    ..ctx.clone()
                };
                self.walk(value, &inner);
            }
        }
    }

    fn type_alias(&mut self, node: Node, anchor: Node, exported: bool, ctx: &Ctx) {
        let Some(name) = self.name_of(node) else {
            return;
        };
        let value = node.child_by_field_name("value");
        let end = value.map_or(node.end_byte(), |v| v.start_byte());
        let signature = collapse(self.sink.between(node.start_byte(), end));
        let doc = self.doc(anchor);
        let visibility = self.visibility(exported, ctx);
        let symbol = self
            .sink
            .symbol(&ctx.owner, name, SymbolKind::TypeAlias, anchor);
        symbol.visibility = visibility;
        symbol.signature = signature;
        symbol.doc = doc;
        let inner = Ctx {
            owner: symbol.qualified.clone(),
            ..ctx.clone()
        };
        for part in [node.child_by_field_name("type_parameters"), value]
            .into_iter()
            .flatten()
        {
            self.walk(part, &inner);
        }
    }

    /// `const`, `let` and `var` at module or namespace level. A variable
    /// holding a function is a function; anything else is a constant
    /// (`const`) or a static (`let`, `var`).
    fn variables(&mut self, node: Node, anchor: Node, exported: bool, ctx: &Ctx) {
        let keyword = self
            .sink
            .text(node)
            .split_whitespace()
            .next()
            .unwrap_or("const");
        for declarator in named(node) {
            if declarator.kind() != "variable_declarator" {
                continue;
            }
            let value = declarator.child_by_field_name("value");
            let Some(name) = declarator
                .child_by_field_name("name")
                .filter(|n| n.kind() == "identifier")
                .map(|n| self.sink.text(n))
            else {
                // A destructuring declaration binds no single symbol.
                if let Some(value) = value {
                    self.walk(value, ctx);
                }
                continue;
            };
            let function = value.filter(|v| is_function_value(v.kind()));
            let (kind, end) = match function {
                Some(f) => (
                    SymbolKind::Function,
                    f.child_by_field_name("body")
                        .map_or(f.end_byte(), |b| b.start_byte()),
                ),
                None => (
                    if keyword == "const" {
                        SymbolKind::Const
                    } else {
                        SymbolKind::Static
                    },
                    value.map_or(declarator.end_byte(), |v| v.start_byte()),
                ),
            };
            let declared = self.sink.between(declarator.start_byte(), end);
            let signature = collapse(&format!("{keyword} {declared}"));
            let doc = self.doc(anchor);
            let visibility = self.visibility(exported, ctx);
            let symbol = self.sink.symbol(&ctx.owner, name, kind, anchor);
            symbol.visibility = visibility;
            symbol.signature = signature;
            symbol.doc = doc;
            let qualified = symbol.qualified.clone();
            let inner = Ctx {
                owner: qualified.clone(),
                ..ctx.clone()
            };
            if let Some(ty) = declarator.child_by_field_name("type") {
                self.walk(ty, &inner);
            }
            match (function, value) {
                (Some(f), _) => self.callable(f, &qualified, None, ctx),
                (None, Some(value)) => self.walk(value, &inner),
                (None, None) => {}
            }
        }
    }

    fn namespace(&mut self, node: Node, anchor: Node, exported: bool, ctx: &Ctx) {
        // `declare module "name"` augments another package; it defines
        // nothing in this repository's namespace.
        let Some(name) = node
            .child_by_field_name("name")
            .filter(|n| n.kind() == "identifier")
            .map(|n| self.sink.text(n))
        else {
            return;
        };
        let signature = self.signature(node);
        let doc = self.doc(anchor);
        let visibility = self.visibility(exported, ctx);
        let symbol = self
            .sink
            .symbol(&ctx.owner, name, SymbolKind::Module, anchor);
        symbol.visibility = visibility;
        symbol.signature = signature;
        symbol.doc = doc;
        let inner = Ctx {
            owner: symbol.qualified.clone(),
            self_type: None,
            locals: ctx.locals.clone(),
            public: visibility == Visibility::Public,
            nested: ctx.nested,
        };
        if let Some(body) = node.child_by_field_name("body") {
            for child in named(body) {
                self.item(child, child, false, &inner);
            }
        }
    }

    fn children(&mut self, node: Node, ctx: &Ctx) {
        for child in named(node) {
            self.walk(child, ctx);
        }
    }

    /// Record what an expression, statement or type mentions.
    fn walk(&mut self, node: Node, ctx: &Ctx) {
        match node.kind() {
            "comment" | "import_statement" => {}
            "identifier" | "shorthand_property_identifier" => {
                let name = self.sink.text(node);
                if !ctx.locals.contains(name) {
                    self.sink.value(&ctx.owner, vec![name.to_string()]);
                }
            }
            "type_identifier" => {
                let name = self.sink.text(node).to_string();
                self.sink.type_ref(&ctx.owner, vec![name]);
            }
            "nested_type_identifier" => {
                let path = dotted(self.sink.text(node));
                self.sink.type_ref(&ctx.owner, path);
            }
            "member_expression" => self.member(node, ctx),
            "call_expression" => self.call(node, ctx),
            "new_expression" => {
                let chain = node
                    .child_by_field_name("constructor")
                    .and_then(|c| self.chain(c));
                if let Some(mut path) = chain.filter(|c| !ctx.locals.contains(&c[0])) {
                    path.push("constructor".to_string());
                    self.sink.call(&ctx.owner, path, false, None);
                }
                self.children(node, ctx);
            }
            // The parameter's name is a binding, not a use.
            "required_parameter" | "optional_parameter" => {
                for field in ["type", "value"] {
                    if let Some(part) = node.child_by_field_name(field) {
                        self.walk(part, ctx);
                    }
                }
            }
            "function_declaration" | "generator_function_declaration" => {
                self.function(node, node, false, ctx);
            }
            "class_declaration" | "abstract_class_declaration" => {
                self.class(node, node, false, ctx);
            }
            "arrow_function" => {
                let owner = ctx.owner.clone();
                self.callable(node, &owner, ctx.self_type.clone(), ctx);
            }
            "function_expression" | "generator_function" | "method_definition" => {
                let owner = ctx.owner.clone();
                self.callable(node, &owner, None, ctx);
            }
            _ => self.children(node, ctx),
        }
    }

    fn member(&mut self, node: Node, ctx: &Ctx) {
        match self.chain(node) {
            Some(chain) => {
                if ctx.locals.contains(&chain[0]) {
                    return;
                }
                self.sink.value(&ctx.owner, vec![chain[0].clone()]);
                self.sink.value(&ctx.owner, chain);
            }
            None => {
                if let Some(object) = node.child_by_field_name("object") {
                    self.walk(object, ctx);
                }
            }
        }
    }

    fn call(&mut self, node: Node, ctx: &Ctx) {
        let Some(function) = node.child_by_field_name("function") else {
            return self.children(node, ctx);
        };
        match function.kind() {
            "identifier" => {
                let name = self.sink.text(function);
                if !ctx.locals.contains(name) {
                    self.sink
                        .call(&ctx.owner, vec![name.to_string()], false, None);
                }
            }
            "member_expression" => {
                let name = function
                    .child_by_field_name("property")
                    .map(|p| self.sink.text(p).to_string())
                    .unwrap_or_default();
                let on_this = function
                    .child_by_field_name("object")
                    .is_some_and(|o| o.kind() == "this");
                match self.chain(function) {
                    // `this.name(...)`: a method of the enclosing class, or
                    // one it inherits.
                    _ if on_this => {
                        self.sink
                            .call(&ctx.owner, vec![name], true, ctx.self_type.as_deref());
                    }
                    // `module.name(...)` or `Class.name(...)`: a path.
                    Some(chain)
                        if !ctx.locals.contains(&chain[0]) && self.roots.contains(&chain[0]) =>
                    {
                        self.sink.call(&ctx.owner, chain, false, None);
                    }
                    // A receiver of unknown type.
                    _ if !name.is_empty() => self.sink.call(&ctx.owner, vec![name], true, None),
                    _ => {}
                }
            }
            _ => {}
        }
        self.walk(function, ctx);
        for field in ["type_arguments", "arguments"] {
            if let Some(part) = node.child_by_field_name(field) {
                self.walk(part, ctx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = r#"import React, { useState as useS } from "react";
import fs from "node:fs";
import * as text from "./util/text";
import { Label, shout as yell } from "./util/text";
import type { Shape } from "../shapes";
import Base from "./base";
import "./missing";

export * from "./util/text";
export { helper as aid } from "./util/text";

/**
 * Greets someone.
 * Twice.
 */
export function greet(name: string, label: Label): string {
  function local(x: string) {
    return x.trim();
  }
  const text = local(name);
  return yell(text) + helper(label);
}

// Internal helper.
// Not exported here.
function helper(l: Label): string {
  return text.pad(l.toString());
}

export const LIMIT: number = 3;
let counter = 0;

export const twice = (n: number): number => {
  return n * LIMIT;
};

export interface Named extends Shape {
  name(): string;
  size: number;
}

export type Pair<T> = [T, Named];

export enum Color {
  Red,
  Green = "green",
}

@sealed
export abstract class Widget extends Base implements Named, text.Printable {
  private secret = 1;
  handle = (e: Event) => {
    this.render();
  };

  constructor(private readonly label: Label) {
    super();
  }

  /** Renders the widget. */
  render(): string {
    const made = new Label("x");
    return this.name() + made.show();
  }

  name(): string {
    return Widget.create().toString();
  }

  static create(): Widget {
    return twice(2) as unknown as Widget;
  }

  private hidden(): void {}
  abstract area(): number;
}

namespace Internal {
  export function inner(): void {
    helper(null as unknown as Label);
  }
}

export { helper, counter as count };
export default greet;
"#;

    fn resolver(spec: &str) -> Option<String> {
        match spec {
            "./util/text" => Some("src::util::text".to_string()),
            "../shapes" => Some("shapes".to_string()),
            "./base" => Some("src::base".to_string()),
            _ => None,
        }
    }

    fn parsed() -> ParsedFile {
        parse_typescript("src/app.ts", "src::app", SOURCE, &resolver).unwrap()
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
        assert_eq!(symbol(&p, "src::app").kind, SymbolKind::Module);

        let greet = symbol(&p, "src::app::greet");
        assert_eq!(greet.kind, SymbolKind::Function);
        assert_eq!(greet.visibility, Visibility::Public);
        assert_eq!(
            greet.signature,
            "function greet(name: string, label: Label): string"
        );
        assert_eq!(greet.doc, "Greets someone.\nTwice.");
        assert_eq!(greet.end_line - greet.start_line, 6);
        assert_eq!(
            symbol(&p, "src::app::greet::local").visibility,
            Visibility::Private
        );

        // Exported by the trailing `export { helper }` clause.
        let helper = symbol(&p, "src::app::helper");
        assert_eq!(helper.visibility, Visibility::Public);
        assert_eq!(helper.doc, "Internal helper.\nNot exported here.");

        let limit = symbol(&p, "src::app::LIMIT");
        assert_eq!(limit.kind, SymbolKind::Const);
        assert_eq!(limit.signature, "const LIMIT: number");
        let counter = symbol(&p, "src::app::counter");
        assert_eq!(counter.kind, SymbolKind::Static);
        assert_eq!(counter.visibility, Visibility::Public);

        let twice = symbol(&p, "src::app::twice");
        assert_eq!(twice.kind, SymbolKind::Function);
        assert_eq!(twice.signature, "const twice = (n: number): number =>");

        let named_iface = symbol(&p, "src::app::Named");
        assert_eq!(named_iface.kind, SymbolKind::Interface);
        assert_eq!(named_iface.signature, "interface Named extends Shape");
        assert_eq!(symbol(&p, "src::app::Named::name").kind, SymbolKind::Method);

        let pair = symbol(&p, "src::app::Pair");
        assert_eq!(pair.kind, SymbolKind::TypeAlias);
        assert_eq!(pair.signature, "type Pair<T>");

        assert_eq!(symbol(&p, "src::app::Color").kind, SymbolKind::Enum);
        assert_eq!(symbol(&p, "src::app::Color::Red").kind, SymbolKind::Const);
        assert_eq!(
            symbol(&p, "src::app::Color::Green").signature,
            "Green = \"green\""
        );

        let widget = symbol(&p, "src::app::Widget");
        assert_eq!(widget.kind, SymbolKind::Class);
        assert_eq!(widget.attrs, ["sealed"]);
        assert_eq!(
            widget.signature,
            "abstract class Widget extends Base implements Named, text.Printable"
        );
        let render = symbol(&p, "src::app::Widget::render");
        assert_eq!(render.kind, SymbolKind::Method);
        assert_eq!(render.doc, "Renders the widget.");
        assert_eq!(render.signature, "render(): string");
        assert_eq!(render.visibility, Visibility::Public);
        assert_eq!(
            symbol(&p, "src::app::Widget::hidden").visibility,
            Visibility::Private
        );
        assert_eq!(
            symbol(&p, "src::app::Widget::handle").kind,
            SymbolKind::Method
        );
        assert_eq!(
            symbol(&p, "src::app::Widget::area").kind,
            SymbolKind::Method
        );
        assert!(p.symbols.iter().all(|s| s.name != "secret"));

        let ns = symbol(&p, "src::app::Internal");
        assert_eq!(ns.kind, SymbolKind::Module);
        // Exported from a namespace that is itself not exported.
        assert_eq!(
            symbol(&p, "src::app::Internal::inner").visibility,
            Visibility::Private
        );
    }

    #[test]
    fn extracts_imports() {
        let p = parsed();
        let has = |path: &[&str], alias: &str, glob: bool| {
            p.uses
                .iter()
                .any(|u| u.path == path && u.alias == alias && u.glob == glob)
        };
        // Packages keep one segment; builtins and unresolved paths vanish.
        assert!(has(&["react"], "", false));
        assert!(!p.uses.iter().any(|u| u.path[0].contains("fs")));
        assert!(!p.uses.iter().any(|u| u.path[0].contains("missing")));

        assert!(has(&["src", "util", "text"], "text", false));
        assert!(has(&["src", "util", "text"], "", false));
        assert!(has(&["src", "util", "text", "Label"], "Label", false));
        assert!(has(&["src", "util", "text", "shout"], "yell", false));
        assert!(has(&["shapes", "Shape"], "Shape", false));
        assert!(has(&["src", "base", "default"], "Base", false));
        // Re-exports.
        assert!(has(&["src", "util", "text"], "", true));
        assert!(has(&["src", "util", "text", "helper"], "aid", false));
        // Local export aliases, including the default export.
        assert!(has(&["src", "app", "counter"], "count", false));
        assert!(has(&["src", "app", "greet"], "default", false));
    }

    #[test]
    fn extracts_calls_and_inheritance() {
        let p = parsed();
        let call = |caller: &str, path: &[&str]| {
            p.calls
                .iter()
                .find(|c| c.caller == caller && c.path == path)
                .unwrap_or_else(|| panic!("missing call {caller} -> {path:?}"))
        };
        assert!(!call("src::app::greet", &["local"]).method);
        assert!(!call("src::app::greet", &["yell"]).method);
        assert!(!call("src::app::greet", &["helper"]).method);
        // `text` is an import in `helper` but a local in `greet`.
        assert!(!call("src::app::helper", &["text", "pad"]).method);
        assert!(call("src::app::helper", &["toString"]).method);
        assert!(
            !p.value_refs
                .iter()
                .any(|v| v.from == "src::app::greet" && v.path == ["text"])
        );

        let on_this = call("src::app::Widget::render", &["name"]);
        assert!(on_this.method);
        assert_eq!(on_this.self_type.as_deref(), Some("src::app::Widget"));
        let on_local = call("src::app::Widget::render", &["show"]);
        assert!(on_local.method && on_local.self_type.is_none());
        assert!(!call("src::app::Widget::render", &["Label", "constructor"]).method);
        assert!(!call("src::app::Widget::name", &["Widget", "create"]).method);
        assert!(!call("src::app::Widget::create", &["twice"]).method);
        // An arrow-function field keeps `this`.
        assert!(
            call("src::app::Widget::handle", &["render"])
                .self_type
                .is_some()
        );
        assert!(!call("src::app::Internal::inner", &["helper"]).method);

        let supers: Vec<Vec<String>> = p
            .impls
            .iter()
            .filter(|i| i.type_path == ["Widget"])
            .filter_map(|i| i.trait_path.clone())
            .collect();
        assert_eq!(
            supers,
            [vec!["Base"], vec!["Named"], vec!["text", "Printable"]]
        );
        assert!(
            p.impls
                .iter()
                .any(|i| i.type_path == ["Named"] && i.trait_path == Some(vec!["Shape".into()]))
        );

        let type_ref = |from: &str, name: &str| {
            p.type_refs
                .iter()
                .any(|t| t.from == from && t.path == [name])
        };
        assert!(type_ref("src::app::greet", "Label"));
        assert!(type_ref("src::app::Pair", "Named"));
        assert!(!type_ref("src::app::Pair", "T"));
        assert!(
            p.value_refs
                .iter()
                .any(|v| v.from == "src::app::twice" && v.path == ["LIMIT"])
        );
    }

    #[test]
    fn parses_tsx() {
        let source = "import { Label } from \"./label\";\nexport function App(props: { title: string }) {\n  return <Label text={props.title} />;\n}\n";
        let p = parse_typescript("src/App.tsx", "src::App", source, &|_| {
            Some("src::label".to_string())
        })
        .unwrap();
        assert!(!p.has_errors);
        assert_eq!(symbol(&p, "src::App::App").kind, SymbolKind::Function);
        // A component used as an element is a reference to it.
        assert!(
            p.value_refs
                .iter()
                .any(|v| v.from == "src::App::App" && v.path == ["Label"])
        );
    }
}
