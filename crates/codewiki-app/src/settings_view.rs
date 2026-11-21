//! The Settings page as the app's view draws and runs it.
//!
//! The page fills the window, as Home does: the categories at the left, the
//! page of the chosen one at the right. A category's page is a [`Doc`]
//! (`settings_pages.rs`), drawn here with `doc_view::column`, in this view
//! and not in the cached page view: its text inputs and the list of a select
//! take their events through this view, as the search box and the theme menu
//! do.
//!
//! A change takes effect when it is made. A switch, a select or a choice
//! acts on its click; a field that is typed is saved on Enter or when the
//! focus leaves it, and says "Saved" or why not beside itself. The one
//! exception is the backend on another machine, whose address and token go
//! together and are checked before they are used.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use cw_client::{Backend, Origin};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde_json::{Value, json};

use crate::doc::{Action, Choice};
use crate::doc_view::{self, Controls, SelectRequest};
use crate::mui::button::{Button, IconButton};
use crate::mui::feedback::{self, Tone};
use crate::mui::list::ListItemButton;
use crate::mui::menu::{Menu, MenuItem, MenuSection};
use crate::mui::motion::{NOTE_HOLD, scaled};
use crate::mui::tokens::Variant;
use crate::mui::{select, surfaces};
use crate::settings::{Category, FieldId, Note};
use crate::settings_layout as layout;
use crate::settings_pages::{self, Model, Tested};
use crate::zoom::px;
use crate::{AppView, Page, Tab, glass, icons, motion, prefs, themes, update, zoom};

/// The name the app's log level is saved under (`prefs.rs`).
pub const LOG_LEVEL_PREF: &str = "log_level";
/// How long the app waits before it asks again how far the embedding job is.
const JOB_PAUSE: std::time::Duration = std::time::Duration::from_secs(1);

/// The list of a select that is open.
struct OpenMenu {
    /// The path of the select's block, which tells it from the others.
    path: String,
    anchor: Bounds<Pixels>,
    options: Vec<Choice>,
    /// The row the keys act on.
    highlighted: usize,
}

/// What the Settings page remembers between frames.
pub struct State {
    /// The category shown last, which "Settings" opens again.
    last: Category,
    /// The backend's settings (`GET /v1/settings`), once they have arrived.
    backend: Option<Value>,
    /// Why they could not be read.
    error: Option<String>,
    /// The text input of each field. They live as long as the app.
    fields: HashMap<FieldId, Entity<InputState>>,
    /// What each field holds as far as the backend knows. A field whose
    /// input differs was typed into and is not saved yet.
    saved: HashMap<FieldId, String>,
    /// Inputs to fill or empty on the next frame, where there is a window.
    pending: HashMap<FieldId, String>,
    notes: HashMap<FieldId, Note>,
    /// Saved secrets whose input is open to take another in their place.
    typing: HashSet<FieldId>,
    embed_test: Tested,
    /// The embedding job (`GET /v1/embed/job`): the one that runs, or the
    /// last that ran. `None` when there is none, or on a backend elsewhere.
    embed_job: Option<Value>,
    /// Whether the loop that asks again while the job runs is going.
    embed_job_watched: bool,
    /// The news about the last try to connect to a backend elsewhere.
    connect: Option<Note>,
    menu: Option<OpenMenu>,
    /// Holds the focus while the list of a select is open.
    menu_focus: FocusHandle,
}

impl State {
    /// The state, and the subscriptions that keep it up to date; the view
    /// holds them for as long as it lives.
    pub fn new(window: &mut Window, cx: &mut Context<AppView>) -> (Self, Vec<Subscription>) {
        let mut fields = HashMap::new();
        let mut subscriptions = Vec::new();
        for id in FieldId::ALL {
            let input = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(id.placeholder())
                    .masked(id.secret())
            });
            subscriptions.push(cx.subscribe_in(
                &input,
                window,
                move |this, _, event: &InputEvent, _, cx| this.settings_field_event(id, event, cx),
            ));
            fields.insert(id, input);
        }
        // The offer of a built-in model on Home is gone, and so is its mark.
        prefs::remove("model_hint");
        let menu_focus = cx.focus_handle();
        // Tab moves the focus on from the list, which then closes.
        subscriptions.push(cx.on_blur(&menu_focus, window, |this, _, cx| {
            if this.settings.menu.take().is_some() {
                cx.notify();
            }
        }));
        let state = Self {
            last: Category::Appearance,
            backend: None,
            error: None,
            fields,
            saved: HashMap::new(),
            pending: HashMap::new(),
            notes: HashMap::new(),
            typing: HashSet::new(),
            embed_test: Tested::Not,
            embed_job: None,
            embed_job_watched: false,
            connect: None,
            menu: None,
            menu_focus,
        };
        (state, subscriptions)
    }
}

/// What a field shows for the setting `entry` describes: the value that is
/// saved or that the environment gives. A default is not typed in; the
/// input's placeholder stands for it.
fn shown_value(entry: &Value) -> String {
    match entry["source"].as_str() {
        Some("saved" | "env") => entry["value"].as_str().unwrap_or("").to_string(),
        _ => String::new(),
    }
}

/// The line a test of a provider left.
fn test_line(result: Result<Value, String>) -> Tested {
    let answer = match result {
        Ok(answer) => answer,
        Err(error) => {
            return Tested::Done {
                ok: false,
                text: error,
            };
        }
    };
    let name = answer["name"].as_str().unwrap_or("");
    if answer["ok"] == true {
        let mut text = format!("Works: {name}");
        if let Some(dimensions) = answer["dimensions"].as_u64() {
            text.push_str(&format!(", vectors of {dimensions}"));
        }
        if let Some(ms) = answer["ms"].as_u64() {
            text.push_str(&format!(", answered in {ms} ms"));
        }
        return Tested::Done { ok: true, text };
    }
    let error = answer["error"].as_str().unwrap_or("no reason was given");
    Tested::Done {
        ok: false,
        text: if name.is_empty() {
            error.to_string()
        } else {
            format!("{name}: {error}")
        },
    }
}

/// Whether the options of a select are the themes.
fn lists_themes(options: &[Choice]) -> bool {
    !options.is_empty()
        && options
            .iter()
            .all(|option| matches!(option.action, Action::Theme(_)))
}

/// The body of a `PUT /v1/settings` that changes one field.
fn change(section: &str, field: &str, value: Value) -> Value {
    json!({ section: { field: value } })
}

/// Whether a change touches the embedder, after which the job the app last
/// read may be out of date.
fn changes_the_embedder(body: &Value) -> bool {
    body.get("embed").is_some()
}

/// The job in the body of an answer from the job routes; `None` for `null`.
fn job_in(body: &Value) -> Option<Value> {
    Some(body["job"].clone()).filter(|job| !job.is_null())
}

/// Whether a job that ran has ended: the settings are read again then.
fn job_ended(before: Option<&Value>, after: Option<&Value>) -> bool {
    before.is_some_and(settings_pages::job_runs) && !after.is_some_and(settings_pages::job_runs)
}

impl AppView {
    pub(crate) fn on_settings_page(&self) -> bool {
        matches!(self.page, Page::Settings(_))
    }

    /// Show the settings: at `category`, or where they were left.
    pub(crate) fn open_settings(&mut self, category: Option<Category>, cx: &mut Context<Self>) {
        let category = category.unwrap_or(self.settings.last);
        if self.on_settings_page() {
            self.show_settings_category(category, cx);
            return;
        }
        self.set_page(Page::Settings(category), true);
        if !self.on_settings_page() {
            // Unsaved edits kept the page that was open.
            cx.notify();
            return;
        }
        self.settings.last = category;
        self.settings.connect = None;
        // The address of the saved backend, to see and to change.
        let saved = cw_client::saved_remote().map(|remote| remote.url);
        self.settings
            .pending
            .insert(FieldId::RemoteUrl, saved.unwrap_or_default());
        self.settings
            .pending
            .insert(FieldId::RemoteToken, String::new());
        self.load_settings(cx);
        if category == Category::Search {
            self.load_embed_job(cx);
        }
        cx.notify();
    }

    /// Go to another category. The page left is not kept in the history:
    /// Back leaves the settings in one step.
    fn show_settings_category(&mut self, category: Category, cx: &mut Context<Self>) {
        self.commit_settings_fields(cx);
        self.settings.menu = None;
        if category == Category::Search {
            // A Sync may have set a job going since.
            self.load_embed_job(cx);
        }
        self.settings.last = category;
        self.page = Page::Settings(category);
        self.page_serial += 1;
        cx.notify();
    }

    /// Leave the settings for the page before them, or for Home when the app
    /// started on them.
    pub(crate) fn leave_settings(&mut self, cx: &mut Context<Self>) {
        self.commit_settings_fields(cx);
        self.settings.menu = None;
        if self.history.is_empty() {
            self.set_page(Page::Home, false);
            cx.notify();
        } else {
            self.go_back(cx);
        }
    }

    /// Run a request to the backend off the UI thread and hand over what
    /// came back, an error included. Unlike `run`, it shows no progress bar
    /// and leaves the reporting to `done`: a setting says its own news.
    fn settings_request(
        &mut self,
        cx: &mut Context<Self>,
        work: impl FnOnce(&Backend) -> Result<Value, String> + Send + 'static,
        done: impl FnOnce(&mut Self, Result<Value, String>, &mut Context<Self>) + 'static,
    ) {
        let Some(backend) = self.backend.clone() else {
            done(
                self,
                Err("the app is not connected to a backend".into()),
                cx,
            );
            cx.notify();
            return;
        };
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { work(&backend) })
                .await;
            _ = this.update(cx, |this, cx| {
                done(this, result, cx);
                cx.notify();
            });
        })
        .detach();
    }

    /// Ask the backend for the embedding job (`GET /v1/embed/job`), and go on
    /// asking while it runs. A backend on another machine refuses to say, and
    /// is not asked.
    pub(crate) fn load_embed_job(&mut self, cx: &mut Context<Self>) {
        if self.backend.as_ref().is_none_or(Backend::is_remote) {
            self.settings.embed_job = None;
            return;
        }
        self.settings_request(
            cx,
            |backend| backend.get("/v1/embed/job"),
            |this, result, cx| match result {
                Ok(body) => this.take_embed_job(job_in(&body), cx),
                Err(error) => log::warn!("could not read the embedding job: {error}"),
            },
        );
    }

    /// Keep what the backend says about the job.
    fn take_embed_job(&mut self, job: Option<Value>, cx: &mut Context<Self>) {
        let ended = job_ended(self.settings.embed_job.as_ref(), job.as_ref());
        let runs = job.as_ref().is_some_and(settings_pages::job_runs);
        self.settings.embed_job = job;
        if runs {
            self.watch_embed_job(cx);
        }
        // A job that ended may have changed what the settings say.
        if ended {
            self.load_settings(cx);
        }
    }

    /// Ask for the job again in a moment. One such wait at a time.
    fn watch_embed_job(&mut self, cx: &mut Context<Self>) {
        if self.settings.embed_job_watched {
            return;
        }
        self.settings.embed_job_watched = true;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(JOB_PAUSE).await;
            _ = this.update(cx, |this, cx| {
                this.settings.embed_job_watched = false;
                this.load_embed_job(cx);
            });
        })
        .detach();
    }

    /// Ask the backend for its settings.
    fn load_settings(&mut self, cx: &mut Context<Self>) {
        self.settings_request(
            cx,
            |backend| backend.get("/v1/settings"),
            |this, result, _| match result {
                Ok(settings) => this.take_settings(settings),
                Err(error) => {
                    log::warn!("could not read the backend's settings: {error}");
                    this.settings.backend = None;
                    this.settings.error = Some(error);
                }
            },
        );
    }

    /// Keep what the backend says about its settings, and bring the inputs
    /// in line with it. An input the user is typing into is left alone.
    fn take_settings(&mut self, settings: Value) {
        for id in FieldId::ALL {
            let Some((section, field)) = id.place() else {
                continue;
            };
            if id.secret() {
                continue;
            }
            let value = shown_value(&settings[section][field]);
            self.settings.pending.insert(id, value.clone());
            self.settings.saved.insert(id, value);
        }
        self.settings.backend = Some(settings);
        self.settings.error = None;
    }

    /// Fill the inputs that wait for a value. Called while drawing, where
    /// there is a window. An input that holds something typed and not saved
    /// keeps it.
    pub(crate) fn sync_settings_fields(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings.pending.is_empty() {
            return;
        }
        for (id, value) in std::mem::take(&mut self.settings.pending) {
            let Some(input) = self.settings.fields.get(&id) else {
                continue;
            };
            let current = input.read(cx).value().to_string();
            let focused = input.read(cx).focus_handle(cx).is_focused(window);
            // The user is at work on a setting of the backend: what they
            // typed is saved when they leave the field.
            let typing = focused && id.place().is_some() && !id.secret();
            if current != value && !typing {
                input.update(cx, |input, cx| input.set_value(value, window, cx));
            }
        }
    }

    fn settings_field_event(&mut self, id: FieldId, event: &InputEvent, cx: &mut Context<Self>) {
        match event {
            // Typing takes away the news about what was there before.
            InputEvent::Change => {
                if self.settings.notes.remove(&id).is_some() {
                    cx.notify();
                }
            }
            InputEvent::PressEnter { .. } | InputEvent::Blur => self.commit_settings_field(id, cx),
            InputEvent::Focus => {}
        }
    }

    /// Save a field that was typed into, when what it holds is new.
    fn commit_settings_field(&mut self, id: FieldId, cx: &mut Context<Self>) {
        // The address and token of another backend wait for "Connect".
        let Some((section, field)) = id.place() else {
            return;
        };
        if !self.on_settings_page() || self.settings.backend.is_none() {
            return;
        }
        let Some(input) = self.settings.fields.get(&id) else {
            return;
        };
        let value = input.read(cx).value().trim().to_string();
        let body = if id.secret() {
            if value.is_empty() {
                return;
            }
            change(section, field, json!({ "set": value }))
        } else {
            if self
                .settings
                .saved
                .get(&id)
                .is_some_and(|saved| *saved == value)
            {
                return;
            }
            let value = if value.is_empty() {
                Value::Null
            } else {
                json!(value)
            };
            change(section, field, value)
        };
        self.put_settings(body, Some(id), cx);
    }

    /// Save every field of the page that was typed into. A field also saves
    /// itself when the focus leaves it; leaving the page must not depend on
    /// that, since the field is gone by then.
    fn commit_settings_fields(&mut self, cx: &mut Context<Self>) {
        for id in FieldId::ALL {
            self.commit_settings_field(id, cx);
        }
    }

    /// Send a change to the backend. `field` is the field to put the news
    /// beside; a change with none reports a failure at the top of the page.
    fn put_settings(&mut self, body: Value, field: Option<FieldId>, cx: &mut Context<Self>) {
        if let Some(id) = field {
            self.settings.notes.insert(id, Note::Working);
        }
        self.error = None;
        cx.notify();
        let embedder = changes_the_embedder(&body);
        self.settings_request(
            cx,
            move |backend| backend.put("/v1/settings", &body),
            move |this, result, cx| match (result, field) {
                (Ok(settings), field) => {
                    this.take_settings(settings);
                    if embedder {
                        // A new embedder may have set a job going.
                        this.load_embed_job(cx);
                    }
                    // What the embedder was tested as may no longer be what
                    // is chosen.
                    this.settings.embed_test = Tested::Not;
                    if let Some(id) = field {
                        this.settings.notes.insert(id, Note::Saved);
                        if id.secret() {
                            // The key is saved; nothing of it stays on screen.
                            this.settings.typing.remove(&id);
                            this.settings.pending.insert(id, String::new());
                        }
                        this.forget_saved_note(id, cx);
                    }
                }
                (Err(error), Some(id)) => {
                    this.settings.notes.insert(id, Note::Error(error));
                }
                (Err(error), None) => this.error = Some(error),
            },
        );
    }

    /// Take "Saved" away again after a while.
    fn forget_saved_note(&mut self, id: FieldId, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(scaled(NOTE_HOLD)).await;
            _ = this.update(cx, |this, cx| {
                if this.settings.notes.get(&id) == Some(&Note::Saved) {
                    this.settings.notes.remove(&id);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Send the chosen embedder one small request.
    fn test_embedder(&mut self, cx: &mut Context<Self>) {
        self.settings.embed_test = Tested::Running;
        cx.notify();
        self.settings_request(
            cx,
            |backend| backend.post("/v1/settings/test/embed", &json!({})),
            |this, result, _| this.settings.embed_test = test_line(result),
        );
    }

    /// Forget what the app holds of the backend it used, and connect anew:
    /// to the saved backend, or to the one on this machine.
    fn switch_backend(&mut self, cx: &mut Context<Self>) {
        self.backend = None;
        self.repo = None;
        self.repos.clear();
        self.missing.clear();
        self.notes.clear();
        self.notes_loaded = false;
        self.pending_proposals.clear();
        self.history.clear();
        self.tabs = vec![Tab {
            page: Page::Message(String::new()),
            history: Vec::new(),
        }];
        self.active_tab = 0;
        self.settings.backend = None;
        self.busy = Some("Connecting".into());
        self.error = None;
        self.set_page(Page::Message(String::new()), false);
        cx.notify();
        self.connect(cx);
    }

    /// Check the backend typed into the fields and, when it answers and
    /// takes the token, save it and use it.
    fn connect_remote(&mut self, cx: &mut Context<Self>) {
        let read = |id: FieldId| {
            self.settings
                .fields
                .get(&id)
                .map(|input| input.read(cx).value().trim().to_string())
                .unwrap_or_default()
        };
        let (url, token) = (read(FieldId::RemoteUrl), read(FieldId::RemoteToken));
        self.settings.connect = Some(Note::Working);
        cx.notify();
        // The backend on this machine has nobody left to stop it once the
        // app uses another.
        let local = self
            .backend
            .clone()
            .filter(|backend| backend.origin() == Origin::Local);
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let token = (!token.is_empty()).then_some(token.as_str());
                    cw_client::check_remote(&url, token)?;
                    cw_client::save_remote(&url, token)?;
                    if let Some(local) = local
                        && let Err(error) = local.shutdown()
                    {
                        log::warn!("could not ask the local backend to stop: {error}");
                    }
                    Ok::<_, String>(())
                })
                .await;
            _ = this.update(cx, |this, cx| match result {
                Ok(()) => {
                    log::info!("using the backend saved in the settings");
                    this.settings.connect = Some(Note::Saved);
                    this.switch_backend(cx);
                }
                Err(error) => {
                    log::warn!("the backend was not saved: {error}");
                    this.settings.connect = Some(Note::Error(error));
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Do what a control of the Settings page asks for. Returns false for an
    /// action that is not one of the settings'.
    pub(crate) fn run_settings_action(&mut self, action: &Action, cx: &mut Context<Self>) -> bool {
        match action {
            Action::Settings(category) => self.open_settings(*category, cx),
            Action::Theme(name) => self.select_theme(name, cx),
            Action::SetSetting {
                section,
                field,
                value,
            } => {
                let value = value.clone().map_or(Value::Null, Value::String);
                self.put_settings(change(section, field, value), None, cx);
            }
            Action::LogLevel(name) => {
                if let Some(level) = cw_core::logs::level_named(name) {
                    prefs::write(LOG_LEVEL_PREF, name);
                    cw_core::logs::set_level(level);
                    log::info!("the log level is now {name}");
                    cx.notify();
                }
            }
            Action::ReplaceSecret(id) => {
                self.settings.typing.insert(*id);
                self.settings.notes.remove(id);
                self.settings.pending.insert(*id, String::new());
                cx.notify();
            }
            Action::KeepSecret(id) => {
                self.settings.typing.remove(id);
                self.settings.notes.remove(id);
                self.settings.pending.insert(*id, String::new());
                cx.notify();
            }
            Action::RemoveSecret(id) => {
                if let Some((section, field)) = id.place() {
                    self.settings.typing.remove(id);
                    self.put_settings(
                        change(section, field, json!({ "remove": true })),
                        Some(*id),
                        cx,
                    );
                }
            }
            Action::TestEmbedder => self.test_embedder(cx),
            Action::ConnectRemote => self.connect_remote(cx),
            Action::UseLocalBackend => match cw_client::use_local() {
                Ok(()) => {
                    log::info!("going back to the backend on this machine");
                    self.settings.connect = None;
                    self.switch_backend(cx);
                }
                Err(error) => {
                    self.settings.connect = Some(Note::Error(error));
                    cx.notify();
                }
            },
            Action::ForgetRemote => {
                let in_use = cw_client::saved_remote().is_some_and(|remote| remote.in_use);
                match cw_client::forget_remote() {
                    Ok(()) => {
                        self.settings.connect = None;
                        self.settings
                            .pending
                            .insert(FieldId::RemoteUrl, String::new());
                        self.settings
                            .pending
                            .insert(FieldId::RemoteToken, String::new());
                        if in_use && cw_client::env_remote().is_none() {
                            self.switch_backend(cx);
                        }
                    }
                    Err(error) => self.settings.connect = Some(Note::Error(error)),
                }
                cx.notify();
            }
            Action::CancelEmbedJob => {
                log::info!("cancelling the embedding job");
                self.settings_request(
                    cx,
                    |backend| backend.post("/v1/embed/cancel", &json!({})),
                    |this, result, cx| match result {
                        Ok(body) => this.take_embed_job(job_in(&body), cx),
                        Err(error) => {
                            log::warn!("the backend refused to cancel the job: {error}");
                            this.error = Some(error);
                        }
                    },
                );
            }
            Action::OpenDataFolder => {
                let dir = cw_core::paths::data_dir();
                log::info!("opening the data folder {}", dir.display());
                cx.open_with_system(&dir);
            }
            _ => return false,
        }
        true
    }

    /// Open the list of the select that was clicked, or close it when it is
    /// the one that is open.
    fn open_settings_menu(
        &mut self,
        request: SelectRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .settings
            .menu
            .as_ref()
            .is_some_and(|menu| menu.path == request.path)
        {
            self.close_settings_menu(window, cx);
            return;
        }
        let highlighted = request
            .options
            .iter()
            .position(|option| option.selected)
            .unwrap_or(0);
        self.settings.menu = Some(OpenMenu {
            path: request.path,
            anchor: request.anchor,
            options: request.options,
            highlighted,
        });
        window.focus(&self.settings.menu_focus, cx);
        cx.notify();
    }

    pub(crate) fn close_settings_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings.menu.take().is_some() {
            window.focus(&self.focus, cx);
            cx.notify();
        }
    }

    /// The list of the open select, when one is open.
    pub(crate) fn settings_menu_view(&self, cx: &mut Context<Self>) -> Option<Menu> {
        let menu = self.settings.menu.as_ref()?;
        let themes_listed = lists_themes(&menu.options);
        let sections: Vec<MenuSection> = if themes_listed {
            // As the toolbar's theme menu lists them: dark, then light, each
            // with its icon, its colors, and a mark on the one in use. The
            // options are in that order already.
            let active = cx.theme().theme_name().clone();
            let entries = themes::entries(cx);
            [(true, "Dark"), (false, "Light")]
                .into_iter()
                .filter_map(|(dark, label)| {
                    let items: Vec<MenuItem> = menu
                        .options
                        .iter()
                        .filter_map(|option| {
                            let Action::Theme(name) = &option.action else {
                                return None;
                            };
                            let entry = entries.iter().find(|entry| entry.name == *name)?;
                            (entry.dark == dark)
                                .then(|| crate::theme_menu_item(entry, entry.name == active, cx))
                        })
                        .collect();
                    (!items.is_empty()).then(|| MenuSection::new(label).items(items))
                })
                .collect()
        } else {
            let items = menu.options.iter().map(|option| {
                let item = MenuItem::new(option.label.clone()).selected(option.selected);
                match option.icon {
                    Some(icon) => item.icon(icon).tint(doc_view::choice_tint(option, cx)),
                    None => item,
                }
            });
            vec![MenuSection::plain().items(items)]
        };
        let (select, highlight, close) = (
            cx.entity().downgrade(),
            cx.entity().downgrade(),
            cx.entity().downgrade(),
        );
        Some(
            select::menu(
                "settings-select",
                menu.anchor,
                &self.settings.menu_focus,
                sections,
            )
            .highlighted(menu.highlighted)
            .on_select(move |index, window, cx| {
                _ = select.update(cx, |this, cx| {
                    let action = this
                        .settings
                        .menu
                        .as_ref()
                        .and_then(|menu| menu.options.get(index))
                        .map(|option| option.action.clone());
                    this.close_settings_menu(window, cx);
                    if let Some(action) = action {
                        this.run_action(&action, cx);
                    }
                });
            })
            .on_highlight(move |index, _, cx| {
                _ = highlight.update(cx, |this, cx| {
                    if let Some(menu) = &mut this.settings.menu {
                        menu.highlighted = index;
                        cx.notify();
                    }
                });
            })
            .on_close(move |window, cx| {
                _ = close.update(cx, |this, cx| this.close_settings_menu(window, cx));
            }),
        )
    }

    /// The button that opens the settings, for the foot of the sidebar and
    /// of Home: the settings icon and the word.
    pub(crate) fn settings_entry(&self, cx: &mut Context<Self>) -> Div {
        div().p(px(layout::ENTRY_PADDING)).child(
            ListItemButton::new("open-settings", "Settings")
                .icon(icons::SETTINGS)
                .tint(icons::tint(icons::SETTINGS, cx))
                .on_click(cx.listener(|this, _, _, cx| this.open_settings(None, cx))),
        )
    }

    /// The Settings page: everything below the title bar.
    pub(crate) fn settings_frame(
        &mut self,
        category: Category,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let doc = {
            let theme = cx.theme().theme_name().to_string();
            let entries = themes::entries(cx);
            // Dark themes first, as in the theme menu.
            let themes: Vec<String> = [true, false]
                .into_iter()
                .flat_map(|dark| {
                    entries
                        .iter()
                        .filter(move |entry| entry.dark == dark)
                        .map(|entry| entry.name.to_string())
                })
                .collect();
            let repos: Vec<(String, String)> = self
                .repos
                .iter()
                .map(|repo| (repo.name.clone(), repo.path.clone()))
                .collect();
            let saved_remote = cw_client::saved_remote();
            let env_url = cw_client::env_remote();
            let logs_dir = cw_core::logs::dir().display().to_string();
            let data_dir = cw_core::paths::data_dir().display().to_string();
            let level = cw_core::logs::level_name(cw_core::logs::current_level());
            settings_pages::page(
                category,
                &Model {
                    theme: &theme,
                    themes: &themes,
                    motion: motion::current(cx),
                    animations_on: !cx.reduce_motion(),
                    zoom: zoom::percent(cx),
                    settings: self.settings.backend.as_ref(),
                    settings_error: self.settings.error.as_deref(),
                    notes: &self.settings.notes,
                    typing: &self.settings.typing,
                    embed_test: &self.settings.embed_test,
                    job: self.settings.embed_job.as_ref(),
                    backend_url: self.backend.as_ref().map(Backend::url),
                    origin: self.backend.as_ref().map(Backend::origin),
                    env_url: env_url.as_deref(),
                    saved_remote: saved_remote.as_ref(),
                    connect: self.settings.connect.as_ref(),
                    repos: &repos,
                    update: &self.update,
                    auto_update: update::auto_check(),
                    autostart: crate::autostart::enabled(),
                    open_last: crate::startup::open_last(),
                    log_level: level,
                    log_level_locked: cw_core::logs::level_in_env().is_some(),
                    logs_dir: &logs_dir,
                    data_dir: &data_dir,
                    error_reports: cw_telemetry::reporting(),
                    error_reports_on: cw_telemetry::consent(),
                },
            )
        };

        let (act, open) = (cx.entity().downgrade(), cx.entity().downgrade());
        let on_action: doc_view::OnAction = Rc::new(move |action, _, cx| {
            let action = action.clone();
            _ = act.update(cx, |this, cx| this.run_action(&action, cx));
        });
        let controls = Rc::new(Controls {
            fields: self.settings.fields.clone(),
            open_select: self.settings.menu.as_ref().map(|menu| menu.path.clone()),
            on_select: Rc::new(move |request, window, cx| {
                _ = open.update(cx, |this, cx| this.open_settings_menu(request, window, cx));
            }),
        });
        let page = doc_view::column(&doc, self.page_serial, on_action, controls, window, cx);

        let theme = cx.theme();
        let categories = v_flex()
            .w(px(layout::CATEGORY_LIST_WIDTH))
            .h_full()
            .flex_shrink_0()
            .border_r_1()
            .border_color(glass::tinted_border(cx))
            .bg(self.glass.chrome(glass::tinted(theme.sidebar, cx)))
            .text_color(theme.sidebar_foreground)
            .child(
                h_flex()
                    .h(px(layout::HEADER_HEIGHT))
                    .px(px(layout::CATEGORY_LIST_PADDING))
                    .gap(px(4.))
                    .items_center()
                    .child(
                        IconButton::new("settings-back", icons::BACK)
                            .tooltip("Back (Alt+Left)")
                            .on_click(cx.listener(|this, _, _, cx| this.leave_settings(cx))),
                    )
                    .child(surfaces::text(Variant::H4, "Settings")),
            )
            .child(
                div()
                    .id("settings-categories")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(
                        v_flex()
                            .px(px(layout::CATEGORY_LIST_PADDING))
                            .pb(px(layout::CATEGORY_LIST_PADDING))
                            .children(Category::ALL.into_iter().map(|item| {
                                ListItemButton::new(
                                    SharedString::from(format!("settings:{}", item.slug())),
                                    item.label(),
                                )
                                .icon(item.icon())
                                .tint(icons::tint(item.icon(), cx))
                                .selected(item == category)
                                .on_click(cx.listener(
                                    move |this, _, _, cx| this.show_settings_category(item, cx),
                                ))
                            })),
                    ),
            );

        let error = self.error.clone();
        let retry = self.settings.error.is_some();
        h_flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .items_start()
            .child(categories)
            .child(
                div()
                    // A category starts at its top.
                    .id(SharedString::from(format!(
                        "settings-pane:{}",
                        category.slug()
                    )))
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_y_scrollbar()
                    .child(
                        v_flex()
                            .max_w(px(layout::PANE_MEASURE + 2.0 * layout::PANE_PADDING))
                            .p(px(layout::PANE_PADDING))
                            .gap(px(16.))
                            .when_some(error, |pane, error| {
                                pane.child(feedback::alert(Tone::Error, error, cx))
                            })
                            .child(page)
                            .when(retry, |pane| {
                                pane.child(div().child(
                                    Button::new("settings-reload").label("Try again").on_click(
                                        cx.listener(|this, _, _, cx| this.load_settings(cx)),
                                    ),
                                ))
                            }),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the kit's glob brings in GPUI's own `test` attribute,
    // which would stand in for the standard one.
    use serde_json::{Value, json};

    use super::{change, changes_the_embedder, job_ended, job_in, shown_value, test_line};
    use crate::settings_pages::Tested;

    #[test]
    fn the_job_is_what_the_backend_names_or_nothing() {
        assert_eq!(job_in(&json!({ "job": null })), None);
        assert_eq!(job_in(&json!({})), None);
        let job = json!({ "phase": "embedding", "repo": "r" });
        assert_eq!(job_in(&json!({ "job": job })), Some(job));
    }

    #[test]
    fn the_settings_are_read_again_only_when_a_job_ends() {
        let running = json!({ "phase": "embedding" });
        let done = json!({ "phase": "done" });
        assert!(job_ended(Some(&running), Some(&done)));
        assert!(job_ended(Some(&running), None));
        assert!(!job_ended(Some(&running), Some(&running)));
        assert!(!job_ended(Some(&done), Some(&done)));
        assert!(!job_ended(None, Some(&running)));
        assert!(!job_ended(None, None));
    }

    #[test]
    fn a_change_of_the_embedder_reads_the_job_again() {
        assert!(changes_the_embedder(&change(
            "embed",
            "provider",
            json!("local")
        )));
        // An address it may read is not the embedder itself.
        assert!(!changes_the_embedder(&change(
            "ollama",
            "host",
            json!("http://nas:11434")
        )));
    }

    #[test]
    fn a_field_shows_what_is_saved_or_set_and_not_a_default() {
        assert_eq!(
            shown_value(&json!({ "value": "llama", "source": "saved" })),
            "llama"
        );
        assert_eq!(
            shown_value(&json!({ "value": "gpt", "source": "env" })),
            "gpt"
        );
        // The placeholder stands for a default.
        assert_eq!(
            shown_value(&json!({ "value": "https://api.openai.com/v1", "source": "default" })),
            ""
        );
        assert_eq!(
            shown_value(&json!({ "value": null, "source": "default" })),
            ""
        );
        assert_eq!(shown_value(&Value::Null), "");
    }

    #[test]
    fn a_test_says_what_answered_or_why_not() {
        assert_eq!(
            test_line(Ok(json!({ "ok": true, "name": "ollama:llama", "ms": 812 }))),
            Tested::Done {
                ok: true,
                text: "Works: ollama:llama, answered in 812 ms".into()
            }
        );
        assert_eq!(
            test_line(Ok(
                json!({ "ok": true, "name": "local-hash-v1:384", "dimensions": 384, "ms": 2 })
            )),
            Tested::Done {
                ok: true,
                text: "Works: local-hash-v1:384, vectors of 384, answered in 2 ms".into()
            }
        );
        assert_eq!(
            test_line(Ok(
                json!({ "ok": false, "name": "openai:gpt", "error": "HTTP 401: Incorrect API key" })
            )),
            Tested::Done {
                ok: false,
                text: "openai:gpt: HTTP 401: Incorrect API key".into()
            }
        );
        assert_eq!(
            test_line(Ok(
                json!({ "ok": false, "name": null, "error": "no provider is chosen" })
            )),
            Tested::Done {
                ok: false,
                text: "no provider is chosen".into()
            }
        );
        // The backend itself did not answer.
        assert_eq!(
            test_line(Err("backend request failed".into())),
            Tested::Done {
                ok: false,
                text: "backend request failed".into()
            }
        );
    }

    #[test]
    fn a_change_names_one_field_of_one_section() {
        assert_eq!(
            change("embed", "provider", json!("ollama")),
            json!({ "embed": { "provider": "ollama" } })
        );
        assert_eq!(
            change("openai", "api_key", json!({ "remove": true })),
            json!({ "openai": { "api_key": { "remove": true } } })
        );
        assert_eq!(
            change("embed", "model", Value::Null),
            json!({ "embed": { "model": null } })
        );
    }
}
