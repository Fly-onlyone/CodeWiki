//! Static evaluation of a program into a tree of elements.
//!
//! Illustrations in CodeWiki are static documents: there are no queries,
//! mutations or user actions. Evaluation resolves references from the `root`
//! statement, computes expressions, and checks every component call against a
//! library. Problems are collected with the error codes of the OpenUI Lang
//! 1.0 specification and never abort rendering of the rest.

use std::collections::BTreeMap;

use crate::parser::{Expr, Program, format_number};

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Null,
    Bool(bool),
    Number(f64),
    Text(String),
    List(Vec<Node>),
    Map(BTreeMap<String, Node>),
    Element {
        name: String,
        props: BTreeMap<String, Node>,
    },
}

impl Node {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Node::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_list(&self) -> &[Node] {
        match self {
            Node::List(items) => items,
            _ => &[],
        }
    }

    fn truthy(&self) -> bool {
        match self {
            Node::Null => false,
            Node::Bool(b) => *b,
            Node::Number(n) => *n != 0.0 && !n.is_nan(),
            Node::Text(s) => !s.is_empty(),
            _ => true,
        }
    }

    /// Text form used by string concatenation.
    fn display(&self) -> String {
        match self {
            Node::Null => String::new(),
            Node::Bool(b) => b.to_string(),
            Node::Number(n) => format_number(*n),
            Node::Text(s) => s.clone(),
            _ => String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Text,
    Number,
    Bool,
    /// A list of anything.
    List,
    /// One component.
    Element,
    Any,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: &'static str,
    pub kind: Kind,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    pub name: &'static str,
    /// Positional parameters; required ones come first.
    pub params: Vec<Param>,
}

/// The set of components a program may use. Positional arguments map to
/// parameters by order.
#[derive(Debug, Clone, Default)]
pub struct Library {
    components: BTreeMap<&'static str, Component>,
    /// Component that may serve as the entry when no `root` statement exists.
    pub root: Option<&'static str>,
}

impl Library {
    pub fn new(root: Option<&'static str>) -> Self {
        Self {
            components: BTreeMap::new(),
            root,
        }
    }

    /// Add a component. `params` lists `(name, kind, required)`.
    pub fn component(mut self, name: &'static str, params: &[(&'static str, Kind, bool)]) -> Self {
        self.components.insert(
            name,
            Component {
                name,
                params: params
                    .iter()
                    .map(|(name, kind, required)| Param {
                        name,
                        kind: *kind,
                        required: *required,
                    })
                    .collect(),
            },
        );
        self
    }

    pub fn get(&self, name: &str) -> Option<&Component> {
        self.components.get(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.components.keys().copied()
    }

    /// One line per component, in the signature form the specification uses
    /// in prompts: `Stat(label: string, value: string, trend?: string)`.
    pub fn signatures(&self) -> String {
        let type_name = |kind: Kind| match kind {
            Kind::Text => "string",
            Kind::Number => "number",
            Kind::Bool => "boolean",
            Kind::List => "array",
            Kind::Element => "component",
            Kind::Any => "any",
        };
        self.components
            .values()
            .map(|c| {
                let params: Vec<String> = c
                    .params
                    .iter()
                    .map(|p| {
                        format!(
                            "{}{}: {}",
                            p.name,
                            if p.required { "" } else { "?" },
                            type_name(p.kind)
                        )
                    })
                    .collect();
                format!("{}({})", c.name, params.join(", "))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LangError {
    /// Specification error code, for example `unknown-component`.
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    /// Statement the problem was found in.
    pub statement: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// `None` when there is no usable entry.
    pub root: Option<Node>,
    pub errors: Vec<LangError>,
    /// True when the source had to be auto-closed (partial input).
    pub incomplete: bool,
}

impl Document {
    /// True when the document renders and no error-level problem was found.
    pub fn is_valid(&self) -> bool {
        self.root.is_some() && self.errors.iter().all(|e| e.severity != Severity::Error)
    }
}

/// References deeper than this are a cycle or a pathological program.
const MAX_REF_DEPTH: usize = 64;

struct Evaluator<'a> {
    program: &'a Program,
    library: &'a Library,
    errors: Vec<LangError>,
    /// Statements being resolved, innermost last; detects cycles.
    stack: Vec<String>,
}

impl Evaluator<'_> {
    fn error(&mut self, code: &'static str, severity: Severity, message: String) {
        let statement = self.stack.last().cloned().unwrap_or_default();
        // The same problem is reached once per reference; report it once.
        if !self
            .errors
            .iter()
            .any(|e| e.code == code && e.message == message && e.statement == statement)
        {
            self.errors.push(LangError {
                code,
                severity,
                message,
                statement,
            });
        }
    }

    fn resolve(&mut self, name: &str) -> Node {
        if self.stack.iter().any(|s| s == name) || self.stack.len() >= MAX_REF_DEPTH {
            self.error(
                "runtime-error",
                Severity::Error,
                format!("`{name}` refers to itself"),
            );
            return Node::Null;
        }
        let Some(statement) = self.program.get(name) else {
            // Forward references to statements that never arrive render as
            // nothing; a streamed program may simply not be finished.
            return Node::Null;
        };
        self.stack.push(name.to_string());
        let value = self.eval(&statement.expr);
        self.stack.pop();
        value
    }

    fn eval(&mut self, expr: &Expr) -> Node {
        match expr {
            Expr::Str(s) => Node::Text(s.clone()),
            Expr::Num(n) => Node::Number(*n),
            Expr::Bool(b) => Node::Bool(*b),
            Expr::Null => Node::Null,
            Expr::Array(items) => Node::List(items.iter().map(|e| self.eval(e)).collect()),
            Expr::Object(entries) => Node::Map(
                entries
                    .iter()
                    .map(|(k, v)| (k.clone(), self.eval(v)))
                    .collect(),
            ),
            Expr::Ref(name) | Expr::StateRef(name) => self.resolve(name),
            Expr::Call {
                name,
                args,
                function: true,
            } => self.function(name, args),
            Expr::Call { name, args, .. } => self.component(name, args),
            Expr::Unary { op, operand } => {
                let value = self.eval(operand);
                match (*op, &value) {
                    ("!", _) => Node::Bool(!value.truthy()),
                    ("-", Node::Number(n)) => Node::Number(-n),
                    _ => Node::Null,
                }
            }
            Expr::Binary { op, left, right } => {
                // Logical operators short-circuit and return an operand.
                if *op == "&&" || *op == "||" {
                    let l = self.eval(left);
                    return if l.truthy() == (*op == "||") {
                        l
                    } else {
                        self.eval(right)
                    };
                }
                let (l, r) = (self.eval(left), self.eval(right));
                binary(op, &l, &r)
            }
            Expr::Ternary {
                cond,
                then,
                otherwise,
            } => {
                if self.eval(cond).truthy() {
                    self.eval(then)
                } else {
                    self.eval(otherwise)
                }
            }
            Expr::Member { object, field } => member(&self.eval(object), field),
            Expr::Index { object, index } => {
                let (object, index) = (self.eval(object), self.eval(index));
                match (&object, &index) {
                    (Node::List(items), Node::Number(n)) if *n >= 0.0 => {
                        items.get(*n as usize).cloned().unwrap_or(Node::Null)
                    }
                    (_, Node::Text(key)) => member(&object, key),
                    _ => Node::Null,
                }
            }
        }
    }

    fn function(&mut self, name: &str, args: &[Expr]) -> Node {
        let values: Vec<Node> = args.iter().map(|a| self.eval(a)).collect();
        let numbers = |node: Option<&Node>| -> Vec<f64> {
            node.map(|n| n.as_list())
                .unwrap_or(&[])
                .iter()
                .filter_map(|n| match n {
                    Node::Number(x) => Some(*x),
                    _ => None,
                })
                .collect()
        };
        let first = values.first();
        match name {
            "Count" => Node::Number(first.map(|n| n.as_list().len()).unwrap_or(0) as f64),
            "Sum" => Node::Number(numbers(first).iter().sum()),
            "Avg" => {
                let n = numbers(first);
                if n.is_empty() {
                    Node::Null
                } else {
                    Node::Number(n.iter().sum::<f64>() / n.len() as f64)
                }
            }
            "Min" => numbers(first)
                .into_iter()
                .reduce(f64::min)
                .map_or(Node::Null, Node::Number),
            "Max" => numbers(first)
                .into_iter()
                .reduce(f64::max)
                .map_or(Node::Null, Node::Number),
            "First" => first
                .and_then(|n| n.as_list().first().cloned())
                .unwrap_or(Node::Null),
            "Last" => first
                .and_then(|n| n.as_list().last().cloned())
                .unwrap_or(Node::Null),
            "Round" => match (first, values.get(1)) {
                (Some(Node::Number(x)), places) => {
                    let places = match places {
                        Some(Node::Number(p)) => p.clamp(0.0, 12.0) as i32,
                        _ => 0,
                    };
                    let scale = 10f64.powi(places);
                    Node::Number((x * scale).round() / scale)
                }
                _ => Node::Null,
            },
            "Abs" | "Floor" | "Ceil" => match first {
                Some(Node::Number(x)) => Node::Number(match name {
                    "Abs" => x.abs(),
                    "Floor" => x.floor(),
                    _ => x.ceil(),
                }),
                _ => Node::Null,
            },
            _ => {
                // Actions, queries and custom functions have no meaning in a
                // static document.
                self.error(
                    "unknown-function",
                    Severity::Warning,
                    format!("`@{name}` is not available in a static illustration"),
                );
                Node::Null
            }
        }
    }

    fn component(&mut self, name: &str, args: &[Expr]) -> Node {
        if matches!(name, "Query" | "Mutation" | "Action") {
            self.error(
                "inline-reserved",
                Severity::Warning,
                format!("`{name}` is not available in a static illustration"),
            );
            return Node::Null;
        }
        let Some(component) = self.library.get(name).cloned() else {
            self.error(
                "unknown-component",
                Severity::Error,
                format!("`{name}` is not a component of this library"),
            );
            return Node::Null;
        };
        if args.len() > component.params.len() {
            self.error(
                "excess-args",
                Severity::Warning,
                format!(
                    "`{name}` takes {} arguments, {} were given; the extras are ignored",
                    component.params.len(),
                    args.len()
                ),
            );
        }
        let mut props = BTreeMap::new();
        let mut usable = true;
        for (i, param) in component.params.iter().enumerate() {
            let value = args.get(i).map(|a| self.eval(a));
            match value {
                None | Some(Node::Null) => {
                    if param.required {
                        let code = if value.is_none() {
                            "missing-required"
                        } else {
                            "null-required"
                        };
                        self.error(
                            code,
                            Severity::Error,
                            format!("`{name}` needs `{}`", param.name),
                        );
                        usable = false;
                    }
                }
                Some(value) => {
                    if matches_kind(&value, param.kind) {
                        props.insert(param.name.to_string(), value);
                    } else {
                        self.error(
                            "type-mismatch",
                            Severity::Error,
                            format!("`{name}.{}` has the wrong type", param.name),
                        );
                        if param.required {
                            usable = false;
                        }
                    }
                }
            }
        }
        // A component missing a required value is dropped, as the
        // specification prescribes; the rest of the document still renders.
        if usable {
            Node::Element {
                name: name.to_string(),
                props,
            }
        } else {
            Node::Null
        }
    }
}

fn matches_kind(value: &Node, kind: Kind) -> bool {
    match kind {
        Kind::Any => true,
        Kind::Text => matches!(value, Node::Text(_) | Node::Number(_)),
        Kind::Number => matches!(value, Node::Number(_)),
        Kind::Bool => matches!(value, Node::Bool(_)),
        Kind::List => matches!(value, Node::List(_)),
        Kind::Element => matches!(value, Node::Element { .. }),
    }
}

/// `object.field`; on a list it plucks the field from every element.
fn member(object: &Node, field: &str) -> Node {
    match object {
        Node::Map(map) => map.get(field).cloned().unwrap_or(Node::Null),
        Node::Element { props, .. } => props.get(field).cloned().unwrap_or(Node::Null),
        Node::List(items) => match field.parse::<usize>() {
            Ok(index) => items.get(index).cloned().unwrap_or(Node::Null),
            Err(_) => Node::List(items.iter().map(|item| member(item, field)).collect()),
        },
        _ => Node::Null,
    }
}

fn binary(op: &str, left: &Node, right: &Node) -> Node {
    match (op, left, right) {
        ("+", Node::Number(a), Node::Number(b)) => Node::Number(a + b),
        // `+` with a string on either side concatenates.
        ("+", Node::Text(_), _) | ("+", _, Node::Text(_)) => {
            Node::Text(left.display() + &right.display())
        }
        ("-", Node::Number(a), Node::Number(b)) => Node::Number(a - b),
        ("*", Node::Number(a), Node::Number(b)) => Node::Number(a * b),
        ("/", Node::Number(a), Node::Number(b)) if *b != 0.0 => Node::Number(a / b),
        ("%", Node::Number(a), Node::Number(b)) if *b != 0.0 => Node::Number(a % b),
        ("==", _, _) => Node::Bool(left == right),
        ("!=", _, _) => Node::Bool(left != right),
        (">", Node::Number(a), Node::Number(b)) => Node::Bool(a > b),
        ("<", Node::Number(a), Node::Number(b)) => Node::Bool(a < b),
        (">=", Node::Number(a), Node::Number(b)) => Node::Bool(a >= b),
        ("<=", Node::Number(a), Node::Number(b)) => Node::Bool(a <= b),
        _ => Node::Null,
    }
}

/// Evaluate a parsed program against a library.
pub fn evaluate(program: &Program, library: &Library) -> Document {
    let mut evaluator = Evaluator {
        program,
        library,
        errors: Vec::new(),
        stack: Vec::new(),
    };
    if program.statements.is_empty() {
        evaluator.error(
            "parse-failed",
            Severity::Error,
            "the text contains no statements".to_string(),
        );
        return Document {
            root: None,
            errors: evaluator.errors,
            incomplete: program.incomplete,
        };
    }
    let mut entry = program.get("root").map(|s| s.id.clone());
    if entry.is_none() {
        // Without `root`, the first statement that calls the library's root
        // component is the entry, and a warning records the fallback.
        entry = program
            .statements
            .iter()
            .find(|s| matches!(&s.expr, Expr::Call { name, function: false, .. } if Some(name.as_str()) == library.root))
            .map(|s| s.id.clone());
        let (severity, message) = match &entry {
            Some(id) => (
                Severity::Warning,
                format!("no `root` statement; using `{id}` as the entry"),
            ),
            None => (Severity::Error, "no `root` statement".to_string()),
        };
        evaluator.error("no-root", severity, message);
    }
    let mut root = entry.map(|id| evaluator.resolve(&id));
    if let Some(node) = &root
        && !matches!(node, Node::Element { .. })
    {
        evaluator.error(
            "no-root",
            Severity::Error,
            "the entry does not produce a component".to_string(),
        );
        root = None;
    }
    Document {
        root,
        errors: evaluator.errors,
        incomplete: program.incomplete,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn library() -> Library {
        Library::new(Some("Card"))
            .component(
                "Card",
                &[("children", Kind::List, true), ("title", Kind::Text, false)],
            )
            .component("TextContent", &[("text", Kind::Text, true)])
            .component(
                "Stat",
                &[("label", Kind::Text, true), ("value", Kind::Text, true)],
            )
    }

    fn run(source: &str) -> Document {
        evaluate(&parse(source), &library())
    }

    fn codes(doc: &Document) -> Vec<&'static str> {
        doc.errors.iter().map(|e| e.code).collect()
    }

    fn element<'a>(node: &'a Node, name: &str) -> &'a BTreeMap<String, Node> {
        match node {
            Node::Element { name: n, props } if n == name => props,
            other => panic!("expected {name}, got {other:?}"),
        }
    }

    #[test]
    fn resolves_forward_references_from_root() {
        let doc = run(
            "root = Card([title, total], \"Orders\")\ntitle = TextContent(\"Open\")\ntotal = TextContent(\"Total: \" + @Sum(items))\nitems = [1, 2, 3.5]\n",
        );
        assert!(doc.is_valid(), "{:?}", doc.errors);
        let card = element(doc.root.as_ref().unwrap(), "Card");
        assert_eq!(card["title"], Node::Text("Orders".into()));
        let children = card["children"].as_list();
        assert_eq!(
            element(&children[0], "TextContent")["text"],
            Node::Text("Open".into())
        );
        assert_eq!(
            element(&children[1], "TextContent")["text"],
            Node::Text("Total: 6.5".into())
        );
    }

    #[test]
    fn evaluates_the_specification_ternary_example() {
        let source = "$compact = false\nroot = $compact ? compactView : fullView\ncompactView = TextContent(\"3 open\")\nfullView = Card([TextContent(\"3 open tickets\")])\n";
        let doc = run(source);
        assert!(doc.is_valid());
        element(doc.root.as_ref().unwrap(), "Card");
        let compact = run(&source.replace("false", "true"));
        element(compact.root.as_ref().unwrap(), "TextContent");
    }

    #[test]
    fn reports_specification_error_codes() {
        let doc = run(
            "root = Card([a, b, c, d])\na = Nope(\"x\")\nb = Stat(\"only label\")\nc = TextContent(\"ok\", \"extra\")\nd = TextContent([1])\n",
        );
        let found = codes(&doc);
        for code in [
            "unknown-component",
            "missing-required",
            "excess-args",
            "type-mismatch",
        ] {
            assert!(found.contains(&code), "missing {code} in {found:?}");
        }
        // Bad components are dropped; the rest still renders.
        let children = element(doc.root.as_ref().unwrap(), "Card")["children"]
            .as_list()
            .to_vec();
        assert_eq!(children.len(), 4);
        assert_eq!(children[0], Node::Null);
        assert_eq!(children[1], Node::Null);
        assert_eq!(
            element(&children[2], "TextContent")["text"],
            Node::Text("ok".into())
        );
        assert!(!doc.is_valid());
        assert_eq!(
            doc.errors
                .iter()
                .find(|e| e.code == "missing-required")
                .unwrap()
                .statement,
            "b"
        );
    }

    #[test]
    fn handles_missing_root() {
        let fallback = run("title = TextContent(\"Orders\")\npage = Card([title])\n");
        assert_eq!(codes(&fallback), ["no-root"]);
        assert_eq!(fallback.errors[0].severity, Severity::Warning);
        element(fallback.root.as_ref().unwrap(), "Card");
        assert!(fallback.is_valid());

        let none = run("title = TextContent(\"Orders\")\n");
        assert!(none.root.is_none() && !none.is_valid());
        let data = run("root = [1, 2]\n");
        assert!(data.root.is_none());
        assert_eq!(codes(&run("just prose\n")), ["parse-failed"]);
    }

    #[test]
    fn computes_expressions_and_plucks_columns() {
        let doc = run(
            "rows = [{title: \"a\", n: 2}, {title: \"b\", n: 5}]\nroot = Card([TextContent(rows.title[1] + \":\" + (rows.n[0] * 3 > 5 ? \"big\" : \"small\") + @Count(rows))])\n",
        );
        let card = element(doc.root.as_ref().unwrap(), "Card");
        let text = &element(&card["children"].as_list()[0], "TextContent")["text"];
        assert_eq!(*text, Node::Text("b:big2".into()));
    }

    #[test]
    fn static_documents_reject_actions_and_cycles() {
        let doc = run(
            "root = Card([TextContent(\"x\" + @ToAssistant(\"hi\"))])\ndata = Query(\"t\", {})\n",
        );
        assert_eq!(codes(&doc), ["unknown-function"]);
        assert!(doc.is_valid(), "a warning does not stop rendering");

        let cycle = run("root = Card([a])\na = b\nb = a\n");
        assert!(codes(&cycle).contains(&"runtime-error"));
        assert!(cycle.root.is_some());
    }

    #[test]
    fn prints_signatures_for_prompts() {
        let text = library().signatures();
        assert!(text.contains("Card(children: array, title?: string)"));
        assert!(text.contains("Stat(label: string, value: string)"));
    }
}
