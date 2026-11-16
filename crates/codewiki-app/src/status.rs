//! What the app says about itself in the tray: what the backend is doing and
//! whether an agent is using it.
//!
//! The backend's answer to `GET /v1/status` is read here, and put into the
//! words of the tooltip and of the tray menu's first rows. Everything in
//! this module is a function of its arguments, so that the words can be
//! tested without a backend or a tray.

use serde_json::Value;

/// An agent's MCP bridge that called within this many seconds is "active".
const ACTIVE_FOR: u64 = 5 * 60;

/// The longest tooltip Windows shows, in UTF-16 units: the field holds 128
/// with its end mark.
pub const TOOLTIP_UNITS: usize = 127;

/// What the backend last said it is doing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Backend {
    /// Ids of the repositories whose wiki is being updated, whoever asked.
    pub updating: Vec<String>,
    /// When an agent's MCP bridge last called.
    pub bridge: Bridge,
}

/// When an agent's MCP bridge last called the backend.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Bridge {
    /// Not asked yet, or the backend did not answer.
    #[default]
    Unknown,
    /// The backend is from before it kept track.
    OldBackend,
    /// No bridge has called this backend since it started.
    Never,
    /// A bridge last called this many seconds ago.
    Called(u64),
}

/// Read the body of `GET /v1/status`. Times are ages by the backend's own
/// clock (`now`), so that a backend on another machine with another clock
/// reads the same.
pub fn parse(body: &Value) -> Backend {
    let now = body["now"].as_u64().unwrap_or_default();
    let updating = body["updates"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|update| update["repo"].as_str().map(str::to_string))
        .collect();
    let bridge = match body["bridge"]["last_call"].as_u64() {
        Some(at) => Bridge::Called(now.saturating_sub(at)),
        None => Bridge::Never,
    };
    Backend { updating, bridge }
}

/// An age in the largest unit that fits: "just now", "12 min ago", "3 h ago",
/// "2 days ago".
fn ago(seconds: u64) -> String {
    match seconds {
        0..60 => "just now".to_string(),
        60..3600 => format!("{} min ago", seconds / 60),
        3600..172_800 => format!("{} h ago", seconds / 3600),
        _ => format!("{} days ago", seconds / 86_400),
    }
}

/// What follows "MCP:" in the tooltip and the menu.
pub fn mcp_text(bridge: Bridge) -> String {
    match bridge {
        Bridge::Unknown => "unknown".to_string(),
        Bridge::OldBackend => "unknown (older backend)".to_string(),
        Bridge::Never => "no calls yet".to_string(),
        Bridge::Called(age) if age <= ACTIVE_FOR => "active".to_string(),
        Bridge::Called(age) => format!("idle (last call {})", ago(age)),
    }
}

/// What the app and its backend are doing, in a few words.
///
/// `busy` is what the app itself is waiting for, as its toolbar says it.
/// `updating` names the repositories whose wiki the backend is updating:
/// the open one is `current`, any other is named.
pub fn state_text(
    connected: bool,
    busy: Option<&str>,
    updating: &[String],
    current: Option<&str>,
) -> String {
    if let Some(busy) = busy {
        // The app's own update of the wiki reads as the backend's does.
        return if busy == "Syncing wiki" {
            "Updating wiki".to_string()
        } else {
            busy.to_string()
        };
    }
    if !connected {
        return "Backend not running".to_string();
    }
    match updating {
        [] => "Idle".to_string(),
        names if current.is_some_and(|current| names.iter().any(|name| name == current)) => {
            "Updating wiki".to_string()
        }
        [name] => format!("Updating wiki of {name}"),
        names => format!("Updating {} wikis", names.len()),
    }
}

/// The tooltip of the tray icon: `CodeWiki - <repo> - <state> - MCP: <mcp>`,
/// at most [`TOOLTIP_UNITS`] long. The repository's name comes first and is
/// what gets shortened when the whole does not fit, so that the state and
/// the MCP status stay readable.
pub fn tooltip(repo: Option<&str>, state: &str, mcp: &str) -> String {
    let tail = format!("{state} - MCP: {mcp}");
    let whole = |repo: &str| format!("CodeWiki - {repo} - {tail}");
    let Some(repo) = repo else {
        return cut_utf16(&format!("CodeWiki - {tail}"), TOOLTIP_UNITS);
    };
    let full = whole(repo);
    let over = full.encode_utf16().count().saturating_sub(TOOLTIP_UNITS);
    if over == 0 {
        return full;
    }
    // One more unit goes to the "…" that marks the cut.
    let keep = repo.encode_utf16().count().saturating_sub(over + 1);
    let short = format!("{}…", cut_utf16(repo, keep));
    cut_utf16(&whole(&short), TOOLTIP_UNITS)
}

/// The start of `text` that fits in `units` UTF-16 units, never ending in
/// the middle of a character.
pub fn cut_utf16(text: &str, units: usize) -> String {
    let mut used = 0;
    text.chars()
        .take_while(|c| {
            used += c.len_utf16();
            used <= units
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn the_backend_status_is_read_as_ages() {
        let body = json!({
            "version": "0.2.0",
            "now": 1_763_190_000u64,
            "updates": [{ "repo": "alpha", "started": 1_763_189_990u64 }],
            "bridge": { "last_call": 1_763_189_880u64 },
        });
        assert_eq!(
            parse(&body),
            Backend {
                updating: names(&["alpha"]),
                bridge: Bridge::Called(120),
            }
        );
        // No bridge yet, nothing running.
        let quiet = json!({ "now": 5, "updates": [], "bridge": { "last_call": null } });
        assert_eq!(
            parse(&quiet),
            Backend {
                updating: vec![],
                bridge: Bridge::Never,
            }
        );
        // A call the backend's clock puts in the future is "just now".
        let ahead = json!({ "now": 5, "updates": [], "bridge": { "last_call": 9 } });
        assert_eq!(parse(&ahead).bridge, Bridge::Called(0));
        // A body that is not what is expected names nothing.
        assert_eq!(
            parse(&json!("nonsense")),
            Backend {
                updating: vec![],
                bridge: Bridge::Never,
            }
        );
    }

    #[test]
    fn a_bridge_is_active_for_five_minutes_after_a_call() {
        assert_eq!(mcp_text(Bridge::Called(0)), "active");
        assert_eq!(mcp_text(Bridge::Called(300)), "active");
        assert_eq!(mcp_text(Bridge::Called(301)), "idle (last call 5 min ago)");
        assert_eq!(
            mcp_text(Bridge::Called(12 * 60 + 59)),
            "idle (last call 12 min ago)"
        );
        assert_eq!(
            mcp_text(Bridge::Called(3 * 3600 + 5)),
            "idle (last call 3 h ago)"
        );
        assert_eq!(
            mcp_text(Bridge::Called(47 * 3600)),
            "idle (last call 47 h ago)"
        );
        assert_eq!(
            mcp_text(Bridge::Called(3 * 86_400)),
            "idle (last call 3 days ago)"
        );
        assert_eq!(mcp_text(Bridge::Never), "no calls yet");
        assert_eq!(mcp_text(Bridge::OldBackend), "unknown (older backend)");
        assert_eq!(mcp_text(Bridge::Unknown), "unknown");
    }

    #[test]
    fn the_state_names_what_is_running() {
        assert_eq!(state_text(true, None, &[], Some("alpha")), "Idle");
        assert_eq!(state_text(true, None, &[], None), "Idle");
        // An update of the open repository, whoever started it.
        assert_eq!(
            state_text(true, None, &names(&["alpha"]), Some("alpha")),
            "Updating wiki"
        );
        assert_eq!(
            state_text(true, Some("Syncing wiki"), &[], Some("alpha")),
            "Updating wiki"
        );
        // An update of another repository is named.
        assert_eq!(
            state_text(true, None, &names(&["beta"]), Some("alpha")),
            "Updating wiki of beta"
        );
        assert_eq!(
            state_text(true, None, &names(&["beta", "gamma"]), Some("alpha")),
            "Updating 2 wikis"
        );
        assert_eq!(
            state_text(true, None, &names(&["beta", "alpha"]), Some("alpha")),
            "Updating wiki"
        );
        // What the app itself waits for comes first.
        assert_eq!(
            state_text(false, Some("Connecting"), &[], None),
            "Connecting"
        );
        assert_eq!(state_text(false, None, &[], None), "Backend not running");
    }

    #[test]
    fn the_tooltip_names_repository_state_and_mcp() {
        assert_eq!(
            tooltip(Some("CodeWiki"), "Updating wiki", "active"),
            "CodeWiki - CodeWiki - Updating wiki - MCP: active"
        );
        assert_eq!(
            tooltip(None, "Idle", "no calls yet"),
            "CodeWiki - Idle - MCP: no calls yet"
        );
    }

    #[test]
    fn a_long_repository_name_is_what_the_tooltip_shortens() {
        let long = "r".repeat(200);
        let tip = tooltip(Some(&long), "Updating wiki", "idle (last call 12 min ago)");
        assert_eq!(tip.encode_utf16().count(), TOOLTIP_UNITS);
        assert!(tip.starts_with("CodeWiki - rrrr"));
        assert!(tip.ends_with("… - Updating wiki - MCP: idle (last call 12 min ago)"));
        // One unit over is enough to shorten.
        let state = "Idle";
        let fits = TOOLTIP_UNITS - "CodeWiki -  - Idle - MCP: active".len();
        let exact = "x".repeat(fits);
        assert_eq!(
            tooltip(Some(&exact), state, "active")
                .encode_utf16()
                .count(),
            TOOLTIP_UNITS
        );
        assert!(!tooltip(Some(&exact), state, "active").contains('…'));
        let over = "x".repeat(fits + 1);
        let tip = tooltip(Some(&over), state, "active");
        assert_eq!(tip.encode_utf16().count(), TOOLTIP_UNITS);
        assert!(tip.contains("x… - Idle"));
    }

    #[test]
    fn a_cut_never_splits_a_character() {
        assert_eq!(cut_utf16("abc", 2), "ab");
        assert_eq!(cut_utf16("abc", 3), "abc");
        assert_eq!(cut_utf16("abc", 9), "abc");
        assert_eq!(cut_utf16("abc", 0), "");
        // "𝄞" takes two UTF-16 units: it goes whole or not at all.
        assert_eq!(cut_utf16("a𝄞b", 2), "a");
        assert_eq!(cut_utf16("a𝄞b", 3), "a𝄞");
        // A name of such characters still gives a tooltip that fits.
        let clefs = "𝄞".repeat(100);
        let tip = tooltip(Some(&clefs), "Idle", "active");
        assert!(tip.encode_utf16().count() <= TOOLTIP_UNITS);
        assert!(tip.ends_with("… - Idle - MCP: active"));
    }
}
