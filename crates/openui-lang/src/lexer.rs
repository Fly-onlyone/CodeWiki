//! Tokenizer for OpenUI Lang.
//!
//! Follows the token rules of the reference implementation
//! (`@openuidev/lang-core`, MIT): newlines are significant, `//` and `#` start
//! line comments, PascalCase words are component names, `$name` is a state
//! variable, `@Name` is a function call, and unknown characters are skipped.

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Newline,
    LParen,
    RParen,
    LBrack,
    RBrack,
    LBrace,
    RBrace,
    Comma,
    Colon,
    Equals,
    EqEq,
    Not,
    NotEq,
    Greater,
    GreaterEq,
    Less,
    LessEq,
    And,
    Or,
    Dot,
    Question,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Str(String),
    Num(f64),
    True,
    False,
    Null,
    /// Lowercase identifier: a reference to another statement.
    Ident(String),
    /// PascalCase identifier: a component name.
    Type(String),
    /// `$name`, stored with the `$`.
    StateVar(String),
    /// `@Name`, stored without the `@`.
    Builtin(String),
    Eof,
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn is_word_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

/// Decode a double-quoted string body with JSON escapes. Returns `None` when
/// an escape is malformed, in which case the caller keeps the raw text.
fn unescape_json(body: &str) -> Option<String> {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next()? {
            '"' => out.push('"'),
            '\\' => out.push('\\'),
            '/' => out.push('/'),
            'b' => out.push('\u{8}'),
            'f' => out.push('\u{c}'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'u' => {
                let unit = |chars: &mut std::str::Chars| -> Option<u32> {
                    let hex: String = chars.take(4).collect();
                    (hex.len() == 4)
                        .then(|| u32::from_str_radix(&hex, 16).ok())
                        .flatten()
                };
                let first = unit(&mut chars)?;
                let code = if (0xD800..0xDC00).contains(&first) {
                    // A surrogate pair encodes one character above U+FFFF.
                    if chars.next()? != '\\' || chars.next()? != 'u' {
                        return None;
                    }
                    let second = unit(&mut chars)?;
                    if !(0xDC00..0xE000).contains(&second) {
                        return None;
                    }
                    0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
                } else {
                    first
                };
                out.push(char::from_u32(code)?);
            }
            _ => return None,
        }
    }
    Some(out)
}

pub fn tokenize(src: &str) -> Vec<Token> {
    let chars: Vec<char> = src.chars().collect();
    let n = chars.len();
    let mut tokens: Vec<Token> = Vec::new();
    let mut i = 0;
    while i < n {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        let two = |first: Token, second: Token, tokens: &mut Vec<Token>, i: &mut usize| {
            if next == Some('=') {
                tokens.push(second);
                *i += 2;
            } else {
                tokens.push(first);
                *i += 1;
            }
        };
        match c {
            ' ' | '\t' | '\r' => i += 1,
            '\n' => {
                tokens.push(Token::Newline);
                i += 1;
            }
            '(' | ')' | '[' | ']' | '{' | '}' | ',' | ':' | '.' | '?' | '+' | '*' | '%' => {
                tokens.push(match c {
                    '(' => Token::LParen,
                    ')' => Token::RParen,
                    '[' => Token::LBrack,
                    ']' => Token::RBrack,
                    '{' => Token::LBrace,
                    '}' => Token::RBrace,
                    ',' => Token::Comma,
                    ':' => Token::Colon,
                    '.' => Token::Dot,
                    '?' => Token::Question,
                    '+' => Token::Plus,
                    '*' => Token::Star,
                    _ => Token::Percent,
                });
                i += 1;
            }
            '=' => two(Token::Equals, Token::EqEq, &mut tokens, &mut i),
            '!' => two(Token::Not, Token::NotEq, &mut tokens, &mut i),
            '>' => two(Token::Greater, Token::GreaterEq, &mut tokens, &mut i),
            '<' => two(Token::Less, Token::LessEq, &mut tokens, &mut i),
            // A single `&` or `|` is accepted as the logical operator.
            '&' | '|' => {
                tokens.push(if c == '&' { Token::And } else { Token::Or });
                i += if next == Some(c) { 2 } else { 1 };
            }
            '#' => {
                while i < n && chars[i] != '\n' {
                    i += 1;
                }
            }
            '/' if next == Some('/') => {
                while i < n && chars[i] != '\n' {
                    i += 1;
                }
            }
            '/' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            '"' => {
                let start = i + 1;
                i += 1;
                let mut end = None;
                while i < n {
                    match chars[i] {
                        '\\' => i += 2,
                        '"' => {
                            end = Some(i);
                            i += 1;
                            break;
                        }
                        _ => i += 1,
                    }
                }
                // An unterminated string (streamed input) runs to the end.
                let body: String = chars[start..end.unwrap_or(n).min(n)].iter().collect();
                tokens.push(Token::Str(unescape_json(&body).unwrap_or(body)));
            }
            '\'' => {
                i += 1;
                let mut out = String::new();
                while i < n {
                    match chars[i] {
                        '\\' => {
                            i += 1;
                            if i < n {
                                out.push(match chars[i] {
                                    'n' => '\n',
                                    't' => '\t',
                                    other => other,
                                });
                                i += 1;
                            }
                        }
                        '\'' => {
                            i += 1;
                            break;
                        }
                        other => {
                            out.push(other);
                            i += 1;
                        }
                    }
                }
                tokens.push(Token::Str(out));
            }
            '-' | '0'..='9' => {
                let digit_follows = next.is_some_and(|d| d.is_ascii_digit());
                let after_value = matches!(
                    tokens.last(),
                    Some(
                        Token::Num(_)
                            | Token::Str(_)
                            | Token::Ident(_)
                            | Token::Type(_)
                            | Token::RParen
                            | Token::RBrack
                            | Token::True
                            | Token::False
                            | Token::Null
                            | Token::StateVar(_)
                            | Token::Builtin(_)
                    )
                );
                // `-` is part of a number only when it cannot be subtraction.
                if c == '-' && (after_value || !digit_follows) {
                    tokens.push(Token::Minus);
                    i += 1;
                    continue;
                }
                let start = i;
                if chars[i] == '-' {
                    i += 1;
                }
                while i < n && chars[i].is_ascii_digit() {
                    i += 1;
                }
                if i + 1 < n && chars[i] == '.' && chars[i + 1].is_ascii_digit() {
                    i += 1;
                    while i < n && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                if i < n && matches!(chars[i], 'e' | 'E') {
                    i += 1;
                    if i < n && matches!(chars[i], '+' | '-') {
                        i += 1;
                    }
                    while i < n && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                let text: String = chars[start..i].iter().collect();
                tokens.push(Token::Num(text.parse().unwrap_or(f64::NAN)));
            }
            '$' if next.is_some_and(is_word_start) => {
                let start = i;
                i += 1;
                while i < n && is_word(chars[i]) {
                    i += 1;
                }
                tokens.push(Token::StateVar(chars[start..i].iter().collect()));
            }
            '@' if next.is_some_and(is_word_start) => {
                i += 1;
                let start = i;
                while i < n && is_word(chars[i]) {
                    i += 1;
                }
                tokens.push(Token::Builtin(chars[start..i].iter().collect()));
            }
            c if is_word_start(c) => {
                let start = i;
                while i < n && is_word(chars[i]) {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                tokens.push(match word.as_str() {
                    "true" => Token::True,
                    "false" => Token::False,
                    "null" => Token::Null,
                    _ if c.is_ascii_uppercase() => Token::Type(word),
                    _ => Token::Ident(word),
                });
            }
            _ => i += 1,
        }
    }
    tokens.push(Token::Eof);
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_a_statement() {
        let tokens = tokenize("root = Card([title], \"a\\n\\u00e9\") // note\n");
        assert_eq!(
            tokens,
            [
                Token::Ident("root".into()),
                Token::Equals,
                Token::Type("Card".into()),
                Token::LParen,
                Token::LBrack,
                Token::Ident("title".into()),
                Token::RBrack,
                Token::Comma,
                Token::Str("a\né".into()),
                Token::RParen,
                Token::Newline,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn distinguishes_negative_numbers_from_subtraction() {
        assert_eq!(tokenize("-5")[0], Token::Num(-5.0));
        assert_eq!(tokenize("[-1.5e2]")[1], Token::Num(-150.0));
        assert_eq!(
            tokenize("a -5")[..3],
            [Token::Ident("a".into()), Token::Minus, Token::Num(5.0)]
        );
        assert_eq!(tokenize("- x")[0], Token::Minus);
        assert_eq!(tokenize("1.")[..2], [Token::Num(1.0), Token::Dot]);
    }

    #[test]
    fn handles_strings_variables_and_comments() {
        assert_eq!(tokenize("'it\\'s'")[0], Token::Str("it's".into()));
        assert_eq!(
            tokenize("\"open")[0],
            Token::Str("open".into()),
            "unterminated"
        );
        assert_eq!(tokenize("\"\\ud83d\\ude00\"")[0], Token::Str("😀".into()));
        assert_eq!(
            tokenize("\"bad \\q\"")[0],
            Token::Str("bad \\q".into()),
            "raw on bad escape"
        );
        assert_eq!(tokenize("$days")[0], Token::StateVar("$days".into()));
        assert_eq!(tokenize("@Count")[0], Token::Builtin("Count".into()));
        assert_eq!(
            tokenize("# all comment\nx")[..2],
            [Token::Newline, Token::Ident("x".into())]
        );
        assert_eq!(tokenize("a && b | c")[1], Token::And);
        assert_eq!(tokenize("a && b | c")[3], Token::Or);
        assert_eq!(
            tokenize("a 😀 b").len(),
            3,
            "unknown characters are skipped"
        );
    }
}
