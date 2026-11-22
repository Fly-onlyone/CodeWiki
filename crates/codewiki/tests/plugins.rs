//! The agent plugins in `plugins/` and the marketplaces that list them: what a
//! version bump or a change to the note lint could leave behind.

use std::path::{Path, PathBuf};

use cw_vault::NoteKind;
use serde_json::Value;

const CLAUDE: &str = "plugins/claude-code";
const CODEX: &str = "plugins/codex";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A text file of the repository, with line endings as git may check them out.
fn read(rel: &str) -> String {
    let path = repo().join(rel);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .replace("\r\n", "\n")
}

fn json(rel: &str) -> Value {
    serde_json::from_str(&read(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// The value of `key` in a skill's YAML frontmatter.
fn frontmatter(text: &str, key: &str) -> Option<String> {
    let body = text.strip_prefix("---\n")?;
    let end = body.find("\n---")?;
    body[..end].lines().find_map(|line| {
        line.strip_prefix(key)
            .and_then(|rest| rest.strip_prefix(':'))
            .map(|value| value.trim().to_string())
    })
}

#[test]
fn plugin_versions_follow_the_workspace() {
    for manifest in [
        format!("{CLAUDE}/.claude-plugin/plugin.json"),
        format!("{CODEX}/.codex-plugin/plugin.json"),
    ] {
        let manifest_json = json(&manifest);
        assert_eq!(manifest_json["name"], "codewiki", "{manifest}");
        assert_eq!(
            manifest_json["version"],
            env!("CARGO_PKG_VERSION"),
            "{manifest}"
        );
    }
}

#[test]
fn both_plugins_start_the_same_server_and_hook() {
    let codex = json(&format!("{CODEX}/.mcp.json"));
    assert_eq!(json(&format!("{CLAUDE}/.mcp.json")), codex);
    let server = &codex["mcpServers"]["codewiki"];
    assert_eq!(server["command"], "codewiki-mcp");
    // Without `cwd` the agent starts the server in the session's folder, which
    // `codewiki-mcp` takes as the default repository. Codex's portable format
    // (`type: stdio`) would start it in the plugin's own folder instead.
    assert!(server.get("cwd").is_none() && server.get("type").is_none());

    for plugin in [CLAUDE, CODEX] {
        let hooks = json(&format!("{plugin}/hooks/hooks.json"));
        let groups = hooks["hooks"]["SessionStart"].as_array().expect(plugin);
        let commands: Vec<&str> = groups
            .iter()
            .flat_map(|group| group["hooks"].as_array().into_iter().flatten())
            .filter_map(|hook| hook["command"].as_str())
            .collect();
        assert_eq!(commands, ["codewiki hook session-start"], "{plugin}");
    }
}

#[test]
fn codex_manifest_points_at_files_that_exist() {
    let root = repo().join(CODEX);
    let manifest = json(&format!("{CODEX}/.codex-plugin/plugin.json"));
    let interface = &manifest["interface"];
    for value in [
        &manifest["skills"],
        &manifest["mcpServers"],
        &manifest["hooks"],
        &interface["logo"],
        &interface["composerIcon"],
    ] {
        let path = value.as_str().expect("a path");
        assert!(path.starts_with("./"), "`{path}` must start with ./");
        assert!(root.join(path).exists(), "`{path}` is missing");
    }
    // Codex's own limits for what it shows in its plugin list.
    let short = interface["shortDescription"].as_str().unwrap();
    assert!(short.chars().count() <= 30, "{short}");
    let prompts = interface["defaultPrompt"].as_array().unwrap();
    assert!(prompts.len() <= 3);
    for prompt in prompts {
        assert!(prompt.as_str().unwrap().chars().count() <= 128, "{prompt}");
    }
}

#[test]
fn every_marketplace_lists_a_plugin_that_exists() {
    // (marketplace file, marketplace root, manifest the plugin must have)
    let marketplaces = [
        (
            ".claude-plugin/marketplace.json",
            "",
            ".claude-plugin/plugin.json",
        ),
        (
            "plugins/.claude-plugin/marketplace.json",
            "plugins",
            ".claude-plugin/plugin.json",
        ),
        (
            ".agents/plugins/marketplace.json",
            "",
            ".codex-plugin/plugin.json",
        ),
        (
            "plugins/.agents/plugins/marketplace.json",
            "plugins",
            ".codex-plugin/plugin.json",
        ),
    ];
    for (file, root, manifest) in marketplaces {
        let marketplace = json(file);
        assert_eq!(marketplace["name"], "codewiki", "{file}");
        let plugins = marketplace["plugins"].as_array().expect(file);
        assert_eq!(plugins.len(), 1, "{file}");
        assert_eq!(plugins[0]["name"], "codewiki", "{file}");
        // Claude Code takes a path; Codex a `local` source with a path. Both
        // are relative to the marketplace root.
        let source = &plugins[0]["source"];
        let path = source
            .as_str()
            .or_else(|| source["path"].as_str())
            .expect(file);
        assert!(path.starts_with("./"), "{file}: {path}");
        assert!(
            repo().join(root).join(path).join(manifest).is_file(),
            "{file}: {path} has no {manifest}"
        );
    }
}

#[test]
fn codex_skills_are_named_after_their_folders() {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(repo().join(CODEX).join("skills")).unwrap() {
        let folder = entry.unwrap().file_name().to_string_lossy().into_owned();
        let text = read(&format!("{CODEX}/skills/{folder}/SKILL.md"));
        assert_eq!(
            frontmatter(&text, "name").as_deref(),
            Some(folder.as_str()),
            "{folder}"
        );
        assert!(
            frontmatter(&text, "description").is_some_and(|d| !d.is_empty()),
            "{folder} has no description"
        );
        names.push(folder);
    }
    names.sort();
    assert_eq!(names, ["using-codewiki", "wiki-note-style"]);
}

/// Whether notes of `kind` follow a section template. A new kind does not
/// compile here until it is sorted in, and then the skill must list it.
fn has_template(kind: NoteKind) -> bool {
    use NoteKind::*;
    match kind {
        Home | Index => false,
        Flow | Concept | Pattern | Integration | Operations | Glossary | Decision => true,
    }
}

#[test]
fn note_style_skill_follows_the_lint() {
    use NoteKind::*;
    let skill = read(&format!("{CODEX}/skills/wiki-note-style/SKILL.md"));
    for kind in [
        Home,
        Index,
        Flow,
        Concept,
        Pattern,
        Integration,
        Operations,
        Glossary,
        Decision,
    ] {
        assert_eq!(has_template(kind), !kind.sections().is_empty(), "{kind:?}");
        if has_template(kind) {
            let template = kind.sections().join(", ");
            assert!(
                skill.contains(&template),
                "the skill lacks the {kind:?} template: {template}"
            );
        }
    }
    let length = format!(
        "{} to {} words",
        cw_vault::lint::MIN_WORDS,
        cw_vault::lint::MAX_WORDS
    );
    assert!(skill.contains(&length), "the skill lacks `{length}`");
    // The kinds with a limit of their own.
    for (kind, limit) in [
        (Concept, cw_vault::lint::CONCEPT_MAX_WORDS),
        (Integration, cw_vault::lint::INTEGRATION_MAX_WORDS),
    ] {
        assert_eq!(cw_vault::lint::max_words(kind), limit);
        let length = format!("{limit} words");
        assert!(skill.contains(&length), "the skill lacks `{length}`");
    }
    assert_eq!(cw_vault::lint::max_words(Flow), cw_vault::lint::MAX_WORDS);
}

/// The fenced blocks of `text` that are marked with `language`.
fn fenced(text: &str, language: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut open: Option<String> = None;
    for line in text.lines() {
        match open.take() {
            Some(block) if line.trim() == "```" => blocks.push(block),
            Some(block) => open = Some(format!("{block}{line}\n")),
            None if line.trim() == format!("```{language}") => open = Some(String::new()),
            None => {}
        }
    }
    blocks
}

#[test]
fn claude_skills_are_named_after_their_folders() {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(repo().join(CLAUDE).join("skills")).unwrap() {
        let folder = entry.unwrap().file_name().to_string_lossy().into_owned();
        let text = read(&format!("{CLAUDE}/skills/{folder}/SKILL.md"));
        assert_eq!(
            frontmatter(&text, "name").as_deref(),
            Some(folder.as_str()),
            "{folder}"
        );
        assert!(
            frontmatter(&text, "description").is_some_and(|d| !d.is_empty()),
            "{folder} has no description"
        );
        names.push(folder);
    }
    // Claude Code makes a skill a slash command by its folder's name:
    // `/codewiki:write-notes`.
    assert_eq!(names, ["write-notes"]);
}

#[test]
fn write_notes_skill_follows_the_tools_and_the_limits() {
    use NoteKind::*;
    let skill = read(&format!("{CLAUDE}/skills/write-notes/SKILL.md"));
    // Every tool it tells an agent to call is one the bridge has.
    let bridge = read("crates/codewiki-mcp/src/main.rs");
    for tool in [
        "sync_wiki",
        "list_waiting_notes",
        "get_note_brief",
        "submit_note",
    ] {
        assert!(skill.contains(&format!("`{tool}`")), "{tool}");
        assert!(
            bridge.contains(&format!("async fn {tool}(")),
            "the bridge has no tool {tool}"
        );
    }
    // The arguments of those tools that the skill names.
    for argument in ["folder", "files", "limit", "base", "read", "model"] {
        assert!(skill.contains(&format!("`{argument}`")), "{argument}");
        assert!(
            bridge.contains(&format!("    {argument}: ")),
            "no tool takes {argument}"
        );
    }
    // The words of an answer that the skill tells an agent to act on.
    for answer in [
        "Not saved. Fix each of these",
        "Call get_note_brief again",
        "Call sync_wiki",
        "Not written",
        "STOP",
    ] {
        assert!(skill.contains(answer), "the skill lacks `{answer}`");
        assert!(bridge.contains(answer), "the bridge never says `{answer}`");
    }

    // Each section an agent writes, of every kind of note, is described.
    for kind in [Flow, Concept, Pattern, Integration, Operations] {
        for heading in kind.agent_sections() {
            assert!(
                skill.contains(&format!("| {heading} |")),
                "{kind:?}: the skill does not describe `{heading}`"
            );
        }
    }
    // The numbers it states are the ones the backend holds an agent to.
    use cw_wiki::explanations::{DEFAULT_LIMIT, MAX_LIMIT, MAX_SECTION_WORDS};
    assert!(skill.contains(&format!("At most {MAX_SECTION_WORDS} words in a section")));
    assert!(skill.contains(&format!("{DEFAULT_LIMIT} unless a number is given")));
    assert!(skill.contains(&format!("Never more than {MAX_LIMIT} in one run")));
    // It sends the agent to the reference beside it, which exists.
    assert!(skill.contains("`pictures.md`"));
    assert!(
        repo()
            .join(CLAUDE)
            .join("skills/write-notes/pictures.md")
            .is_file()
    );
}

#[test]
fn skills_stop_at_the_agents_refusal() {
    // The bridge's refusal and the skills use one phrase, so an agent that
    // reads the skill knows the refusal when it sees it.
    let phrase = "CodeWiki is turned off for this project";
    let order = "stop using CodeWiki tools for the rest of this session";
    let refusal = cw_core::agents_off::refusal(Path::new("x"));
    assert!(
        refusal.starts_with(phrase),
        "the bridge never says `{phrase}`"
    );
    for skill in [
        format!("{CLAUDE}/skills/write-notes/SKILL.md"),
        format!("{CODEX}/skills/using-codewiki/SKILL.md"),
        format!("{CODEX}/skills/wiki-note-style/SKILL.md"),
    ] {
        let text = read(&skill);
        assert!(text.contains(phrase), "{skill} lacks `{phrase}`");
        assert!(text.contains(order), "{skill} lacks `{order}`");
    }
}

#[test]
fn picture_reference_follows_the_library() {
    use cw_illustrate::{
        MAX_CALLOUT_TEXT, MAX_CALLOUTS, MAX_PICTURES, MAX_SOURCE, MAX_STEP_TEXT, MAX_STEP_TITLE,
        MAX_STEPS, MIN_STEPS, TONES,
    };
    let reference = read(&format!("{CLAUDE}/skills/write-notes/pictures.md"));
    let library = cw_illustrate::library();
    let mut components = 0;
    for name in library.names() {
        assert!(
            reference.contains(&format!("| `{name}` |")),
            "the reference does not list {name}"
        );
        components += 1;
    }
    assert!(reference.contains("## The four components"));
    assert_eq!(components, 4);
    for tone in TONES {
        assert!(reference.contains(&format!("`{tone}`")), "{tone}");
    }
    for limit in [
        format!("at most {MAX_PICTURES} pictures"),
        format!("at most {MAX_CALLOUTS} `Callout`s"),
        format!("{MIN_STEPS} to {MAX_STEPS} steps"),
        format!("title has at most {MAX_STEP_TITLE} characters, its text at most {MAX_STEP_TEXT}"),
        format!(
            "text has at most {MAX_CALLOUT_TEXT} characters, its title at most {MAX_STEP_TITLE}"
        ),
        format!("at most {MAX_SOURCE} bytes"),
    ] {
        assert!(reference.contains(&limit), "the reference lacks `{limit}`");
    }
    // An example that would be refused teaches the wrong thing.
    let examples = fenced(&reference, "openui");
    assert_eq!(examples.len(), 2);
    for example in examples {
        assert_eq!(
            cw_illustrate::review(&example),
            Vec::<String>::new(),
            "{example}"
        );
    }
}
