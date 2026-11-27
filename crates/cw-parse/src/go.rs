//! Go symbol and fact extraction.
//!
//! Written against the node kinds and field names published in the grammar's
//! own `node-types.json` (tree-sitter-go 0.25). No query files are used.
//!
//! How Go maps onto the shared model:
//!
//! - **A package is a module.** All `.go` files of one folder share one
//!   package; the caller names it (see [`crate::gomod::GoModules::package_of`])
//!   so that the import path `example.com/app/internal/store` is the module
//!   `example_com::app::internal::store`. The module symbol's signature is
//!   `package <name>`, which is how an entry point is told from a library.
//!   An external test package (`package store_test` in a `_test.go` file)
//!   is a module of its own, `<package>_test`.
//! - **Functions** are `<package>::Name`; a **method** belongs to its
//!   receiver's type, value or pointer alike (`<package>::Type::Name`), so
//!   the type and its methods nest as a class and its methods do elsewhere.
//! - **Types.** A struct is a [`SymbolKind::Struct`], an interface a
//!   [`SymbolKind::Interface`] whose methods are method symbols, and every
//!   other defined type and every alias a [`SymbolKind::TypeAlias`].
//!   Fields are no symbols. Constants are [`SymbolKind::Const`] and package
//!   variables [`SymbolKind::Static`].
//! - **Exported** (a name that starts with an upper-case letter) is
//!   [`Visibility::Public`]; any other name is visible to its package alone,
//!   which is [`Visibility::Crate`].
//! - **Inheritance.** Go has none; the shared edge for it, "implements", is
//!   given to what stands closest: a struct that embeds a type, an interface
//!   that embeds an interface, and `var _ Iface = (*T)(nil)`, the idiom that
//!   states `T` implements `Iface`. A type that satisfies an interface by
//!   having its methods leaves no trace in the text and gets no edge.
//! - **Calls.** `pkg.Func()` through an import name is a path; `x.Method()` on
//!   the receiver of the enclosing method is a call on the receiver's type;
//!   `x.Method()` on anything else is a method call on a receiver of unknown
//!   type, as in Java. A name bound inside the function (parameter, `:=`,
//!   `var`, range variable) is never a package-level name.
//! - **Doc comments** are the `//` lines (or a `/* */` block) that end on the
//!   line above a declaration; compiler directives such as `//go:generate`
//!   are no documentation.
//! - **Generated code.** A file with a line `// Code generated ... DO NOT
//!   EDIT.` before its `package` clause is parsed like any other, so its
//!   symbols exist and are called, and each of its symbols carries the
//!   attribute [`GENERATED_ATTR`], which keeps it out of the notes' rankings.

use std::collections::HashSet;

use cw_core::{Error, Result, SymbolKind, Visibility};
use tree_sitter::{Node, Parser};

use crate::gomod::guess_package_name;
use crate::lang::{Sink, collapse, field_all, named};
use crate::rust::ParsedFile;

/// The attribute of a symbol declared in a generated file.
pub const GENERATED_ATTR: &str = "generated";

/// Names the compiler provides. A call to one is no call to repository code,
/// and a type of this list names no type of the repository.
const BUILTIN_FUNCTIONS: [&str; 16] = [
    "append", "cap", "clear", "close", "complex", "copy", "delete", "imag", "len", "make", "max",
    "min", "new", "panic", "print", "println",
];
const BUILTIN_TYPES: [&str; 22] = [
    "any",
    "bool",
    "byte",
    "comparable",
    "complex128",
    "complex64",
    "error",
    "float32",
    "float64",
    "int",
    "int16",
    "int32",
    "int64",
    "int8",
    "rune",
    "string",
    "uint",
    "uint16",
    "uint32",
    "uint64",
    "uint8",
    "uintptr",
];

/// The name in the `package` clause of a Go file, read without a parser, or
/// `None` when the file has none before its first declaration.
pub fn package_name(source: &str) -> Option<&str> {
    let mut rest = source;
    loop {
        rest = rest.trim_start();
        if let Some(after) = rest.strip_prefix("//") {
            rest = after.split_once('\n').map_or("", |(_, tail)| tail);
        } else if let Some(after) = rest.strip_prefix("/*") {
            rest = after.split_once("*/")?.1;
        } else {
            break;
        }
    }
    let after = rest.strip_prefix("package")?;
    after
        .trim_start()
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .next()
        .filter(|n| !n.is_empty())
}

/// True for a name Go exports: one that starts with an upper-case letter.
pub fn is_exported(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase)
}

fn visibility(name: &str) -> Visibility {
    if is_exported(name) {
        Visibility::Public
    } else {
        Visibility::Crate
    }
}

/// Parse one Go source file.
///
/// `module` is the qualified name of the package the file belongs to.
/// `import_name` gives the name a package of the repository declares for
/// itself, for an import path (`example.com/app/store` for `package db`);
/// where it knows none, the last element of the path stands in.
pub fn parse_go(
    file: &str,
    source: &str,
    module: &str,
    import_name: &dyn Fn(&str) -> Option<String>,
) -> Result<ParsedFile> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_go::LANGUAGE.into())
        .map_err(|e| Error::external("loading the Go grammar", e))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| Error::invalid(format!("parser produced no tree for {file}")))?;
    let root = tree.root_node();
    let clause = named(root)
        .into_iter()
        .find(|n| n.kind() == "package_clause");
    let package = clause
        .and_then(|c| named(c).into_iter().next())
        .map(|n| n.utf8_text(source.as_bytes()).unwrap_or("").to_string())
        .unwrap_or_default();
    // `package store_test` in `store_test.go` is another package than `store`.
    let external_test = file.ends_with("_test.go") && package.ends_with("_test");
    let module = if external_test {
        format!("{module}_test")
    } else {
        module.to_string()
    };
    let module = if module.is_empty() {
        crate::lang::sanitize(&package)
    } else {
        module
    };
    let mut ex = Extractor {
        sink: Sink::new(file, source, &module, root.has_error()),
        import_name,
        imported: HashSet::new(),
        generated: false,
    };
    ex.generated = named(root)
        .into_iter()
        .take_while(|n| n.kind() != "package_clause")
        .any(|n| n.kind() == "comment" && is_generated_marker(ex.sink.text(n)));
    let (start, end) = clause.map_or((1, 1), |c| {
        (
            c.start_position().row as u32 + 1,
            c.end_position().row as u32 + 1,
        )
    });
    let doc = clause.map(|c| ex.doc(c)).unwrap_or_default();
    ex.sink
        .module_symbol(format!("package {package}"), doc, start, end);
    for child in named(root) {
        if child.kind() == "import_declaration" {
            ex.imports(child);
        }
    }
    for child in named(root) {
        match child.kind() {
            "function_declaration" => ex.function(child),
            "method_declaration" => ex.method(child),
            "type_declaration" => ex.types(child),
            "const_declaration" | "var_declaration" => ex.values(child),
            _ => {}
        }
    }
    Ok(ex.sink.out)
}

/// True for the line Go tools read as "this file is generated":
/// `// Code generated ... DO NOT EDIT.`
fn is_generated_marker(comment: &str) -> bool {
    comment.lines().any(|line| {
        let line = line.trim();
        line.starts_with("// Code generated") && line.ends_with("DO NOT EDIT.")
    })
}

/// The names bound inside one function, which no package-level name of the
/// same spelling can be: parameters, results, receivers, type parameters,
/// `:=` and `var` names, range variables, local types.
struct Fun {
    /// Qualified name of the symbol the code belongs to.
    owner: String,
    locals: HashSet<String>,
    /// The receiver's name and the qualified type it stands for.
    receiver: Option<(String, String)>,
}

struct Extractor<'a, 'n> {
    sink: Sink<'a>,
    import_name: &'n dyn Fn(&str) -> Option<String>,
    /// Names the file's imports bind, the standard library's included, so
    /// that `fmt.Println` is never taken for a method on a variable `fmt`.
    imported: HashSet<String>,
    generated: bool,
}

impl<'a> Extractor<'a, '_> {
    fn text(&self, node: Node) -> &'a str {
        self.sink.text(node)
    }

    /// The documentation above `node`: the comments that end on the line
    /// above it, with no blank line between, leaving out the trailing
    /// comment of the previous declaration and compiler directives.
    fn doc(&self, node: Node) -> String {
        let mut lines: Vec<String> = Vec::new();
        let mut next_row = node.start_position().row;
        let mut current = node.prev_sibling();
        while let Some(comment) = current.filter(|c| c.kind() == "comment") {
            if comment.end_position().row + 1 != next_row {
                break;
            }
            // A comment after code on its own line is that code's.
            let trailing = comment.prev_sibling().is_some_and(|before| {
                before.kind() != "comment"
                    && before.end_position().row == comment.start_position().row
            });
            if trailing {
                break;
            }
            lines.push(self.text(comment).to_string());
            next_row = comment.start_position().row;
            current = comment.prev_sibling();
        }
        lines.reverse();
        let mut out: Vec<String> = Vec::new();
        for comment in lines {
            if let Some(block) = comment.strip_prefix("/*") {
                let block = block.strip_suffix("*/").unwrap_or(block);
                out.extend(block.lines().map(|l| l.trim().to_string()));
            } else {
                let line = comment.trim_start_matches("//");
                if is_directive(line) {
                    continue;
                }
                out.push(
                    line.strip_prefix(' ')
                        .unwrap_or(line)
                        .trim_end()
                        .to_string(),
                );
            }
        }
        out.join("\n").trim().to_string()
    }

    /// The documentation of a spec in a declaration: its own, else the
    /// declaration's when `group` allows it.
    fn spec_doc(&self, spec: Node, decl: Node, group: bool) -> String {
        let own = self.doc(spec);
        if !own.is_empty() || !group {
            return own;
        }
        self.doc(decl)
    }

    fn symbol(
        &mut self,
        scope: &str,
        name: &str,
        kind: SymbolKind,
        span: Node,
        signature: String,
        doc: String,
    ) -> String {
        let generated = self.generated;
        let symbol = self.sink.symbol(scope, name, kind, span);
        symbol.visibility = visibility(name);
        symbol.signature = signature;
        symbol.doc = doc;
        if generated {
            symbol.attrs.push(GENERATED_ATTR.to_string());
        }
        symbol.qualified.clone()
    }

    fn imports(&mut self, decl: Node) {
        for child in named(decl) {
            match child.kind() {
                "import_spec" => self.import(child),
                "import_spec_list" => {
                    for spec in named(child) {
                        if spec.kind() == "import_spec" {
                            self.import(spec);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn import(&mut self, spec: Node) {
        let Some(path_node) = spec.child_by_field_name("path") else {
            return;
        };
        let path = self.text(path_node).trim_matches(['"', '`']);
        // `import "C"` is cgo, not a package.
        if path.is_empty() || path == "C" {
            return;
        }
        let segments: Vec<String> = path.split('/').map(str::to_string).collect();
        let bound = spec.child_by_field_name("name");
        match bound.map(|n| n.kind()) {
            Some("dot") => self.sink.import(segments, "", true, false),
            // A blank import only says the file depends on the package.
            Some("blank_identifier") => self.sink.import(segments, "", false, false),
            Some(_) => {
                let alias = self.text(bound.unwrap_or(spec)).to_string();
                self.imported.insert(alias.clone());
                self.sink.import(segments, &alias, false, false);
            }
            None => {
                let alias = (self.import_name)(path).unwrap_or_else(|| guess_package_name(path));
                self.imported.insert(alias.clone());
                self.sink.import(segments, &alias, false, false);
            }
        }
    }

    // ----- declarations ---------------------------------------------------

    fn function(&mut self, node: Node) {
        let Some(name_node) = node.child_by_field_name("name") else {
            return;
        };
        let name = self.text(name_node);
        if name == "_" {
            return;
        }
        let end = node
            .child_by_field_name("body")
            .map_or(node.end_byte(), |b| b.start_byte());
        let signature = collapse(self.sink.between(node.start_byte(), end));
        let doc = self.doc(node);
        let module = self.sink.module.clone();
        let owner = self.symbol(&module, name, SymbolKind::Function, node, signature, doc);
        let mut fun = Fun {
            owner,
            locals: HashSet::new(),
            receiver: None,
        };
        collect_locals(self.sink.src, node, &mut fun.locals);
        self.walk_parts(node, name_node, &fun);
    }

    fn method(&mut self, node: Node) {
        let (Some(name_node), Some(receiver)) = (
            node.child_by_field_name("name"),
            node.child_by_field_name("receiver"),
        ) else {
            return;
        };
        let name = self.text(name_node);
        let Some(declared) = named(receiver)
            .into_iter()
            .find(|n| n.kind() == "parameter_declaration")
        else {
            return;
        };
        let Some(base) = declared
            .child_by_field_name("type")
            .and_then(|t| self.receiver_base(t))
        else {
            return;
        };
        if name == "_" {
            return;
        }
        let end = node
            .child_by_field_name("body")
            .map_or(node.end_byte(), |b| b.start_byte());
        let signature = collapse(self.sink.between(node.start_byte(), end));
        let doc = self.doc(node);
        let scope = format!("{}::{base}", self.sink.module);
        let owner = self.symbol(&scope, name, SymbolKind::Method, node, signature, doc);
        let mut fun = Fun {
            owner,
            locals: HashSet::new(),
            receiver: None,
        };
        collect_locals(self.sink.src, node, &mut fun.locals);
        // `func (s *Set[T]) Add`: `T` is the receiver's type parameter.
        if let Some(ty) = declared.child_by_field_name("type") {
            type_parameters_of(self.sink.src, ty, &mut fun.locals);
        }
        let receiver_name = declared
            .child_by_field_name("name")
            .map(|n| self.text(n).to_string())
            .filter(|n| n != "_");
        fun.receiver = receiver_name.map(|n| (n, scope));
        self.walk_parts(node, name_node, &fun);
    }

    /// Walk what a function or method mentions: everything but its name.
    fn walk_parts(&mut self, node: Node, name_node: Node, fun: &Fun) {
        for child in named(node) {
            if child.id() != name_node.id() {
                self.walk(child, fun);
            }
        }
    }

    /// The type name a receiver is declared on: `Store` for `*Store`,
    /// `Set` for `Set[T]`.
    fn receiver_base(&self, ty: Node) -> Option<String> {
        match ty.kind() {
            "type_identifier" => Some(self.text(ty).to_string()),
            "pointer_type" | "parenthesized_type" => named(ty)
                .into_iter()
                .next()
                .and_then(|n| self.receiver_base(n)),
            "generic_type" => ty
                .child_by_field_name("type")
                .and_then(|n| self.receiver_base(n)),
            _ => None,
        }
    }

    fn types(&mut self, decl: Node) {
        let specs: Vec<Node> = named(decl)
            .into_iter()
            .filter(|n| matches!(n.kind(), "type_spec" | "type_alias"))
            .collect();
        let group = specs.len() == 1;
        for spec in specs {
            self.type_spec(spec, decl, group);
        }
    }

    fn type_spec(&mut self, spec: Node, decl: Node, group: bool) {
        let (Some(name_node), Some(ty)) = (
            spec.child_by_field_name("name"),
            spec.child_by_field_name("type"),
        ) else {
            return;
        };
        let name = self.text(name_node);
        if name == "_" {
            return;
        }
        let kind = match ty.kind() {
            "struct_type" => SymbolKind::Struct,
            "interface_type" => SymbolKind::Interface,
            _ => SymbolKind::TypeAlias,
        };
        // A struct or interface is named without its body.
        let head_end = match kind {
            SymbolKind::Struct | SymbolKind::Interface => {
                ty.start_byte() + if kind == SymbolKind::Struct { 6 } else { 9 }
            }
            _ => spec.end_byte(),
        };
        let signature = collapse(&format!(
            "type {}",
            self.sink.between(spec.start_byte(), head_end)
        ));
        let doc = self.spec_doc(spec, decl, group);
        let module = self.sink.module.clone();
        let qualified = self.symbol(&module, name, kind, spec, signature, doc);
        let mut fun = Fun {
            owner: qualified.clone(),
            locals: HashSet::new(),
            receiver: None,
        };
        collect_locals(self.sink.src, spec, &mut fun.locals);
        fun.locals.remove(name);
        if let Some(parameters) = spec.child_by_field_name("type_parameters") {
            self.walk(parameters, &fun);
        }
        match kind {
            SymbolKind::Struct => self.struct_body(ty, &qualified, &fun),
            SymbolKind::Interface => self.interface_body(ty, &qualified, &fun),
            _ => self.walk(ty, &fun),
        }
    }

    fn struct_body(&mut self, ty: Node, qualified: &str, fun: &Fun) {
        for list in named(ty) {
            for field in named(list) {
                if field.kind() != "field_declaration" {
                    continue;
                }
                let Some(field_type) = field.child_by_field_name("type") else {
                    continue;
                };
                // A field with no name embeds its type.
                if field.child_by_field_name("name").is_none() {
                    self.embeds(qualified, field_type);
                }
                self.walk(field_type, fun);
            }
        }
    }

    fn interface_body(&mut self, ty: Node, qualified: &str, fun: &Fun) {
        for member in named(ty) {
            match member.kind() {
                "method_elem" => {
                    let Some(name_node) = member.child_by_field_name("name") else {
                        continue;
                    };
                    let name = self.text(name_node);
                    let signature =
                        collapse(self.sink.between(member.start_byte(), member.end_byte()));
                    let doc = self.doc(member);
                    let owner =
                        self.symbol(qualified, name, SymbolKind::Method, member, signature, doc);
                    let inner = Fun {
                        owner,
                        locals: fun.locals.clone(),
                        receiver: None,
                    };
                    self.walk_parts(member, name_node, &inner);
                }
                "type_elem" => {
                    // `~int | string` is a constraint; one plain type is an
                    // embedded interface.
                    let text = self.text(member);
                    if !text.contains(['|', '~'])
                        && let Some(only) = named(member).into_iter().next()
                    {
                        self.embeds(qualified, only);
                    }
                    self.walk(member, fun);
                }
                _ => self.walk(member, fun),
            }
        }
    }

    /// `qualified` embeds the type `ty` names.
    fn embeds(&mut self, qualified: &str, ty: Node) {
        let path = self.type_path(ty);
        self.sink.inherits(qualified, path);
    }

    /// The path a type is written with, `["Store"]` or `["pkg", "Store"]`,
    /// through pointers, parentheses and type arguments.
    fn type_path(&self, ty: Node) -> Vec<String> {
        match ty.kind() {
            "type_identifier" => vec![self.text(ty).to_string()],
            "qualified_type" => {
                let package = ty.child_by_field_name("package").map(|n| self.text(n));
                let name = ty.child_by_field_name("name").map(|n| self.text(n));
                match (package, name) {
                    (Some(p), Some(n)) => vec![p.to_string(), n.to_string()],
                    _ => Vec::new(),
                }
            }
            "generic_type" => ty
                .child_by_field_name("type")
                .map(|n| self.type_path(n))
                .unwrap_or_default(),
            "pointer_type" | "parenthesized_type" => named(ty)
                .into_iter()
                .next()
                .map(|n| self.type_path(n))
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    fn values(&mut self, decl: Node) {
        let constant = decl.kind() == "const_declaration";
        let mut specs: Vec<Node> = Vec::new();
        for child in named(decl) {
            match child.kind() {
                "const_spec" | "var_spec" => specs.push(child),
                "var_spec_list" => specs.extend(named(child)),
                _ => {}
            }
        }
        // A lone spec is described by the comment above the declaration; so
        // is a group of constants, which is usually one enumeration.
        let group = specs.len() == 1 || constant;
        for spec in specs {
            let names = field_all(spec, "name");
            let ty = spec.child_by_field_name("type");
            let value = spec.child_by_field_name("value");
            let doc = self.spec_doc(spec, decl, group);
            let mut fun = Fun {
                owner: self.sink.module.clone(),
                locals: HashSet::new(),
                receiver: None,
            };
            collect_locals(self.sink.src, spec, &mut fun.locals);
            let mut owner = None;
            for name_node in &names {
                let name = self.text(*name_node);
                if name == "_" {
                    continue;
                }
                let signature = self.value_signature(constant, name, ty, value);
                let module = self.sink.module.clone();
                let kind = if constant {
                    SymbolKind::Const
                } else {
                    SymbolKind::Static
                };
                let qualified = self.symbol(&module, name, kind, spec, signature, doc.clone());
                owner.get_or_insert(qualified);
            }
            // `var _ Iface = (*T)(nil)` says `T` implements `Iface`.
            if names.iter().all(|n| self.text(*n) == "_")
                && let (Some(interface), Some(value)) = (ty, value)
                && let Some(implementor) = named(value)
                    .into_iter()
                    .next()
                    .and_then(|v| self.assertion_target(v))
                && implementor.len() == 1
            {
                let local = format!("{}::{}", self.sink.module, implementor[0]);
                let path = self.type_path(interface);
                self.sink.inherits(&local, path);
            }
            // What the declaration mentions belongs to the first name; the
            // module owns it when every name is blank.
            fun.owner = owner.unwrap_or_else(|| self.sink.module.clone());
            if let Some(ty) = ty {
                self.walk(ty, &fun);
            }
            if let Some(value) = value {
                self.walk(value, &fun);
            }
        }
    }

    /// `const Name Type`, or `const Name = value` for a short untyped value.
    fn value_signature(
        &self,
        constant: bool,
        name: &str,
        ty: Option<Node>,
        value: Option<Node>,
    ) -> String {
        let keyword = if constant { "const" } else { "var" };
        if let Some(ty) = ty {
            return collapse(&format!("{keyword} {name} {}", self.text(ty)));
        }
        match value.map(|v| self.text(v)) {
            Some(text) if constant && text.len() <= 40 && !text.contains('\n') => {
                format!("{keyword} {name} = {text}")
            }
            _ => format!("{keyword} {name}"),
        }
    }

    /// The type of `(*T)(nil)`, `&T{}` or `T{}`.
    fn assertion_target(&self, expr: Node) -> Option<Vec<String>> {
        match expr.kind() {
            "type_conversion_expression" => {
                let is_nil = expr
                    .child_by_field_name("operand")
                    .is_some_and(|o| o.kind() == "nil");
                let path = self.type_path(expr.child_by_field_name("type")?);
                (is_nil && !path.is_empty()).then_some(path)
            }
            // `(*T)(nil)` reads as a call of the parenthesised `*T` on `nil`.
            "call_expression" => {
                let only_nil = expr
                    .child_by_field_name("arguments")
                    .is_some_and(|a| named(a).len() == 1 && named(a)[0].kind() == "nil");
                let function = expr.child_by_field_name("function")?;
                let pointer = named(function).into_iter().next()?;
                let operand = pointer.child_by_field_name("operand")?;
                let path = match operand.kind() {
                    "identifier" => vec![self.text(operand).to_string()],
                    "selector_expression" => {
                        let package = operand.child_by_field_name("operand")?;
                        let name = operand.child_by_field_name("field")?;
                        vec![self.text(package).to_string(), self.text(name).to_string()]
                    }
                    _ => return None,
                };
                (only_nil
                    && function.kind() == "parenthesized_expression"
                    && pointer.kind() == "unary_expression")
                    .then_some(path)
            }
            "unary_expression" => self.assertion_target(expr.child_by_field_name("operand")?),
            "composite_literal" => {
                let path = self.type_path(expr.child_by_field_name("type")?);
                (!path.is_empty()).then_some(path)
            }
            "parenthesized_expression" => self.assertion_target(named(expr).into_iter().next()?),
            _ => None,
        }
    }

    // ----- what code mentions ---------------------------------------------

    /// Record what an expression, statement or type mentions.
    fn walk(&mut self, node: Node, fun: &Fun) {
        match node.kind() {
            "comment" => {}
            "type_identifier" => {
                let name = self.text(node);
                if !BUILTIN_TYPES.contains(&name) && !fun.locals.contains(name) {
                    self.sink.type_ref(&fun.owner, vec![name.to_string()]);
                }
            }
            "qualified_type" => {
                let path = self.type_path(node);
                if path.len() == 2 {
                    self.sink.type_ref(&fun.owner, path);
                }
            }
            "identifier" => {
                let name = self.text(node);
                if name != "_" && !fun.locals.contains(name) && !BUILTIN_FUNCTIONS.contains(&name) {
                    self.sink.value(&fun.owner, vec![name.to_string()]);
                }
            }
            "selector_expression" => self.selector(node, fun),
            "call_expression" => self.call(node, fun),
            "type_conversion_expression" => self.conversion(node, fun),
            // Names are declarations; only the types and values around them
            // are mentions.
            "parameter_declaration" | "variadic_parameter_declaration" | "field_declaration" => {
                if let Some(ty) = node.child_by_field_name("type") {
                    self.walk(ty, fun);
                }
            }
            "type_parameter_declaration" => {
                if let Some(ty) = node.child_by_field_name("type") {
                    self.walk(ty, fun);
                }
            }
            "short_var_declaration" | "range_clause" | "receive_statement" => {
                if let Some(right) = node.child_by_field_name("right") {
                    self.walk(right, fun);
                }
            }
            "var_spec" | "const_spec" => {
                for field in ["type", "value"] {
                    if let Some(part) = node.child_by_field_name(field) {
                        self.walk(part, fun);
                    }
                }
            }
            "type_spec" | "type_alias" => {
                if let Some(ty) = node.child_by_field_name("type") {
                    self.walk(ty, fun);
                }
            }
            "method_elem" => {
                for field in ["parameters", "result"] {
                    if let Some(part) = node.child_by_field_name(field) {
                        self.walk(part, fun);
                    }
                }
            }
            "keyed_element" => {
                // `Store{db: x}` names a field; a bare key is never a value.
                let mut parts = named(node).into_iter();
                if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                    let bare = named(key).first().is_some_and(|k| k.kind() == "identifier");
                    if !bare {
                        self.walk(key, fun);
                    }
                    self.walk(value, fun);
                }
            }
            _ => {
                for child in named(node) {
                    self.walk(child, fun);
                }
            }
        }
    }

    /// `pkg.Name` through an import, or `x.field`.
    fn selector(&mut self, node: Node, fun: &Fun) {
        let (Some(operand), Some(field)) = (
            node.child_by_field_name("operand"),
            node.child_by_field_name("field"),
        ) else {
            return;
        };
        match self.package_of(operand, fun) {
            Some(package) => {
                let name = self.text(field).to_string();
                self.sink.value(&fun.owner, vec![package, name]);
            }
            None => self.walk(operand, fun),
        }
    }

    /// The import name `node` is, when it is one and nothing inside the
    /// function hides it.
    fn package_of(&self, node: Node, fun: &Fun) -> Option<String> {
        if node.kind() != "identifier" {
            return None;
        }
        let name = self.text(node);
        (self.imported.contains(name) && !fun.locals.contains(name)).then(|| name.to_string())
    }

    fn call(&mut self, node: Node, fun: &Fun) {
        if let Some(function) = node.child_by_field_name("function") {
            self.callee(function, fun);
        }
        for field in ["type_arguments", "arguments"] {
            if let Some(part) = node.child_by_field_name(field) {
                self.walk(part, fun);
            }
        }
    }

    /// The thing a call calls.
    fn callee(&mut self, function: Node, fun: &Fun) {
        match function.kind() {
            "identifier" => {
                let name = self.text(function);
                // A variable holding a function is called through its own
                // name, which is no symbol.
                if !BUILTIN_FUNCTIONS.contains(&name) && !fun.locals.contains(name) {
                    self.sink
                        .call(&fun.owner, vec![name.to_string()], false, None, function);
                }
            }
            "selector_expression" => self.method_call(function, fun),
            // `Map[int, string](x)` instantiates a generic function.
            "index_expression" => {
                if let Some(operand) = function.child_by_field_name("operand") {
                    self.callee(operand, fun);
                }
                if let Some(index) = function.child_by_field_name("index") {
                    self.walk(index, fun);
                }
            }
            _ => self.walk(function, fun),
        }
    }

    fn method_call(&mut self, function: Node, fun: &Fun) {
        let (Some(operand), Some(field)) = (
            function.child_by_field_name("operand"),
            function.child_by_field_name("field"),
        ) else {
            return;
        };
        let name = self.text(field).to_string();
        if let Some((receiver, ty)) = &fun.receiver
            && operand.kind() == "identifier"
            && self.text(operand) == receiver
        {
            self.sink
                .call(&fun.owner, vec![name], true, Some(ty), field);
            return;
        }
        match self.package_of(operand, fun) {
            Some(package) => self
                .sink
                .call(&fun.owner, vec![package, name], false, None, field),
            None => {
                self.sink.call(&fun.owner, vec![name], true, None, field);
                self.walk(operand, fun);
            }
        }
    }

    /// `Map[int](x)` is read as a conversion to the generic type `Map[int]`:
    /// when the type is written like a name, it is a call of a generic
    /// function as well.
    fn conversion(&mut self, node: Node, fun: &Fun) {
        if let Some(ty) = node.child_by_field_name("type") {
            if ty.kind() == "generic_type" {
                let path = self.type_path(ty);
                let at = ty.child_by_field_name("type").unwrap_or(ty);
                let local = path.len() == 1 && fun.locals.contains(&path[0]);
                if !path.is_empty() && !local {
                    let package_call = path.len() == 2 && !self.imported.contains(&path[0]);
                    if !package_call {
                        self.sink.call(&fun.owner, path, false, None, at);
                    }
                }
            }
            self.walk(ty, fun);
        }
        if let Some(operand) = node.child_by_field_name("operand") {
            self.walk(operand, fun);
        }
    }
}

/// True for a comment line that is a compiler or tool directive
/// (`go:generate`, `nolint:errcheck`), which Go leaves out of documentation.
fn is_directive(line: &str) -> bool {
    let Some((word, rest)) = line.split_once(':') else {
        return line.starts_with("export ");
    };
    !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && rest.starts_with(|c: char| c.is_ascii_alphanumeric())
}

/// Every name bound anywhere inside `node`.
fn collect_locals(src: &[u8], node: Node, out: &mut HashSet<String>) {
    let text = |n: Node| n.utf8_text(src).unwrap_or("").to_string();
    match node.kind() {
        "parameter_declaration"
        | "variadic_parameter_declaration"
        | "type_parameter_declaration"
        | "var_spec"
        | "const_spec" => {
            for name in field_all(node, "name") {
                out.insert(text(name));
            }
        }
        "short_var_declaration" | "range_clause" | "receive_statement" => {
            if let Some(left) = node.child_by_field_name("left") {
                out.extend(
                    named(left)
                        .into_iter()
                        .filter(|n| n.kind() == "identifier")
                        .map(text),
                );
            }
        }
        "type_switch_statement" => {
            if let Some(alias) = node.child_by_field_name("alias") {
                out.extend(
                    named(alias)
                        .into_iter()
                        .filter(|n| n.kind() == "identifier")
                        .map(text),
                );
            }
        }
        "type_spec" | "type_alias" => {
            if let Some(name) = node.child_by_field_name("name") {
                out.insert(text(name));
            }
        }
        _ => {}
    }
    for child in named(node) {
        collect_locals(src, child, out);
    }
}

/// The type parameters a receiver type lists: `T` in `*Set[T]`.
fn type_parameters_of(src: &[u8], ty: Node, out: &mut HashSet<String>) {
    if let Some(arguments) = (ty.kind() == "generic_type")
        .then(|| ty.child_by_field_name("type_arguments"))
        .flatten()
    {
        for element in named(arguments) {
            for part in named(element) {
                if part.kind() == "type_identifier" {
                    out.insert(part.utf8_text(src).unwrap_or("").to_string());
                }
            }
        }
    }
    for child in named(ty) {
        type_parameters_of(src, child, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cw_core::Symbol;

    fn parse(file: &str, source: &str) -> ParsedFile {
        parse_go(file, source, "example_com::app::store", &|_| None).unwrap()
    }

    fn symbol<'a>(p: &'a ParsedFile, q: &str) -> &'a Symbol {
        p.symbols
            .iter()
            .find(|s| s.qualified == q)
            .unwrap_or_else(|| {
                let all: Vec<&str> = p.symbols.iter().map(|s| s.qualified.as_str()).collect();
                panic!("missing symbol {q} in {all:?}")
            })
    }

    const M: &str = "example_com::app::store";
    const SOURCE: &str = r#"// Package store keeps things.
//
// Second paragraph.
package store

import (
	"fmt"
	str "strings"
	_ "embed"
	. "math"
	"example.com/app/internal/text"
	"github.com/spf13/cobra"
)

// MaxItems is the most a store holds.
const MaxItems = 10

const (
	// Active means the item counts.
	Active Status = iota
	Idle
)

var ErrMissing = errors.New("missing")
var cache, backup map[string]Item

var _ Reader = (*Store)(nil)
var _ Writer = &Store{}

//go:generate stringer -type=Status
type Status int

type Alias = text.Label

// Store is a keyed store.
type Store struct {
	Base
	*text.Label
	items map[string]Item `json:"items"`
}

// Reader reads.
type Reader interface {
	io.Reader
	Sizer
	// Read reads one item.
	Read(key string) (Item, error)
	size() int
}

type Number interface {
	~int | ~float64
}

type Set[T comparable] struct {
	items map[T]struct{}
}

// New makes a store.
func New(name string) *Store {
	s := &Store{items: map[string]Item{}}
	s.init()
	fmt.Println(text.Shout(name), MaxItems)
	return s
}

// Get reads one item.
func (s *Store) Get(key string) (Item, error) {
	s.items[key] = helper(key)
	return s.read(key)
}

func (s Store) read(key string) (Item, error) {
	other := cache
	other.Get(key)
	return Item{}, ErrMissing
}

func (s *Set[T]) Add(x T) { Map[int](x) }

func helper(key string) Item {
	handler := func(x int) { helper2(x) }
	handler(1)
	str.ToUpper(key)
	cobra.Execute()
	return Item{}
}
"#;

    #[test]
    fn extracts_symbols() {
        let p = parse("internal/store/store.go", SOURCE);
        assert!(!p.has_errors);

        let package = symbol(&p, M);
        assert_eq!(package.kind, SymbolKind::Module);
        assert_eq!(package.signature, "package store");
        assert_eq!(
            package.doc,
            "Package store keeps things.\n\nSecond paragraph."
        );
        assert_eq!((package.start_line, package.end_line), (4, 4));
        assert_eq!(package.parent.as_deref(), Some("example_com::app"));

        let max = symbol(&p, &format!("{M}::MaxItems"));
        assert_eq!(max.kind, SymbolKind::Const);
        assert_eq!(max.visibility, Visibility::Public);
        assert_eq!(max.signature, "const MaxItems = 10");
        assert_eq!(max.doc, "MaxItems is the most a store holds.");

        // A group of constants takes the comment of each spec.
        let active = symbol(&p, &format!("{M}::Active"));
        assert_eq!(active.signature, "const Active Status");
        assert_eq!(active.doc, "Active means the item counts.");
        assert_eq!(symbol(&p, &format!("{M}::Idle")).kind, SymbolKind::Const);

        let missing = symbol(&p, &format!("{M}::ErrMissing"));
        assert_eq!(missing.kind, SymbolKind::Static);
        assert_eq!(missing.signature, "var ErrMissing");
        assert_eq!(
            symbol(&p, &format!("{M}::cache")).visibility,
            Visibility::Crate
        );
        assert_eq!(symbol(&p, &format!("{M}::backup")).kind, SymbolKind::Static);
        // The blank names declare nothing.
        assert!(p.symbols.iter().all(|s| s.name != "_"));

        // A directive is no documentation.
        let status = symbol(&p, &format!("{M}::Status"));
        assert_eq!(status.kind, SymbolKind::TypeAlias);
        assert_eq!(status.signature, "type Status int");
        assert_eq!(status.doc, "");
        let alias = symbol(&p, &format!("{M}::Alias"));
        assert_eq!(alias.kind, SymbolKind::TypeAlias);
        assert_eq!(alias.signature, "type Alias = text.Label");

        let store = symbol(&p, &format!("{M}::Store"));
        assert_eq!(store.kind, SymbolKind::Struct);
        assert_eq!(store.signature, "type Store struct");
        assert_eq!(store.doc, "Store is a keyed store.");
        // Fields are no symbols.
        assert!(p.symbols.iter().all(|s| s.name != "items"));

        let reader = symbol(&p, &format!("{M}::Reader"));
        assert_eq!(reader.kind, SymbolKind::Interface);
        assert_eq!(reader.signature, "type Reader interface");
        let read = symbol(&p, &format!("{M}::Reader::Read"));
        assert_eq!(read.kind, SymbolKind::Method);
        assert_eq!(read.signature, "Read(key string) (Item, error)");
        assert_eq!(read.doc, "Read reads one item.");
        assert_eq!(
            symbol(&p, &format!("{M}::Reader::size")).visibility,
            Visibility::Crate
        );

        let set = symbol(&p, &format!("{M}::Set"));
        assert_eq!(set.signature, "type Set[T comparable] struct");

        let new = symbol(&p, &format!("{M}::New"));
        assert_eq!(new.kind, SymbolKind::Function);
        assert_eq!(new.signature, "func New(name string) *Store");
        assert_eq!(new.doc, "New makes a store.");
        assert_eq!(new.parent.as_deref(), Some(M));
        assert_eq!(
            symbol(&p, &format!("{M}::helper")).visibility,
            Visibility::Crate
        );
    }

    #[test]
    fn a_method_belongs_to_its_receiver_type() {
        let p = parse("internal/store/store.go", SOURCE);
        // Pointer and value receivers alike, and a generic one.
        let get = symbol(&p, &format!("{M}::Store::Get"));
        assert_eq!(get.kind, SymbolKind::Method);
        assert_eq!(get.parent.as_deref(), Some(format!("{M}::Store").as_str()));
        assert_eq!(
            get.signature,
            "func (s *Store) Get(key string) (Item, error)"
        );
        assert_eq!(get.visibility, Visibility::Public);
        assert_eq!(get.doc, "Get reads one item.");
        let read = symbol(&p, &format!("{M}::Store::read"));
        assert_eq!(
            read.signature,
            "func (s Store) read(key string) (Item, error)"
        );
        assert_eq!(read.visibility, Visibility::Crate);
        let add = symbol(&p, &format!("{M}::Set::Add"));
        assert_eq!(add.signature, "func (s *Set[T]) Add(x T)");
    }

    #[test]
    fn extracts_imports() {
        let p = parse("internal/store/store.go", SOURCE);
        let has = |path: &[&str], alias: &str, glob: bool| {
            p.uses
                .iter()
                .any(|u| u.path == path && u.alias == alias && u.glob == glob)
        };
        assert!(has(&["fmt"], "fmt", false));
        // An alias is the name the file uses.
        assert!(has(&["strings"], "str", false));
        // A blank import binds nothing; a dot import opens the package.
        assert!(has(&["embed"], "", false));
        assert!(has(&["math"], "", true));
        assert!(has(
            &["example.com", "app", "internal", "text"],
            "text",
            false
        ));
        assert!(has(&["github.com", "spf13", "cobra"], "cobra", false));
        assert!(p.uses.iter().all(|u| u.scope == M));
    }

    #[test]
    fn an_import_goes_by_the_name_its_package_declares() {
        let names = |path: &str| (path == "example.com/app/db").then(|| "sqlstore".to_string());
        let p = parse_go(
            "a.go",
            "package a\nimport \"example.com/app/db\"\nimport \"github.com/go-chi/chi/v5\"\nfunc f() { sqlstore.Open(); chi.NewRouter() }\n",
            "a",
            &names,
        )
        .unwrap();
        let aliases: Vec<&str> = p.uses.iter().map(|u| u.alias.as_str()).collect();
        assert_eq!(aliases, ["sqlstore", "chi"]);
        let call = |path: &[&str]| p.calls.iter().find(|c| c.path == path);
        assert!(!call(&["sqlstore", "Open"]).unwrap().method);
        assert!(!call(&["chi", "NewRouter"]).unwrap().method);
    }

    #[test]
    fn extracts_calls() {
        let p = parse("internal/store/store.go", SOURCE);
        let call = |caller: &str, path: &[&str]| {
            p.calls
                .iter()
                .find(|c| c.caller == format!("{M}::{caller}") && c.path == path)
                .unwrap_or_else(|| panic!("missing call {caller} -> {path:?}"))
        };
        // Through an import name, standard library and repository alike.
        assert!(!call("New", &["text", "Shout"]).method);
        assert!(!call("New", &["fmt", "Println"]).method);
        // A call on the receiver of the method is a call on its type.
        let on_receiver = call("New", &["init"]);
        assert!(on_receiver.method && on_receiver.self_type.is_none());
        let read = call("Store::Get", &["read"]);
        assert!(read.method);
        assert_eq!(
            read.self_type.as_deref(),
            Some(format!("{M}::Store").as_str())
        );
        // A bare name is a package-level function.
        assert!(!call("Store::Get", &["helper"]).method);
        // A variable of unknown type: a method on an unknown receiver.
        let unknown = call("Store::read", &["Get"]);
        assert!(unknown.method && unknown.self_type.is_none());
        // A generic function, with its type arguments.
        assert!(!call("Set::Add", &["Map"]).method);
        // A closure called through its variable is no call of the package.
        assert!(
            !p.calls
                .iter()
                .any(|c| c.caller == format!("{M}::helper") && c.path == ["handler"])
        );
        assert!(!call("helper", &["str", "ToUpper"]).method);
        assert!(!call("helper", &["cobra", "Execute"]).method);
        // What a closure calls belongs to the function around it.
        assert!(!call("helper", &["helper2"]).method);
        // A builtin is no call.
        assert!(
            p.calls
                .iter()
                .all(|c| c.path != ["make"] && c.path != ["len"])
        );
    }

    #[test]
    fn a_call_knows_where_it_stands() {
        let source = "package p\nfunc run() {\n\tthird(second(first()))\n\tx.alpha().beta()\n}\n";
        let p = parse_go("p/a.go", source, "p", &|_| None).unwrap();
        let mut calls: Vec<(u32, &str)> = p
            .calls
            .iter()
            .map(|c| (c.at, c.path[c.path.len() - 1].as_str()))
            .collect();
        calls.sort();
        for (at, name) in &calls {
            assert!(source[*at as usize..].starts_with(name), "{name} at {at}");
        }
        let names: Vec<&str> = calls.iter().map(|(_, n)| *n).collect();
        assert_eq!(names, ["third", "second", "first", "alpha", "beta"]);
    }

    #[test]
    fn names_bound_in_a_function_are_not_package_names() {
        let source = "package p\nvar limit = 3\nfunc run(cache int) int {\n\tlimit := 1\n\tfor i, v := range xs { use(i, v) }\n\treturn cache + limit + total\n}\n";
        let p = parse_go("p/a.go", source, "p", &|_| None).unwrap();
        let values: Vec<Vec<&str>> = p
            .value_refs
            .iter()
            .filter(|v| v.from == "p::run")
            .map(|v| v.path.iter().map(String::as_str).collect())
            .collect();
        // `total` is the one name nothing binds; `xs` is read too.
        assert!(values.contains(&vec!["total"]));
        assert!(values.contains(&vec!["xs"]));
        assert!(!values.contains(&vec!["limit"]));
        assert!(!values.contains(&vec!["cache"]));
        assert!(!values.contains(&vec!["i"]) && !values.contains(&vec!["v"]));
    }

    #[test]
    fn a_value_through_an_import_is_a_path() {
        let source = "package p\nimport \"example.com/cfg\"\nfunc run() int { return cfg.Limit + x.field }\n";
        let p = parse_go("p/a.go", source, "p", &|_| None).unwrap();
        assert!(
            p.value_refs
                .iter()
                .any(|v| v.from == "p::run" && v.path == ["cfg", "Limit"])
        );
        assert!(p.value_refs.iter().all(|v| v.path != ["field"]));
    }

    #[test]
    fn extracts_embedding_as_inheritance() {
        let p = parse("internal/store/store.go", SOURCE);
        let supers = |ty: &[&str]| -> Vec<Vec<String>> {
            p.impls
                .iter()
                .filter(|i| i.type_path == ty)
                .filter_map(|i| i.trait_path.clone())
                .collect()
        };
        // A struct embeds a type of the package and a pointer to another's.
        assert_eq!(
            supers(&["Store"]),
            [
                vec!["Reader"],
                vec!["Writer"],
                vec!["Base"],
                vec!["text", "Label"]
            ]
        );
        // An interface embeds interfaces; a union is a constraint.
        assert_eq!(supers(&["Reader"]), [vec!["io", "Reader"], vec!["Sizer"]]);
        assert!(supers(&["Number"]).is_empty());
        assert!(
            p.type_refs
                .iter()
                .any(|t| t.from == format!("{M}::Store") && t.path == ["text", "Label"])
        );
        assert!(
            p.type_refs
                .iter()
                .any(|t| t.from == format!("{M}::Store") && t.path == ["Item"])
        );
        // A type parameter and a predeclared type are no references.
        assert!(
            !p.type_refs
                .iter()
                .any(|t| t.from == format!("{M}::Set") && t.path == ["T"])
        );
        assert!(
            p.type_refs
                .iter()
                .all(|t| t.path != ["string"] && t.path != ["error"])
        );
    }

    #[test]
    fn a_struct_literal_key_is_no_value() {
        let source =
            "package p\nvar db = 1\nfunc New() *Store { return &Store{db: 2, other: db} }\n";
        let p = parse_go("p/a.go", source, "p", &|_| None).unwrap();
        let names: Vec<&str> = p
            .value_refs
            .iter()
            .filter(|v| v.from == "p::New")
            .map(|v| v.path[0].as_str())
            .collect();
        // Only the value `db`, never the key.
        assert_eq!(names, ["db"]);
        assert!(
            p.type_refs
                .iter()
                .any(|t| t.from == "p::New" && t.path == ["Store"])
        );
    }

    #[test]
    fn marks_generated_files() {
        let source = "// Code generated by protoc-gen-go. DO NOT EDIT.\n// source: a.proto\n\npackage pb\n\ntype Msg struct{}\n\nfunc (m *Msg) Reset() {}\n";
        let p = parse_go("pb/a.pb.go", source, "pb", &|_| None).unwrap();
        assert_eq!(symbol(&p, "pb::Msg").attrs, [GENERATED_ATTR]);
        assert_eq!(symbol(&p, "pb::Msg::Reset").attrs, [GENERATED_ATTR]);
        // The package itself belongs to every file of the folder.
        assert!(symbol(&p, "pb").attrs.is_empty());
        let plain = parse_go(
            "pb/b.go",
            "package pb\n\ntype Other struct{}\n",
            "pb",
            &|_| None,
        )
        .unwrap();
        assert!(symbol(&plain, "pb::Other").attrs.is_empty());
        let late = parse_go(
            "pb/c.go",
            "package pb\n\n// Code generated by hand. DO NOT EDIT.\ntype X struct{}\n",
            "pb",
            &|_| None,
        )
        .unwrap();
        assert!(symbol(&late, "pb::X").attrs.is_empty());
    }

    #[test]
    fn an_external_test_package_is_a_module_of_its_own() {
        let source = "package store_test\n\nimport \"testing\"\n\nfunc TestNew(t *testing.T) {}\n";
        let p = parse_go("internal/store/store_test.go", source, M, &|_| None).unwrap();
        let module = symbol(&p, &format!("{M}_test"));
        assert_eq!(module.signature, "package store_test");
        assert_eq!(
            symbol(&p, &format!("{M}_test::TestNew")).kind,
            SymbolKind::Function
        );
        // An in-package test file stays in the package.
        let inside = parse_go(
            "internal/store/a_test.go",
            "package store\nfunc TestA(t *testing.T) {}\n",
            M,
            &|_| None,
        )
        .unwrap();
        assert_eq!(
            symbol(&inside, &format!("{M}::TestA")).kind,
            SymbolKind::Function
        );
    }

    #[test]
    fn names_a_file_with_no_module_by_its_package_clause() {
        let p = parse_go("main.go", "package tool\nfunc main() {}\n", "", &|_| None).unwrap();
        assert_eq!(symbol(&p, "tool::main").kind, SymbolKind::Function);
        assert_eq!(symbol(&p, "tool").signature, "package tool");
    }

    #[test]
    fn reads_the_package_clause_without_a_parser() {
        assert_eq!(package_name("package store\n"), Some("store"));
        assert_eq!(
            package_name("// Copyright\n/* block\n package x */\n\npackage db // d\n"),
            Some("db")
        );
        assert_eq!(package_name("import \"x\"\n"), None);
        assert_eq!(package_name(""), None);
    }

    #[test]
    fn leaves_documentation_of_a_neighbour_alone() {
        let source = "package p\n\nconst A = 1 // about A\nconst B = 2\n\n// stray\n\n// Doc of C.\nfunc C() {}\n";
        let p = parse_go("p/a.go", source, "p", &|_| None).unwrap();
        assert_eq!(symbol(&p, "p::B").doc, "");
        assert_eq!(symbol(&p, "p::C").doc, "Doc of C.");
    }

    #[test]
    fn a_syntax_error_is_reported() {
        let p = parse_go("p/a.go", "package p\nfunc (", "p", &|_| None).unwrap();
        assert!(p.has_errors);
    }
}
