//! Statement splitting and expression parsing.
//!
//! A program is a list of `identifier = expression` statements separated by
//! newlines. Newlines inside brackets, and before a ternary `?` or `:`, do not
//! end a statement. Expressions use a Pratt parser with the precedence of the
//! reference implementation.

use crate::lexer::{Token, tokenize};

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
    Array(Vec<Expr>),
    Object(Vec<(String, Expr)>),
    /// Reference to another statement by name.
    Ref(String),
    /// `$name`, stored with the `$`.
    StateRef(String),
    /// `Name(args...)`. `function` is true for `@Name(...)`.
    Call {
        name: String,
        args: Vec<Expr>,
        function: bool,
    },
    Unary {
        op: &'static str,
        operand: Box<Expr>,
    },
    Binary {
        op: &'static str,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Ternary {
        cond: Box<Expr>,
        then: Box<Expr>,
        otherwise: Box<Expr>,
    },
    Member {
        object: Box<Expr>,
        field: String,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Statement {
    /// Left-hand name; state variables keep their `$`.
    pub id: String,
    pub expr: Expr,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Program {
    /// Statements in source order. A later statement with the same id wins.
    pub statements: Vec<Statement>,
    /// True when brackets or a string had to be closed to parse the input.
    pub incomplete: bool,
}

impl Program {
    /// The statement that defines `id`: the last one with that name.
    pub fn get(&self, id: &str) -> Option<&Statement> {
        self.statements.iter().rev().find(|s| s.id == id)
    }
}

/// Close unterminated strings and brackets so partial input still parses.
pub fn auto_close(input: &str) -> (String, bool) {
    let mut stack: Vec<char> = Vec::new();
    let mut in_string: Option<char> = None;
    let mut escaped = false;
    for c in input.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        match in_string {
            Some(_) if c == '\\' => escaped = true,
            Some(quote) => {
                if c == quote {
                    in_string = None;
                }
            }
            None => match c {
                '"' | '\'' => in_string = Some(c),
                '(' | '[' | '{' => stack.push(c),
                ')' if stack.last() == Some(&'(') => {
                    stack.pop();
                }
                ']' if stack.last() == Some(&'[') => {
                    stack.pop();
                }
                '}' if stack.last() == Some(&'{') => {
                    stack.pop();
                }
                _ => {}
            },
        }
    }
    if in_string.is_none() && stack.is_empty() {
        return (input.to_string(), false);
    }
    let mut out = input.to_string();
    if let Some(quote) = in_string {
        if escaped {
            out.push('\\');
        }
        out.push(quote);
    }
    for open in stack.iter().rev() {
        out.push(match open {
            '(' => ')',
            '[' => ']',
            _ => '}',
        });
    }
    (out, true)
}

/// Split tokens into `(id, expression tokens)` pairs.
fn split(tokens: &[Token]) -> Vec<(String, Vec<Token>)> {
    let mut out = Vec::new();
    let mut pos = 0;
    let skip_line = |pos: &mut usize| {
        while *pos < tokens.len() && !matches!(tokens[*pos], Token::Newline | Token::Eof) {
            *pos += 1;
        }
    };
    while pos < tokens.len() {
        while pos < tokens.len() && tokens[pos] == Token::Newline {
            pos += 1;
        }
        if pos >= tokens.len() || tokens[pos] == Token::Eof {
            break;
        }
        let id = match &tokens[pos] {
            Token::Ident(name) | Token::Type(name) | Token::StateVar(name) => name.clone(),
            _ => {
                skip_line(&mut pos);
                continue;
            }
        };
        pos += 1;
        if tokens.get(pos) != Some(&Token::Equals) {
            skip_line(&mut pos);
            continue;
        }
        pos += 1;

        let mut expr = Vec::new();
        let (mut depth, mut ternary) = (0i32, 0i32);
        while pos < tokens.len() && tokens[pos] != Token::Eof {
            let token = &tokens[pos];
            if *token == Token::Newline && depth <= 0 && ternary <= 0 {
                // A ternary may continue on the next line with `?` or `:`.
                let mut peek = pos + 1;
                while peek < tokens.len() && tokens[peek] == Token::Newline {
                    peek += 1;
                }
                match tokens.get(peek) {
                    Some(Token::Question) => {
                        pos += 1;
                        continue;
                    }
                    Some(Token::Colon) if ternary > 0 => {
                        pos += 1;
                        continue;
                    }
                    _ => break,
                }
            }
            match token {
                Token::Newline => {
                    pos += 1;
                    continue;
                }
                Token::LParen | Token::LBrack | Token::LBrace => depth += 1,
                Token::RParen | Token::RBrack | Token::RBrace if depth > 0 => depth -= 1,
                Token::Question if depth == 0 => ternary += 1,
                Token::Colon if depth == 0 && ternary > 0 => ternary -= 1,
                _ => {}
            }
            expr.push(token.clone());
            pos += 1;
        }
        if !expr.is_empty() {
            out.push((id, expr));
        }
    }
    out
}

const PREC_TERNARY: u8 = 1;
const PREC_OR: u8 = 2;
const PREC_AND: u8 = 3;
const PREC_EQ: u8 = 4;
const PREC_CMP: u8 = 5;
const PREC_ADD: u8 = 6;
const PREC_MUL: u8 = 7;
const PREC_UNARY: u8 = 8;
const PREC_MEMBER: u8 = 9;

/// Nesting beyond this is treated as malformed input, not parsed deeper.
const MAX_DEPTH: usize = 200;

struct ExprParser<'a> {
    tokens: &'a [Token],
    pos: usize,
    depth: usize,
}

impl ExprParser<'_> {
    fn cur(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn eat(&mut self, token: &Token) {
        if self.cur() == token {
            self.pos += 1;
        }
    }

    fn infix_prec(token: &Token) -> u8 {
        match token {
            Token::Question => PREC_TERNARY,
            Token::Or => PREC_OR,
            Token::And => PREC_AND,
            Token::EqEq | Token::NotEq => PREC_EQ,
            Token::Greater | Token::Less | Token::GreaterEq | Token::LessEq => PREC_CMP,
            Token::Plus | Token::Minus => PREC_ADD,
            Token::Star | Token::Slash | Token::Percent => PREC_MUL,
            Token::Dot | Token::LBrack => PREC_MEMBER,
            _ => 0,
        }
    }

    fn expr(&mut self, min_prec: u8) -> Expr {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            self.pos = self.tokens.len();
            self.depth -= 1;
            return Expr::Null;
        }
        let mut left = self.prefix();
        while Self::infix_prec(self.cur()) > min_prec {
            left = self.infix(left);
        }
        self.depth -= 1;
        left
    }

    fn list(&mut self, close: &Token) -> Vec<Expr> {
        let mut items = Vec::new();
        while self.cur() != close && *self.cur() != Token::Eof {
            let before = self.pos;
            items.push(self.expr(0));
            if *self.cur() == Token::Comma {
                self.pos += 1;
            }
            // A token that cannot start an expression must not stall the loop.
            if self.pos == before {
                self.pos += 1;
            }
        }
        self.eat(close);
        items
    }

    fn call(&mut self, name: String, function: bool) -> Expr {
        self.pos += 1;
        self.eat(&Token::LParen);
        let args = self.list(&Token::RParen);
        Expr::Call {
            name,
            args,
            function,
        }
    }

    fn key(&mut self) -> String {
        let key = match self.cur() {
            Token::Ident(s) | Token::Type(s) | Token::Str(s) => s.clone(),
            Token::Num(n) => format_number(*n),
            Token::StateVar(s) => s.trim_start_matches('$').to_string(),
            _ => "?".to_string(),
        };
        self.pos += 1;
        key
    }

    fn prefix(&mut self) -> Expr {
        let token = self.cur().clone();
        let next_is_paren = self.tokens.get(self.pos + 1) == Some(&Token::LParen);
        match token {
            Token::Str(s) => {
                self.pos += 1;
                Expr::Str(s)
            }
            Token::Num(n) => {
                self.pos += 1;
                Expr::Num(n)
            }
            Token::True => {
                self.pos += 1;
                Expr::Bool(true)
            }
            Token::False => {
                self.pos += 1;
                Expr::Bool(false)
            }
            Token::Null => {
                self.pos += 1;
                Expr::Null
            }
            Token::LBrack => {
                self.pos += 1;
                Expr::Array(self.list(&Token::RBrack))
            }
            Token::LBrace => {
                self.pos += 1;
                let mut entries = Vec::new();
                while *self.cur() != Token::RBrace && *self.cur() != Token::Eof {
                    let key = self.key();
                    self.eat(&Token::Colon);
                    entries.push((key, self.expr(0)));
                    if *self.cur() == Token::Comma {
                        self.pos += 1;
                    }
                }
                self.eat(&Token::RBrace);
                Expr::Object(entries)
            }
            Token::StateVar(name) => {
                self.pos += 1;
                Expr::StateRef(name)
            }
            Token::Type(name) if next_is_paren => self.call(name, false),
            Token::Builtin(name) if next_is_paren => self.call(name, true),
            Token::Type(name) | Token::Builtin(name) | Token::Ident(name) => {
                self.pos += 1;
                Expr::Ref(name)
            }
            Token::Not => {
                self.pos += 1;
                Expr::Unary {
                    op: "!",
                    operand: Box::new(self.expr(PREC_UNARY)),
                }
            }
            Token::Minus => {
                self.pos += 1;
                Expr::Unary {
                    op: "-",
                    operand: Box::new(self.expr(PREC_UNARY)),
                }
            }
            Token::LParen => {
                self.pos += 1;
                let inner = self.expr(0);
                self.eat(&Token::RParen);
                inner
            }
            _ => {
                self.pos += 1;
                Expr::Null
            }
        }
    }

    fn infix(&mut self, left: Expr) -> Expr {
        let token = self.cur().clone();
        self.pos += 1;
        let binary = |this: &mut Self, op: &'static str, prec: u8| Expr::Binary {
            op,
            left: Box::new(left.clone()),
            right: Box::new(this.expr(prec)),
        };
        match token {
            Token::Plus => binary(self, "+", PREC_ADD),
            Token::Minus => binary(self, "-", PREC_ADD),
            Token::Star => binary(self, "*", PREC_MUL),
            Token::Slash => binary(self, "/", PREC_MUL),
            Token::Percent => binary(self, "%", PREC_MUL),
            Token::EqEq => binary(self, "==", PREC_EQ),
            Token::NotEq => binary(self, "!=", PREC_EQ),
            Token::Greater => binary(self, ">", PREC_CMP),
            Token::Less => binary(self, "<", PREC_CMP),
            Token::GreaterEq => binary(self, ">=", PREC_CMP),
            Token::LessEq => binary(self, "<=", PREC_CMP),
            Token::And => binary(self, "&&", PREC_AND),
            Token::Or => binary(self, "||", PREC_OR),
            Token::Question => {
                let then = self.expr(0);
                self.eat(&Token::Colon);
                // Right-associative: the else branch parses at the lowest level.
                let otherwise = self.expr(0);
                Expr::Ternary {
                    cond: Box::new(left),
                    then: Box::new(then),
                    otherwise: Box::new(otherwise),
                }
            }
            Token::Dot => Expr::Member {
                object: Box::new(left),
                field: self.key(),
            },
            Token::LBrack => {
                let index = self.expr(0);
                self.eat(&Token::RBrack);
                Expr::Index {
                    object: Box::new(left),
                    index: Box::new(index),
                }
            }
            _ => left,
        }
    }
}

/// Render a number the way the language prints it: integers without `.0`.
pub fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// Parse a program. Never fails: malformed lines are skipped, as in the
/// reference parser, so streamed or partly wrong input still yields a tree.
pub fn parse(source: &str) -> Program {
    let (text, incomplete) = auto_close(source);
    let tokens = tokenize(&text);
    let statements = split(&tokens)
        .into_iter()
        .map(|(id, tokens)| {
            let mut parser = ExprParser {
                tokens: &tokens,
                pos: 0,
                depth: 0,
            };
            Statement {
                id,
                expr: parser.expr(0),
            }
        })
        .collect();
    Program {
        statements,
        incomplete,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expr(source: &str) -> Expr {
        parse(&format!("x = {source}")).statements.remove(0).expr
    }

    fn num(n: f64) -> Box<Expr> {
        Box::new(Expr::Num(n))
    }

    #[test]
    fn parses_the_specification_example() {
        let program = parse(
            "root = Root([header, chart])           // 1. Entry Point\nheader = Header(\"Q4 Revenue\", \"YTD\")   // 2. Component Call\nchart = BarChart(labels, [s1, s2])     // 3. Forward Reference\nlabels = [\"Jan\", \"Feb\", \"Mar\"]         // 4. Data Definition\ns1 = Series(\"Product A\", [10, 20, 30])\n",
        );
        let ids: Vec<&str> = program.statements.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["root", "header", "chart", "labels", "s1"]);
        assert!(!program.incomplete);
        assert_eq!(
            program.get("header").unwrap().expr,
            Expr::Call {
                name: "Header".into(),
                args: vec![Expr::Str("Q4 Revenue".into()), Expr::Str("YTD".into())],
                function: false,
            }
        );
    }

    #[test]
    fn respects_operator_precedence() {
        assert_eq!(
            expr("1 + 2 * 3"),
            Expr::Binary {
                op: "+",
                left: num(1.0),
                right: Box::new(Expr::Binary {
                    op: "*",
                    left: num(2.0),
                    right: num(3.0)
                }),
            }
        );
        // Left-associative subtraction: (5 - 2) - 1.
        assert_eq!(
            expr("5 - 2 - 1"),
            Expr::Binary {
                op: "-",
                left: Box::new(Expr::Binary {
                    op: "-",
                    left: num(5.0),
                    right: num(2.0)
                }),
                right: num(1.0),
            }
        );
        let Expr::Ternary { cond, .. } = expr("a > 1 && !b ? \"y\" : \"n\"") else {
            panic!("expected a ternary");
        };
        assert!(matches!(*cond, Expr::Binary { op: "&&", .. }));
    }

    #[test]
    fn parses_members_indexes_objects_and_functions() {
        assert_eq!(
            expr("data.rows[0].title"),
            Expr::Member {
                object: Box::new(Expr::Index {
                    object: Box::new(Expr::Member {
                        object: Box::new(Expr::Ref("data".into())),
                        field: "rows".into(),
                    }),
                    index: num(0.0),
                }),
                field: "title".into(),
            }
        );
        assert_eq!(
            expr("{name: \"a\", \"k 2\": [1], 3: null}"),
            Expr::Object(vec![
                ("name".into(), Expr::Str("a".into())),
                ("k 2".into(), Expr::Array(vec![Expr::Num(1.0)])),
                ("3".into(), Expr::Null),
            ])
        );
        assert_eq!(
            expr("@Count(items)"),
            Expr::Call {
                name: "Count".into(),
                args: vec![Expr::Ref("items".into())],
                function: true,
            }
        );
        // A PascalCase word without parentheses is a reference.
        assert_eq!(expr("Sidebar"), Expr::Ref("Sidebar".into()));
    }

    #[test]
    fn statements_span_lines_inside_brackets_and_ternaries() {
        let program = parse(
            "main_chart = LineChart(\n  [\"Mon\", \"Tue\"],\n  [Series(\"Visits\", [100, 450])]\n)\nroot = $compact\n  ? small\n  : large\nlast = 1\n",
        );
        let ids: Vec<&str> = program.statements.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["main_chart", "root", "last"]);
        assert!(matches!(
            program.get("root").unwrap().expr,
            Expr::Ternary { .. }
        ));
    }

    #[test]
    fn skips_invalid_lines_and_keeps_the_last_definition() {
        let program = parse("this is prose\n= 3\ntitle = \"first\"\n42 = x\ntitle = \"second\"\n");
        assert_eq!(program.statements.len(), 2);
        assert_eq!(
            program.get("title").unwrap().expr,
            Expr::Str("second".into())
        );
        assert!(parse("").statements.is_empty());
    }

    #[test]
    fn closes_partial_input() {
        let program = parse("root = Card([TextContent(\"3 open");
        assert!(program.incomplete);
        assert_eq!(
            program.get("root").unwrap().expr,
            Expr::Call {
                name: "Card".into(),
                args: vec![Expr::Array(vec![Expr::Call {
                    name: "TextContent".into(),
                    args: vec![Expr::Str("3 open".into())],
                    function: false,
                }])],
                function: false,
            }
        );
        assert_eq!(
            auto_close("a = [1, {b: \"x\\"),
            ("a = [1, {b: \"x\\\\\"}]".to_string(), true)
        );
    }

    #[test]
    fn survives_hostile_nesting() {
        let deep = format!("x = {}1{}", "(".repeat(5000), ")".repeat(5000));
        let _ = parse(&deep);
        let _ = parse(&format!("x = {}", "[".repeat(5000)));
        let _ = parse("x = ) ] } , : ? . + * / %");
    }
}
