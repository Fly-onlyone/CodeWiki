//! A light lexical scan of Rust source, shared by the test map and the
//! security rules.
//!
//! The structural parser deliberately skips unit-test modules, and neither
//! analysis needs a syntax tree: they need to know which bytes are code, which
//! are string literals, and which belong to test-only items. Comments and
//! literal contents are blanked in [`Scanned::code`] so a pattern can never
//! match inside either; offsets and line breaks are preserved.

/// One string literal. Offsets index into [`Scanned::code`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Literal {
    /// Offset of the opening quote.
    pub open: usize,
    /// Offset one past the closing quote (and any raw-string hashes).
    pub close: usize,
    /// Text between the quotes, escapes left as written.
    pub value: String,
}

/// A `#[test]`-like function found in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TestFn {
    pub name: String,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct Scanned {
    /// The source with comments and literal contents replaced by spaces.
    pub code: String,
    /// String literals in source order.
    pub literals: Vec<Literal>,
    pub tests: Vec<TestFn>,
    line_starts: Vec<usize>,
    /// Byte ranges of items that only exist in test builds.
    test_regions: Vec<(usize, usize)>,
}

/// True for paths under a `tests`, `benches` or `examples` directory.
pub(crate) fn is_test_path(path: &str) -> bool {
    path.split('/')
        .any(|s| matches!(s, "tests" | "benches" | "examples"))
}

fn is_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 0x80
}

fn blank(code: &mut [u8], from: usize, to: usize) {
    let to = to.min(code.len());
    for byte in &mut code[from.min(to)..to] {
        if *byte != b'\n' {
            *byte = b' ';
        }
    }
}

/// Blank comments and literal contents; collect string literals.
fn mask(source: &str) -> (String, Vec<Literal>) {
    let bytes = source.as_bytes();
    let n = bytes.len();
    let mut code = bytes.to_vec();
    let mut literals = Vec::new();
    let mut i = 0;
    while i < n {
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                let end = bytes[i..]
                    .iter()
                    .position(|b| *b == b'\n')
                    .map_or(n, |p| i + p);
                blank(&mut code, i, end);
                i = end;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                // Block comments nest.
                let mut depth = 1;
                let mut j = i + 2;
                while j < n && depth > 0 {
                    if bytes[j] == b'/' && bytes.get(j + 1) == Some(&b'*') {
                        depth += 1;
                        j += 2;
                    } else if bytes[j] == b'*' && bytes.get(j + 1) == Some(&b'/') {
                        depth -= 1;
                        j += 2;
                    } else {
                        j += 1;
                    }
                }
                blank(&mut code, i, j);
                i = j;
            }
            b'"' => i = string(source, &mut code, &mut literals, i),
            b'\'' => i = char_or_lifetime(source, &mut code, i),
            b if is_ident(b) => {
                let mut j = i;
                while j < n && is_ident(bytes[j]) {
                    j += 1;
                }
                let word = &source[i..j];
                let hashes = bytes[j..].iter().take_while(|b| **b == b'#').count();
                if matches!(word, "r" | "br" | "cr") && bytes.get(j + hashes) == Some(&b'"') {
                    i = raw_string(source, &mut code, &mut literals, j + hashes, hashes);
                } else if matches!(word, "b" | "c") && bytes.get(j) == Some(&b'"') {
                    i = string(source, &mut code, &mut literals, j);
                } else if word == "b" && bytes.get(j) == Some(&b'\'') {
                    i = char_or_lifetime(source, &mut code, j);
                } else {
                    i = j;
                }
            }
            _ => i += 1,
        }
    }
    // Only ASCII bytes were overwritten, and always whole characters.
    (String::from_utf8_lossy(&code).into_owned(), literals)
}

/// Consume a `"…"` literal whose opening quote is at `open`.
fn string(source: &str, code: &mut [u8], literals: &mut Vec<Literal>, open: usize) -> usize {
    let bytes = source.as_bytes();
    let mut j = open + 1;
    while j < bytes.len() && bytes[j] != b'"' {
        j += if bytes[j] == b'\\' { 2 } else { 1 };
    }
    let end = j.min(bytes.len());
    blank(code, open + 1, end);
    let close = (end + 1).min(bytes.len());
    literals.push(Literal {
        open,
        close,
        value: String::from_utf8_lossy(&bytes[open + 1..end]).into_owned(),
    });
    close
}

/// Consume a raw string whose opening quote is at `open`, closed by a quote
/// and `hashes` hash signs.
fn raw_string(
    source: &str,
    code: &mut [u8],
    literals: &mut Vec<Literal>,
    open: usize,
    hashes: usize,
) -> usize {
    let bytes = source.as_bytes();
    let mut j = open + 1;
    while j < bytes.len() {
        if bytes[j] == b'"'
            && bytes[j + 1..]
                .iter()
                .take(hashes)
                .filter(|b| **b == b'#')
                .count()
                == hashes
        {
            break;
        }
        j += 1;
    }
    let end = j.min(bytes.len());
    blank(code, open + 1, end);
    let close = (end + 1 + hashes).min(bytes.len());
    literals.push(Literal {
        open,
        close,
        value: String::from_utf8_lossy(&bytes[open + 1..end]).into_owned(),
    });
    close
}

/// Blank a character literal starting at `open`, or step over a lifetime.
fn char_or_lifetime(source: &str, code: &mut [u8], open: usize) -> usize {
    let bytes = source.as_bytes();
    if bytes.get(open + 1) == Some(&b'\\') {
        let mut j = open + 3;
        while j < bytes.len() && bytes[j] != b'\'' && bytes[j] != b'\n' {
            j += 1;
        }
        blank(code, open + 1, j);
        return (j + 1).min(bytes.len());
    }
    let width = source
        .get(open + 1..)
        .and_then(|rest| rest.chars().next())
        .map_or(1, char::len_utf8);
    if bytes.get(open + 1 + width) == Some(&b'\'') {
        blank(code, open + 1, open + 1 + width);
        open + width + 2
    } else {
        open + 1
    }
}

/// Offset one past the bracket that closes the one at `open`.
fn matching(code: &[u8], open: usize) -> usize {
    let (left, right) = match code[open] {
        b'(' => (b'(', b')'),
        b'[' => (b'[', b']'),
        _ => (b'{', b'}'),
    };
    let mut depth = 0usize;
    for (offset, byte) in code.iter().enumerate().skip(open) {
        if *byte == left {
            depth += 1;
        } else if *byte == right {
            depth -= 1;
            if depth == 0 {
                return offset + 1;
            }
        }
    }
    code.len()
}

/// What an attribute says about the item it is on.
enum Attr {
    /// Marks a test function (`#[test]`, `#[tokio::test]`, `#[rstest]`).
    Test,
    /// Compiles the item only for tests (`#[cfg(test)]`).
    CfgTest,
    Other,
}

fn classify(attr: &str) -> Attr {
    let compact: String = attr.chars().filter(|c| !c.is_whitespace()).collect();
    let path = compact.split('(').next().unwrap_or("");
    if path == "test" || path.ends_with("::test") || matches!(path, "rstest" | "test_case") {
        return Attr::Test;
    }
    if path == "cfg" && !compact.contains("not(") {
        let inner = &compact[3..];
        let is_word = |pos: usize| {
            let before = inner.as_bytes()[pos - 1];
            let after = inner.as_bytes().get(pos + 4).copied().unwrap_or(b')');
            !is_ident(before) && !is_ident(after)
        };
        if inner.match_indices("test").any(|(pos, _)| is_word(pos)) {
            return Attr::CfgTest;
        }
    }
    Attr::Other
}

/// Find test functions and test-only byte ranges in masked code.
fn find_tests(code: &str, line_of: impl Fn(usize) -> u32) -> (Vec<TestFn>, Vec<(usize, usize)>) {
    let bytes = code.as_bytes();
    let n = bytes.len();
    let mut tests: Vec<TestFn> = Vec::new();
    let mut regions = Vec::new();
    let mut i = 0;
    while i + 1 < n {
        if bytes[i] != b'#' {
            i += 1;
            continue;
        }
        let inner = bytes[i + 1] == b'!';
        let open = i + 1 + usize::from(inner);
        if bytes.get(open) != Some(&b'[') {
            i += 1;
            continue;
        }
        let close = matching(bytes, open);
        let kind = classify(&code[open + 1..close.saturating_sub(1).max(open + 1)]);
        if matches!(kind, Attr::Other) {
            i = close;
            continue;
        }
        if inner {
            // `#![cfg(test)]` covers the rest of the enclosing module; at the
            // top of a file that is the whole file.
            regions.push((i, n));
            i = close;
            continue;
        }
        // Skip the item's remaining attributes, then walk its header to the
        // body or the terminating semicolon.
        let mut j = close;
        loop {
            while j < n && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if bytes.get(j) == Some(&b'#') && bytes.get(j + 1) == Some(&b'[') {
                j = matching(bytes, j + 1);
            } else {
                break;
            }
        }
        let header = j;
        let mut end = n;
        while j < n {
            match bytes[j] {
                b'(' | b'[' => j = matching(bytes, j),
                b';' => {
                    end = j + 1;
                    break;
                }
                b'{' => {
                    end = matching(bytes, j);
                    break;
                }
                _ => j += 1,
            }
        }
        if matches!(kind, Attr::Test)
            && let Some((name, at)) = fn_name(code, header, j.min(n))
            && !tests
                .iter()
                .any(|t| t.name == name && t.line == line_of(at))
        {
            tests.push(TestFn {
                name,
                line: line_of(at),
            });
        }
        regions.push((i, end));
        // Step past the attribute only: test functions nest inside test modules.
        i = close;
    }
    (tests, regions)
}

/// Name and offset of the function declared in `code[from..to]`.
fn fn_name(code: &str, from: usize, to: usize) -> Option<(String, usize)> {
    let header = code.get(from..to)?;
    let bytes = header.as_bytes();
    for (pos, _) in header.match_indices("fn") {
        let before = pos.checked_sub(1).map(|p| bytes[p]);
        let after = bytes.get(pos + 2).copied();
        if before.is_some_and(is_ident) || !after.is_some_and(|b| b.is_ascii_whitespace()) {
            continue;
        }
        let name: String = header[pos + 2..]
            .trim_start()
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            return Some((name, from + pos));
        }
    }
    None
}

impl Scanned {
    pub fn new(source: &str) -> Self {
        let (code, literals) = mask(source);
        let mut line_starts = vec![0];
        line_starts.extend(
            code.bytes()
                .enumerate()
                .filter(|(_, b)| *b == b'\n')
                .map(|(i, _)| i + 1),
        );
        let line_of = |offset: usize| line_starts.partition_point(|s| *s <= offset) as u32;
        let (tests, test_regions) = find_tests(&code, line_of);
        Self {
            code,
            literals,
            tests,
            line_starts,
            test_regions,
        }
    }

    /// 1-based line of a byte offset.
    pub fn line_of(&self, offset: usize) -> u32 {
        self.line_starts.partition_point(|s| *s <= offset) as u32
    }

    /// True when `offset` lies in an item compiled only for tests.
    pub fn in_test(&self, offset: usize) -> bool {
        self.test_regions
            .iter()
            .any(|(from, to)| (*from..*to).contains(&offset))
    }

    /// The literal whose opening quote is at `open`.
    pub fn literal_at(&self, open: usize) -> Option<&Literal> {
        self.literals
            .binary_search_by_key(&open, |l| l.open)
            .ok()
            .map(|i| &self.literals[i])
    }

    /// Offsets where `word` occurs in code as a whole identifier or path.
    pub fn find<'a>(&'a self, word: &'a str) -> impl Iterator<Item = usize> + 'a {
        let bytes = self.code.as_bytes();
        let first_is_ident = word.bytes().next().is_some_and(is_ident);
        let last_is_ident = word.bytes().last().is_some_and(is_ident);
        self.code.match_indices(word).filter_map(move |(pos, _)| {
            let before = pos.checked_sub(1).map(|p| bytes[p]);
            let after = bytes.get(pos + word.len()).copied();
            let cut_before = !first_is_ident || !before.is_some_and(is_ident);
            let cut_after = !last_is_ident || !after.is_some_and(is_ident);
            (cut_before && cut_after).then_some(pos)
        })
    }

    /// Offset one past the bracket closing the one at `open`.
    pub fn matching(&self, open: usize) -> usize {
        matching(self.code.as_bytes(), open)
    }

    /// End of the statement containing `from`: the next `;` outside nested
    /// brackets, or the end of the enclosing block.
    pub fn statement_end(&self, from: usize) -> usize {
        let bytes = self.code.as_bytes();
        let mut j = from;
        while j < bytes.len() {
            match bytes[j] {
                b'(' | b'[' | b'{' => j = matching(bytes, j),
                b';' | b')' | b']' | b'}' => return j,
                _ => j += 1,
            }
        }
        bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blanks_comments_and_literal_contents_but_keeps_offsets() {
        let source = "let a = \"x // y\"; // unsafe { }\n/* outer /* inner */ still */ let b = 'c';\nlet r = r#\"raw \"quoted\" {\"#; let l: &'static str = b\"\\\"\";\n";
        let s = Scanned::new(source);
        assert_eq!(s.code.len(), source.len());
        assert_eq!(s.code.lines().count(), source.lines().count());
        assert!(!s.code.contains("unsafe") && !s.code.contains("inner"));
        assert!(!s.code.contains("quoted") && !s.code.contains('{'));
        assert!(s.code.contains("let b = ' ';"));
        assert!(s.code.contains("&'static str"), "lifetimes survive");
        let values: Vec<&str> = s.literals.iter().map(|l| l.value.as_str()).collect();
        assert_eq!(values, ["x // y", "raw \"quoted\" {", "\\\""]);
        let raw = &s.literals[1];
        assert_eq!(&source[raw.open..raw.close], "\"raw \"quoted\" {\"#");
        assert_eq!(s.line_of(raw.open), 3);
        assert_eq!(s.literal_at(raw.open), Some(raw));
    }

    #[test]
    fn finds_tests_and_test_only_regions() {
        let source = "pub fn real() { helper(); }\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn adds() { assert_eq!(1 + 1, 2); }\n\n    #[tokio::test]\n    #[ignore]\n    async fn waits() {}\n\n    fn helper() {}\n}\n\n#[cfg(not(test))]\nfn shipped() {}\n\n#[cfg(all(test, feature = \"slow\"))]\nfn slow_only() {}\n";
        let s = Scanned::new(source);
        let names: Vec<(&str, u32)> = s.tests.iter().map(|t| (t.name.as_str(), t.line)).collect();
        assert_eq!(names, [("adds", 8), ("waits", 12)]);
        let at = |needle: &str| source.find(needle).unwrap();
        assert!(!s.in_test(at("real")));
        assert!(s.in_test(at("helper() {}")));
        assert!(s.in_test(at("assert_eq")));
        assert!(!s.in_test(at("shipped")));
        assert!(s.in_test(at("slow_only")));
    }

    #[test]
    fn test_attributes_inside_strings_are_ignored() {
        let s = Scanned::new("const FIXTURE: &str = \"#[test]\\nfn fake() {}\";\nfn real() {}\n");
        assert!(s.tests.is_empty());
        assert!(!s.in_test(50));
    }

    #[test]
    fn finds_whole_words_and_statement_ends() {
        let s = Scanned::new("fn f() { unsafe_thing(); unsafe { g(a, [1; 2]); h() }; done() }");
        let hits: Vec<usize> = s.find("unsafe").collect();
        assert_eq!(hits.len(), 1);
        let end = s.statement_end(hits[0]);
        assert_eq!(&s.code[end..end + 8], "; done()");
        assert_eq!(s.find("::new").count(), 0);
    }
}
