//! What is taken out of a report before it leaves the machine.
//!
//! The logs follow rules of their own (no token, key, query, prompt, LLM
//! answer or note text), but they do name folders, and a folder names its
//! repository and often the user. So every absolute path in a message, an
//! exception or a breadcrumb becomes [`PATH`]. The host name, the user, and
//! the query string, headers, cookies and body of a request are dropped. A
//! loaded program is named by its file name alone.

use std::borrow::Cow;

use sentry::protocol::{Breadcrumb, DebugImage, Event};

/// What an absolute path is replaced by.
pub const PATH: &str = "<path>";

/// `event` without what must not leave the machine, tagged with the program
/// that sent it.
pub fn event(mut event: Event<'static>, program: &str) -> Event<'static> {
    event.server_name = None;
    event.user = None;
    if let Some(request) = &mut event.request {
        request.query_string = None;
        request.cookies = None;
        request.data = None;
        request.headers.clear();
        request.env.clear();
        if let Some(url) = &mut request.url {
            url.set_query(None);
            url.set_fragment(None);
        }
    }
    scrub_option(&mut event.message);
    if let Some(entry) = &mut event.logentry {
        scrub_string(&mut entry.message);
    }
    if let Some(culprit) = &mut event.culprit {
        scrub_string(culprit);
    }
    for exception in &mut event.exception.values {
        scrub_option(&mut exception.value);
    }
    for crumb in &mut event.breadcrumbs.values {
        scrub_option(&mut crumb.message);
    }
    if !event.debug_meta.images.is_empty() {
        for image in &mut event.debug_meta.to_mut().images {
            if let DebugImage::Symbolic(image) = image {
                image.name = file_name(&image.name).to_string();
            }
        }
    }
    event.tags.insert("program".into(), program.into());
    event
}

/// `breadcrumb` with the paths in its message replaced.
pub fn breadcrumb(mut breadcrumb: Breadcrumb) -> Breadcrumb {
    scrub_option(&mut breadcrumb.message);
    breadcrumb
}

fn scrub_option(text: &mut Option<String>) {
    if let Some(text) = text {
        scrub_string(text);
    }
}

fn scrub_string(text: &mut String) {
    if let Cow::Owned(scrubbed) = paths(text) {
        *text = scrubbed;
    }
}

/// The last part of a Windows or Unix path.
fn file_name(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

/// `text` with every absolute Windows path replaced by [`PATH`]: one that
/// starts with a drive (`C:\`, `D:/`), a share (`\\server\`) or the verbatim
/// prefix (`\\?\`). A path runs to a quote, a colon, a pipe, an angle
/// bracket or a space; past a space only when one of the next few words
/// holds its separator, as in `C:\Users\Software Tester\AppData`.
/// Punctuation that ends a sentence stays outside it.
pub fn paths(text: &str) -> Cow<'_, str> {
    let mut scrubbed: Option<String> = None;
    let mut copied = 0;
    let mut at = 0;
    while at < text.len() {
        if let Some(end) = path_at(text, at) {
            let out = scrubbed.get_or_insert_with(|| String::with_capacity(text.len()));
            out.push_str(&text[copied..at]);
            out.push_str(PATH);
            copied = end;
            at = end;
        } else {
            at += text[at..].chars().next().map_or(1, char::len_utf8);
        }
    }
    match scrubbed {
        Some(mut out) => {
            out.push_str(&text[copied..]);
            Cow::Owned(out)
        }
        None => Cow::Borrowed(text),
    }
}

/// The end of the absolute path that starts at byte `at`, if one does.
fn path_at(text: &str, at: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let before = at.checked_sub(1).map(|i| bytes[i]);
    if before.is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return None;
    }
    let rest = &bytes[at..];
    let body = if rest.starts_with(br"\\?\") {
        let rest = &rest[4..];
        if is_drive(rest) { 7 } else { 4 }
    } else if is_drive(rest) {
        3
    } else if rest.len() > 2 && rest[0] == b'\\' && rest[1] == b'\\' && is_part(rest[2]) {
        2
    } else {
        return None;
    };
    // The separator the path is written with; a share's is the backslash.
    let separator = if body == 2 || body == 4 {
        b'\\'
    } else {
        bytes[at + body - 1]
    };
    let mut end = at + body;
    loop {
        while end < bytes.len() && is_part(bytes[end]) {
            end += 1;
        }
        match continued(bytes, end, separator) {
            Some(further) => end = further,
            None => break,
        }
    }
    while end > at + body && matches!(bytes[end - 1], b'.' | b',' | b';' | b')' | b']') {
        end -= 1;
    }
    Some(end)
}

/// How many words after a space are looked at for the rest of a path, as in
/// `C:\Program Files (x86)\CodeWiki`.
const WORDS_AHEAD: usize = 3;

/// Where a path that stopped at a space at `end` goes on to: the end of the
/// first of the next [`WORDS_AHEAD`] words that holds its `separator`, or
/// `None` when none does and the path ends at `end`.
fn continued(bytes: &[u8], end: usize, separator: u8) -> Option<usize> {
    let mut probe = end;
    for _ in 0..WORDS_AHEAD {
        if bytes.get(probe) != Some(&b' ') {
            return None;
        }
        let start = probe + 1;
        let len = bytes[start..].iter().take_while(|b| is_part(**b)).count();
        if len == 0 {
            return None;
        }
        probe = start + len;
        if bytes[start..probe].contains(&separator) {
            return Some(probe);
        }
    }
    None
}

/// A drive and a separator: `C:\` or `C:/`.
fn is_drive(bytes: &[u8]) -> bool {
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

/// Whether `byte` can be part of a path. Spaces are handled by the caller.
fn is_part(byte: u8) -> bool {
    !byte.is_ascii_whitespace()
        && !matches!(
            byte,
            b'"' | b'\'' | b'`' | b'<' | b'>' | b'|' | b':' | b'*' | b'?'
        )
}

#[cfg(test)]
mod tests {
    use sentry::protocol::{Exception, LogEntry, Request, SymbolicDebugImage, User, Values};

    use super::*;

    fn scrubbed(text: &str) -> String {
        paths(text).into_owned()
    }

    #[test]
    fn a_path_becomes_a_placeholder() {
        assert_eq!(
            scrubbed(r"opened D:\Work\secret\src\main.rs in 3 ms"),
            "opened <path> in 3 ms"
        );
        assert_eq!(
            scrubbed("sync of D:/Work/secret done"),
            "sync of <path> done"
        );
        assert_eq!(scrubbed(r"cloned \\server\share\repo."), "cloned <path>.");
        assert_eq!(scrubbed(r"\\?\C:\Users\me\x.txt"), "<path>");
        assert_eq!(scrubbed(r"C:\Users\me"), "<path>");
    }

    #[test]
    fn a_folder_name_with_a_space_stays_inside_the_path() {
        assert_eq!(
            scrubbed(r"could not read C:\Users\Software Tester\AppData\x.txt: denied"),
            "could not read <path>: denied"
        );
        assert_eq!(
            scrubbed(r"in C:\Program Files (x86)\CodeWiki\codewiki.exe now"),
            "in <path> now"
        );
        assert_eq!(scrubbed(r"from C:\a to D:\b"), "from <path> to <path>");
    }

    #[test]
    fn quoted_and_escaped_paths_are_found() {
        assert_eq!(
            scrubbed(r#"open "C:\\Users\\PC\\notes.md": not found"#),
            r#"open "<path>": not found"#
        );
        assert_eq!(scrubbed(r"('D:\repo', 3)"), "('<path>', 3)");
        assert_eq!(scrubbed(r"[C:\a\b]"), "[<path>]");
    }

    #[test]
    fn what_is_not_a_path_is_left_alone() {
        for text in [
            "GET /v1/repos/3/notes 200 in 12 ms",
            "http://127.0.0.1:7777/v1/health",
            "at 10:30 the job ended",
            "a:b and c: d",
            "ratio 3:4",
            r"a lone \ and \\",
            "No path here",
            "",
        ] {
            assert!(matches!(paths(text), Cow::Borrowed(_)), "{text}");
        }
    }

    #[test]
    fn text_around_a_path_keeps_its_characters() {
        assert_eq!(
            scrubbed(r"日本 C:\データ\x — fertig"),
            "日本 <path> — fertig"
        );
    }

    fn image(name: &str) -> DebugImage {
        DebugImage::Symbolic(SymbolicDebugImage {
            name: name.into(),
            arch: None,
            image_addr: 0.into(),
            image_size: 0,
            image_vmaddr: 0.into(),
            id: Default::default(),
            code_id: None,
            debug_file: Some("codewiki.pdb".into()),
        })
    }

    #[test]
    fn a_report_keeps_no_host_user_request_data_or_folder() {
        let mut event = Event {
            server_name: Some("DESKTOP-OWNER".into()),
            user: Some(User {
                username: Some("me".into()),
                ..Default::default()
            }),
            request: Some(Request {
                url: "http://127.0.0.1:7777/v1/repos/1/search?q=secret#top"
                    .parse()
                    .ok(),
                query_string: Some("q=secret".into()),
                cookies: Some("a=b".into()),
                data: Some("{\"text\":\"note\"}".into()),
                headers: [("authorization".to_string(), "Bearer t".to_string())]
                    .into_iter()
                    .collect(),
                ..Default::default()
            }),
            message: Some(r"sync of D:\Work\secret failed".into()),
            logentry: Some(LogEntry {
                message: r"read C:\Users\me\a.txt".into(),
                params: vec![],
            }),
            exception: Values {
                values: vec![Exception {
                    ty: "panic".into(),
                    value: Some(r"called unwrap on D:\x\y".into()),
                    ..Default::default()
                }],
            },
            breadcrumbs: Values {
                values: vec![Breadcrumb {
                    message: Some(r"opened D:\Work\secret".into()),
                    ..Default::default()
                }],
            },
            ..Default::default()
        };
        event.debug_meta.to_mut().images = vec![
            image(r"C:\Users\me\AppData\Local\Programs\CodeWiki\codewiki.exe"),
            image(r"C:\Windows\System32\ntdll.dll"),
        ];

        let event = super::event(event, "backend");
        assert_eq!(event.server_name, None);
        assert!(event.user.is_none());
        let request = event.request.unwrap();
        assert_eq!(request.query_string, None);
        assert_eq!(request.cookies, None);
        assert_eq!(request.data, None);
        assert!(request.headers.is_empty());
        assert_eq!(
            request.url.unwrap().as_str(),
            "http://127.0.0.1:7777/v1/repos/1/search"
        );
        assert_eq!(event.message.as_deref(), Some("sync of <path> failed"));
        assert_eq!(event.logentry.unwrap().message, "read <path>");
        assert_eq!(
            event.exception.values[0].value.as_deref(),
            Some("called unwrap on <path>")
        );
        assert_eq!(
            event.breadcrumbs.values[0].message.as_deref(),
            Some("opened <path>")
        );
        let names: Vec<&str> = event
            .debug_meta
            .images
            .iter()
            .map(|image| match image {
                DebugImage::Symbolic(image) => image.name.as_str(),
                _ => "",
            })
            .collect();
        assert_eq!(names, ["codewiki.exe", "ntdll.dll"]);
        assert_eq!(
            event.tags.get("program").map(String::as_str),
            Some("backend")
        );
    }

    #[test]
    fn a_breadcrumb_loses_its_paths() {
        let crumb = breadcrumb(Breadcrumb {
            message: Some(
                r"backend started; logs in C:\Users\me\AppData\Local\CodeWiki\logs".into(),
            ),
            ..Default::default()
        });
        assert_eq!(
            crumb.message.as_deref(),
            Some("backend started; logs in <path>")
        );
    }
}
