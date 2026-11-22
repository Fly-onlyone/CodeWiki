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
use std::path::PathBuf;

use cw_client::Origin;
use cw_telemetry::Reporting;
use serde_json::Value;

use crate::doc::{
    Action, Block, Cell, Choice, Column, Doc, Press, SecretState, Span, Table, code, text,
};
use crate::icons;
use crate::motion::MotionPref;
use crate::mui::feedback::Tone;
use crate::settings::{Category, FieldId, Note};
use crate::{update, zoom};

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

/// A folder on the list of those kept from coding agents. Whether it still
/// exists is looked up when the list is read, never while a page is drawn:
/// a folder on a share that is offline can take long to answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptFolder {
    pub path: PathBuf,
    pub missing: bool,
}

/// Everything the pages read.
pub struct Model<'a> {
    pub theme: &'a str,
    /// Every theme by name, in the order of the theme menu.
    pub themes: &'a [String],
    pub motion: MotionPref,
    /// What the choice in `motion` comes to.
    pub animations_on: bool,
    /// The zoom level, in percent.
    pub zoom: u16,
    /// The backend's settings, once they have arrived.
    pub settings: Option<&'a Value>,
    /// Why they could not be read or changed.
    pub settings_error: Option<&'a str>,
    /// The news beside each field.
    pub notes: &'a HashMap<FieldId, Note>,
    /// The saved secrets whose input is open to take another in their place.
    pub typing: &'a HashSet<FieldId>,
    pub embed_test: &'a Tested,
    /// The embedding job (`GET /v1/embed/job`): the one that runs, or the
    /// last that ran. `None` when there is none.
    pub job: Option<&'a Value>,
    /// The models of the chosen embedding server
    /// (`GET /v1/settings/embed/models`). `None` until they are read.
    pub embed_models: Option<&'a Value>,
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
    /// The folders kept from coding agents; the error is the text that says
    /// the list cannot be read.
    pub agents_off: &'a Result<Vec<KeptFolder>, String>,
    /// What the last change to that list answered, when it is worth saying.
    pub agents_note: Option<&'a str>,
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
    /// Whether this build can send error reports, and how; and whether the
    /// user turned them on.
    pub error_reports: Reporting,
    pub error_reports_on: bool,
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
        zoom_card(model),
        startup(model),
    ]
}

/// How large the whole app is drawn.
fn zoom_card(model: &Model) -> Block {
    card(
        "Zoom",
        vec![
            Block::Select {
                label: "Zoom".into(),
                help: Some("The whole window except its title bar".into()),
                options: zoom::LEVELS
                    .into_iter()
                    .map(|level| Choice {
                        label: if level == zoom::DEFAULT {
                            format!("{} (default)", zoom::label(level))
                        } else {
                            zoom::label(level)
                        },
                        action: Action::Zoom(level),
                        selected: level == model.zoom,
                        icon: None,
                    })
                    .collect(),
                locked: None,
                note: None,
            },
            Block::Text(vec![
                text("Ctrl+wheel, "),
                code("Ctrl+="),
                text(" and "),
                code("Ctrl+-"),
                text(" zoom in and out; "),
                code("Ctrl+0"),
                text(" goes back to 100%."),
            ]),
        ],
    )
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
    select(model, settings, section, "provider", label, help, name)
}

/// A select over the `options` the backend lists for `section.field`.
fn select(
    model: &Model,
    settings: &Value,
    section: &str,
    field: &str,
    label: &str,
    help: &str,
    name: fn(&str) -> &str,
) -> Block {
    let entry = &settings[section][field];
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
                    field: field.into(),
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

fn embedder_name(provider: &str) -> &str {
    match provider {
        "local" => "Word forms: no model needed",
        "lmstudio" => "LM Studio",
        "ollama" => "Ollama",
        "llamacpp" => "Llama / llama.cpp",
        "lemonade" => "Lemonade",
        "mistralrs" => "mistral.rs",
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

fn search(model: &Model) -> Vec<Block> {
    let Some(settings) = model.settings else {
        return without_settings(model);
    };
    let chooser = provider(
        model,
        settings,
        "embed",
        "Embedder",
        "Finds notes and symbols by meaning",
        embedder_name,
    );
    let provider_id = settings["embed"]["provider"]["value"]
        .as_str()
        .unwrap_or("");
    let mut embedder = vec![chooser];
    embedder.extend(server_rows(model, settings, provider_id));
    embedder.extend([
        in_use(settings, "embed", "Semantic search is off."),
        Block::Text(vec![text(
            "A new embedder serves the next search. The stored vectors are rebuilt at the next Sync; until then search is full-text alone.",
        )]),
        Block::Buttons(vec![
            Press::new("Test", Action::TestEmbedder).disabled(*model.embed_test == Tested::Running),
            Press::new("Sync now", Action::Sync),
        ]),
    ]);
    embedder.extend(tested(model.embed_test));
    let in_use = settings["embed"]["name"].as_str();
    embedder.extend(
        model
            .job
            .filter(|j| job_runs(j) || belongs_to(j, in_use))
            .into_iter()
            .flat_map(job),
    );
    let mut blocks: Vec<Block> = remote_notice(model).into_iter().collect();
    blocks.push(card("Embedder", embedder));
    blocks
}

/// The address and key rows of a server, then its model, then how to start
/// it. Nothing for the word-form embedder and for none.
fn server_rows(model: &Model, settings: &Value, provider: &str) -> Vec<Block> {
    let address = |id: FieldId, label: &str, help: &str| field(model, settings, id, label, help);
    let key = |id: FieldId, label: &str| {
        secret(
            model,
            settings,
            id,
            label,
            "Only if the server asks for one",
        )
    };
    let mut rows = match provider {
        "lmstudio" => vec![
            address(FieldId::LmStudioUrl, "LM Studio address", ""),
            key(FieldId::LmStudioKey, "LM Studio key"),
        ],
        "ollama" => vec![address(FieldId::OllamaHost, "Ollama address", "")],
        "llamacpp" => vec![
            address(FieldId::LlamaCppUrl, "Llama address", ""),
            key(FieldId::LlamaCppKey, "Llama key"),
        ],
        "lemonade" => vec![
            address(FieldId::LemonadeUrl, "Lemonade address", ""),
            key(FieldId::LemonadeKey, "Lemonade key"),
        ],
        "mistralrs" => vec![address(FieldId::MistralRsUrl, "mistral.rs address", "")],
        "openai" => vec![
            secret(
                model,
                settings,
                FieldId::OpenAiKey,
                "OpenAI API key",
                "For the OpenAI embedder",
            ),
            address(
                FieldId::OpenAiBaseUrl,
                "OpenAI address",
                "Any server that speaks the OpenAI protocol",
            ),
        ],
        _ => return Vec::new(),
    };
    let keyed = rows.iter().any(|row| matches!(row, Block::Secret { .. }));
    rows.extend(model_rows(model, settings));
    if let Some(help) = serve_help(provider) {
        rows.push(Block::Text(help));
    }
    if keyed {
        rows.push(Block::Text(vec![text(
            "A key is stored encrypted for your Windows user and is never shown again.",
        )]));
    }
    rows
}

/// The model of the chosen server: a choice from the list the server gave,
/// or a typed name when it gave none, and a button to ask again.
fn model_rows(model: &Model, settings: &Value) -> Vec<Block> {
    let entry = &settings["embed"]["model"];
    let chosen = entry["value"].as_str().unwrap_or("");
    // A list of another server's models, read before the choice changed, is
    // not offered.
    let provider = settings["embed"]["provider"].get("value");
    let current = model
        .embed_models
        .filter(|models| provider.is_some_and(|chosen| models["provider"] == *chosen));
    let listed = current
        .and_then(|models| models["models"].as_array())
        .filter(|models| !models.is_empty());
    let mut rows = Vec::new();
    match listed {
        Some(models) => {
            rows.push(Block::Select {
                label: "Model".into(),
                help: Some("The models the server lists; embedding ones come first".into()),
                options: Some(chosen)
                    .filter(|chosen| {
                        !chosen.is_empty() && !models.iter().any(|listed| listed["id"] == *chosen)
                    })
                    .into_iter()
                    .chain(models.iter().filter_map(|listed| listed["id"].as_str()))
                    .map(|id| Choice {
                        label: id.to_string(),
                        action: Action::SetSetting {
                            section: "embed".into(),
                            field: "model".into(),
                            value: Some(id.into()),
                        },
                        selected: id == chosen,
                        icon: None,
                    })
                    .collect(),
                locked: lock(model, settings, entry),
                note: None,
            });
            // A server may not serve a model it does not list. llama.cpp and
            // mistral.rs would embed with the one they hold, under the name
            // of the one chosen.
            let provider = provider.and_then(Value::as_str).unwrap_or("");
            if !chosen.is_empty() && !lists(models, provider, chosen) {
                rows.push(Block::Alert {
                    tone: Tone::Warning,
                    text: format!(
                        "{} does not list {chosen}; choose a model it serves.",
                        server_label(provider)
                    ),
                    button: None,
                });
            }
        }
        None => {
            rows.push(field(
                model,
                settings,
                FieldId::EmbedModel,
                "Model",
                "The name the server knows the model by",
            ));
            let problem = current
                .filter(|models| models["reachable"] == false)
                .and_then(|models| models["problem"].as_str());
            rows.extend(problem.map(|problem| Block::Alert {
                tone: Tone::Warning,
                text: problem.to_string(),
                button: None,
            }));
        }
    }
    rows.push(Block::Buttons(vec![Press::new(
        "Refresh",
        Action::RefreshEmbedModels,
    )]));
    rows
}

/// Whether the models a server listed hold `chosen`. Ollama takes a name
/// without a tag as its `latest`: "The tag is optional and, if not provided,
/// will default to latest" (https://github.com/ollama/ollama/blob/main/docs/api.md).
fn lists(models: &[Value], provider: &str, chosen: &str) -> bool {
    let latest = format!("{chosen}:latest");
    models.iter().any(|listed| {
        listed["id"] == chosen || (provider == "ollama" && listed["id"] == latest.as_str())
    })
}

/// How the page names the server of `provider`, as its address row does.
fn server_label(provider: &str) -> &str {
    match provider {
        "lmstudio" => "LM Studio",
        "ollama" => "Ollama",
        "llamacpp" => "Llama",
        "lemonade" => "Lemonade",
        "mistralrs" => "mistral.rs",
        "openai" => "OpenAI",
        other => other,
    }
}

/// How to get an embedding model going in the server, one line.
// Commands checked against each server's docs:
// Ollama: https://ollama.com/library/embeddinggemma (`ollama pull embeddinggemma`).
// mistral.rs: https://docs.mistralrs.dev/reference/supported-models/ lists
// Qwen/Qwen3-Embedding-0.6B as an embedding model, and `mistralrs serve -m <model>`
// is the serve form of its README, on port 1234.
// Lemonade: nomic-embed-text-v1-GGUF is in
// https://github.com/lemonade-sdk/lemonade/blob/main/src/cpp/resources/server_models.json
// llama-server: `-ub` is the physical batch (512 by default) and `-b` the
// logical one; a bidirectional encoder's whole input must fit the physical
// batch (https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md).
fn serve_help(provider: &str) -> Option<Vec<Span>> {
    Some(match provider {
        "lmstudio" => vec![text(
            "In LM Studio, download an embedding model and start the server on the Developer tab. LM Studio and mistral.rs both use port 1234.",
        )],
        "ollama" => vec![
            text("Run "),
            code("ollama pull embeddinggemma"),
            text(", or name another embedding model."),
        ],
        "llamacpp" => vec![
            text(
                "Load an embedding model (GGUF) in Llama for Windows, or start llama-server with one. Both use port 9931. For long texts, start llama-server with ",
            ),
            code("-ub 2048 -b 2048"),
            text(", or add those flags to the custom serve arguments of Llama for Windows."),
        ],
        "lemonade" => vec![text(
            "Pull an embedding model in Lemonade, such as nomic-embed-text-v1-GGUF.",
        )],
        "mistralrs" => vec![
            text("Run "),
            code("mistralrs serve -m Qwen/Qwen3-Embedding-0.6B"),
            text(". mistral.rs and LM Studio both use port 1234."),
        ],
        _ => return None,
    })
}

/// Whether an embedding job is under way.
pub fn job_runs(job: &Value) -> bool {
    job["phase"] == "embedding"
}

/// Whether a job that is over was the embedder's in use. After a change of
/// embedder the last job is the old one's, and its word would be about a
/// server no longer chosen. A job that names no model is shown.
fn belongs_to(job: &Value, in_use: Option<&str>) -> bool {
    match job["model"].as_str() {
        Some(model) => in_use == Some(model),
        None => true,
    }
}

/// What the embedding job that runs, or the last that ran, has to say.
fn job(job: &Value) -> Vec<Block> {
    let count = |name: &str| job[name].as_u64().unwrap_or(0);
    let alert = |tone: Tone, text: String| Block::Alert {
        tone,
        text,
        button: None,
    };
    match job["phase"].as_str() {
        Some("embedding") => {
            let (done, total) = (count("docs_done"), count("docs_total"));
            vec![
                Block::Progress {
                    label: format!(
                        "Embedding {}",
                        job["repo"].as_str().unwrap_or("a repository")
                    ),
                    share: (total > 0).then(|| done as f32 / total as f32),
                    detail: format!("{done} of {total}"),
                    moving: true,
                },
                Block::Buttons(vec![Press::new("Cancel", Action::CancelEmbedJob)]),
            ]
        }
        Some("failed") => vec![alert(
            Tone::Error,
            format!(
                "The embedding did not work: {}",
                job["error"].as_str().unwrap_or("no reason was given")
            ),
        )],
        Some("cancelled") => vec![alert(
            Tone::Info,
            "Stopped. What was embedded is kept; the next Sync goes on from there.".into(),
        )],
        Some("done") => {
            let mut out = Vec::new();
            if let Some(note) = job["note"].as_str() {
                out.push(alert(Tone::Info, note.to_string()));
            }
            let skipped: Vec<String> = job["skipped"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|skipped| {
                    format!(
                        "{} ({})",
                        skipped["repo"].as_str().unwrap_or("a repository"),
                        skipped["error"].as_str().unwrap_or("no reason was given")
                    )
                })
                .collect();
            if !skipped.is_empty() {
                out.push(alert(
                    Tone::Warning,
                    format!("Left out: {}.", skipped.join("; ")),
                ));
            }
            out
        }
        _ => Vec::new(),
    }
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
    let mut blocks = Vec::new();
    if model.repos.is_empty() {
        blocks.push(Block::Empty(
            "No repository is registered. Open a folder from Home.".into(),
        ));
    } else {
        let mut table = Table::new([
            Column::new("Repository").weight(0.6),
            Column::new("Folder").weight(1.6),
        ]);
        for (name, path) in model.repos {
            table.row([Cell::text(name), code(path).into()]);
        }
        blocks.push(Block::Table(table));
    }
    blocks.push(agents_card(model));
    blocks
}

/// The folders kept from coding agents, with what takes one off the list or
/// puts one on it.
fn agents_card(model: &Model) -> Block {
    let mut body = vec![Block::Text(vec![text(
        "Claude Code and Codex get no CodeWiki tools in these folders and everything below them. The app and the command line still work there.",
    )])];
    match model.agents_off {
        // A table, as the repositories above: each path on one line.
        Ok(folders) if !folders.is_empty() => {
            let mut table = Table::new([
                Column::new("Folder").weight(2.0),
                Column::new("").weight(0.6),
                Column::new("").weight(0.4),
            ]);
            for folder in folders {
                let path = folder.path.display().to_string();
                table.row([
                    code(path.clone()).into(),
                    Cell::text(if folder.missing {
                        "Folder not found"
                    } else {
                        ""
                    }),
                    Span::Link {
                        label: "Remove".into(),
                        code: false,
                        action: Action::LetAgentsIn(path),
                    }
                    .into(),
                ]);
            }
            body.push(Block::Table(table));
        }
        Ok(_) => {}
        Err(unreadable) => body.push(Block::Alert {
            tone: Tone::Error,
            text: unreadable.clone(),
            button: Some(("Start a new list".into(), Action::ResetAgentsList)),
        }),
    }
    body.extend(model.agents_note.map(|note| Block::Alert {
        tone: Tone::Info,
        text: note.to_string(),
        button: None,
    }));
    body.push(Block::Buttons(vec![Press::new(
        "Add folder…",
        Action::PickAgentsFolder,
    )]));
    card("Kept from coding agents", body)
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
        ("Ctrl+=", "Zoom in"),
        ("Ctrl+-", "Zoom out"),
        ("Ctrl+0", "Back to 100%"),
        ("Ctrl+wheel", "Zoom in or out"),
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
        error_reports(model),
    ]
}

/// The switch for error reports, which only a build that carries a DSN
/// offers. `SENTRY_DSN` turns them on for a run, whatever the switch says.
fn error_reports(model: &Model) -> Block {
    let (on, locked) = match model.error_reports {
        Reporting::Switch => (model.error_reports_on, None),
        Reporting::Environment => (true, Some(format!("Set by {}", cw_telemetry::DSN_ENV))),
        Reporting::Unavailable => (false, Some("This build has no error reporting".into())),
    };
    card(
        "Error reports",
        vec![
            Block::Switch {
                label: "Send error reports".into(),
                help: Some("Turning them on applies from the next start".into()),
                on,
                action: Action::ErrorReports(!on),
                locked,
            },
            Block::Text(vec![text(
                "A crash or an error of the app, the backend or the agents' bridge goes to the developer through Sentry: what failed, where in the code, the version of CodeWiki and of Windows, and the log lines just before it, which can name a repository or a note. Never a file path, note text, a search query or a key. Sentry keeps the city and country of the network a report comes from, not its address.",
            )]),
        ],
    )
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
            "embed": {
                "provider": { "value": "local", "source": "default", "env": "CODEWIKI_EMBED",
                    "options": ["local", "lmstudio", "ollama", "llamacpp", "lemonade", "mistralrs", "openai", "none"] },
                "model": { "value": null, "source": "default", "env": "CODEWIKI_EMBED_MODEL" },
                "name": "local-hash-v1:384", "problem": null,
            },
            "openai": {
                "api_key": { "state": "unset", "hint": "", "env": "OPENAI_API_KEY" },
                "base_url": { "value": "https://api.openai.com/v1", "source": "default", "env": "OPENAI_BASE_URL" },
            },
            "ollama": { "host": { "value": "http://127.0.0.1:11434", "source": "default", "env": "OLLAMA_HOST" } },
            "lmstudio": {
                "url": { "value": "http://127.0.0.1:1234", "source": "default", "env": "CODEWIKI_LMSTUDIO_URL" },
                "api_key": { "state": "unset", "hint": "", "env": "CODEWIKI_LMSTUDIO_KEY" },
            },
            "llamacpp": {
                "url": { "value": "http://127.0.0.1:9931", "source": "default", "env": "CODEWIKI_LLAMACPP_URL" },
                "api_key": { "state": "unset", "hint": "", "env": "CODEWIKI_LLAMACPP_KEY" },
            },
            "lemonade": {
                "url": { "value": "http://127.0.0.1:13305", "source": "default", "env": "CODEWIKI_LEMONADE_URL" },
                "api_key": { "state": "unset", "hint": "", "env": "CODEWIKI_LEMONADE_KEY" },
            },
            "mistralrs": {
                "url": { "value": "http://127.0.0.1:1234", "source": "default", "env": "CODEWIKI_MISTRALRS_URL" },
            },
        })
    }

    /// Owns what a [`Model`] borrows.
    struct Fixture {
        themes: Vec<String>,
        settings: Option<Value>,
        error: Option<String>,
        job: Option<Value>,
        embed_models: Option<Value>,
        notes: HashMap<FieldId, Note>,
        typing: HashSet<FieldId>,
        embed_test: Tested,
        url: Option<String>,
        origin: Option<Origin>,
        env_url: Option<String>,
        saved: Option<cw_client::Remote>,
        connect: Option<Note>,
        repos: Vec<(String, String)>,
        agents_off: Result<Vec<KeptFolder>, String>,
        agents_note: Option<String>,
        update: update::State,
        auto_update: bool,
        autostart: Option<bool>,
        open_last: bool,
        log_level_locked: bool,
        error_reports: Reporting,
        error_reports_on: bool,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                themes: vec!["CodeWiki Violet".into(), "Nord".into()],
                settings: Some(defaults()),
                error: None,
                job: None,
                embed_models: None,
                notes: HashMap::new(),
                typing: HashSet::new(),
                embed_test: Tested::Not,
                url: Some("http://127.0.0.1:7411".into()),
                origin: Some(Origin::Local),
                env_url: None,
                saved: None,
                connect: None,
                repos: vec![("demo".into(), "C:/demo".into())],
                agents_off: Ok(Vec::new()),
                agents_note: None,
                update: update::State::UpToDate,
                auto_update: true,
                autostart: Some(false),
                open_last: false,
                log_level_locked: false,
                error_reports: Reporting::Switch,
                error_reports_on: false,
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
                    zoom: 125,
                    settings: self.settings.as_ref(),
                    settings_error: self.error.as_deref(),
                    notes: &self.notes,
                    typing: &self.typing,
                    embed_test: &self.embed_test,
                    job: self.job.as_ref(),
                    embed_models: self.embed_models.as_ref(),
                    backend_url: self.url.as_deref(),
                    origin: self.origin,
                    env_url: self.env_url.as_deref(),
                    saved_remote: self.saved.as_ref(),
                    connect: self.connect.as_ref(),
                    repos: &self.repos,
                    agents_off: &self.agents_off,
                    agents_note: self.agents_note.as_deref(),
                    update: &self.update,
                    auto_update: self.auto_update,
                    autostart: self.autostart,
                    open_last: self.open_last,
                    log_level: "info",
                    log_level_locked: self.log_level_locked,
                    logs_dir: "C:/data/logs",
                    data_dir: "C:/data",
                    error_reports: self.error_reports,
                    error_reports_on: self.error_reports_on,
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
    fn the_appearance_page_offers_every_zoom_level() {
        let doc = Fixture::new().page(Category::Appearance);
        let zoom = doc
            .flat()
            .into_iter()
            .find_map(|block| match block {
                Block::Select { label, options, .. } if label == "Zoom" => Some(options.clone()),
                _ => None,
            })
            .expect("a zoom select");
        let levels: Vec<Action> = zoom.iter().map(|option| option.action.clone()).collect();
        let expected: Vec<Action> = zoom::LEVELS.into_iter().map(Action::Zoom).collect();
        assert_eq!(levels, expected);
        // The fixture is at 125%, and only that level is chosen.
        let chosen: Vec<&str> = zoom
            .iter()
            .filter(|option| option.selected)
            .map(|option| option.label.as_str())
            .collect();
        assert_eq!(chosen, ["125%"]);
        assert!(zoom.iter().any(|option| option.label == "100% (default)"));
    }

    #[test]
    fn a_saved_key_shows_its_end_and_can_be_replaced_or_removed() {
        let mut fixture = Fixture::new();
        let mut settings = defaults();
        settings["openai"]["api_key"] =
            json!({ "state": "saved", "hint": "1234", "env": "OPENAI_API_KEY" });
        settings["embed"]["provider"]["value"] = json!("openai");
        fixture.settings = Some(settings);
        fixture.notes.insert(FieldId::OpenAiKey, Note::Saved);

        let doc = fixture.page(Category::Search);
        let words = doc.words();
        assert!(
            words.contains("OpenAI API key: Saved, ends in 1234 (saved)"),
            "{words}"
        );
        let actions = doc.actions();
        assert!(actions.contains(&Action::ReplaceSecret(FieldId::OpenAiKey)));
        assert!(actions.contains(&Action::RemoveSecret(FieldId::OpenAiKey)));
        // A saved key has no input until it is to be replaced.
        assert!(!doc.fields().contains(&FieldId::OpenAiKey));

        fixture.typing.insert(FieldId::OpenAiKey);
        let doc = fixture.page(Category::Search);
        assert!(doc.fields().contains(&FieldId::OpenAiKey));
        assert!(
            doc.actions()
                .contains(&Action::KeepSecret(FieldId::OpenAiKey))
        );

        // A key this user cannot decrypt says so.
        fixture.typing.clear();
        fixture.settings.as_mut().unwrap()["openai"]["api_key"]["state"] = json!("unreadable");
        let words = fixture.page(Category::Search).words();
        assert!(
            words.contains("OpenAI API key: Saved, but it cannot be read"),
            "{words}"
        );
    }

    #[test]
    fn a_setting_from_the_environment_is_locked_and_says_which_variable() {
        let mut fixture = Fixture::new();
        let mut settings = defaults();
        settings["embed"]["provider"]["value"] = json!("openai");
        settings["embed"]["provider"]["source"] = json!("env");
        settings["embed"]["model"] = json!({
            "value": "text-embedding-3-small", "source": "env", "env": "CODEWIKI_EMBED_MODEL"
        });
        settings["openai"]["api_key"]["state"] = json!("env");
        settings["embed"]["name"] = json!(null);
        settings["embed"]["problem"] = json!("the openai embedder needs a key");
        fixture.settings = Some(settings);

        let doc = fixture.page(Category::Search);
        let words = doc.words();
        assert!(
            words.contains("[Set by CODEWIKI_EMBED in the backend's environment]"),
            "{words}"
        );
        assert!(
            words.contains(
                "Model: <typed> [Set by CODEWIKI_EMBED_MODEL in the backend's environment]"
            ),
            "{words}"
        );
        assert!(words.contains("[Set by OPENAI_API_KEY in the backend's environment]"));
        assert!(words.contains("the openai embedder needs a key"));
        // A locked select offers no choice; what is not locked still does.
        let actions = doc.actions();
        assert!(!actions.iter().any(|a| matches!(
            a,
            Action::SetSetting { section, field, .. } if section == "embed" && field == "provider"
        )));
        assert!(actions.contains(&Action::TestEmbedder));
    }

    #[test]
    fn a_backend_on_another_machine_shows_its_settings_and_takes_none() {
        let mut fixture = Fixture::new();
        fixture.origin = Some(Origin::Saved);
        fixture.url = Some("http://nas:7411".into());
        let doc = fixture.page(Category::Search);
        let words = doc.words();
        assert!(words.contains("runs on another machine"), "{words}");
        assert!(words.contains("[Set on the backend at http://nas:7411]"));
        assert!(
            !doc.actions()
                .iter()
                .any(|a| matches!(a, Action::SetSetting { .. } | Action::ReplaceSecret(_)))
        );
    }

    #[test]
    fn a_backend_that_cannot_save_a_key_says_where_to_put_it() {
        let mut fixture = Fixture::new();
        let mut settings = defaults();
        settings["secrets"] = json!("env_only");
        settings["embed"]["provider"]["value"] = json!("openai");
        fixture.settings = Some(settings);
        let words = fixture.page(Category::Search).words();
        assert!(
            words.contains("This backend cannot save a key; set OPENAI_API_KEY in its environment"),
            "{words}"
        );
    }

    #[test]
    fn a_page_that_needs_the_backend_says_so_while_it_waits_or_fails() {
        let mut fixture = Fixture::new();
        fixture.settings = None;
        assert!(
            fixture
                .page(Category::Search)
                .words()
                .contains("Reading the backend's settings")
        );
        fixture.error = Some("backend returned 404".into());
        let words = fixture.page(Category::Search).words();
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
            words.contains("Embedder: Word forms: no model needed ("),
            "{words}"
        );
        assert!(words.contains("In use: local-hash-v1:384."));
        assert!(words.contains("rebuilt at the next Sync"));
        assert!(words.contains("Works: local-hash-v1:384"));
        let actions = doc.actions();
        for name in [
            "local",
            "lmstudio",
            "ollama",
            "llamacpp",
            "lemonade",
            "mistralrs",
            "openai",
            "none",
        ] {
            assert!(actions.contains(&set("embed", name)), "{name}");
        }
        assert!(actions.contains(&Action::TestEmbedder));
        assert!(actions.contains(&Action::Sync));
        // The word-form embedder connects to nothing: no rows of a server.
        assert!(doc.fields().is_empty(), "{:?}", doc.fields());
        assert!(!words.contains("Refresh"), "{words}");

        // While a test runs, it cannot be started again.
        fixture.embed_test = Tested::Running;
        let doc = fixture.page(Category::Search);
        assert!(doc.words().contains("Testing…"));
        assert!(!doc.actions().contains(&Action::TestEmbedder));
    }

    /// The page with `provider` chosen.
    fn search_with(provider: &str, models: Option<Value>) -> Doc {
        let mut fixture = Fixture::new();
        fixture.settings.as_mut().unwrap()["embed"]["provider"]["value"] = json!(provider);
        fixture.embed_models = models;
        fixture.page(Category::Search)
    }

    #[test]
    fn each_server_shows_only_its_own_rows() {
        use FieldId::*;
        let rows = [
            ("lmstudio", vec![LmStudioUrl, LmStudioKey]),
            ("ollama", vec![OllamaHost]),
            ("llamacpp", vec![LlamaCppUrl, LlamaCppKey]),
            ("lemonade", vec![LemonadeUrl, LemonadeKey]),
            ("mistralrs", vec![MistralRsUrl]),
            ("openai", vec![OpenAiKey, OpenAiBaseUrl]),
        ];
        for (provider, mut expected) in rows {
            // No list is loaded, so the model is typed.
            expected.push(EmbedModel);
            let doc = search_with(provider, None);
            assert_eq!(doc.fields(), expected, "{provider}");
            assert!(
                doc.actions().contains(&Action::RefreshEmbedModels),
                "{provider}"
            );
        }
        for provider in ["local", "none"] {
            let doc = search_with(provider, None);
            assert!(doc.fields().is_empty(), "{provider}");
            assert!(
                !doc.actions().contains(&Action::RefreshEmbedModels),
                "{provider}"
            );
        }
        // How to start each server, and what keeps a key.
        // A long text needs a physical batch larger than llama-server's own.
        let llama = search_with("llamacpp", None).words();
        assert!(
            llama.contains("-ub 2048 -b 2048") && llama.contains("port 9931"),
            "{llama}"
        );
        let words = search_with("mistralrs", None).words();
        assert!(words.contains("mistralrs serve -m Qwen/Qwen3-Embedding-0.6B"));
        // A command is set in the code font, not between backticks.
        for (provider, command) in [
            ("ollama", "ollama pull embeddinggemma"),
            ("mistralrs", "mistralrs serve -m Qwen/Qwen3-Embedding-0.6B"),
            ("llamacpp", "-ub 2048 -b 2048"),
        ] {
            let doc = search_with(provider, None);
            let spans: Vec<&Span> = doc
                .flat()
                .into_iter()
                .filter_map(|block| match block {
                    Block::Text(line) => Some(line),
                    _ => None,
                })
                .flatten()
                .collect();
            assert!(spans.contains(&&code(command)), "{provider}: {spans:?}");
            assert!(!doc.words().contains('`'), "{provider}");
        }
        assert!(!words.contains("stored encrypted"), "{words}");
        assert!(
            search_with("lmstudio", None)
                .words()
                .contains("stored encrypted")
        );
    }

    #[test]
    fn a_listed_server_offers_its_models_to_choose() {
        let mut fixture = Fixture::new();
        {
            let settings = fixture.settings.as_mut().unwrap();
            settings["embed"]["provider"]["value"] = json!("lmstudio");
            settings["embed"]["model"]["value"] = json!("a");
        }
        fixture.embed_models = Some(json!({
            "provider": "lmstudio", "reachable": true, "problem": null,
            "models": [{ "id": "a", "embedding": true }, { "id": "b", "embedding": null }],
        }));
        let doc = fixture.page(Category::Search);
        let words = doc.words();
        assert!(words.contains("Model: a (a / b)"), "{words}");
        let choose_b = Action::SetSetting {
            section: "embed".into(),
            field: "model".into(),
            value: Some("b".into()),
        };
        assert!(doc.actions().contains(&choose_b));
        assert!(!doc.fields().contains(&FieldId::EmbedModel));
        assert!(doc.actions().contains(&Action::RefreshEmbedModels));
    }

    #[test]
    fn a_server_that_does_not_answer_keeps_the_typed_model_and_says_why() {
        let doc = search_with(
            "llamacpp",
            Some(json!({
                "provider": "llamacpp", "reachable": false, "models": null,
                "problem": "Llama is not running at http://127.0.0.1:9931",
            })),
        );
        let words = doc.words();
        assert!(
            words.contains("Llama is not running at http://127.0.0.1:9931"),
            "{words}"
        );
        assert!(doc.fields().contains(&FieldId::EmbedModel));
        assert!(doc.actions().contains(&Action::RefreshEmbedModels));
    }

    #[test]
    fn a_model_that_is_not_listed_stays_chosen() {
        let mut fixture = Fixture::new();
        {
            let settings = fixture.settings.as_mut().unwrap();
            settings["embed"]["provider"]["value"] = json!("lmstudio");
            settings["embed"]["model"]["value"] = json!("zzz");
        }
        fixture.embed_models = Some(json!({
            "provider": "lmstudio", "reachable": true, "problem": null,
            "models": [{ "id": "a", "embedding": true }, { "id": "b", "embedding": null }],
        }));
        let words = fixture.page(Category::Search).words();
        assert!(words.contains("Model: zzz (zzz / a / b)"), "{words}");
        // Without a model, none is claimed.
        fixture.settings.as_mut().unwrap()["embed"]["model"]["value"] = json!(null);
        let words = fixture.page(Category::Search).words();
        assert!(words.contains("Model:  (a / b)"), "{words}");
    }

    #[test]
    fn a_model_the_server_does_not_list_is_warned_about() {
        let mut fixture = Fixture::new();
        {
            let settings = fixture.settings.as_mut().unwrap();
            settings["embed"]["provider"]["value"] = json!("llamacpp");
            settings["embed"]["model"]["value"] = json!("zzz");
        }
        fixture.embed_models = Some(json!({
            "provider": "llamacpp", "reachable": true, "problem": null,
            "models": [{ "id": "a", "embedding": null }],
        }));
        let warning = "Llama does not list zzz; choose a model it serves.";
        let words = fixture.page(Category::Search).words();
        assert!(words.contains(warning), "{words}");
        // The choice stays as it is.
        assert!(words.contains("Model: zzz (zzz / a)"), "{words}");

        // A listed model, or none, is not warned about.
        for chosen in [json!("a"), json!(null)] {
            fixture.settings.as_mut().unwrap()["embed"]["model"]["value"] = chosen;
            let words = fixture.page(Category::Search).words();
            assert!(!words.contains("does not list"), "{words}");
        }

        // Ollama takes a name without a tag as its latest.
        {
            let settings = fixture.settings.as_mut().unwrap();
            settings["embed"]["provider"]["value"] = json!("ollama");
            settings["embed"]["model"]["value"] = json!("embeddinggemma");
        }
        fixture.embed_models = Some(json!({
            "provider": "ollama", "reachable": true, "problem": null,
            "models": [{ "id": "embeddinggemma:latest", "embedding": true }],
        }));
        let words = fixture.page(Category::Search).words();
        assert!(!words.contains("does not list"), "{words}");
    }

    #[test]
    fn the_models_of_another_server_are_not_offered() {
        let doc = search_with(
            "ollama",
            Some(json!({
                "provider": "lmstudio", "reachable": true, "problem": null,
                "models": [{ "id": "a", "embedding": true }],
            })),
        );
        assert!(doc.fields().contains(&FieldId::EmbedModel));
        assert!(doc.words().contains("Model: <typed>"), "{}", doc.words());
    }

    #[test]
    fn a_model_set_by_the_environment_is_not_offered_for_choosing() {
        let mut fixture = Fixture::new();
        {
            let settings = fixture.settings.as_mut().unwrap();
            settings["embed"]["provider"]["value"] = json!("lmstudio");
            settings["embed"]["model"] = json!({
                "value": "a", "source": "env", "env": "CODEWIKI_EMBED_MODEL"
            });
        }
        fixture.embed_models = Some(json!({
            "provider": "lmstudio", "reachable": true, "problem": null,
            "models": [{ "id": "a", "embedding": true }, { "id": "b", "embedding": null }],
        }));
        let doc = fixture.page(Category::Search);
        let words = doc.words();
        assert!(
            words.contains(
                "Model: a (a / b) [Set by CODEWIKI_EMBED_MODEL in the backend's environment]"
            ),
            "{words}"
        );
        assert!(!doc.actions().iter().any(|a| matches!(
            a,
            Action::SetSetting { section, field, .. } if section == "embed" && field == "model"
        )));
    }

    /// The job as `GET /v1/embed/job` gives it.
    fn embedding(done: u64) -> Value {
        json!({
            "phase": "embedding", "model": "m", "repo": "r", "repo_index": 1,
            "repo_count": 1, "docs_done": done, "docs_total": 20,
            "skipped": [], "note": null, "error": null,
        })
    }

    #[test]
    fn a_running_job_shows_how_far_it_is_and_can_be_cancelled() {
        let mut fixture = Fixture::new();
        fixture.job = Some(json!({
            "phase": "embedding", "repo": "r", "docs_done": 5, "docs_total": 20,
        }));
        let doc = fixture.page(Category::Search);
        let words = doc.words();
        assert!(words.contains("Embedding r 25%: 5 of 20"), "{words}");
        assert!(doc.actions().contains(&Action::CancelEmbedJob));
        assert!(job_runs(&embedding(3)));
    }

    #[test]
    fn a_failed_job_says_why() {
        let mut fixture = Fixture::new();
        fixture.job = Some(json!({
            "phase": "failed", "repo": "r",
            "error": "LM Studio is not running at http://127.0.0.1:1234",
        }));
        let doc = fixture.page(Category::Search);
        let words = doc.words();
        assert!(
            words.contains("LM Studio is not running at http://127.0.0.1:1234"),
            "{words}"
        );
        assert!(!doc.actions().contains(&Action::CancelEmbedJob));
        assert!(!job_runs(fixture.job.as_ref().unwrap()));
    }

    #[test]
    fn the_last_job_of_another_embedder_is_left_out() {
        // Found in the clean-Windows test: llama.cpp's failed job still showed
        // under Ollama after the embedder was changed.
        let mut fixture = Fixture::new();
        fixture.job = Some(json!({
            "phase": "failed", "model": "llamacpp:vm-embed-missing", "repo": "r",
            "error": "llama.cpp does not list vm-embed-missing; choose a model it serves",
        }));
        let words = fixture.page(Category::Search).words();
        assert!(!words.contains("did not work"), "{words}");

        // The same failure of the embedder in use is shown.
        fixture.job.as_mut().unwrap()["model"] = json!("local-hash-v1:384");
        let words = fixture.page(Category::Search).words();
        assert!(words.contains("did not work"), "{words}");

        // A running job shows whichever embedder it is for: it can be cancelled.
        let mut running = embedding(4);
        running["model"] = json!("llamacpp:vm-embed-stub");
        fixture.job = Some(running);
        assert!(
            fixture
                .page(Category::Search)
                .actions()
                .contains(&Action::CancelEmbedJob)
        );
    }

    #[test]
    fn a_job_that_is_over_leaves_its_word() {
        let mut fixture = Fixture::new();
        fixture.job = Some(json!({ "phase": "cancelled" }));
        let doc = fixture.page(Category::Search);
        assert!(doc.words().contains("Stopped."), "{}", doc.words());
        assert!(!doc.actions().contains(&Action::CancelEmbedJob));

        fixture.job = Some(json!({
            "phase": "done", "note": "Every repository was embedded.",
            "skipped": [{ "repo": "old", "error": "the index is damaged" }],
        }));
        let words = fixture.page(Category::Search).words();
        assert!(words.contains("Every repository was embedded."), "{words}");
        assert!(
            words.contains("Left out: old (the index is damaged)."),
            "{words}"
        );

        // A job that went well says nothing.
        fixture.job = Some(json!({ "phase": "done", "note": null, "skipped": [] }));
        assert!(!fixture.page(Category::Search).words().contains("Left out"));

        // No job at all shows nothing either.
        fixture.job = None;
        assert!(
            !fixture
                .page(Category::Search)
                .words()
                .contains("Embedding r")
        );
    }

    #[test]
    fn the_search_page_offers_no_built_in_model() {
        let doc = Fixture::new().page(Category::Search);
        let words = doc.words();
        assert!(!words.contains("Built-in model"), "{words}");
        assert!(!words.contains("Install"), "{words}");
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

    const AGENTS_LINE: &str = "Claude Code and Codex get no CodeWiki tools in these folders and everything below them. The app and the command line still work there.";

    fn missing_folder() -> PathBuf {
        std::env::temp_dir().join("codewiki-no-such-folder-for-a-test")
    }

    #[test]
    fn agents_card_lists_entries_with_remove() {
        let mut fixture = Fixture::new();
        let here = std::env::temp_dir();
        fixture.agents_off = Ok(vec![KeptFolder {
            path: here.clone(),
            missing: false,
        }]);
        let doc = fixture.page(Category::Repositories);
        let words = doc.words();
        assert!(words.contains("Kept from coding agents"), "{words}");
        assert!(words.contains(AGENTS_LINE), "{words}");
        assert!(
            words.contains(&format!("{} |  | Remove", here.display())),
            "{words}"
        );
        assert!(!words.contains("Folder not found"), "{words}");
        let actions = doc.actions();
        assert!(actions.contains(&Action::LetAgentsIn(here.display().to_string())));
        assert!(actions.contains(&Action::PickAgentsFolder));
        assert!(!actions.contains(&Action::ResetAgentsList));
    }

    #[test]
    fn agents_card_offers_add_folder_when_empty() {
        let mut fixture = Fixture::new();
        fixture.agents_off = Ok(Vec::new());
        // The card stands even where no repository is registered.
        fixture.repos.clear();
        let doc = fixture.page(Category::Repositories);
        let words = doc.words();
        assert!(words.contains("Kept from coding agents"), "{words}");
        assert!(words.contains(AGENTS_LINE), "{words}");
        assert!(words.contains("Add folder…"), "{words}");
        assert_eq!(doc.actions(), [Action::PickAgentsFolder]);
    }

    #[test]
    fn agents_card_shows_a_broken_list_with_reset() {
        let mut fixture = Fixture::new();
        let unreadable =
            cw_core::agents_off::unreadable(std::path::Path::new("C:/data/agents-off.json"));
        fixture.agents_off = Err(unreadable.clone());
        let doc = fixture.page(Category::Repositories);
        let words = doc.words();
        assert!(words.contains(&unreadable), "{words}");
        assert!(doc.actions().contains(&Action::ResetAgentsList));
        assert!(
            doc.actions()
                .iter()
                .all(|action| !matches!(action, Action::LetAgentsIn(_)))
        );
    }

    #[test]
    fn agents_card_marks_a_missing_folder() {
        let mut fixture = Fixture::new();
        let gone = missing_folder();
        fixture.agents_off = Ok(vec![KeptFolder {
            path: gone.clone(),
            missing: true,
        }]);
        let words = fixture.page(Category::Repositories).words();
        assert!(
            words.contains(&format!("{} | Folder not found | Remove", gone.display())),
            "{words}"
        );
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
            "Ctrl+= | Zoom in",
            "Ctrl+- | Zoom out",
            "Ctrl+0 | Back to 100%",
            "Ctrl+wheel | Zoom in or out",
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

    #[test]
    fn error_reports_are_off_until_the_user_turns_them_on() {
        let mut fixture = Fixture::new();
        let off = fixture.page(Category::Diagnostics);
        let words = off.words();
        assert!(words.contains("Send error reports: off"), "{words}");
        assert!(words.contains("Never a file path, note text, a search query or a key."));
        assert!(off.actions().contains(&Action::ErrorReports(true)));

        fixture.error_reports_on = true;
        let on = fixture.page(Category::Diagnostics);
        assert!(on.words().contains("Send error reports: on"));
        assert!(on.actions().contains(&Action::ErrorReports(false)));
    }

    #[test]
    fn error_reports_cannot_be_switched_where_the_build_or_the_environment_decides() {
        let mut fixture = Fixture::new();
        fixture.error_reports = Reporting::Unavailable;
        let doc = fixture.page(Category::Diagnostics);
        assert!(doc.words().contains("[This build has no error reporting]"));
        assert!(
            !doc.actions()
                .iter()
                .any(|a| matches!(a, Action::ErrorReports(_)))
        );

        fixture.error_reports = Reporting::Environment;
        let doc = fixture.page(Category::Diagnostics);
        assert!(doc.words().contains("Send error reports: on"));
        assert!(doc.words().contains("[Set by SENTRY_DSN]"));
    }
}
