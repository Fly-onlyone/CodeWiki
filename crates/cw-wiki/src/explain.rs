//! What a note says beyond its subject's own comment: the walk-through of
//! what happens when the code is used, and the list of its items.
//!
//! Both are built from facts alone. The walk-through follows resolved calls,
//! in the order they stand in the source, and so says nothing it cannot
//! know: no condition, no loop, no reason. A writer that reads the bodies
//! adds those.

use std::collections::{BTreeSet, HashSet};

use cw_core::{Confidence, EdgeKind, Language, Symbol, SymbolKind, Visibility};
use cw_vault::lint::is_line_ref;

use crate::doctext::{first_sentence, item_text};
use crate::render::{RenderCtx, lower_first};

/// Steps of one walk-through.
const MAX_STEPS: usize = 5;
/// Calls named in one step.
const MAX_CALLS: usize = 6;
/// Callers named for one entry point.
const MAX_CALLERS: usize = 2;
/// Functions of the note itself that a step goes on to describe.
const MAX_HELPERS: usize = 2;
/// Calls named in one step of an illustration, which has less room.
const OUTLINE_CALLS: usize = 4;
/// Entry points named after the steps.
const MAX_MORE: usize = 6;
/// Items listed in a note.
const MAX_ITEMS: usize = 20;
/// Items listed when the note has almost none that are not private.
const FEW_ITEMS: usize = 8;
/// Names given as the users of an item that has no comment.
const MAX_USERS: usize = 4;
/// A longer signature is replaced by the item's name.
const MAX_SIGNATURE: usize = 100;

/// The callers of a function from other notes, each with the id of its note.
type Callers<'a> = Vec<(&'a Symbol, &'a str)>;

/// One step of a walk-through: a function of the note, who calls it from
/// outside, and what it calls.
pub(crate) struct Step<'a> {
    pub entry: &'a Symbol,
    /// Callers in other notes, each with the id of its note.
    callers: Vec<(&'a Symbol, &'a str)>,
    /// True when a program starts at `entry`.
    started: bool,
    /// What `entry` calls, in source order, and how many more it calls.
    pub calls: Vec<&'a Symbol>,
    more_calls: usize,
    /// Functions of the note among `calls`, with what they call in turn.
    helpers: Vec<(&'a Symbol, Vec<&'a Symbol>, usize)>,
}

/// What happens when the code of one note is used.
pub(crate) struct Walk<'a> {
    pub steps: Vec<Step<'a>>,
    /// Entry points beyond the steps.
    more: Vec<&'a Symbol>,
    /// False when no other note calls in; the steps are then the note's own
    /// functions in source order.
    entered: bool,
}

fn is_rust(symbol: &Symbol) -> bool {
    Language::from_path(&symbol.file) == Language::Rust
}

/// How a reader names `symbol`: a method with its type.
pub(crate) fn short_name(symbol: &Symbol) -> String {
    name_in(symbol, "")
}

/// [`short_name`] in the note about `subject`: a method of the subject
/// itself goes by its own name there.
fn name_in(symbol: &Symbol, subject: &str) -> String {
    match symbol.parent.as_deref() {
        Some(parent) if symbol.kind == SymbolKind::Method && parent != subject => {
            let owner = parent.rsplit("::").next().unwrap_or(parent);
            let sep = if is_rust(symbol) { "::" } else { "." };
            format!("{owner}{sep}{}", symbol.name)
        }
        _ => symbol.name.clone(),
    }
}

/// `items` as `a, then b, then c`, with `and N more` for the rest. Each
/// item is a name and, when it lives in another note, the link to that
/// note. Neighbours in one note share its link: `a, then b (both in [[N]])`.
fn in_order(items: &[(String, Option<String>)], more: usize) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut i = 0;
    while i < items.len() {
        let link = &items[i].1;
        let run = items[i..].iter().take_while(|item| item.1 == *link).count();
        let names: Vec<&str> = items[i..i + run]
            .iter()
            .map(|item| item.0.as_str())
            .collect();
        let names = names.join(", then ");
        parts.push(match (link, run) {
            (None, _) => names,
            (Some(link), 1) => format!("{names} in {link}"),
            (Some(link), 2) => format!("{names} (both in {link})"),
            (Some(link), _) => format!("{names} (all in {link})"),
        });
        i += run;
    }
    let mut text = parts.join(", then ");
    if more > 0 {
        text.push_str(&format!(", and {more} more"));
    }
    text
}

impl<'a> Walk<'a> {
    /// Qualified names of every symbol the walk-through names.
    pub fn names(&self) -> HashSet<&'a str> {
        let mut names: HashSet<&str> = self.more.iter().map(|s| s.qualified.as_str()).collect();
        for step in &self.steps {
            names.insert(step.entry.qualified.as_str());
            names.extend(step.calls.iter().map(|s| s.qualified.as_str()));
            for (helper, calls, _) in &step.helpers {
                names.insert(helper.qualified.as_str());
                names.extend(calls.iter().map(|s| s.qualified.as_str()));
            }
        }
        names
    }

    /// The walk-through for an illustration: each step's name, and one line
    /// of plain text on who calls it and what it calls. An illustration
    /// draws no code span and no link, so names stand bare.
    pub fn outline(&self, subject: &str) -> Vec<(String, String)> {
        let listed = |names: Vec<String>, shown: usize, joint: &str, more: usize| {
            let more = more + names.len().saturating_sub(shown);
            let mut text = names[..names.len().min(shown)].join(joint);
            if more > 0 {
                text.push_str(&format!(" and {more} more"));
            }
            text
        };
        self.steps
            .iter()
            .map(|step| {
                let mut parts: Vec<String> = Vec::new();
                if step.started {
                    parts.push("The program starts here".to_string());
                } else if !step.callers.is_empty() {
                    let names = step.callers.iter().map(|(s, _)| short_name(s)).collect();
                    parts.push(format!("Called by {}", listed(names, MAX_CALLERS, ", ", 0)));
                }
                if !step.calls.is_empty() {
                    let names = step.calls.iter().map(|s| name_in(s, subject)).collect();
                    parts.push(format!(
                        "Calls {}",
                        listed(names, OUTLINE_CALLS, ", then ", step.more_calls)
                    ));
                }
                let detail = if parts.is_empty() {
                    String::new()
                } else {
                    format!("{}.", parts.join(". "))
                };
                (name_in(step.entry, subject), detail)
            })
            .collect()
    }

    /// The walk-through as numbered steps, for the note `note_id` about
    /// `subject`. `noun` is what the subject is: `module`, `crate`, `type`.
    /// Empty when there is nothing to walk.
    pub fn text(&self, ctx: &RenderCtx, note_id: &str, subject: &str, noun: &str) -> String {
        if self.steps.is_empty() {
            return String::new();
        }
        // A symbol as the reader meets it: its name, and its note when that
        // is another one.
        let named = |symbol: &Symbol| {
            let code = format!("`{}`", name_in(symbol, subject));
            let link = match ctx.plan.owner.get(&symbol.qualified) {
                Some(id) if id != note_id => ctx.link(id),
                _ => None,
            };
            (code, link)
        };
        let calls = |from: &Symbol, to: &[&Symbol], more: usize| {
            let listed: Vec<(String, Option<String>)> = to.iter().map(|s| named(s)).collect();
            format!(
                "`{}` calls {}.",
                name_in(from, subject),
                in_order(&listed, more)
            )
        };
        let mut lines: Vec<String> = Vec::new();
        if !self.entered {
            // No call from another note was resolved. That does not say the
            // code is unused: a call through a value whose type is not known
            // is not followed.
            lines.push(format!("What the functions of this {noun} call:"));
            lines.push(String::new());
        }
        for (number, step) in self.steps.iter().enumerate() {
            let mut sentences: Vec<String> = Vec::new();
            let entry = format!("`{}`", name_in(step.entry, subject));
            if step.started {
                sentences.push(format!("The program starts at {entry}."));
            } else if !step.callers.is_empty() {
                sentences.push(self.callers_sentence(ctx, step, &entry));
            }
            if !step.calls.is_empty() {
                sentences.push(calls(step.entry, &step.calls, step.more_calls));
            }
            for (helper, helper_calls, more) in &step.helpers {
                sentences.push(calls(helper, helper_calls, *more));
            }
            lines.push(format!("{}. {}", number + 1, sentences.join(" ")));
        }
        if !self.more.is_empty() {
            let mut names: Vec<String> = self
                .more
                .iter()
                .take(MAX_MORE)
                .map(|s| format!("`{}`", name_in(s, subject)))
                .collect();
            if self.more.len() > MAX_MORE {
                names.push(format!("and {} more", self.more.len() - MAX_MORE));
            }
            lines.push(String::new());
            lines.push(format!(
                "{} more entry {}: {}.",
                self.more.len(),
                if self.more.len() == 1 {
                    "point"
                } else {
                    "points"
                },
                names.join(", ")
            ));
        }
        let ordered = self
            .steps
            .iter()
            .any(|s| s.calls.len() > 1 || s.helpers.iter().any(|(_, calls, _)| calls.len() > 1));
        if ordered {
            lines.push(String::new());
            lines.push(
                "Calls are listed in the order of the source; a condition or a loop around a call is not shown."
                    .to_string(),
            );
        }
        lines.join("\n")
    }

    /// `a` and `b` in [[X]] call `entry`.
    fn callers_sentence(&self, ctx: &RenderCtx, step: &Step, entry: &str) -> String {
        let shown = &step.callers[..step.callers.len().min(MAX_CALLERS)];
        let part = |(symbol, id): &(&Symbol, &str)| {
            let code = format!("`{}`", short_name(symbol));
            (code, ctx.link(id))
        };
        let parts: Vec<(String, Option<String>)> = shown.iter().map(part).collect();
        let same_note = parts.len() == 2 && parts[0].1 == parts[1].1;
        let place = |link: &Option<String>| match link {
            Some(link) => format!(" in {link}"),
            None => String::new(),
        };
        let mut who = match parts.as_slice() {
            [one] => format!("{}{}", one.0, place(&one.1)),
            [a, b] if same_note => format!("{} and {}{}", a.0, b.0, place(&a.1)),
            [a, b] => format!("{}{} and {}{}", a.0, place(&a.1), b.0, place(&b.1)),
            _ => String::new(),
        };
        let more = step.callers.len() - shown.len();
        if more > 0 {
            who.push_str(&format!(", and {more} more,"));
        }
        let verb = if step.callers.len() == 1 {
            "calls"
        } else {
            "call"
        };
        format!("{who} {verb} {entry}.")
    }
}

/// Walk through the note with `note_id`: its functions that other notes
/// call, each with what it calls in turn.
pub(crate) fn walk<'a>(ctx: &RenderCtx<'a>, note_id: &str) -> Walk<'a> {
    let repo = ctx.repo;
    let owned = |q: &str| ctx.plan.owner.get(q).is_some_and(|id| id == note_id);
    let members: Vec<&'a Symbol> = repo
        .facts
        .symbols
        .iter()
        .filter(|s| matches!(s.kind, SymbolKind::Function | SymbolKind::Method))
        .filter(|s| owned(&s.qualified) && !repo.is_test(s))
        .collect();
    // What a function calls, where the resolver is sure enough to say so.
    let calls_of = |symbol: &Symbol| -> Vec<&'a Symbol> {
        repo.calls_from(&symbol.qualified)
            .into_iter()
            .filter(|e| e.confidence != Confidence::Low)
            .filter_map(|e| repo.symbols.get(e.dst.as_str()).copied())
            .filter(|s| !repo.is_test(s))
            .collect()
    };
    let starts: HashSet<&str> = repo
        .units
        .list
        .iter()
        .flat_map(|u| u.entries.iter().map(|e| e.symbol.as_str()))
        .collect();
    let callers_of = |symbol: &Symbol| -> Vec<(&'a Symbol, &'a str)> {
        let mut callers: Vec<(&Symbol, &str)> = repo
            .incoming
            .get(symbol.qualified.as_str())
            .into_iter()
            .flatten()
            .filter(|e| e.kind == EdgeKind::Calls && e.confidence != Confidence::Low)
            .filter_map(|e| {
                let caller = repo.symbols.get(e.src.as_str()).copied()?;
                let id = ctx.plan.owner.get(&caller.qualified)?;
                (id != note_id && !repo.is_test(caller)).then_some((caller, id.as_str()))
            })
            .collect();
        callers.sort_by(|a, b| a.1.cmp(b.1).then(a.0.qualified.cmp(&b.0.qualified)));
        callers.dedup_by(|a, b| a.0.qualified == b.0.qualified);
        callers
    };
    let in_source = |a: &&Symbol, b: &&Symbol| {
        a.file
            .cmp(&b.file)
            .then(a.start_line.cmp(&b.start_line))
            .then(a.qualified.cmp(&b.qualified))
    };

    let mut entries: Vec<(&Symbol, Callers, bool)> = members
        .iter()
        .map(|m| (*m, callers_of(m), starts.contains(m.qualified.as_str())))
        .filter(|(_, callers, started)| *started || !callers.is_empty())
        .collect();
    let notes_calling = |callers: &[(&Symbol, &str)]| {
        callers
            .iter()
            .map(|(_, id)| *id)
            .collect::<BTreeSet<_>>()
            .len()
    };
    entries.sort_by(|a, b| {
        b.2.cmp(&a.2)
            .then(notes_calling(&b.1).cmp(&notes_calling(&a.1)))
            .then(in_source(&a.0, &b.0))
    });
    let entered = !entries.is_empty();
    if !entered {
        // Nothing calls in: the functions others could call, as they stand.
        let mut own: Vec<&Symbol> = members
            .iter()
            .copied()
            .filter(|m| m.visibility != Visibility::Private && !calls_of(m).is_empty())
            .collect();
        own.sort_by(in_source);
        entries = own.into_iter().map(|m| (m, Vec::new(), false)).collect();
    }

    let more: Vec<&Symbol> = entries.iter().skip(MAX_STEPS).map(|(s, _, _)| *s).collect();
    entries.truncate(MAX_STEPS);
    // Each function is described once, where the walk first meets it.
    let mut described: HashSet<&str> = entries
        .iter()
        .map(|(s, _, _)| s.qualified.as_str())
        .collect();
    described.extend(more.iter().map(|s| s.qualified.as_str()));
    let capped = |mut calls: Vec<&'a Symbol>| {
        let more = calls.len().saturating_sub(MAX_CALLS);
        calls.truncate(MAX_CALLS);
        (calls, more)
    };
    let mut steps = Vec::new();
    for (entry, callers, started) in entries {
        let (calls, more_calls) = capped(calls_of(entry));
        let mut helpers = Vec::new();
        for callee in &calls {
            if helpers.len() == MAX_HELPERS {
                break;
            }
            if !owned(&callee.qualified) || described.contains(callee.qualified.as_str()) {
                continue;
            }
            let (inner, inner_more) = capped(calls_of(callee));
            if !inner.is_empty() {
                described.insert(callee.qualified.as_str());
                helpers.push((*callee, inner, inner_more));
            }
        }
        steps.push(Step {
            entry,
            callers,
            started,
            calls,
            more_calls,
            helpers,
        });
    }
    Walk {
        steps,
        more,
        entered,
    }
}

/// The type a function returns, as its signature writes it, or `None` when
/// the signature names none or cannot be read with confidence.
fn return_type(signature: &str, language: Language) -> Option<String> {
    let text = match language {
        Language::Rust | Language::Python => {
            // The first `->` outside every bracket: one inside is part of a
            // parameter such as `impl Fn(u8) -> u8`.
            let bytes = signature.as_bytes();
            let (mut round, mut angle) = (0usize, 0usize);
            let mut found = None;
            let mut i = 0;
            while i < bytes.len() {
                match bytes[i] {
                    b'-' if bytes.get(i + 1) == Some(&b'>') => {
                        if round == 0 && angle == 0 {
                            found = Some(i + 2);
                            break;
                        }
                        i += 1;
                    }
                    b'(' | b'[' => round += 1,
                    b')' | b']' => round = round.saturating_sub(1),
                    b'<' => angle += 1,
                    b'>' => angle = angle.saturating_sub(1),
                    _ => {}
                }
                i += 1;
            }
            let rest = &signature[found?..];
            rest.split(" where ").next().unwrap_or(rest)
        }
        Language::TypeScript | Language::JavaScript => {
            // After the parameter list: `(x: number): string`.
            let open = signature.find('(')?;
            let mut depth = 0usize;
            let close = signature[open..].char_indices().find_map(|(i, c)| {
                match c {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    _ => {}
                }
                (depth == 0).then_some(open + i)
            })?;
            let rest = signature[close + 1..].trim_start().strip_prefix(':')?;
            rest.split(" =>").next().unwrap_or(rest)
        }
        _ => return None,
    };
    let text = text.trim().trim_end_matches([':', '{']).trim();
    let unit = matches!(text, "" | "()" | "None" | "void");
    (!unit && !text.contains('`') && text.chars().count() <= 60).then(|| text.to_string())
}

/// The code span an item's bullet starts with: its signature, or its name
/// when the signature is too long to read in a list.
fn label(symbol: &Symbol) -> String {
    let mut signature = symbol.signature.trim();
    // A constant's signature runs on into its value.
    if matches!(symbol.kind, SymbolKind::Const | SymbolKind::Static)
        && let Some((head, _)) = signature.split_once(" =")
    {
        signature = head.trim();
    }
    let unfit = signature.is_empty()
        || signature.contains('`')
        || signature.chars().count() > MAX_SIGNATURE
        || is_line_ref(signature);
    if unfit {
        format!("`{}`", symbol.name)
    } else {
        format!("`{signature}`")
    }
}

/// What the code alone says of an item that has no comment: how visible it
/// is, what kind of thing, what it returns, and who uses it.
fn facts(ctx: &RenderCtx, symbol: &Symbol) -> String {
    let repo = ctx.repo;
    let visibility = match symbol.visibility {
        Visibility::Public => "public",
        Visibility::Crate => "internal",
        Visibility::Private => "private",
    };
    let kind = match symbol.kind {
        SymbolKind::Const => "constant".to_string(),
        kind => kind.as_str().replace('_', " "),
    };
    let callable = matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method);
    let mut text = format!("{visibility} {kind}");
    if callable
        && let Some(returns) = return_type(&symbol.signature, Language::from_path(&symbol.file))
    {
        text.push_str(&format!(", returns `{returns}`"));
    }
    let mut users: Vec<String> = repo
        .incoming
        .get(symbol.qualified.as_str())
        .into_iter()
        .flatten()
        .filter(|e| matches!(e.kind, EdgeKind::Calls | EdgeKind::References))
        .filter(|e| e.confidence != Confidence::Low)
        .filter_map(|e| repo.symbols.get(e.src.as_str()).copied())
        .filter(|user| !repo.is_test(user))
        .map(short_name)
        .collect();
    users.sort();
    users.dedup();
    if !users.is_empty() {
        let more = users.len().saturating_sub(MAX_USERS);
        let listed: Vec<String> = users
            .iter()
            .take(MAX_USERS)
            .map(|name| format!("`{name}`"))
            .collect();
        let verb = if callable { "called by" } else { "used by" };
        text.push_str(&format!("; {verb} {}", listed.join(", ")));
        if more > 0 {
            text.push_str(&format!(" and {more} more"));
        }
    }
    text.push('.');
    text
}

/// The items directly inside `parent`, one bullet each, in source order:
/// every item that is not private, and the private ones `walk` names. An
/// item with a comment gives the whole comment; one without gives what the
/// code says of it. Past [`MAX_ITEMS`] the rest is counted in a last line.
pub(crate) fn items(ctx: &RenderCtx, parent: &str, walk: &Walk) -> String {
    let repo = ctx.repo;
    let all: Vec<&Symbol> = repo
        .children_of(parent)
        .iter()
        .copied()
        .filter(|s| s.kind != SymbolKind::Module && !repo.is_test(s))
        .collect();
    let named = walk.names();
    let weight = |s: &Symbol| {
        (
            s.visibility == Visibility::Public,
            s.visibility != Visibility::Private,
            named.contains(s.qualified.as_str()),
        )
    };
    let by_weight = |a: &&Symbol, b: &&Symbol| {
        weight(b)
            .cmp(&weight(a))
            .then(
                repo.subtree_rank(&b.qualified)
                    .total_cmp(&repo.subtree_rank(&a.qualified)),
            )
            .then(a.qualified.cmp(&b.qualified))
    };
    let mut chosen: Vec<&Symbol> = all
        .iter()
        .copied()
        .filter(|s| {
            s.visibility != Visibility::Private
                || named.contains(s.qualified.as_str())
                // Someone wrote the comment to be read.
                || !s.doc.trim().is_empty()
        })
        .collect();
    // A module of private items still shows the ones that matter most.
    if chosen.len() < 3 {
        let mut ranked = all.clone();
        ranked.sort_by(by_weight);
        ranked.truncate(FEW_ITEMS);
        chosen = ranked;
    }
    chosen.sort_by(by_weight);
    let fitting = chosen.len();
    chosen.truncate(MAX_ITEMS);
    let left_out = all.len() - chosen.len();
    // Unless the limit cut the list, what is left out is private, has no
    // comment and plays no part in the walk-through.
    let only_private = (3..=MAX_ITEMS).contains(&fitting);
    chosen.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.start_line.cmp(&b.start_line))
            .then(a.qualified.cmp(&b.qualified))
    });

    let mut lines: Vec<String> = chosen
        .iter()
        .map(|s| {
            // An item with a note of its own is explained there.
            if let Some(link) = ctx.link(&format!("type:{}", s.qualified)) {
                let summary = first_sentence(&s.doc);
                return if summary.is_empty() {
                    format!("- {link} — {}", s.kind.as_str().replace('_', " "))
                } else {
                    format!("- {link} — {}", lower_first(&summary))
                };
            }
            let text = item_text(&s.doc, Language::from_path(&s.file));
            if text.is_empty() {
                format!("- {} — {}", label(s), facts(ctx, s))
            } else {
                format!("- {} — {}", label(s), lower_first(&text))
            }
        })
        .collect();
    if left_out > 0 && !lines.is_empty() {
        lines.push(String::new());
        let items = if left_out == 1 { "item" } else { "items" };
        lines.push(if only_private {
            format!("And {left_out} more private {items} with no comment.")
        } else {
            format!("And {left_out} more {items}.")
        });
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_type_a_function_returns() {
        let rust = |signature| return_type(signature, Language::Rust);
        assert_eq!(
            rust("fn read(key: &str) -> Option<String>").as_deref(),
            Some("Option<String>")
        );
        // An arrow inside a parameter is not the function's own.
        assert_eq!(
            rust(
                "pub fn each<F: Fn(u8) -> u8>(f: F, g: impl Fn() -> bool) -> Vec<u8> where F: Send"
            )
            .as_deref(),
            Some("Vec<u8>")
        );
        assert_eq!(rust("fn pair() -> (u8, u8)").as_deref(), Some("(u8, u8)"));
        assert_eq!(rust("fn run(x: u8)"), None);
        assert_eq!(rust("fn unit() -> ()"), None);
        assert_eq!(
            return_type("def load(path: str) -> dict[str, int]:", Language::Python).as_deref(),
            Some("dict[str, int]")
        );
        assert_eq!(
            return_type("def run(self) -> None:", Language::Python),
            None
        );
        assert_eq!(
            return_type("function loud(s: string): string", Language::TypeScript).as_deref(),
            Some("string")
        );
        assert_eq!(return_type("name(): void", Language::TypeScript), None);
        assert_eq!(return_type("function f(x)", Language::JavaScript), None);
        assert_eq!(return_type("public String name()", Language::Java), None);
    }

    #[test]
    fn lists_calls_in_order() {
        let here = |name: &str| (format!("`{name}`"), None);
        let there = |name: &str, note: &str| (format!("`{name}`"), Some(format!("[[{note}]]")));
        let items = [here("a"), here("b"), here("c")];
        assert_eq!(in_order(&items, 0), "`a`, then `b`, then `c`");
        assert_eq!(in_order(&items[..1], 0), "`a`");
        assert_eq!(in_order(&items[..2], 3), "`a`, then `b`, and 3 more");
        // Neighbours in one note name it once.
        let mixed = [
            there("open", "Store"),
            here("check"),
            there("read", "Store"),
            there("parse", "Store"),
            there("log", "Logs"),
            there("x", "Io"),
            there("y", "Io"),
            there("z", "Io"),
        ];
        assert_eq!(
            in_order(&mixed, 0),
            "`open` in [[Store]], then `check`, then `read`, then `parse` (both in [[Store]]), then `log` in [[Logs]], then `x`, then `y`, then `z` (all in [[Io]])"
        );
    }
}
