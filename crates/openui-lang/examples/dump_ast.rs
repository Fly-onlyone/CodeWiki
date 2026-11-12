//! Print the AST of every program in a corpus file as JSON, in the node shape
//! of the reference parser, so the two implementations can be compared.
//!
//! Usage: `cargo run -p openui-lang --example dump_ast -- corpus.txt [out.json]`
//! Programs in the corpus are separated by a line holding `-----`.

use openui_lang::{Expr, parse};

fn string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json(expr: &Expr) -> String {
    let list = |items: &[Expr]| items.iter().map(json).collect::<Vec<_>>().join(",");
    match expr {
        Expr::Str(s) => format!("{{\"k\":\"Str\",\"v\":{}}}", string(s)),
        Expr::Num(n) if n.is_finite() => format!("{{\"k\":\"Num\",\"v\":{n}}}"),
        Expr::Num(_) => "{\"k\":\"Num\",\"v\":null}".to_string(),
        Expr::Bool(b) => format!("{{\"k\":\"Bool\",\"v\":{b}}}"),
        Expr::Null => "{\"k\":\"Null\"}".to_string(),
        Expr::Array(items) => format!("{{\"k\":\"Arr\",\"els\":[{}]}}", list(items)),
        Expr::Object(entries) => {
            let body: Vec<String> = entries
                .iter()
                .map(|(key, value)| format!("[{},{}]", string(key), json(value)))
                .collect();
            format!("{{\"k\":\"Obj\",\"entries\":[{}]}}", body.join(","))
        }
        Expr::Ref(name) => format!("{{\"k\":\"Ref\",\"n\":{}}}", string(name)),
        Expr::StateRef(name) => format!("{{\"k\":\"StateRef\",\"n\":{}}}", string(name)),
        Expr::Call { name, args, .. } => {
            format!(
                "{{\"k\":\"Comp\",\"name\":{},\"args\":[{}]}}",
                string(name),
                list(args)
            )
        }
        Expr::Unary { op, operand } => {
            format!(
                "{{\"k\":\"UnaryOp\",\"op\":{},\"operand\":{}}}",
                string(op),
                json(operand)
            )
        }
        Expr::Binary { op, left, right } => format!(
            "{{\"k\":\"BinOp\",\"op\":{},\"left\":{},\"right\":{}}}",
            string(op),
            json(left),
            json(right)
        ),
        Expr::Ternary {
            cond,
            then,
            otherwise,
        } => format!(
            "{{\"k\":\"Ternary\",\"cond\":{},\"then\":{},\"else\":{}}}",
            json(cond),
            json(then),
            json(otherwise)
        ),
        Expr::Member { object, field } => {
            format!(
                "{{\"k\":\"Member\",\"obj\":{},\"field\":{}}}",
                json(object),
                string(field)
            )
        }
        Expr::Index { object, index } => {
            format!(
                "{{\"k\":\"Index\",\"obj\":{},\"index\":{}}}",
                json(object),
                json(index)
            )
        }
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("corpus file");
    let text = std::fs::read_to_string(path)
        .expect("readable corpus")
        .replace("\r\n", "\n");
    let programs: Vec<String> = text
        .split("\n-----\n")
        .map(|source| {
            let statements: Vec<String> = parse(source)
                .statements
                .iter()
                .map(|s| format!("{{\"id\":{},\"expr\":{}}}", string(&s.id), json(&s.expr)))
                .collect();
            format!("[{}]", statements.join(","))
        })
        .collect();
    let json = format!("[{}]", programs.join(","));
    // Writing the file directly keeps the bytes UTF-8 whatever the console uses.
    match std::env::args().nth(2) {
        Some(out) => std::fs::write(out, json).expect("writable output"),
        None => println!("{json}"),
    }
}
