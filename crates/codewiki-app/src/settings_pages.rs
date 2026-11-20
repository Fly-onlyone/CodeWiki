//! The pages of the Settings: one for each [`Category`], built from what the
//! app knows and from what the backend says about its own settings
//! (`GET /v1/settings`). Each is a [`Doc`], so a test reads it without a
//! window.
//!
//! A setting the backend keeps can be out of the user's hands here for one
//! of three reasons, and its control then says which: an environment
//! variable of the backend sets it; the backend runs on another machine; or
//! the backend cannot save a key at all.

use std::collections::{HashMap, HashSet};

use cw_client::Origin;
use serde_json::Value;

use crate::doc::{
    Action, Block, Cell, Choice, Column, Doc, Press, SecretState, Span, Table, code, text,
};
use crate::icons;
use crate::motion::MotionPref;
use crate::mui::feedback::Tone;
use crate::settings::{Category, FieldId, Note};
use crate::update;

/// Where a test of a provider stands.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Tested {
    #[default]
    Not,
    Running,
    /// What came back: whether it answered, and the line to show.
    Done {
        ok: bool,
        text: String,
    },
}

/// Everything the pages read.
pub struct Model<'a> {
    pub theme: &'a str,
    /// Every theme by name, in the order of the theme menu.
    pub themes: &'a [String],
    pub motion: MotionPref,
    /// What the choice in `motion` comes to.
    pub animations_on: bool,
    /// The backend's settings, once they have arrived.
    pub settings: Option<&'a Value>,
    /// Why they could not be read or changed.
    pub settings_error: Option<&'a str>,
    /// The news beside each field.
    pub notes: &'a HashMap<FieldId, Note>,
    /// The saved secrets whose input is open to take another in their place.
    pub typing: &'a HashSet<FieldId>,
    pub llm_test: &'a Tested,
    pub embed_test: &'a Tested,
    /// The backend in use, and how the app came to it.
    pub backend_url: Option<&'a str>,
    pub origin: Option<Origin>,
    /// The address `CODEWIKI_URL` names, when it is set.
    pub env_url: Option<&'a str>,
    /// The backend saved in the app's settings, in use or not.
    pub saved_remote: Option<&'a cw_client::Remote>,
    /// The news about the last try to connect to a saved backend.
    pub connect: Option<&'a Note>,
    /// `(name, folder)` of each registered repository.
    pub repos: &'a [(String, String)],
    pub update: &'a update::State,
    /// Whether the app looks for updates by itself.
    pub auto_update: bool,
    /// Whether Windows starts the app at sign-in; `None` for a copy that is
    /// not installed, which cannot be started that way.
    pub autostart: Option<bool>,
    /// Whether the app starts on the wiki map of the repository it opens.
    pub open_last: bool,
    /// The level of the app's log, by its name, and whether `CODEWIKI_LOG`
    /// fixes it.
    pub log_level: &'a str,
    pub log_level_locked: bool,
    pub logs_dir: &'a str,
    pub data_dir: &'a str,
}

impl Model<'_> {
    /// Whether the backend runs somewhere else than on this machine.
    fn remote(&self) -> bool {
        matches!(self.origin, Some(Origin::Env | Origin::Saved))
    }
}

/// The page of `category`.
pub fn page(category: Category, model: &Model) -> Doc {
    let mut doc = Doc::new(category.label());
    let blocks = match category {
        Category::Appearance => appearance(model),
        Category::AiModel => ai_model(model),
        Category::Search => search(model),
        Category::Backend => backend(model),
        Category::Repositories => repositories(model),
        Category::Updates => updates(model),
        Category::Keyboard => keyboard(),
        Category::Diagnostics => diagnostics(model),
    };
    doc.blocks = blocks;
    doc
}

fn card(title: &str, body: Vec<Block>) -> Block {
    Block::Card {
        title: title.into(),
        chips: Vec::new(),
        body,
    }
}

fn appearance(model: &Model) -> Vec<Block> {
    vec![
        card(
            "Theme",
            vec![
                Block::Select {
                    label: "Theme".into(),
                    help: Some("Applies at once".into()),
                    options: model
                        .themes
                        .iter()
                        .map(|name| Choice {
                            label: name.clone(),
                            action: Action::Theme(name.clone()),
                            selected: name == model.theme,
                            // The same icon the theme has in the toolbar's
                            // theme menu.
                            icon: Some(icons::for_theme(name)),
                        })
                        .collect(),
                    locked: None,
                    note: None,
                },
                Block::Text(vec![
                    text("Your own themes load from the "),
                    code("themes"),
                    text(" folder in "),
                    code("%APPDATA%\\CodeWiki"),
                    text(", in the same JSON format."),
                ]),
            ],
        ),
        card(
            "Motion",
            vec![
                Block::Choices {
                    label: "Animations".into(),
                    choices: MotionPref::ALL
                        .into_iter()
                        .map(|pref| Choice {
                            label: pref.label().into(),
                            action: Action::Motion(pref),
                            selected: pref == model.motion,
                            icon: None,
                        })
                        .collect(),
                },
                Block::Text(vec![text(format!(
                    "Animations are {}.",
                    if model.animations_on { "on" } else { "off" }
                ))]),
            ],
        ),
        startup(model),
    ]
}

/// Where the app starts, whether Windows starts it at sign-in, and what
/// closing the window does: the last two belong together, since an app
/// started that way is seen only as its tray icon.
fn startup(model: &Model) -> Block {
    card(
        "Startup",
        vec![
            Block::Switch {
                label: "Open the last repository".into(),
                help: Some("At start, on its wiki map instead of Home".into()),
                on: model.open_last,
                action: Action::OpenLast(!model.open_last),
                locked: None,
            },
            Block::Switch {
                label: "Start with Windows".into(),
                help: Some("In the tray, with no window".into()),
                on: model.autostart == Some(true),
                action: Action::Autostart(model.autostart != Some(true)),
                // A build from the sources would leave Windows a program in
                // `target\`, which the next build replaces.
                locked: model
                    .autostart
                    .is_none()
                    .then(|| "Only an installed copy can start with Windows".into()),
            },
            Block::Text(vec![text(
                "Closing the window keeps CodeWiki running in the tray, so that agents find its backend at once. Click the tray icon to open the window again; right-click it to quit.",
            )]),
        ],
    )
}

/// What a page that needs the backend's settings shows without them.
fn without_settings(model: &Model) -> Vec<Block> {
    match model.settings_error {
        Some(error) => vec![Block::Alert {
            tone: Tone::Error,
            text: format!("The backend's settings could not be read: {error}"),
            button: None,
        }],
        None => vec![Block::Empty("Reading the backend's settings…".into())],
    }
}

/// Why the setting `entry` describes cannot be changed here, when it cannot.
fn lock(model: &Model, settings: &Value, entry: &Value) -> Option<String> {
    if model.remote() {
        return Some(format!(
            "Set on the backend at {}",
            model.backend_url.unwrap_or("another machine")
        ));
    }
    if settings["writable"] == false {
        return Some("This backend takes its settings from its own environment".into());
    }
    let from_env = entry["source"] == "env" || entry["state"] == "env";
    from_env.then(|| {
        format!(
            "Set by {} in the backend's environment",
            entry["env"].as_str().unwrap_or("a variable")
        )
    })
}

fn entry(settings: &Value, id: FieldId) -> &Value {
    match id.place() {
        Some((section, field)) => &settings[section][field],
        None => &Value::Null,
    }
}

fn field(model: &Model, settings: &Value, id: FieldId, label: &str, help: &str) -> Block {
    Block::Field {
        id,
        label: label.into(),
        help: (!help.is_empty()).then(|| help.to_string()),
        locked: lock(model, settings, entry(settings, id)),
        note: model.notes.get(&id).cloned(),
    }
}

fn secret(model: &Model, settings: &Value, id: FieldId, label: &str, help: &str) -> Block {
    let entry = entry(settings, id);
    let locked = lock(model, settings, entry).or_else(|| {
        // A backend that is not on Windows has nowhere safe to keep a key.
        (settings["secrets"] == "env_only").then(|| {
            format!(
                "This backend cannot save a key; set {} in its environment",
                entry["env"].as_str().unwrap_or("the variable")
            )
        })
    });
    let state = if model.typing.contains(&id) {
        SecretState::Typing
    } else {
        match entry["state"].as_str() {
            Some("saved") => SecretState::Saved {
                hint: entry["hint"].as_str().unwrap_or("").to_string(),
            },
            Some("unreadable") => SecretState::Unreadable,
            _ => SecretState::Empty,
        }
    };
    Block::Secret {
        id,
        label: label.into(),
        help: (!help.is_empty()).then(|| help.to_string()),
        state,
        locked,
        note: model.notes.get(&id).cloned(),
    }
}

/// A select over the `options` the backend lists for `section.provider`.
fn provider(
    model: &Model,
    settings: &Value,
    section: &str,
    label: &str,
    help: &str,
    name: fn(&str) -> &str,
) -> Block {
    let entry = &settings[section]["provider"];
    let chosen = entry["value"].as_str().unwrap_or("");
    Block::Select {
        label: label.into(),
        help: Some(help.into()),
        options: entry["options"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(|option| Choice {
                label: name(option).to_string(),
                action: Action::SetSetting {
                    section: section.into(),
                    field: "provider".into(),
                    value: Some(option.into()),
                },
                selected: option == chosen,
                icon: None,
            })
            .collect(),
        locked: lock(model, settings, entry),
        note: None,
    }
}

fn llm_name(provider: &str) -> &str {
    match provider {
        "none" => "None: notes keep their structural draft",
        "anthropic" => "Anthropic",
        "openai" => "OpenAI-compatible",
        "ollama" => "Ollama",
        "claude-cli" => "Claude Code (your own login)",
        "codex-cli" => "Codex (your own login)",
        other => other,
    }
}

fn embedder_name(provider: &str) -> &str {
    match provider {
        "local" => "Built in: no model needed",
        "ollama" => "Ollama",
        "openai" => "OpenAI-compatible",
        "none" => "Off: full-text search only",
        other => other,
    }
}

/// What the settings of `section` select, or why they select nothing usable.
fn in_use(settings: &Value, section: &str, nothing: &str) -> Block {
    let chosen = &settings[section];
    match (chosen["problem"].as_str(), chosen["name"].as_str()) {
        (Some(problem), _) => Block::Alert {
            tone: Tone::Warning,
            text: problem.to_string(),
            button: None,
        },
        (None, Some(name)) => Block::Text(vec![text("In use: "), code(name), text(".")]),
        (None, None) => Block::Text(vec![text(nothing)]),
    }
}

/// The line a test left, when one ran.
fn tested(test: &Tested) -> Option<Block> {
    let (tone, text) = match test {
        Tested::Not => return None,
        Tested::Running => (Tone::Info, "Testing…".to_string()),
        Tested::Done { ok: true, text } => (Tone::Success, text.clone()),
        Tested::Done { ok: false, text } => (Tone::Error, text.clone()),
    };
    Some(Block::Alert {
        tone,
        text,
        button: None,
    })
}

/// Said at the top of a page whose settings belong to another machine.
fn remote_notice(model: &Model) -> Option<Block> {
    model.remote().then(|| Block::Alert {
        tone: Tone::Info,
        text: format!(
            "The backend at {} runs on another machine and takes these settings from its own environment variables. They are shown here and changed there.",
            model.backend_url.unwrap_or("that address")
        ),
        button: None,
    })
}

fn ai_model(model: &Model) -> Vec<Block> {
    let Some(settings) = model.settings else {
        return without_settings(model);
    };
    let mut provider_card = vec![
        provider(
            model,
            settings,
            "llm",
            "Provider",
            "Writes the prose of notes and answers questions",
            llm_name,
        ),
        field(
            model,
            settings,
            FieldId::LlmModel,
            "Model",
            "Anthropic has a default; the others need a name",
        ),
        in_use(settings, "llm", "No provider is chosen."),
        Block::Buttons(vec![
            Press::new("Test", Action::TestLlm).disabled(*model.llm_test == Tested::Running),
        ]),
    ];
    provider_card.extend(tested(model.llm_test));
    let mut blocks: Vec<Block> = remote_notice(model).into_iter().collect();
    blocks.push(card("Provider", provider_card));
    blocks.push(card(
        "Keys and addresses",
        vec![
            secret(
                model,
                settings,
                FieldId::AnthropicKey,
                "Anthropic API key",
                "",
            ),
            secret(
                model,
                settings,
                FieldId::OpenAiKey,
                "OpenAI API key",
                "Also used by the OpenAI embedder",
            ),
            field(
                model,
                settings,
                FieldId::OpenAiBaseUrl,
                "OpenAI address",
                "Any server that speaks the OpenAI protocol",
            ),
            field(
                model,
                settings,
                FieldId::OllamaHost,
                "Ollama address",
                "Also used by the Ollama embedder",
            ),
            Block::Text(vec![text(
                "A key is stored encrypted for your Windows user and is never shown again.",
            )]),
        ],
    ));
    blocks
}

fn search(model: &Model) -> Vec<Block> {
    let Some(settings) = model.settings else {
        return without_settings(model);
    };
    let mut embedder = vec![
        provider(
            model,
            settings,
            "embed",
            "Embedder",
            "Finds notes and symbols by meaning",
            embedder_name,
        ),
        field(
            model,
            settings,
            FieldId::EmbedModel,
            "Model",
            "For Ollama and OpenAI; empty takes their default",
        ),
        in_use(settings, "embed", "Semantic search is off."),
        Block::Text(vec![text(
            "A new embedder serves the next search. The stored vectors are rebuilt at the next Sync; until then search is full-text alone.",
        )]),
        Block::Buttons(vec![
            Press::new("Test", Action::TestEmbedder).disabled(*model.embed_test == Tested::Running),
            Press::new("Sync now", Action::Sync),
        ]),
    ];
    embedder.extend(tested(model.embed_test));
    let mut blocks: Vec<Block> = remote_notice(model).into_iter().collect();
    blocks.push(card("Embedder", embedder));
    blocks.push(Block::Text(vec![
        text("The OpenAI key and address and the Ollama address are under "),
        Span::Link {
            label: "AI model".into(),
            code: false,
            action: Action::Settings(Some(Category::AiModel)),
        },
        text("."),
    ]));
    blocks
}

fn backend(model: &Model) -> Vec<Block> {
    let how = match model.origin {
        Some(Origin::Local) => "It runs on this machine and stops when CodeWiki closes.",
        Some(Origin::Env) => "CODEWIKI_URL names it. It is left running when CodeWiki closes.",
        Some(Origin::Saved) => {
            "It is the backend saved below. It is left running when CodeWiki closes."
        }
        None => "",
    };
    let connection = match model.backend_url {
        Some(url) => vec![
            Block::Text(vec![text("Connected to "), code(url), text(".")]),
            Block::Text(vec![text(how)]),
        ],
        None => vec![Block::Text(vec![text("Not connected to a backend.")])],
    };

    // The variable wins, so the fields would change nothing.
    let locked = model
        .env_url
        .map(|_| "CODEWIKI_URL is set and wins over an address saved here".to_string());
    let saved = model.saved_remote;
    let token_help = match saved {
        Some(remote) if remote.token_hint.is_empty() => {
            "One is saved; leave empty to keep it".into()
        }
        Some(remote) => format!(
            "One is saved, ending in {}; leave empty to keep it",
            remote.token_hint
        ),
        None => "Stored encrypted for your Windows user".to_string(),
    };
    let mut other = vec![
        Block::Text(vec![text(
            "Use a backend in Docker or on another machine. The app and an agent's MCP bridge on this machine then read the same wikis.",
        )]),
        Block::Field {
            id: FieldId::RemoteUrl,
            label: "Address".into(),
            help: None,
            locked: locked.clone(),
            note: None,
        },
        Block::Field {
            id: FieldId::RemoteToken,
            label: "Token".into(),
            help: Some(token_help),
            locked: locked.clone(),
            note: None,
        },
    ];
    let working = model.connect == Some(&Note::Working);
    let mut buttons = vec![
        Press::new("Connect", Action::ConnectRemote)
            .primary()
            .disabled(locked.is_some() || working),
    ];
    if saved.is_some_and(|remote| remote.in_use) {
        buttons.push(Press::new("Use local backend", Action::UseLocalBackend).disabled(working));
    }
    if saved.is_some() {
        buttons.push(Press::new("Forget", Action::ForgetRemote).disabled(working));
    }
    other.push(Block::Buttons(buttons));
    other.extend(model.connect.map(|note| {
        let (tone, text) = match note {
            Note::Working => (Tone::Info, "Checking the backend…".to_string()),
            Note::Saved => (Tone::Success, "Connected.".to_string()),
            Note::Error(error) => (Tone::Error, error.clone()),
        };
        Block::Alert {
            tone,
            text,
            button: None,
        }
    }));
    vec![
        card("Connection", connection),
        card("Backend on another machine", other),
    ]
}

fn repositories(model: &Model) -> Vec<Block> {
    if model.repos.is_empty() {
        return vec![Block::Empty(
            "No repository is registered. Open a folder from Home.".into(),
        )];
    }
    let mut table = Table::new([
        Column::new("Repository").weight(0.6),
        Column::new("Folder").weight(1.6),
    ]);
    for (name, path) in model.repos {
        table.row([Cell::text(name), code(path).into()]);
    }
    vec![Block::Table(table)]
}

/// The version, what the last check found, and whether the app checks by
/// itself.
fn updates(model: &Model) -> Vec<Block> {
    use update::State;
    let button = match model.update {
        State::Available(_) => Some(("Install and restart".to_string(), Action::InstallUpdate)),
        State::Idle | State::UpToDate | State::Unavailable(_) => {
            Some(("Check now".to_string(), Action::CheckUpdates))
        }
        State::NotInstalled | State::Checking | State::Installing(_) | State::Installed(_) => None,
    };
    let mut body = vec![Block::Setting {
        label: "Version".into(),
        value: model.update.describe(),
        button,
    }];
    // A development build does not update itself.
    if *model.update != State::NotInstalled {
        body.push(Block::Switch {
            label: "Daily check".into(),
            // Says how often, not when: the check runs at the app's start
            // today, and may come to run while the app stays open.
            help: Some("Looks for a newer release once a day".into()),
            on: model.auto_update,
            action: Action::AutoUpdate(!model.auto_update),
            locked: None,
        });
    }
    vec![card("Updates", body)]
}

fn keyboard() -> Vec<Block> {
    let mut keys = Table::new([
        Column::new("Keys").weight(0.6),
        Column::new("Action").weight(1.6),
    ]);
    for (chord, action) in [
        ("Ctrl+K", "Command palette"),
        ("Ctrl+,", "Settings"),
        ("Ctrl+T", "New tab"),
        ("Ctrl+W", "Close tab"),
        ("Ctrl+Tab", "Next tab"),
        ("Alt+Left", "Back"),
        ("Ctrl+B", "Hide or show the sidebar"),
        ("Ctrl+Alt+B", "Hide or show the details pane"),
        ("Ctrl+S", "Save the note being edited"),
    ] {
        keys.row([Cell::chip(chord, Tone::Neutral), Cell::text(action)]);
    }
    vec![Block::Table(keys)]
}

/// The levels a log can be set to, with what each keeps.
const LOG_LEVELS: [(&str, &str); 5] = [
    ("error", "Errors only"),
    ("warn", "Warnings and errors"),
    ("info", "Info: the usual amount"),
    ("debug", "Debug: every request and call"),
    ("trace", "Trace: everything"),
];

fn diagnostics(model: &Model) -> Vec<Block> {
    vec![
        card(
            "Logs",
            vec![
                Block::Select {
                    label: "App log level".into(),
                    help: Some("What app.log keeps; the backend's log is not changed".into()),
                    options: LOG_LEVELS
                        .into_iter()
                        .map(|(level, label)| Choice {
                            label: label.into(),
                            action: Action::LogLevel(level.into()),
                            selected: level == model.log_level,
                            icon: None,
                        })
                        .collect(),
                    locked: model
                        .log_level_locked
                        .then(|| "Set by CODEWIKI_LOG".to_string()),
                    note: None,
                },
                Block::Setting {
                    label: "Logs folder".into(),
                    value: model.logs_dir.into(),
                    button: Some(("Open".into(), Action::OpenLogs)),
                },
                Block::Text(vec![
                    text("Attach "),
                    code("app.log"),
                    text(" and "),
                    code("backend.log"),
                    text(" to a bug report; they name the folders you open."),
                ]),
            ],
        ),
        card(
            "Data",
            vec![
                Block::Setting {
                    label: "Data folder".into(),
                    value: model.data_dir.into(),
                    button: Some(("Open".into(), Action::OpenDataFolder)),
                },
                Block::Text(vec![text(
                    "Holds the indexes, the list of repositories and the saved settings. A wiki itself is in its repository.",
                )]),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// What a backend on this machine reports before anything is saved.
    fn defaults() -> Value {
        json!({
            "secrets": "dpapi",
            "writable": true,
            "llm": {
                "provider": { "value": "none", "source": "default", "env": "CODEWIKI_LLM",
                    "options": ["none", "anthropic", "openai", "ollama", "claude-cli", "codex-cli"] },
                "model": { "value": null, "source": "default", "env": "CODEWIKI_MODEL" },
                "name": null, "problem": null,
            },
            "embed": {
                "provider": { "value": "local", "source": "default", "env": "CODEWIKI_EMBED",
                    "options": ["local", "ollama", "openai", "none"] },
                "model": { "value": null, "source": "default", "env": "CODEWIKI_EMBED_MODEL" },
                "name": "local-hash-v1:384", "problem": null,
            },
            "anthropic": { "api_key": { "state": "unset", "hint": "", "env": "ANTHROPIC_API_KEY" } },
            "openai": {
                "api_key": { "state": "unset", "hint": "", "env": "OPENAI_API_KEY" },
                "base_url": { "value": "https://api.openai.com/v1", "source": "default", "env": "OPENAI_BASE_URL" },
            },
            "ollama": { "host": { "value": "http://127.0.0.1:11434", "source": "default", "env": "OLLAMA_HOST" } },
        })
    }

    /// Owns what a [`Model`] borrows.
    struct Fixture {
        themes: Vec<String>,
        settings: Option<Value>,
        error: Option<String>,
        notes: HashMap<FieldId, Note>,
        typing: HashSet<FieldId>,
        llm_test: Tested,
        embed_test: Tested,
        url: Option<String>,
        origin: Option<Origin>,
        env_url: Option<String>,
        saved: Option<cw_client::Remote>,
        connect: Option<Note>,
        repos: Vec<(String, String)>,
        update: update::State,
        auto_update: bool,
        autostart: Option<bool>,
        open_last: bool,
        log_level_locked: bool,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                themes: vec!["CodeWiki Violet".into(), "Nord".into()],
                settings: Some(defaults()),
                error: None,
                notes: HashMap::new(),
                typing: HashSet::new(),
                llm_test: Tested::Not,
                embed_test: Tested::Not,
                url: Some("http://127.0.0.1:7411".into()),
                origin: Some(Origin::Local),
                env_url: None,
                saved: None,
                connect: None,
                repos: vec![("demo".into(), "C:/demo".into())],
                update: update::State::UpToDate,
                auto_update: true,
                autostart: Some(false),
                open_last: false,
                log_level_locked: false,
            }
        }

        fn page(&self, category: Category) -> Doc {
            page(
                category,
                &Model {
                    theme: "Nord",
                    themes: &self.themes,
                    motion: MotionPref::Reduced,
                    animations_on: false,
                    settings: self.settings.as_ref(),
                    settings_error: self.error.as_deref(),
                    notes: &self.notes,
                    typing: &self.typing,
                    llm_test: &self.llm_test,
                    embed_test: &self.embed_test,
                    backend_url: self.url.as_deref(),
                    origin: self.origin,
                    env_url: self.env_url.as_deref(),
                    saved_remote: self.saved.as_ref(),
                    connect: self.connect.as_ref(),
                    repos: &self.repos,
                    update: &self.update,
                    auto_update: self.auto_update,
                    autostart: self.autostart,
                    open_last: self.open_last,
                    log_level: "info",
                    log_level_locked: self.log_level_locked,
                    logs_dir: "C:/data/logs",
                    data_dir: "C:/data",
                },
            )
        }
    }

    fn set(section: &str, value: &str) -> Action {
        Action::SetSetting {
            section: section.into(),
            field: "provider".into(),
            value: Some(value.into()),
        }
    }

    #[test]
    fn every_category_has_a_page_under_its_own_name() {
        let fixture = Fixture::new();
        for category in Category::ALL {
            let doc = fixture.page(category);
            assert_eq!(doc.title, category.label());
            assert!(!doc.blocks.is_empty(), "{category:?}");
        }
    }

    #[test]
    fn the_start_with_windows_is_offered_only_to_an_installed_copy() {
        let mut fixture = Fixture::new();
        let off = fixture.page(Category::Appearance);
        assert!(
            off.words().contains("Start with Windows: off"),
            "{}",
            off.words()
        );
        assert!(off.words().contains("In the tray, with no window"));
        assert!(off.actions().contains(&Action::Autostart(true)));

        fixture.autostart = Some(true);
        let on = fixture.page(Category::Appearance);
        assert!(on.words().contains("Start with Windows: on"));
        assert!(on.actions().contains(&Action::Autostart(false)));

        // A build from the sources says why its switch is locked.
        fixture.autostart = None;
        let dev = fixture.page(Category::Appearance);
        assert!(
            dev.words()
                .contains("Only an installed copy can start with Windows"),
            "{}",
            dev.words()
        );
        // Whoever closes the window is told where the app went.
        for page in [off, on, dev] {
            assert!(
                page.words()
                    .contains("Closing the window keeps CodeWiki running in the tray")
            );
        }
    }

    #[test]
    fn the_last_repository_can_be_opened_at_start() {
        let mut fixture = Fixture::new();
        let off = fixture.page(Category::Appearance);
        assert!(
            off.words().contains("Open the last repository: off"),
            "{}",
            off.words()
        );
        assert!(
            off.words()
                .contains("At start, on its wiki map instead of Home")
        );
        assert!(off.actions().contains(&Action::OpenLast(true)));

        // An app built from the sources can do it too.
        fixture.open_last = true;
        fixture.autostart = None;
        let on = fixture.page(Category::Appearance);
        assert!(on.words().contains("Open the last repository: on"));
        assert!(on.actions().contains(&Action::OpenLast(false)));
    }

    #[test]
    fn appearance_offers_every_theme_and_animation_choice() {
        let doc = Fixture::new().page(Category::Appearance);
        let words = doc.words();
        assert!(
            words.contains("Theme: Nord (CodeWiki Violet / Nord)"),
            "{words}"
        );
        assert!(words.contains("Your own themes load from the themes folder"));
        assert!(words.contains("Animations: follow Windows / reduced / full"));
        assert!(words.contains("Animations are off."));
        let actions = doc.actions();
        assert!(actions.contains(&Action::Theme("CodeWiki Violet".into())));
        // Every theme has the icon it has in the theme menu; the other
        // selects have none.
        for block in doc.flat() {
            if let Block::Select { options, .. } = block {
                for option in options {
                    let themed = matches!(option.action, Action::Theme(_));
                    assert_eq!(option.icon.is_some(), themed, "{}", option.label);
                    if themed {
                        assert_eq!(option.icon, Some(icons::for_theme(&option.label)));
                    }
                }
            }
        }
        for pref in MotionPref::ALL {
            assert!(actions.contains(&Action::Motion(pref)));
        }
    }

    #[test]
    fn the_ai_model_page_offers_the_providers_and_asks_for_what_they_need() {
        let fixture = Fixture::new();
        let doc = fixture.page(Category::AiModel);
        let words = doc.words();
        assert!(
            words.contains("Provider: None: notes keep their structural draft (None: notes keep their structural draft / Anthropic / OpenAI-compatible / Ollama / Claude Code (your own login) / Codex (your own login))"),
            "{words}"
        );
        assert!(words.contains("No provider is chosen."));
        assert!(words.contains("Anthropic API key: <typed, hidden>"));
        assert!(words.contains("Test"));
        let actions = doc.actions();
        for name in [
            "none",
            "anthropic",
            "openai",
            "ollama",
            "claude-cli",
            "codex-cli",
        ] {
            assert!(actions.contains(&set("llm", name)), "{name}");
        }
        assert!(actions.contains(&Action::TestLlm));
        assert_eq!(
            doc.fields(),
            [
                FieldId::LlmModel,
                FieldId::AnthropicKey,
                FieldId::OpenAiKey,
                FieldId::OpenAiBaseUrl,
                FieldId::OllamaHost
            ]
        );
    }

    #[test]
    fn a_saved_key_shows_its_end_and_can_be_replaced_or_removed() {
        let mut fixture = Fixture::new();
        let mut settings = defaults();
        settings["llm"]["provider"]["value"] = json!("anthropic");
        settings["llm"]["name"] = json!("anthropic:claude-opus-5-5");
        settings["anthropic"]["api_key"] =
            json!({ "state": "saved", "hint": "1234", "env": "ANTHROPIC_API_KEY" });
        settings["openai"]["api_key"]["state"] = json!("unreadable");
        fixture.settings = Some(settings);
        fixture.notes.insert(FieldId::AnthropicKey, Note::Saved);

        let doc = fixture.page(Category::AiModel);
        let words = doc.words();
        assert!(words.contains("Provider: Anthropic ("), "{words}");
        assert!(words.contains("In use: anthropic:claude-opus-5-5."));
        assert!(
            words.contains("Anthropic API key: Saved, ends in 1234 (saved)"),
            "{words}"
        );
        assert!(words.contains("OpenAI API key: Saved, but it cannot be read"));
        let actions = doc.actions();
        assert!(actions.contains(&Action::ReplaceSecret(FieldId::AnthropicKey)));
        assert!(actions.contains(&Action::RemoveSecret(FieldId::AnthropicKey)));
        assert!(actions.contains(&Action::ReplaceSecret(FieldId::OpenAiKey)));
        // A saved key has no input until it is to be replaced.
        assert!(!doc.fields().contains(&FieldId::AnthropicKey));

        fixture.typing.insert(FieldId::AnthropicKey);
        let doc = fixture.page(Category::AiModel);
        assert!(doc.fields().contains(&FieldId::AnthropicKey));
        assert!(
            doc.actions()
                .contains(&Action::KeepSecret(FieldId::AnthropicKey))
        );
    }

    #[test]
    fn a_setting_from_the_environment_is_locked_and_says_which_variable() {
        let mut fixture = Fixture::new();
        let mut settings = defaults();
        settings["llm"]["provider"]["value"] = json!("openai");
        settings["llm"]["provider"]["source"] = json!("env");
        settings["llm"]["model"] =
            json!({ "value": "gpt", "source": "env", "env": "CODEWIKI_MODEL" });
        settings["openai"]["api_key"]["state"] = json!("env");
        settings["llm"]["problem"] = json!("the openai provider needs a model");
        fixture.settings = Some(settings);

        let doc = fixture.page(Category::AiModel);
        let words = doc.words();
        assert!(
            words.contains("[Set by CODEWIKI_LLM in the backend's environment]"),
            "{words}"
        );
        assert!(
            words.contains("Model: <typed> [Set by CODEWIKI_MODEL in the backend's environment]")
        );
        assert!(words.contains("[Set by OPENAI_API_KEY in the backend's environment]"));
        assert!(words.contains("the openai provider needs a model"));
        // A locked select offers no choice; what is not locked still does.
        let actions = doc.actions();
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, Action::SetSetting { section, .. } if section == "llm"))
        );
        assert!(actions.contains(&Action::TestLlm));
    }

    #[test]
    fn a_backend_on_another_machine_shows_its_settings_and_takes_none() {
        let mut fixture = Fixture::new();
        fixture.origin = Some(Origin::Saved);
        fixture.url = Some("http://nas:7411".into());
        for category in [Category::AiModel, Category::Search] {
            let doc = fixture.page(category);
            let words = doc.words();
            assert!(words.contains("runs on another machine"), "{words}");
            assert!(words.contains("[Set on the backend at http://nas:7411]"));
            assert!(
                !doc.actions()
                    .iter()
                    .any(|a| matches!(a, Action::SetSetting { .. } | Action::ReplaceSecret(_))),
                "{category:?}"
            );
        }
    }

    #[test]
    fn a_backend_that_cannot_save_a_key_says_where_to_put_it() {
        let mut fixture = Fixture::new();
        let mut settings = defaults();
        settings["secrets"] = json!("env_only");
        fixture.settings = Some(settings);
        let words = fixture.page(Category::AiModel).words();
        assert!(
            words.contains(
                "This backend cannot save a key; set ANTHROPIC_API_KEY in its environment"
            ),
            "{words}"
        );
    }

    #[test]
    fn pages_that_need_the_backend_say_so_while_they_wait_or_fail() {
        let mut fixture = Fixture::new();
        fixture.settings = None;
        for category in [Category::AiModel, Category::Search] {
            assert!(
                fixture
                    .page(category)
                    .words()
                    .contains("Reading the backend's settings")
            );
        }
        fixture.error = Some("backend returned 404".into());
        let words = fixture.page(Category::AiModel).words();
        assert!(
            words.contains("could not be read: backend returned 404"),
            "{words}"
        );
        // The other categories do not depend on it.
        assert!(
            fixture
                .page(Category::Appearance)
                .words()
                .contains("Theme: Nord")
        );
    }

    #[test]
    fn semantic_search_offers_the_embedders_a_test_and_a_sync() {
        let mut fixture = Fixture::new();
        fixture.embed_test = Tested::Done {
            ok: true,
            text: "Works: local-hash-v1:384, 384 dimensions, 2 ms".into(),
        };
        let doc = fixture.page(Category::Search);
        let words = doc.words();
        assert!(
            words.contains("Embedder: Built in: no model needed ("),
            "{words}"
        );
        assert!(words.contains("In use: local-hash-v1:384."));
        assert!(words.contains("rebuilt at the next Sync"));
        assert!(words.contains("Works: local-hash-v1:384"));
        let actions = doc.actions();
        for name in ["local", "ollama", "openai", "none"] {
            assert!(actions.contains(&set("embed", name)), "{name}");
        }
        assert!(actions.contains(&Action::TestEmbedder));
        assert!(actions.contains(&Action::Sync));
        assert!(actions.contains(&Action::Settings(Some(Category::AiModel))));
        assert_eq!(doc.fields(), [FieldId::EmbedModel]);

        // While a test runs, it cannot be started again.
        fixture.embed_test = Tested::Running;
        let doc = fixture.page(Category::Search);
        assert!(doc.words().contains("Testing…"));
        assert!(!doc.actions().contains(&Action::TestEmbedder));
    }

    #[test]
    fn the_backend_page_names_the_backend_and_offers_another() {
        let mut fixture = Fixture::new();
        let doc = fixture.page(Category::Backend);
        let words = doc.words();
        assert!(words.contains("Connected to http://127.0.0.1:7411."));
        assert!(words.contains("runs on this machine"));
        assert_eq!(doc.fields(), [FieldId::RemoteUrl, FieldId::RemoteToken]);
        // Nothing is saved: only Connect.
        assert_eq!(doc.actions(), [Action::ConnectRemote]);

        fixture.origin = Some(Origin::Saved);
        fixture.url = Some("http://nas:7411".into());
        fixture.saved = Some(cw_client::Remote {
            url: "http://nas:7411".into(),
            in_use: true,
            token_hint: "0042".into(),
        });
        fixture.connect = Some(Note::Saved);
        let doc = fixture.page(Category::Backend);
        let words = doc.words();
        assert!(words.contains("It is the backend saved below."));
        assert!(words.contains("One is saved, ending in 0042; leave empty to keep it"));
        assert!(words.contains("Connect | Use local backend | Forget"));
        assert!(words.contains("Connected."));
        assert!(doc.actions().contains(&Action::UseLocalBackend));
        assert!(doc.actions().contains(&Action::ForgetRemote));

        // A try that failed says why.
        fixture.connect = Some(Note::Error(
            "no CodeWiki backend answers at http://nas:7411".into(),
        ));
        assert!(
            fixture
                .page(Category::Backend)
                .words()
                .contains("no CodeWiki backend answers")
        );
    }

    #[test]
    fn the_variable_wins_over_a_saved_backend_and_locks_the_fields() {
        let mut fixture = Fixture::new();
        fixture.origin = Some(Origin::Env);
        fixture.env_url = Some("http://docker:7411".into());
        fixture.url = fixture.env_url.clone();
        let doc = fixture.page(Category::Backend);
        let words = doc.words();
        assert!(words.contains("CODEWIKI_URL names it."));
        assert!(words.contains(
            "Address: <typed> [CODEWIKI_URL is set and wins over an address saved here]"
        ));
        assert!(!doc.actions().contains(&Action::ConnectRemote));
    }

    #[test]
    fn repositories_are_listed_with_their_folders() {
        let mut fixture = Fixture::new();
        let doc = fixture.page(Category::Repositories);
        assert_eq!(doc.tables()[0].plain(), [["demo", "C:/demo"]]);
        fixture.repos.clear();
        assert!(
            fixture
                .page(Category::Repositories)
                .words()
                .contains("No repository is registered")
        );
    }

    fn updates_with(state: update::State, auto: bool) -> Doc {
        let mut fixture = Fixture::new();
        fixture.update = state;
        fixture.auto_update = auto;
        fixture.page(Category::Updates)
    }

    #[test]
    fn updates_offer_what_the_state_allows() {
        let version = cw_core::VERSION;

        let checked = updates_with(update::State::UpToDate, true);
        assert!(
            checked
                .words()
                .contains(&format!("Version: {version}, the latest version"))
        );
        assert!(checked.words().contains("Daily check: on"));
        assert!(checked.actions().contains(&Action::CheckUpdates));
        // A click on the switch turns the check off.
        assert!(checked.actions().contains(&Action::AutoUpdate(false)));

        let release = cw_update::Release {
            version: cw_update::Version::new(9, 0, 0),
            zip_name: "CodeWiki-9.0.0-windows-x64.zip".into(),
            zip_url: String::new(),
            sha256_url: String::new(),
        };
        let available = updates_with(update::State::Available(release), false);
        assert!(
            available
                .words()
                .contains(&format!("Version: {version}; 9.0.0 is available"))
        );
        assert!(available.words().contains("Daily check: off"));
        assert!(available.actions().contains(&Action::InstallUpdate));
        assert!(!available.actions().contains(&Action::CheckUpdates));
        assert!(available.actions().contains(&Action::AutoUpdate(true)));

        // Nothing to press while a check or an install runs.
        let busy = updates_with(update::State::Checking, true);
        assert!(
            busy.actions()
                .iter()
                .all(|a| !matches!(a, Action::CheckUpdates | Action::InstallUpdate))
        );

        // A development build says so, and offers neither a check nor the
        // daily setting.
        let dev = updates_with(update::State::NotInstalled, true);
        assert!(dev.words().contains("development build"));
        assert!(
            dev.actions()
                .iter()
                .all(|a| !matches!(a, Action::CheckUpdates | Action::AutoUpdate(_)))
        );
    }

    #[test]
    fn the_keyboard_page_lists_every_shortcut_the_app_binds() {
        let words = Fixture::new().page(Category::Keyboard).words();
        for line in [
            "Ctrl+K | Command palette",
            "Ctrl+, | Settings",
            "Ctrl+T | New tab",
            "Ctrl+W | Close tab",
            "Ctrl+Tab | Next tab",
            "Alt+Left | Back",
            "Ctrl+B | Hide or show the sidebar",
            "Ctrl+Alt+B | Hide or show the details pane",
            "Ctrl+S | Save the note being edited",
        ] {
            assert!(words.contains(line), "{line}");
        }
    }

    #[test]
    fn diagnostics_offer_the_log_level_and_the_two_folders() {
        let mut fixture = Fixture::new();
        let doc = fixture.page(Category::Diagnostics);
        let words = doc.words();
        assert!(
            words.contains("App log level: Info: the usual amount ("),
            "{words}"
        );
        assert!(words.contains("Logs folder: C:/data/logs"));
        assert!(words.contains("Data folder: C:/data"));
        assert!(words.contains("Attach app.log and backend.log to a bug report"));
        let actions = doc.actions();
        for level in ["error", "warn", "info", "debug", "trace"] {
            assert!(actions.contains(&Action::LogLevel(level.into())), "{level}");
        }
        assert!(actions.contains(&Action::OpenLogs));
        assert!(actions.contains(&Action::OpenDataFolder));

        // The variable fixes the level for this run.
        fixture.log_level_locked = true;
        let doc = fixture.page(Category::Diagnostics);
        assert!(doc.words().contains("[Set by CODEWIKI_LOG]"));
        assert!(
            !doc.actions()
                .iter()
                .any(|a| matches!(a, Action::LogLevel(_)))
        );
    }
}
