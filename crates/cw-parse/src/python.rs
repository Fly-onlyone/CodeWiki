//! Python symbol and fact extraction.
//!
//! Written against the node kinds and field names published in the grammar's
//! own `node-types.json` (tree-sitter-python 0.25). No query files are used.

use std::collections::HashSet;

use cw_core::{Error, Result, SymbolKind, Visibility};
use tree_sitter::{Node, Parser};

use crate::lang::{Sink, collapse, dedent, dotted, field_all, marker_name, named, split_qualified};
use crate::rust::ParsedFile;

/// Parse one Python source file. `module` is the qualified module the file
/// defines, for example `pkg::util`; `is_package` is true for `__init__.py`,
/// which changes what a relative import is relative to.
///
/// Imports are recorded as written (relative ones made absolute); the scanner
/// decides afterwards which of them name repository modules.
pub fn parse_python(
    file: &str,
    module: &str,
    is_package: bool,
    source: &str,
) -> Result<ParsedFile> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .map_err(|e| Error::external("loading the Python grammar", e))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| Error::invalid(format!("parser produced no tree for {file}")))?;
    let root = tree.root_node();
    let mut ex = Extractor {
        sink: Sink::new(file, source, module, root.has_error()),
        is_package,
        roots: HashSet::new(),
    };
    ex.collect_roots(root, true);
    let doc = ex.docstring(root);
    ex.sink.module_symbol(
        format!("module {}", module.replace("::", ".")),
        doc,
        1,
        source.lines().count() as u32,
    );
    if has_main_guard(root, ex.sink.src)
        && let Some(module) = ex.sink.out.symbols.last_mut()
    {
        module.attrs.push(MAIN_GUARD_ATTR.to_string());
    }
    let mut locals = HashSet::new();
    bindings(root, ex.sink.src, &mut locals);
    // `handler = wrap(handler)` rebinds a function; it stays a function.
    locals.retain(|name| !ex.roots.contains(name));
    let ctx = Ctx {
        owner: module.to_string(),
        class: None,
        self_type: None,
        self_name: None,
        locals,
        nested: false,
    };
    ex.block(root, &ctx);
    Ok(ex.sink.out)
}

/// Attribute recorded on the module symbol of a file that runs as a script:
/// it has a top-level `if __name__ == "__main__":` block.
pub const MAIN_GUARD_ATTR: &str = "__main__";

/// True when the file has a top-level `if __name__ == "__main__":`.
fn has_main_guard(root: Node, src: &[u8]) -> bool {
    named(root).into_iter().any(|node| {
        node.kind() == "if_statement"
            && node.child_by_field_name("condition").is_some_and(|c| {
                let condition: String = text(c, src)
                    .chars()
                    .filter(|ch| !ch.is_whitespace() && !matches!(ch, '(' | ')'))
                    .map(|ch| if ch == '\'' { '"' } else { ch })
                    .collect();
                condition == "__name__==\"__main__\"" || condition == "\"__main__\"==__name__"
            })
    })
}

#[derive(Clone)]
struct Ctx {
    /// Qualified name new symbols nest under and references are charged to.
    owner: String,
    /// The class whose body is being walked, if any.
    class: Option<String>,
    /// Class of the method being walked.
    self_type: Option<String>,
    /// Name of the instance parameter of that method, usually `self`.
    self_name: Option<String>,
    /// Names bound in the enclosing function scopes. A use of one of these is
    /// not a reference to a module-level symbol of the same name.
    locals: HashSet<String>,
    /// True inside a function body.
    nested: bool,
}

struct Extractor<'a> {
    sink: Sink<'a>,
    is_package: bool,
    /// Names bound by imports and module-level definitions. Only a dotted
    /// call that starts at one of these is resolved as a path.
    roots: HashSet<String>,
}

fn visibility(name: &str, nested: bool) -> Visibility {
    let dunder = name.len() > 4 && name.starts_with("__") && name.ends_with("__");
    if nested || (name.starts_with('_') && !dunder) {
        Visibility::Private
    } else {
        Visibility::Public
    }
}

fn text<'a>(node: Node, src: &'a [u8]) -> &'a str {
    node.utf8_text(src).unwrap_or("")
}

/// Names bound by an assignment target, loop variable or `as` clause.
fn pattern_names(node: Node, src: &[u8], out: &mut HashSet<String>) {
    match node.kind() {
        "identifier" => {
            out.insert(text(node, src).to_string());
        }
        // `obj.attr = ...` and `xs[i] = ...` bind no name.
        "attribute" | "subscript" => {}
        _ if node.named_child_count() == 0 => {
            if node.kind() == "as_pattern_target" {
                out.insert(text(node, src).to_string());
            }
        }
        _ => {
            for child in named(node) {
                pattern_names(child, src, out);
            }
        }
    }
}

fn param_names(params: Node, src: &[u8], out: &mut HashSet<String>) {
    for param in named(params) {
        match param.kind() {
            "default_parameter" | "typed_default_parameter" => {
                if let Some(name) = param.child_by_field_name("name") {
                    pattern_names(name, src, out);
                }
            }
            "typed_parameter" => {
                for child in named(param) {
                    if child.kind() != "type" {
                        pattern_names(child, src, out);
                    }
                }
            }
            _ => pattern_names(param, src, out),
        }
    }
}

/// Names bound anywhere in a scope, not descending into nested scopes.
fn bindings(node: Node, src: &[u8], out: &mut HashSet<String>) {
    for child in named(node) {
        match child.kind() {
            "function_definition" | "class_definition" | "lambda" => continue,
            "assignment" | "for_statement" | "for_in_clause" => {
                if let Some(left) = child.child_by_field_name("left") {
                    pattern_names(left, src, out);
                }
            }
            "named_expression" => {
                if let Some(name) = child.child_by_field_name("name") {
                    pattern_names(name, src, out);
                }
            }
            "as_pattern" | "except_clause" => {
                if let Some(alias) = child.child_by_field_name("alias") {
                    pattern_names(alias, src, out);
                }
            }
            _ => {}
        }
        bindings(child, src, out);
    }
}

impl<'a> Extractor<'a> {
    /// Names an import or module-level definition binds in this file.
    fn collect_roots(&mut self, node: Node, top: bool) {
        for child in named(node) {
            match child.kind() {
                "import_statement" | "import_from_statement" => {
                    for name in field_all(child, "name") {
                        let bound = match name.kind() {
                            "aliased_import" => name
                                .child_by_field_name("alias")
                                .map(|a| self.sink.text(a).to_string()),
                            _ => dotted(self.sink.text(name)).into_iter().next(),
                        };
                        self.roots.extend(bound);
                    }
                }
                "function_definition" | "class_definition" => {
                    if top && let Some(name) = child.child_by_field_name("name") {
                        self.roots.insert(self.sink.text(name).to_string());
                    }
                    self.collect_roots(child, false);
                }
                _ => self.collect_roots(child, top),
            }
        }
    }

    /// The string literal that opens a module, class or function body.
    fn docstring(&self, body: Node) -> String {
        let Some(first) = named(body).into_iter().find(|n| n.kind() != "comment") else {
            return String::new();
        };
        if first.kind() != "expression_statement" {
            return String::new();
        }
        let Some(string) = first.named_child(0).filter(|n| n.kind() == "string") else {
            return String::new();
        };
        let content: String = named(string)
            .into_iter()
            .filter(|n| n.kind() == "string_content")
            .map(|n| self.sink.text(n))
            .collect();
        dedent(&content)
    }

    fn signature(&self, def: Node) -> String {
        let end = def
            .child_by_field_name("body")
            .map_or(def.end_byte(), |b| b.start_byte());
        collapse(self.sink.between(def.start_byte(), end))
    }

    /// Segments of `a.b.c` when every part is a plain name.
    fn chain(&self, node: Node) -> Option<Vec<String>> {
        match node.kind() {
            "identifier" => Some(vec![self.sink.text(node).to_string()]),
            "attribute" => {
                let mut path = self.chain(node.child_by_field_name("object")?)?;
                path.push(
                    self.sink
                        .text(node.child_by_field_name("attribute")?)
                        .to_string(),
                );
                Some(path)
            }
            _ => None,
        }
    }

    fn block(&mut self, node: Node, ctx: &Ctx) {
        for child in named(node) {
            self.visit(child, ctx);
        }
    }

    fn visit(&mut self, node: Node, ctx: &Ctx) {
        match node.kind() {
            "comment" | "global_statement" | "nonlocal_statement" | "future_import_statement" => {}
            "function_definition" => self.function(node, node, ctx, Vec::new()),
            "class_definition" => self.class(node, node, ctx, Vec::new()),
            "decorated_definition" => self.decorated(node, ctx),
            "import_statement" => self.import(node),
            "import_from_statement" => self.import_from(node),
            "type_alias_statement" => self.type_alias(node, ctx),
            "call" => self.call(node, ctx),
            "attribute" => self.attribute(node, ctx),
            "identifier" => {
                let name = self.sink.text(node);
                if !ctx.locals.contains(name) && ctx.self_name.as_deref() != Some(name) {
                    self.sink.value(&ctx.owner, vec![name.to_string()]);
                }
            }
            // The keyword is a parameter name of the callee, not a value.
            "keyword_argument" => {
                if let Some(value) = node.child_by_field_name("value") {
                    self.visit(value, ctx);
                }
            }
            "lambda" => {
                let mut inner = ctx.clone();
                if let Some(params) = node.child_by_field_name("parameters") {
                    param_names(params, self.sink.src, &mut inner.locals);
                }
                if let Some(body) = node.child_by_field_name("body") {
                    self.visit(body, &inner);
                }
            }
            _ => self.block(node, ctx),
        }
    }

    fn decorated(&mut self, node: Node, ctx: &Ctx) {
        let mut attrs = Vec::new();
        for child in named(node) {
            if child.kind() == "decorator" {
                let name = marker_name(self.sink.text(child));
                if !name.is_empty() {
                    attrs.push(name);
                }
                self.block(child, ctx);
            }
        }
        let Some(def) = node.child_by_field_name("definition") else {
            return;
        };
        match def.kind() {
            "function_definition" => self.function(def, node, ctx, attrs),
            "class_definition" => self.class(def, node, ctx, attrs),
            _ => {}
        }
    }

    /// `span` is the definition together with its decorators.
    fn function(&mut self, def: Node, span: Node, ctx: &Ctx, attrs: Vec<String>) {
        let Some(name) = def.child_by_field_name("name").map(|n| self.sink.text(n)) else {
            return;
        };
        let method = ctx.class.is_some();
        let body = def.child_by_field_name("body");
        let params = def.child_by_field_name("parameters");
        let signature = self.signature(def);
        let doc = body.map(|b| self.docstring(b)).unwrap_or_default();
        let is_static = attrs.iter().any(|a| a == "staticmethod");
        let kind = if method {
            SymbolKind::Method
        } else {
            SymbolKind::Function
        };
        let symbol = self.sink.symbol(&ctx.owner, name, kind, span);
        symbol.visibility = visibility(name, ctx.nested);
        symbol.signature = signature;
        symbol.doc = doc;
        symbol.attrs = attrs;
        let qualified = symbol.qualified.clone();

        let mut inner = Ctx {
            owner: qualified,
            class: None,
            self_type: ctx.self_type.clone(),
            self_name: ctx.self_name.clone(),
            locals: ctx.locals.clone(),
            nested: true,
        };
        if method {
            // The first parameter of a method is the instance (or the class,
            // for a classmethod); a staticmethod has neither.
            let first = params
                .and_then(|p| p.named_child(0))
                .filter(|p| p.kind() == "identifier")
                .map(|p| self.sink.text(p).to_string());
            inner.self_name = if is_static { None } else { first };
            inner.self_type = if is_static { None } else { ctx.class.clone() };
        }
        if let Some(params) = params {
            param_names(params, self.sink.src, &mut inner.locals);
        }
        if let Some(body) = body {
            bindings(body, self.sink.src, &mut inner.locals);
        }
        if let Some(params) = params {
            for param in named(params) {
                for field in ["type", "value"] {
                    if let Some(part) = param.child_by_field_name(field) {
                        self.visit(part, &inner);
                    }
                }
            }
        }
        if let Some(ret) = def.child_by_field_name("return_type") {
            self.visit(ret, &inner);
        }
        if let Some(body) = body {
            self.block(body, &inner);
        }
    }

    fn class(&mut self, def: Node, span: Node, ctx: &Ctx, attrs: Vec<String>) {
        let Some(name) = def.child_by_field_name("name").map(|n| self.sink.text(n)) else {
            return;
        };
        let body = def.child_by_field_name("body");
        let signature = self.signature(def);
        let doc = body.map(|b| self.docstring(b)).unwrap_or_default();
        let symbol = self.sink.symbol(&ctx.owner, name, SymbolKind::Class, span);
        symbol.visibility = visibility(name, ctx.nested);
        symbol.signature = signature;
        symbol.doc = doc;
        symbol.attrs = attrs;
        let qualified = symbol.qualified.clone();

        let mut inner = Ctx {
            owner: qualified.clone(),
            class: Some(qualified.clone()),
            self_type: None,
            self_name: None,
            locals: ctx.locals.clone(),
            nested: ctx.nested,
        };
        if let Some(supers) = def.child_by_field_name("superclasses") {
            for base in named(supers) {
                // `metaclass=Meta` and `Generic[T]` are not plain base names.
                if let Some(path) = self.chain(base) {
                    self.sink.inherits(&qualified, path);
                }
            }
            self.block(supers, &inner);
        }
        if let Some(body) = body {
            bindings(body, self.sink.src, &mut inner.locals);
            self.block(body, &inner);
        }
    }

    fn type_alias(&mut self, node: Node, ctx: &Ctx) {
        let (Some(left), Some(right)) = (
            node.child_by_field_name("left"),
            node.child_by_field_name("right"),
        ) else {
            return;
        };
        // `type Pair[T] = ...`: the name is the first identifier on the left.
        let mut name_node = left;
        while name_node.kind() != "identifier" {
            let Some(first) = name_node.named_child(0) else {
                return;
            };
            name_node = first;
        }
        let name = self.sink.text(name_node);
        let signature = collapse(self.sink.between(node.start_byte(), right.start_byte()));
        let symbol = self
            .sink
            .symbol(&ctx.owner, name, SymbolKind::TypeAlias, node);
        symbol.visibility = visibility(name, ctx.nested);
        symbol.signature = signature;
        let inner = Ctx {
            owner: symbol.qualified.clone(),
            ..ctx.clone()
        };
        self.visit(right, &inner);
    }

    fn call(&mut self, node: Node, ctx: &Ctx) {
        let Some(function) = node.child_by_field_name("function") else {
            return self.block(node, ctx);
        };
        match function.kind() {
            "identifier" => {
                let name = self.sink.text(function);
                if !ctx.locals.contains(name) {
                    self.sink
                        .call(&ctx.owner, vec![name.to_string()], false, None);
                }
            }
            "attribute" => {
                let name = function
                    .child_by_field_name("attribute")
                    .map(|a| self.sink.text(a).to_string())
                    .unwrap_or_default();
                match self.chain(function) {
                    // `self.name(...)`: a method of the enclosing class, or
                    // one it inherits.
                    Some(chain) if ctx.self_name.as_deref() == Some(chain[0].as_str()) => {
                        let on_self = (chain.len() == 2)
                            .then_some(ctx.self_type.as_deref())
                            .flatten();
                        self.sink.call(&ctx.owner, vec![name], true, on_self);
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
        self.visit(function, ctx);
        if let Some(arguments) = node.child_by_field_name("arguments") {
            self.visit(arguments, ctx);
        }
    }

    fn attribute(&mut self, node: Node, ctx: &Ctx) {
        match self.chain(node) {
            Some(chain) => {
                let root = &chain[0];
                if ctx.self_name.as_ref() == Some(root) || ctx.locals.contains(root) {
                    return;
                }
                self.sink.value(&ctx.owner, vec![root.clone()]);
                self.sink.value(&ctx.owner, chain);
            }
            None => {
                if let Some(object) = node.child_by_field_name("object") {
                    self.visit(object, ctx);
                }
            }
        }
    }

    fn import(&mut self, node: Node) {
        for name in field_all(node, "name") {
            match name.kind() {
                "aliased_import" => {
                    let path = name
                        .child_by_field_name("name")
                        .map(|n| dotted(self.sink.text(n)))
                        .unwrap_or_default();
                    let alias = name
                        .child_by_field_name("alias")
                        .map(|a| self.sink.text(a))
                        .unwrap_or("");
                    self.sink.import(path, alias, false, false);
                }
                _ => {
                    // `import a.b.c` binds `a` and loads `a.b.c`.
                    let path = dotted(self.sink.text(name));
                    if let Some(first) = path.first().cloned() {
                        self.sink.import(vec![first.clone()], &first, false, false);
                        if path.len() > 1 {
                            self.sink.import(path, "", false, false);
                        }
                    }
                }
            }
        }
    }

    fn import_from(&mut self, node: Node) {
        let Some(source) = node.child_by_field_name("module_name") else {
            return;
        };
        let base = if source.kind() == "relative_import" {
            let mut dots = 0;
            let mut rest = Vec::new();
            for part in named(source) {
                match part.kind() {
                    "import_prefix" => dots = self.sink.text(part).matches('.').count(),
                    "dotted_name" => rest = dotted(self.sink.text(part)),
                    _ => {}
                }
            }
            // One dot is the containing package: the module itself for
            // `__init__.py`, its parent otherwise.
            let mut base = split_qualified(&self.sink.module);
            let up = dots.saturating_sub(usize::from(self.is_package));
            if up > base.len() {
                return;
            }
            base.truncate(base.len() - up);
            base.extend(rest);
            base
        } else {
            dotted(self.sink.text(source))
        };
        if base.is_empty() {
            return;
        }
        self.sink.import(base.clone(), "", false, false);
        if named(node).iter().any(|n| n.kind() == "wildcard_import") {
            self.sink.import(base, "", true, false);
            return;
        }
        for name in field_all(node, "name") {
            let (item, alias) = match name.kind() {
                "aliased_import" => (
                    name.child_by_field_name("name")
                        .map(|n| dotted(self.sink.text(n)))
                        .unwrap_or_default(),
                    name.child_by_field_name("alias")
                        .map(|a| self.sink.text(a).to_string()),
                ),
                _ => (dotted(self.sink.text(name)), None),
            };
            let Some(last) = item.last().cloned() else {
                continue;
            };
            let mut path = base.clone();
            path.extend(item);
            self.sink.import(path, &alias.unwrap_or(last), false, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = r#""""Text helpers.

Second line.
"""
import os.path
import pkg.shapes as shapes
from pkg.base import Base, helper as h
from . import sibling
from .sibling import *
from ..top import thing

LIMIT = 3


def shout(text: str, times: int = LIMIT) -> str:
    """Upper-case the text."""
    def inner(x):
        return x.upper()
    return inner(text) * times


def _hidden():
    pass


class Label(Base, shapes.Shape, metaclass=Meta):
    """A label.

        Indented detail.
    """

    @staticmethod
    def make(text):
        return Label(shout(text))

    @property
    def name(self) -> str:
        return self._render(h(self.text))

    def _render(self, value):
        shout = value.strip()
        return shapes.draw(shout, sibling.pad(value))

    def __repr__(self):
        return self.name.title()


type Pair = tuple[Label, Label]

if __name__ == "__main__":
    print(shout("x"))
"#;

    fn parsed() -> ParsedFile {
        parse_python("pkg/sub/text.py", "pkg::sub::text", false, SOURCE).unwrap()
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

        let module = symbol(&p, "pkg::sub::text");
        assert_eq!(module.kind, SymbolKind::Module);
        assert_eq!(module.doc, "Text helpers.\n\nSecond line.");
        assert_eq!(module.signature, "module pkg.sub.text");
        assert_eq!(module.parent.as_deref(), Some("pkg::sub"));

        let shout = symbol(&p, "pkg::sub::text::shout");
        assert_eq!(shout.kind, SymbolKind::Function);
        assert_eq!(shout.visibility, Visibility::Public);
        assert_eq!(
            shout.signature,
            "def shout(text: str, times: int = LIMIT) -> str"
        );
        assert_eq!(shout.doc, "Upper-case the text.");
        assert_eq!((shout.start_line, shout.end_line), (15, 19));

        let inner = symbol(&p, "pkg::sub::text::shout::inner");
        assert_eq!(inner.visibility, Visibility::Private);
        assert_eq!(
            symbol(&p, "pkg::sub::text::_hidden").visibility,
            Visibility::Private
        );

        let label = symbol(&p, "pkg::sub::text::Label");
        assert_eq!(label.kind, SymbolKind::Class);
        assert_eq!(
            label.signature,
            "class Label(Base, shapes.Shape, metaclass=Meta)"
        );
        assert_eq!(label.doc, "A label.\n\nIndented detail.");

        let make = symbol(&p, "pkg::sub::text::Label::make");
        assert_eq!(make.kind, SymbolKind::Method);
        assert_eq!(make.attrs, ["staticmethod"]);
        assert_eq!(make.parent.as_deref(), Some("pkg::sub::text::Label"));
        // The range covers the decorator.
        assert_eq!(make.start_line + 1, make.end_line - 1);
        assert_eq!(make.signature, "def make(text)");

        assert_eq!(
            symbol(&p, "pkg::sub::text::Label::_render").visibility,
            Visibility::Private
        );
        assert_eq!(
            symbol(&p, "pkg::sub::text::Label::__repr__").visibility,
            Visibility::Public
        );
        let pair = symbol(&p, "pkg::sub::text::Pair");
        assert_eq!(pair.kind, SymbolKind::TypeAlias);
        assert_eq!(pair.signature, "type Pair");
        // The file ends with a main guard, so its module is an entry point.
        assert_eq!(symbol(&p, "pkg::sub::text").attrs, [MAIN_GUARD_ATTR]);
        let plain = parse_python(
            "a.py",
            "a",
            false,
            "if x == 1:
    pass
",
        )
        .unwrap();
        assert!(symbol(&plain, "a").attrs.is_empty());
    }

    #[test]
    fn extracts_imports() {
        let p = parsed();
        let has = |path: &[&str], alias: &str, glob: bool| {
            p.uses
                .iter()
                .any(|u| u.path == path && u.alias == alias && u.glob == glob)
        };
        assert!(has(&["os"], "os", false));
        assert!(has(&["os", "path"], "", false));
        assert!(has(&["pkg", "shapes"], "shapes", false));
        assert!(has(&["pkg", "base", "Base"], "Base", false));
        assert!(has(&["pkg", "base", "helper"], "h", false));
        assert!(has(&["pkg", "base"], "", false));
        // Relative to the containing package `pkg.sub`.
        assert!(has(&["pkg", "sub", "sibling"], "sibling", false));
        assert!(has(&["pkg", "sub", "sibling"], "", true));
        assert!(has(&["pkg", "top", "thing"], "thing", false));
        assert!(p.uses.iter().all(|u| u.scope == "pkg::sub::text"));
    }

    #[test]
    fn relative_imports_in_a_package_start_at_the_package() {
        let p = parse_python("pkg/__init__.py", "pkg", true, "from .text import shout\n").unwrap();
        assert!(
            p.uses
                .iter()
                .any(|u| u.path == ["pkg", "text", "shout"] && u.alias == "shout")
        );
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
        let m = "pkg::sub::text";
        assert!(!call(&format!("{m}::shout"), &["inner"]).method);
        assert!(!call(&format!("{m}::Label::make"), &["shout"]).method);
        assert!(!call(&format!("{m}::Label::make"), &["Label"]).method);
        assert!(!call(&format!("{m}::Label::name"), &["h"]).method);

        let on_self = call(&format!("{m}::Label::name"), &["_render"]);
        assert!(on_self.method);
        assert_eq!(on_self.self_type.as_deref(), Some("pkg::sub::text::Label"));

        // An imported module is a path; a local value is a receiver.
        assert!(!call(&format!("{m}::Label::_render"), &["shapes", "draw"]).method);
        assert!(!call(&format!("{m}::Label::_render"), &["sibling", "pad"]).method);
        let on_local = call(&format!("{m}::Label::_render"), &["strip"]);
        assert!(on_local.method && on_local.self_type.is_none());
        let chained = call(&format!("{m}::Label::__repr__"), &["title"]);
        assert!(chained.method && chained.self_type.is_none());

        // Module-level code is charged to the module.
        assert!(!call(m, &["shout"]).method);

        // `shout` is rebound locally in `_render`, so it is not a reference.
        assert!(
            !p.value_refs
                .iter()
                .any(|v| v.from.ends_with("_render") && v.path == ["shout"])
        );
        assert!(
            p.value_refs
                .iter()
                .any(|v| v.from.ends_with("Label::make") && v.path == ["Label"])
        );

        let bases: Vec<Vec<String>> = p
            .impls
            .iter()
            .filter(|i| i.type_path == ["Label"])
            .filter_map(|i| i.trait_path.clone())
            .collect();
        assert_eq!(bases, [vec!["Base"], vec!["shapes", "Shape"]]);
    }

    #[test]
    fn reports_syntax_errors() {
        let p = parse_python("bad.py", "bad", false, "def broken(:\n    pass\n").unwrap();
        assert!(p.has_errors);
    }
}
