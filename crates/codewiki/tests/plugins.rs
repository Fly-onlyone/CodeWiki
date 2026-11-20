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
}
