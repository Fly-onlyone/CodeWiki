//! CodeWiki desktop app.
//!
//! A native viewer for the vault and the live index. All data comes from the
//! backend over HTTP; this binary links none of the engine crates.

// Release builds are a GUI app: no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backdrop;
mod contrast;
mod editor;
mod fonts;
mod glass;
mod graph;
mod icons;
mod illustration;
mod markdown;
mod motion;
mod pages;
mod palette;
mod prefs;
mod reading;
mod themes;

use std::collections::BTreeMap;

use cw_client::{Backend, encode};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{
    Editor, EditorState, Escape, Input, InputEvent, InputState, MoveDown, MoveUp,
};
use gpui_kit::component::sidebar::{
    Sidebar, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem,
};
use gpui_kit::component::tab::{Tab as TabItem, TabBar};
use gpui_kit::component::text::TextView;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Sizable as _, StyledExt as _, h_flex, v_flex,
};
use gpui_kit::component::{Icon, TitleBar};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde_json::{Value, json};

use markdown::{FILE_SCHEME, NOTE_SCHEME, SETTING_SCHEME, SYMBOL_SCHEME, THEME_SCHEME};
use motion::MotionPref;
use palette::Command;

actions!(
    codewiki,
    [TogglePalette, NewTab, CloseTab, NextTab, GoBack, SaveNote]
);

/// Most entries the palette lists at once.
const PALETTE_ROWS: usize = 14;

#[derive(Clone)]
struct NoteInfo {
    path: String,
    title: String,
    state: String,
}

#[derive(Clone)]
struct Repo {
    id: String,
    name: String,
    /// Root folder as the backend sees it.
    path: String,
}

/// A tab that is not on screen. The active tab's page and history live in
/// the view itself; its slot is refreshed when the user leaves it.
#[derive(Clone)]
struct Tab {
    page: Page,
    history: Vec<Page>,
    show_figure: bool,
}

/// A note being edited.
struct Editing {
    path: String,
    /// The note's text when editing began, with its own line endings.
    original: String,
    state: Entity<EditorState>,
    dirty: bool,
    _subscription: Subscription,
}

/// What the centre pane shows.
#[derive(Clone)]
enum Page {
    /// Nothing loaded yet, or a message for the user.
    Message(String),
    Note {
        path: String,
        text: String,
        sources: Vec<String>,
        state: String,
        /// The note's illustration, when it has a valid one.
        figure: Option<openui_lang::Node>,
    },
    /// Symbol detail, search results and other generated pages.
    Generated {
        title: String,
        markdown: String,
    },
    Proposal {
        id: String,
        markdown: String,
    },
    /// The dependency graph canvas.
    Graph,
}

struct AppView {
    backend: Option<Backend>,
    repos: Vec<Repo>,
    repo: Option<Repo>,
    notes: Vec<NoteInfo>,
    pending_proposals: Vec<(String, String)>,
    page: Page,
    /// Pages visited before the current one.
    history: Vec<Page>,
    tabs: Vec<Tab>,
    active_tab: usize,
    palette_open: bool,
    palette_input: Entity<InputState>,
    /// Index of the highlighted palette entry.
    palette_selected: usize,
    /// Palette query to open with on the next frame; set by a startup argument.
    pending_palette: Option<String>,
    /// Command to run on the next frame; set by a startup argument.
    pending_command: Option<Command>,
    /// Open the editor once the first note has loaded; set by a startup argument.
    pending_edit: bool,
    focus: FocusHandle,
    /// Set while the current note is open in the editor.
    editing: Option<Editing>,
    busy: Option<String>,
    error: Option<String>,
    search: Entity<InputState>,
    /// Bumped on every page change so the viewer rebuilds its state.
    page_serial: usize,
    /// Markdown of the current page, ready for the viewer.
    rendered: String,
    /// Diagrams of the current page, addressed by index from `rendered`.
    diagrams: Vec<std::sync::Arc<Image>>,
    graph: Option<std::rc::Rc<graph::GraphData>>,
    /// Graph camera; `None` until the view has been fitted once.
    camera: Option<graph::Camera>,
    /// Canvas origin and size from the last paint: x, y, width, height.
    graph_bounds: std::rc::Rc<std::cell::Cell<[f32; 4]>>,
    paint_ms: std::rc::Rc<std::cell::Cell<f32>>,
    frame_ms: std::rc::Rc<std::cell::Cell<f32>>,
    last_frame: std::rc::Rc<std::cell::Cell<Option<std::time::Instant>>>,
    drag_last: Option<(f32, f32)>,
    drag_moved: bool,
    /// Illustration of the current note, when it has a valid one.
    figure: Option<openui_lang::Node>,
    /// Whether the centre pane shows the illustration instead of the text.
    show_figure: bool,
    /// Palette diagrams were last drawn with; redrawn when the theme flips.
    palette: cw_mermaid::Palette,
    /// How see-through the surfaces are; worked out again on every frame.
    glass: glass::Levels,
    /// Width of the centre pane in pixels, from the window size of this frame.
    pane_width: f32,
    _subscriptions: Vec<Subscription>,
}

/// Width of the sidebar and of the details pane, in pixels.
const SIDEBAR_WIDTH: f32 = 280.0;
const DETAILS_WIDTH: f32 = 260.0;

/// Repository to open at startup: the first argument, else the working directory.
fn startup_repo() -> Option<String> {
    std::env::args().nth(1).or_else(|| {
        std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().to_string())
    })
}

impl AppView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search notes, symbols, files…"));
        let subscription =
            cx.subscribe_in(&search, window, |this, input, event: &InputEvent, _, cx| {
                if let InputEvent::PressEnter { .. } = event {
                    let query = input.read(cx).value().to_string();
                    this.run_search(query, cx);
                }
            });
        let palette_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Type a note name or a command…"));
        let palette_subscription = cx.subscribe_in(
            &palette_input,
            window,
            |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::Change => {
                    this.palette_selected = 0;
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => this.run_selected(window, cx),
                _ => {}
            },
        );
        // The glass depends on the system mode as well as on the theme.
        let appearance = cx.observe_window_appearance(window, |_, _, cx| cx.notify());
        let mut this = Self {
            backend: None,
            repos: Vec::new(),
            repo: None,
            notes: Vec::new(),
            pending_proposals: Vec::new(),
            page: Page::Message(String::new()),
            history: Vec::new(),
            tabs: vec![Tab {
                page: Page::Message(String::new()),
                history: Vec::new(),
                show_figure: false,
            }],
            active_tab: 0,
            palette_open: false,
            palette_input,
            palette_selected: 0,
            pending_palette: None,
            pending_command: None,
            pending_edit: false,
            focus: cx.focus_handle(),
            editing: None,
            busy: Some("Connecting".into()),
            error: None,
            search,
            page_serial: 0,
            rendered: String::new(),
            diagrams: Vec::new(),
            graph: None,
            camera: None,
            graph_bounds: Default::default(),
            paint_ms: Default::default(),
            frame_ms: Default::default(),
            last_frame: Default::default(),
            drag_last: None,
            drag_moved: false,
            figure: None,
            show_figure: false,
            palette: cw_mermaid::Palette::Dark,
            glass: glass::Levels::OPAQUE,
            pane_width: 0.0,
            _subscriptions: vec![subscription, palette_subscription, appearance],
        };
        this.connect(cx);
        this
    }

    /// Run blocking backend work off the UI thread, then apply the result.
    fn run<T: Send + 'static>(
        &mut self,
        label: &str,
        cx: &mut Context<Self>,
        work: impl FnOnce(&Backend) -> Result<T, String> + Send + 'static,
        done: impl FnOnce(&mut Self, T, &mut Context<Self>) + 'static,
    ) {
        let Some(backend) = self.backend.clone() else {
            return;
        };
        self.busy = Some(label.to_string());
        self.error = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { work(&backend) })
                .await;
            _ = this.update(cx, |this, cx| {
                this.busy = None;
                match result {
                    Ok(value) => done(this, value, cx),
                    Err(message) => this.error = Some(message),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn connect(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let backend = Backend::connect()?;
                    // Register the startup folder; failure here is not fatal,
                    // the user may have other repositories.
                    let opened = startup_repo().and_then(|path| backend.repo_id(&path).ok());
                    let repos = backend.get("/v1/repos")?;
                    Ok::<_, String>((backend, repos, opened))
                })
                .await;
            _ = this.update(cx, |this, cx| {
                this.busy = None;
                match result {
                    Ok((backend, repos, opened)) => {
                        this.backend = Some(backend);
                        this.repos = parse_repos(&repos);
                        let first = opened
                            .and_then(|id| this.repos.iter().find(|r| r.id == id).cloned())
                            .or_else(|| this.repos.first().cloned());
                        match first {
                            Some(repo) => this.select_repo(repo, cx),
                            None => {
                                this.set_page(Page::Message("No repository yet. Start the app from a repository folder, or pass its path as the first argument.".into()), false);
                            }
                        }
                    }
                    Err(message) => {
                        this.error = Some(message);
                        this.set_page(Page::Message("The backend is not reachable.".into()), false);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn select_repo(&mut self, repo: Repo, cx: &mut Context<Self>) {
        if !self.can_leave() {
            cx.notify();
            return;
        }
        self.repo = Some(repo);
        self.history.clear();
        self.tabs.truncate(1);
        self.active_tab = 0;
        self.refresh(true, cx);
    }

    /// Reload the note list and proposals. `open_home` also shows `_HOME`.
    fn refresh(&mut self, open_home: bool, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        self.run(
            "Loading",
            cx,
            move |b| {
                let notes = b.get(&format!("/v1/repos/{}/notes", repo.id))?;
                let proposals = b.get(&format!("/v1/repos/{}/proposals", repo.id))?;
                Ok((notes, proposals))
            },
            move |this, (notes, proposals), cx| {
                this.notes = parse_notes(&notes);
                this.pending_proposals = proposals
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|p| p["status"] == "pending")
                    .filter_map(|p| {
                        Some((
                            p["id"].as_str()?.to_string(),
                            p["note_path"].as_str()?.to_string(),
                        ))
                    })
                    .collect();
                if this.notes.is_empty() {
                    this.set_page(
                        Page::Message(
                            "This repository has no wiki yet. Press Sync to generate it.".into(),
                        ),
                        false,
                    );
                } else if open_home {
                    // A second argument names the note to show first, so other
                    // tools can open the app at a specific note.
                    let first = std::env::args().nth(2).unwrap_or_else(|| "_HOME".into());
                    if first == "graph:" {
                        this.open_graph(cx);
                        return;
                    }
                    // Forms used to open the app on a view from a script.
                    if let Some(note) = first.strip_prefix("edit:") {
                        this.pending_edit = true;
                        this.open_note(note.to_string(), false, cx);
                        return;
                    }
                    if let Some(query) = first.strip_prefix("palette:") {
                        this.pending_palette = Some(query.to_string());
                        this.open_note("_HOME".into(), false, cx);
                        return;
                    }
                    let command = match first.as_str() {
                        "settings:" => Some(Command::Settings),
                        "themes:" => Some(Command::Themes),
                        "decisions:" => Some(Command::Decisions),
                        "health:" => Some(Command::Health),
                        "dead-code:" => Some(Command::DeadCode),
                        "drift:" => Some(Command::Drift),
                        "tests:" => Some(Command::Tests),
                        "ownership:" => Some(Command::Ownership),
                        "coupling:" => Some(Command::Coupling),
                        "workspace:" => Some(Command::Workspace),
                        "contracts:" => Some(Command::Contracts),
                        "coverage:" => Some(Command::Coverage),
                        "security:" => Some(Command::Security),
                        "refactor:" => Some(Command::Refactor),
                        _ => None,
                    };
                    if let Some(command) = command {
                        this.pending_command = Some(command);
                        cx.notify();
                        return;
                    }
                    match first.strip_prefix("file:") {
                        Some(path) => this.open_file(path.to_string(), cx),
                        None => this.open_note(first, false, cx),
                    }
                }
            },
        );
    }

    /// Close the editor before leaving the page. Returns false, with a
    /// message, when unsaved edits would be lost.
    fn can_leave(&mut self) -> bool {
        if self.editing.as_ref().is_some_and(|e| e.dirty) {
            self.error = Some("Save or discard your edits first.".into());
            return false;
        }
        self.editing = None;
        true
    }

    fn start_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Page::Note { path, text, .. } = &self.page else {
            return;
        };
        let (path, original) = (path.clone(), text.clone());
        let value = original.replace("\r\n", "\n");
        let state = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("markdown")
                .soft_wrap(true)
                .default_value(value)
        });
        let subscription = cx.subscribe(&state, |this, _, event: &InputEvent, cx| {
            if let InputEvent::Change = event
                && let Some(editing) = this.editing.as_mut()
            {
                editing.dirty = true;
            }
            cx.notify();
        });
        state.update(cx, |state, cx| state.focus(window, cx));
        self.show_figure = false;
        self.editing = Some(Editing {
            path,
            original,
            state,
            dirty: false,
            _subscription: subscription,
        });
        cx.notify();
    }

    fn discard_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editing = None;
        self.error = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// Write the edited note. The backend then treats it as edited by hand,
    /// so later syncs propose changes instead of overwriting it.
    fn save_edit(&mut self, cx: &mut Context<Self>) {
        let (Some(editing), Some(repo)) = (self.editing.as_ref(), self.repo.clone()) else {
            return;
        };
        if !editing.dirty {
            return;
        }
        let text = editor::restore_line_endings(&editing.original, &editing.state.read(cx).value());
        let path = editing.path.clone();
        let reopen = path.clone();
        self.run(
            "Saving note",
            cx,
            move |b| {
                b.put(
                    &format!("/v1/repos/{}/note?path={}", repo.id, encode(&path)),
                    &json!({ "text": text }),
                )
            },
            move |this, _, cx| {
                this.editing = None;
                this.refresh(false, cx);
                this.open_note(reopen, false, cx);
            },
        );
    }

    fn set_page(&mut self, page: Page, remember: bool) {
        if !self.can_leave() {
            return;
        }
        let previous = std::mem::replace(&mut self.page, page);
        if remember && !matches!(previous, Page::Message(_)) {
            self.history.push(previous);
            if self.history.len() > 50 {
                self.history.remove(0);
            }
        }
        self.show_page();
    }

    /// Rebuild the viewer for `self.page` after it changed.
    fn show_page(&mut self) {
        self.page_serial += 1;
        self.figure = match &self.page {
            Page::Note { figure, .. } => figure.clone(),
            _ => None,
        };
        self.show_figure = false;
        self.prepare_page();
    }

    fn stash_tab(&mut self) {
        self.tabs[self.active_tab] = Tab {
            page: self.page.clone(),
            history: std::mem::take(&mut self.history),
            show_figure: self.show_figure,
        };
    }

    fn load_tab(&mut self) {
        let tab = &mut self.tabs[self.active_tab];
        self.page = tab.page.clone();
        self.history = std::mem::take(&mut tab.history);
        let show_figure = tab.show_figure;
        self.show_page();
        self.show_figure = show_figure && self.figure.is_some();
    }

    fn activate_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index == self.active_tab || index >= self.tabs.len() {
            return;
        }
        if !self.can_leave() {
            cx.notify();
            return;
        }
        self.stash_tab();
        self.active_tab = index;
        self.load_tab();
        cx.notify();
    }

    /// Open a new tab on the current page, so one copy can stay put while
    /// the other navigates.
    fn new_tab(&mut self, cx: &mut Context<Self>) {
        if !self.can_leave() {
            cx.notify();
            return;
        }
        self.stash_tab();
        self.tabs.push(Tab {
            page: self.page.clone(),
            history: Vec::new(),
            show_figure: false,
        });
        self.active_tab = self.tabs.len() - 1;
        self.load_tab();
        cx.notify();
    }

    fn close_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.tabs.len() <= 1 || index >= self.tabs.len() {
            return;
        }
        if index == self.active_tab && !self.can_leave() {
            cx.notify();
            return;
        }
        self.tabs.remove(index);
        if index == self.active_tab {
            self.active_tab = index.min(self.tabs.len() - 1);
            self.load_tab();
        } else if index < self.active_tab {
            self.active_tab -= 1;
        }
        cx.notify();
    }

    fn next_tab(&mut self, cx: &mut Context<Self>) {
        self.activate_tab((self.active_tab + 1) % self.tabs.len(), cx);
    }

    fn toggle_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette_open = !self.palette_open;
        self.palette_selected = 0;
        if self.palette_open {
            self.palette_input.update(cx, |input, cx| {
                input.set_value("", window, cx);
                input.focus(window, cx);
            });
        } else {
            window.focus(&self.focus, cx);
        }
        cx.notify();
    }

    /// Entries the palette lists for what is typed now.
    fn palette_matches(&self, cx: &App) -> Vec<palette::Entry> {
        let mut entries = palette::actions();
        for note in &self.notes {
            let label = match note.title.as_str() {
                "_HOME" => "Home".to_string(),
                "_index" => {
                    let folder = note.path.rsplit('/').nth(1).unwrap_or("Vault");
                    format!("{} overview", zone_label(folder))
                }
                title => title.to_string(),
            };
            entries.push(palette::Entry::new(
                label,
                "Note",
                Command::Note(note.path.clone()),
            ));
        }
        for (name, _) in themes::list(cx) {
            entries.push(palette::Entry::new(
                format!("Theme: {name}"),
                "Theme",
                Command::Theme(name),
            ));
        }
        if self.repos.len() > 1 {
            for repo in &self.repos {
                entries.push(palette::Entry::new(
                    format!("Switch to {}", repo.name),
                    "Repository",
                    Command::Repo(repo.id.clone()),
                ));
            }
        }
        let query = self.palette_input.read(cx).value().to_string();
        palette::filter(&entries, &query, PALETTE_ROWS)
    }

    fn run_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let entries = self.palette_matches(cx);
        let index = self.palette_selected.min(entries.len().saturating_sub(1));
        if let Some(entry) = entries.into_iter().nth(index) {
            self.run_command(entry.command, window, cx);
        }
    }

    fn run_command(&mut self, command: Command, window: &mut Window, cx: &mut Context<Self>) {
        if self.palette_open {
            self.toggle_palette(window, cx);
        }
        match command {
            Command::Note(path) => self.open_note(path, true, cx),
            Command::Theme(name) => {
                themes::select(&name, cx);
                cx.notify();
            }
            Command::Repo(id) => {
                if let Some(repo) = self.repos.iter().find(|r| r.id == id).cloned() {
                    self.select_repo(repo, cx);
                }
            }
            Command::Search(query) => self.run_search(query, cx),
            Command::Sync => self.sync(cx),
            Command::Graph => self.open_graph(cx),
            Command::Hotspots => self.open_hotspots(cx),
            Command::Decisions => {
                self.open_report("Decisions", "decisions", pages::decisions_page, cx)
            }
            Command::DeadCode => {
                self.open_report("Dead code", "dead-code", pages::dead_code_page, cx)
            }
            Command::Health => self.open_report("Code health", "health", pages::health_page, cx),
            Command::Drift => {
                self.open_report("Documentation drift", "doc-drift", pages::drift_page, cx)
            }
            Command::Tests => self.open_report("Tests", "tests", pages::tests_page, cx),
            Command::Coverage => self.open_report("Coverage", "coverage", pages::coverage_page, cx),
            Command::Security => self.open_report("Security", "security", pages::security_page, cx),
            Command::Refactor => {
                self.open_report("Refactoring plan", "refactor", pages::refactor_page, cx)
            }
            Command::Workspace => {
                self.open_global("Workspace", "/v1/workspace", pages::workspace_page, cx)
            }
            Command::Contracts => self.open_global(
                "API contracts",
                "/v1/workspace/contracts",
                pages::contracts_page,
                cx,
            ),
            Command::Ownership => {
                self.open_report("Ownership", "history", pages::ownership_page, cx)
            }
            Command::Coupling => {
                self.open_report("Change coupling", "history", pages::coupling_page, cx)
            }
            Command::Themes => self.open_themes(true, cx),
            Command::Settings => self.open_settings(true, cx),
            Command::Motion(pref) => {
                motion::set(pref, cx);
                // Show the new value where the setting is listed.
                if self.on_settings_page() {
                    self.open_settings(false, cx);
                }
                cx.notify();
            }
            Command::NewTab => self.new_tab(cx),
            Command::CloseTab => self.close_tab(self.active_tab, cx),
            Command::OpenInObsidian => self.open_in_obsidian(cx),
        }
    }

    /// Fetch one analysis from the backend and show it as a generated page.
    fn open_report(
        &mut self,
        title: &'static str,
        route: &'static str,
        render: fn(&Value) -> String,
        cx: &mut Context<Self>,
    ) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        self.run(
            "Loading",
            cx,
            move |b| b.get(&format!("/v1/repos/{}/{route}", repo.id)),
            move |this, data, _| {
                this.set_page(
                    Page::Generated {
                        title: title.into(),
                        markdown: render(&data),
                    },
                    true,
                );
            },
        );
    }

    /// Like [`Self::open_report`], for a route that spans every repository.
    fn open_global(
        &mut self,
        title: &'static str,
        route: &'static str,
        render: fn(&Value) -> String,
        cx: &mut Context<Self>,
    ) {
        self.run(
            "Loading",
            cx,
            move |b| b.get(route),
            move |this, data, _| {
                this.set_page(
                    Page::Generated {
                        title: title.into(),
                        markdown: render(&data),
                    },
                    true,
                );
            },
        );
    }

    fn on_settings_page(&self) -> bool {
        matches!(&self.page, Page::Generated { title, .. } if title == "Settings")
    }

    /// Show the settings page. `remember` puts the page left on the history;
    /// pass false when the page only refreshes itself.
    fn open_settings(&mut self, remember: bool, cx: &mut Context<Self>) {
        let backend = self
            .backend
            .as_ref()
            .map(|b| b.url().to_string())
            .unwrap_or_else(|| "nothing".into());
        let repos: Vec<(String, String)> = self
            .repos
            .iter()
            .map(|r| (r.name.clone(), r.path.clone()))
            .collect();
        let theme = cx.theme().theme_name().to_string();
        let motion = motion::describe(cx);
        self.set_page(
            Page::Generated {
                title: "Settings".into(),
                markdown: pages::settings_page(&backend, &theme, &motion, &repos),
            },
            remember,
        );
        cx.notify();
    }

    /// Path of the current note on this machine, when the repository is local.
    fn local_note_path(&self) -> Option<std::path::PathBuf> {
        let Page::Note { path, .. } = &self.page else {
            return None;
        };
        let file = std::path::Path::new(&self.repo.as_ref()?.path)
            .join("obsidian")
            .join(path);
        file.is_file().then_some(file)
    }

    fn open_in_obsidian(&mut self, cx: &mut Context<Self>) {
        match self.local_note_path() {
            Some(file) => {
                let path = file.to_string_lossy();
                let path = path.strip_prefix(r"\\?\").unwrap_or(&path);
                cx.open_url(&format!("obsidian://open?path={}", encode(path)));
            }
            None => {
                self.error = Some(
                    "Open a note first. Notes of a repository on another machine cannot be opened in Obsidian.".into(),
                );
                cx.notify();
            }
        }
    }

    /// Build the viewer's Markdown for the current page. Diagrams are drawn
    /// once here, not on every frame.
    fn prepare_page(&mut self) {
        self.diagrams.clear();
        let palette = self.palette;
        self.rendered = match &self.page {
            Page::Message(text) => text.clone(),
            Page::Note { text, .. } => {
                let diagrams = &mut self.diagrams;
                markdown::prepare_note_with(text, |source| {
                    let png = cw_mermaid::render_png(source, palette).ok()?;
                    diagrams.push(std::sync::Arc::new(Image::from_bytes(
                        ImageFormat::Png,
                        png,
                    )));
                    Some(format!(
                        "{}{}",
                        markdown::DIAGRAM_SCHEME,
                        diagrams.len() - 1
                    ))
                })
            }
            Page::Generated { markdown, .. } | Page::Proposal { markdown, .. } => markdown.clone(),
            Page::Graph => String::new(),
        };
    }

    fn go_back(&mut self, cx: &mut Context<Self>) {
        if self.history.is_empty() || !self.can_leave() {
            cx.notify();
            return;
        }
        if let Some(page) = self.history.pop() {
            self.page = page;
            self.show_page();
            cx.notify();
        }
    }

    fn open_note(&mut self, path_or_name: String, remember: bool, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        self.run(
            "Opening note",
            cx,
            move |b| {
                b.get(&format!(
                    "/v1/repos/{}/note?path={}",
                    repo.id,
                    encode(&path_or_name)
                ))
            },
            move |this, content, _| {
                let figure = content["illustration"]
                    .as_str()
                    .map(cw_illustrate::check)
                    .filter(|doc| doc.is_valid())
                    .and_then(|doc| doc.root);
                let info = &content["info"];
                this.set_page(
                    Page::Note {
                        path: info["path"].as_str().unwrap_or("").to_string(),
                        text: content["text"].as_str().unwrap_or("").to_string(),
                        sources: info["sources"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(|s| s.as_str().map(str::to_string))
                            .collect(),
                        state: info["state"].as_str().unwrap_or("").to_string(),
                        figure,
                    },
                    remember,
                );
                // `--illustration` as the third argument opens the first note on
                // its illustration.
                this.show_figure = this.figure.is_some()
                    && this.history.is_empty()
                    && std::env::args().nth(3).as_deref() == Some("--illustration");
            },
        );
    }

    fn open_symbol(&mut self, name: String, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        self.run(
            "Loading symbol",
            cx,
            move |b| {
                b.get(&format!(
                    "/v1/repos/{}/symbol?name={}",
                    repo.id,
                    encode(&name)
                ))
            },
            |this, detail, _| {
                let title = detail["symbol"]["name"]
                    .as_str()
                    .unwrap_or("Symbol")
                    .to_string();
                this.set_page(
                    Page::Generated {
                        title,
                        markdown: markdown::symbol_page(&detail),
                    },
                    true,
                );
            },
        );
    }

    /// The theme picker page. `remember` is false when it replaces itself
    /// after a selection, so Back does not walk through every choice.
    fn open_themes(&mut self, remember: bool, cx: &mut Context<Self>) {
        let active = cx.theme().theme_name().to_string();
        let page = Page::Generated {
            title: "Themes".into(),
            markdown: markdown::themes_page(&themes::list(cx), &active),
        };
        self.set_page(page, remember);
        cx.notify();
    }

    /// Live page for a source file: risk, ownership, blast radius, symbols.
    fn open_file(&mut self, path: String, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        let title = path.rsplit('/').next().unwrap_or(&path).to_string();
        self.run(
            "Loading file",
            cx,
            move |b| {
                let base = format!("/v1/repos/{}", repo.id);
                let target = encode(&path);
                Ok((
                    b.get(&format!("{base}/context?target={target}"))?,
                    b.get(&format!("{base}/risk?path={target}"))?,
                    b.get(&format!("{base}/blast-radius?path={target}"))?,
                ))
            },
            move |this, (context, risk, radius), _| {
                this.set_page(
                    Page::Generated {
                        title,
                        markdown: markdown::file_page(&context, &risk, &radius),
                    },
                    true,
                );
            },
        );
    }

    fn open_hotspots(&mut self, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        self.run(
            "Loading hotspots",
            cx,
            move |b| b.get(&format!("/v1/repos/{}/hotspots?limit=40", repo.id)),
            |this, rows, _| {
                this.set_page(
                    Page::Generated {
                        title: "Hotspots".into(),
                        markdown: markdown::hotspots_page(&rows),
                    },
                    true,
                );
            },
        );
    }

    fn run_search(&mut self, query: String, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        if query.trim().is_empty() {
            return;
        }
        let shown = query.clone();
        self.run(
            "Searching",
            cx,
            move |b| {
                b.get(&format!(
                    "/v1/repos/{}/search?q={}&limit=40",
                    repo.id,
                    encode(&query)
                ))
            },
            move |this, hits, _| {
                this.set_page(
                    Page::Generated {
                        title: format!("Search: {shown}"),
                        markdown: markdown::search_page(&shown, &hits),
                    },
                    true,
                );
            },
        );
    }

    fn sync(&mut self, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        self.run(
            "Syncing wiki",
            cx,
            move |b| b.post(&format!("/v1/repos/{}/update", repo.id), &json!({})),
            |this, report, cx| {
                let count = |key: &str| report[key].as_array().map(Vec::len).unwrap_or(0);
                let summary = format!(
                    "# Sync complete\n\n- {} notes written\n- {} unchanged\n- {} proposals queued for review\n- {} removed\n- {} adopted\n- {} total\n- {:.1}% of links resolve\n",
                    count("written"),
                    report["kept"],
                    count("proposed"),
                    count("removed"),
                    count("adopted"),
                    report["notes_total"],
                    report["link_rate"].as_f64().unwrap_or(0.0) * 100.0
                );
                this.set_page(Page::Generated { title: "Sync".into(), markdown: summary }, true);
                this.refresh(false, cx);
            },
        );
    }

    fn open_proposal(&mut self, id: String, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        let request = id.clone();
        self.run(
            "Loading proposal",
            cx,
            move |b| b.get(&format!("/v1/repos/{}/proposals/{request}", repo.id)),
            move |this, detail, _| {
                this.set_page(
                    Page::Proposal {
                        id,
                        markdown: markdown::proposal_page(&detail),
                    },
                    true,
                );
            },
        );
    }

    fn decide_proposal(&mut self, id: String, approve: bool, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        let verb = if approve { "approve" } else { "reject" };
        self.run(
            if approve {
                "Applying proposal"
            } else {
                "Rejecting proposal"
            },
            cx,
            move |b| {
                b.post(
                    &format!("/v1/repos/{}/proposals/{id}/{verb}", repo.id),
                    &json!({}),
                )
            },
            move |this, done, cx| {
                let path = done["note_path"].as_str().unwrap_or("").to_string();
                this.history.clear();
                this.refresh(false, cx);
                if approve {
                    this.open_note(path, false, cx);
                } else {
                    this.set_page(
                        Page::Message("Proposal rejected. The note is unchanged.".into()),
                        false,
                    );
                }
            },
        );
    }

    /// Follow a link clicked in the viewer.
    fn follow_link(&mut self, url: &str, cx: &mut Context<Self>) {
        if let Some(target) = url.strip_prefix(NOTE_SCHEME) {
            self.open_note(markdown::decode(target), true, cx);
        } else if let Some(target) = url.strip_prefix(THEME_SCHEME) {
            if themes::select(&markdown::decode(target), cx) {
                self.open_themes(false, cx);
            }
        } else if let Some(target) = url.strip_prefix(SETTING_SCHEME) {
            if let Some(pref) = target.strip_prefix("motion/").and_then(MotionPref::parse) {
                motion::set(pref, cx);
                self.open_settings(false, cx);
            }
        } else if let Some(target) = url.strip_prefix(FILE_SCHEME) {
            self.open_file(markdown::decode(target), cx);
        } else if let Some(target) = url.strip_prefix(SYMBOL_SCHEME) {
            self.open_symbol(markdown::decode(target), cx);
        } else if url.starts_with("https://") || url.starts_with("http://") {
            cx.open_url(url);
        }
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> Sidebar<SidebarGroup<SidebarMenu>> {
        let current = match &self.page {
            Page::Note { path, .. } => path.as_str(),
            _ => "",
        };
        // Group notes by zone, then by folder below the zone.
        let mut zones: BTreeMap<String, BTreeMap<String, Vec<&NoteInfo>>> = BTreeMap::new();
        for note in &self.notes {
            let mut parts = note.path.splitn(2, '/');
            let (zone, rest) = match (parts.next(), parts.next()) {
                (Some(zone), Some(rest)) => (zone.to_string(), rest),
                _ => (String::new(), note.path.as_str()),
            };
            let folder = rest
                .rsplit_once('/')
                .map(|(f, _)| f)
                .unwrap_or("")
                .to_string();
            zones
                .entry(zone)
                .or_default()
                .entry(folder)
                .or_default()
                .push(note);
        }
        let item = |note: &NoteInfo, cx: &mut Context<Self>| {
            let path = note.path.clone();
            let label = match note.title.as_str() {
                "_index" => "Overview".to_string(),
                "_HOME" => "Home".to_string(),
                title => title.to_string(),
            };
            let icon = match note.title.as_str() {
                "_index" => icons::OVERVIEW,
                "_HOME" => icons::HOME,
                _ => icons::NOTE,
            };
            let active = note.path == current;
            let mut item = SidebarMenuItem::new(label)
                .icon(icon)
                .active(active)
                .on_click(
                    cx.listener(move |this, _, _, cx| this.open_note(path.clone(), true, cx)),
                );
            if active {
                item = item.shadow(glass::selection_bar(cx));
            }
            item
        };

        let glass = self.glass.chrome(cx.theme().sidebar);
        let mut sidebar = Sidebar::new("vault").w(px(SIDEBAR_WIDTH)).bg(glass).header(
            SidebarHeader::new().child(
                v_flex()
                    .flex_1()
                    .overflow_hidden()
                    .child(div().font_bold().child("CodeWiki"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(
                                self.repo
                                    .as_ref()
                                    .map(|r| r.name.clone())
                                    .unwrap_or_default(),
                            ),
                    ),
            ),
        );
        for (zone, folders) in &zones {
            let mut menu = SidebarMenu::new();
            for (folder, notes) in folders {
                let mut notes = notes.clone();
                // Map notes first, then the rest by title.
                notes.sort_by_key(|n| (!n.title.starts_with('_'), n.title.to_lowercase()));
                if folder.is_empty() {
                    for note in notes {
                        menu = menu.child(item(note, cx));
                    }
                } else {
                    let open = notes.iter().any(|n| n.path == current);
                    let children: Vec<SidebarMenuItem> =
                        notes.iter().map(|n| item(n, cx)).collect();
                    menu = menu.child(
                        SidebarMenuItem::new(folder.clone())
                            .icon(icons::FOLDER)
                            .default_open(open)
                            .click_to_toggle(true)
                            .children(children),
                    );
                }
            }
            let label = if zone.is_empty() {
                "Vault".to_string()
            } else {
                zone_label(zone)
            };
            sidebar = sidebar.child(SidebarGroup::new(label).child(menu));
        }
        sidebar
    }

    fn toolbar(&self, cx: &mut Context<Self>) -> Div {
        let pending = self.pending_proposals.len();
        h_flex()
            .w_full()
            .gap_1()
            .p_2()
            .items_center()
            .border_b_1()
            .border_color(glass::tinted_border(cx))
            .bg(self.glass.chrome(cx.theme().background))
            .child(
                Button::new("back")
                    .icon(icons::BACK)
                    .tooltip("Back (Alt+Left)")
                    .small()
                    .ghost()
                    .disabled(self.history.is_empty())
                    .on_click(cx.listener(|this, _, _, cx| this.go_back(cx))),
            )
            .child(
                div()
                    .flex_1()
                    .px_1()
                    .child(Input::new(&self.search).small()),
            )
            .when_some(self.busy.clone(), |this, busy| {
                this.child(
                    div()
                        .px_1()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("{busy}…")),
                )
            })
            .when(
                matches!(self.page, Page::Note { .. }) && self.editing.is_none(),
                |this| {
                    this.child(
                        Button::new("edit")
                            .icon(icons::EDIT)
                            .label("Edit")
                            .small()
                            .on_click(
                                cx.listener(|this, _, window, cx| this.start_edit(window, cx)),
                            ),
                    )
                },
            )
            .when_some(self.editing.as_ref().map(|e| e.dirty), |this, dirty| {
                this.child(
                    Button::new("save")
                        .icon(icons::SAVE)
                        .label("Save")
                        .small()
                        .primary()
                        .disabled(!dirty)
                        .on_click(cx.listener(|this, _, _, cx| this.save_edit(cx))),
                )
                .child(
                    Button::new("discard")
                        .icon(icons::CLOSE)
                        .label(if dirty { "Discard" } else { "Close editor" })
                        .small()
                        .on_click(cx.listener(|this, _, window, cx| this.discard_edit(window, cx))),
                )
            })
            .when(self.figure.is_some() && self.editing.is_none(), |this| {
                this.child(
                    Button::new("figure")
                        .icon(if self.show_figure {
                            icons::NOTE
                        } else {
                            icons::FIGURE
                        })
                        .label(if self.show_figure {
                            "Text"
                        } else {
                            "Illustration"
                        })
                        .small()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.show_figure = !this.show_figure;
                            cx.notify();
                        })),
                )
            })
            .child(
                Button::new("commands")
                    .icon(icons::COMMANDS)
                    .label("Ctrl+K")
                    .tooltip("Commands")
                    .small()
                    .ghost()
                    .on_click(cx.listener(|this, _, window, cx| this.toggle_palette(window, cx))),
            )
            .child(
                Button::new("graph")
                    .icon(icons::GRAPH)
                    .tooltip("Graph")
                    .small()
                    .ghost()
                    .on_click(cx.listener(|this, _, _, cx| this.open_graph(cx))),
            )
            .child(
                Button::new("hotspots")
                    .icon(icons::HOTSPOTS)
                    .tooltip("Hotspots")
                    .small()
                    .ghost()
                    .disabled(self.repo.is_none())
                    .on_click(cx.listener(|this, _, _, cx| this.open_hotspots(cx))),
            )
            .child(
                Button::new("theme")
                    .icon(icons::THEME)
                    .tooltip(format!("Theme: {}", cx.theme().theme_name()))
                    .small()
                    .ghost()
                    .on_click(cx.listener(|this, _, _, cx| this.open_themes(true, cx))),
            )
            .child(
                Button::new("sync")
                    .icon(icons::SYNC)
                    .label("Sync")
                    .small()
                    .disabled(self.busy.is_some() || self.repo.is_none())
                    .on_click(cx.listener(|this, _, _, cx| this.sync(cx))),
            )
            .child(
                Button::new("review")
                    .icon(icons::REVIEW)
                    .label(format!("Review ({pending})"))
                    .small()
                    .when(pending > 0, |b| b.primary())
                    .disabled(pending == 0)
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some((id, _)) = this.pending_proposals.first().cloned() {
                            this.open_proposal(id, cx);
                        }
                    })),
            )
    }

    /// The row of tabs under the toolbar. The underline of the active tab
    /// slides to the next one; gpui-kit animates it.
    fn tab_bar(&self, cx: &mut Context<Self>) -> TabBar {
        let closable = self.tabs.len() > 1;
        let tabs = (0..self.tabs.len()).map(|index| {
            let title = page_title(if index == self.active_tab {
                &self.page
            } else {
                &self.tabs[index].page
            });
            TabItem::new()
                .child(div().max_w(px(180.)).truncate().child(title))
                .when(closable, |tab| {
                    tab.suffix(
                        Button::new(("close-tab", index))
                            .icon(icons::CLOSE)
                            .tooltip("Close tab (Ctrl+W)")
                            .xsmall()
                            .ghost()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.close_tab(index, cx);
                            })),
                    )
                })
        });
        TabBar::new("tabs")
            .underline()
            .small()
            .w_full()
            .px_3()
            .bg(self.glass.chrome(cx.theme().background))
            .selected_index(self.active_tab)
            .children(tabs.collect::<Vec<_>>())
            .on_click(cx.listener(|this, index: &usize, _, cx| this.activate_tab(*index, cx)))
            .suffix(
                Button::new("new-tab")
                    .icon(icons::NEW_TAB)
                    .tooltip("New tab (Ctrl+T)")
                    .xsmall()
                    .ghost()
                    .on_click(cx.listener(|this, _, _, cx| this.new_tab(cx))),
            )
    }

    /// The command palette, drawn over everything else.
    fn palette_view(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let entries = self.palette_matches(cx);
        let selected = self.palette_selected.min(entries.len().saturating_sub(1));
        let (popover, accent, accent_text, muted, backdrop) = {
            let theme = cx.theme();
            (
                theme.popover,
                theme.accent,
                theme.accent_foreground,
                theme.muted_foreground,
                theme.background.opacity(0.6),
            )
        };
        let border = glass::tinted_border(cx);
        let bar = glass::selection_bar(cx);
        let mut list = v_flex().gap_1();
        for (index, entry) in entries.into_iter().enumerate() {
            let command = entry.command;
            let icon = icons::for_command(&command);
            list = list.child(
                h_flex()
                    .id(("palette-entry", index))
                    .px_3()
                    .py_1p5()
                    .gap_2()
                    .rounded_md()
                    .items_center()
                    .cursor_pointer()
                    .when(index == selected, |row| {
                        row.bg(accent).text_color(accent_text).shadow(bar.clone())
                    })
                    .child(Icon::new(icon).small().text_color(muted))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_sm()
                            .truncate()
                            .child(entry.label),
                    )
                    .child(div().text_xs().text_color(muted).child(entry.hint))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.run_command(command.clone(), window, cx)
                    })),
            );
        }
        deferred(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .occlude()
                .bg(backdrop)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, window, cx| {
                        if this.palette_open {
                            this.toggle_palette(window, cx);
                        }
                    }),
                )
                .child(
                    h_flex().w_full().justify_center().pt(px(90.)).child(
                        v_flex()
                            .id("palette")
                            .w(px(560.))
                            .rounded_lg()
                            .border_1()
                            .border_color(border)
                            .bg(popover)
                            .shadow(glass::glow(cx))
                            .overflow_hidden()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(glass::hairline(cx))
                            .child(
                                v_flex()
                                    .p_2()
                                    .gap_2()
                                    .child(Input::new(&self.palette_input))
                                    .child(list),
                            )
                            // Drops into place as it fades in.
                            .with_animation(
                                "palette-in",
                                Animation::new(motion::ENTER_FAST).with_easing(ease_out_quint()),
                                |panel, t| panel.opacity(t).relative().top(px((t - 1.0) * 8.0)),
                            ),
                    ),
                ),
        )
        .with_priority(1)
    }

    /// The editor beside a live preview of what is typed.
    fn edit_view(&self, editing: &Editing, cx: &mut Context<Self>) -> Div {
        let (value, cursor) = {
            let state = editing.state.read(cx);
            (state.value().to_string(), state.cursor())
        };
        let names: Vec<String> = self.notes.iter().map(|n| n.title.clone()).collect();
        let query = editor::wikilink_query(&value, cursor);
        let suggestions: Vec<String> = query
            .map(|query| {
                editor::completions(&names, query, 8)
                    .into_iter()
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        let typed = query.map(str::len).unwrap_or(0);
        // The editor closes brackets as they are typed; the link then already
        // has its `]]` after the cursor.
        let closed = value
            .get(cursor..)
            .is_some_and(|rest| rest.starts_with("]]"));
        let preview = markdown::prepare_note_with(&value, |_| None);
        let (border, muted, mono, mono_size) = {
            let theme = cx.theme();
            (
                theme.border,
                theme.muted_foreground,
                theme.mono_font_family.clone(),
                theme.mono_font_size,
            )
        };
        let mut strip = h_flex()
            .w_full()
            .gap_1()
            .px_2()
            .py_1()
            .flex_wrap()
            .items_center()
            .border_t_1()
            .border_color(border)
            .child(div().text_xs().text_color(muted).child(if query.is_some() {
                "Link to:"
            } else {
                "Type [[ to link a note. Ctrl+S saves."
            }));
        for (index, name) in suggestions.into_iter().enumerate() {
            let state = editing.state.clone();
            strip = strip.child(
                Button::new(("link", index))
                    .label(name.clone())
                    .xsmall()
                    .on_click(move |_, window, cx| {
                        state.update(cx, |state, cx| {
                            // Replace what was typed so the link carries the
                            // note's exact name.
                            let end = state.cursor();
                            let after = if closed { 2 } else { 0 };
                            state.set_selected_range(end.saturating_sub(typed)..end + after, cx);
                            state.replace(format!("{name}]]"), window, cx);
                            state.focus(window, cx);
                        });
                    }),
            );
        }
        h_flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .child(
                v_flex()
                    .w_1_2()
                    .h_full()
                    .border_r_1()
                    .border_color(border)
                    .child(
                        div()
                            .id("editor")
                            .flex_1()
                            .min_h_0()
                            .font_family(mono)
                            .text_size(mono_size)
                            .child(
                                Editor::new(&editing.state)
                                    .h(relative(1.))
                                    .appearance(false),
                            ),
                    )
                    .child(strip),
            )
            .child(div().w_1_2().h_full().py_4().child(reading::frame(
                // A stable id keeps the preview's scroll position while typing.
                TextView::markdown("preview", preview).scrollable(true),
                self.pane_width / 2.0,
                reading::NOTE_MEASURE,
                cx,
            )))
    }

    /// The centre pane for every page except the graph.
    fn page_view(&self, cx: &mut Context<Self>) -> AnyElement {
        match &self.editing {
            Some(editing) => v_flex()
                .flex_1()
                .min_w_0()
                .h_full()
                .when_some(self.error.clone(), |this, error| {
                    this.child(
                        div()
                            .p_2()
                            .text_sm()
                            .bg(cx.theme().danger)
                            .text_color(cx.theme().danger_foreground)
                            .child(error),
                    )
                })
                .child(self.edit_view(editing, cx))
                .into_any_element(),
            None => self.viewer(cx).into_any_element(),
        }
    }

    fn viewer(&self, cx: &mut Context<Self>) -> Div {
        let source = self.rendered.clone();
        let diagrams = self.diagrams.clone();
        let view = cx.entity().downgrade();
        // Notes are prose; everything else is mostly tables and needs room.
        let measure = match self.page {
            Page::Note { .. } => reading::NOTE_MEASURE,
            _ => reading::WIDE_MEASURE,
        };
        v_flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .when_some(self.error.clone(), |this, error| {
                this.child(
                    div()
                        .p_2()
                        .text_sm()
                        .bg(cx.theme().danger)
                        .text_color(cx.theme().danger_foreground)
                        .child(error),
                )
            })
            .when_some(self.proposal_actions(cx), |this, actions| {
                this.child(actions)
            })
            .when_some(
                self.figure.as_ref().filter(|_| self.show_figure),
                |this, figure| {
                    this.child(
                        div()
                            .id("figure")
                            .flex_1()
                            .min_h_0()
                            .p_6()
                            .overflow_y_scroll()
                            .child(illustration::render(figure, cx)),
                    )
                },
            )
            .when(!(self.show_figure && self.figure.is_some()), |this| {
                this.child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .py_4()
                        .child(reading::frame(
                            // A fresh id per page resets scroll position and selection.
                            TextView::markdown(("page", self.page_serial), source)
                                .scrollable(true)
                                .image_source(move |uri| {
                                    uri.strip_prefix(markdown::DIAGRAM_SCHEME)
                                        .and_then(|index| index.parse::<usize>().ok())
                                        .and_then(|index| diagrams.get(index).cloned())
                                        .map(ImageSource::Image)
                                        .unwrap_or_else(|| uri.clone().into())
                                })
                                .on_link_click(move |url, _, _, cx| {
                                    let url = url.to_string();
                                    _ = view.update(cx, |this, cx| this.follow_link(&url, cx));
                                }),
                            self.pane_width,
                            measure,
                            cx,
                        ))
                        // A new page fades in. The id changes with the page, so
                        // the animation runs once per page and not on a redraw.
                        .with_animation(
                            ("page-in", self.page_serial),
                            Animation::new(motion::ENTER).with_easing(ease_out_quint()),
                            |pane, t| pane.opacity(t),
                        ),
                )
            })
    }

    /// Number of synthetic nodes to draw instead of the real graph, for
    /// measuring drawing speed: `CODEWIKI_GRAPH_STRESS=50000`.
    fn stress_nodes() -> Option<usize> {
        std::env::var("CODEWIKI_GRAPH_STRESS").ok()?.parse().ok()
    }

    fn open_graph(&mut self, cx: &mut Context<Self>) {
        if let Some(count) = Self::stress_nodes() {
            self.graph = Some(std::rc::Rc::new(graph::GraphData::stress(count)));
            self.camera = None;
            self.set_page(Page::Graph, true);
            cx.notify();
            return;
        }
        let Some(repo) = self.repo.clone() else {
            return;
        };
        self.run(
            "Loading graph",
            cx,
            move |b| b.get(&format!("/v1/repos/{}/graph", repo.id)),
            |this, value, _| {
                this.graph = Some(std::rc::Rc::new(graph::GraphData::from_json(&value)));
                this.camera = None;
                this.set_page(Page::Graph, true);
            },
        );
    }

    /// The dependency graph: a canvas for edges and nodes, labels on top.
    fn graph_view(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(data) = self.graph.clone() else {
            return div().into_any_element();
        };
        // Bounds are known after the first paint; until then estimate from
        // the window so the first frame is already fitted.
        let [bx, by, bw, bh] = self.graph_bounds.get();
        let viewport = window.viewport_size();
        let (width, height) = if bw > 0.0 {
            (bw, bh)
        } else {
            (
                (viewport.width / px(1.) - SIDEBAR_WIDTH - DETAILS_WIDTH).max(200.0),
                (viewport.height / px(1.) - 48.0).max(200.0),
            )
        };
        let camera = *self
            .camera
            .get_or_insert_with(|| graph::Camera::fit(&data, width, height));
        let stress = Self::stress_nodes().is_some();
        if stress {
            // Redraw continuously so the frame time shown is a real rate.
            window.request_animation_frame();
        }

        let theme = cx.theme();
        let palette = [
            theme.primary,
            theme.success,
            theme.warning,
            theme.danger,
            theme.info,
            theme.link,
        ];
        let edge_color = theme.muted_foreground.opacity(0.22);
        let visible = graph::visible_nodes(&data, &camera, width, height);
        let labels = graph::label_candidates(&data, &visible, 28);
        let hud = format!(
            "{} nodes · {} edges · {} in view · paint {:.1} ms · frame {:.1} ms",
            data.nodes.len(),
            data.edges.len(),
            visible.len(),
            self.paint_ms.get(),
            self.frame_ms.get()
        );

        let bounds_cell = self.graph_bounds.clone();
        let paint_ms = self.paint_ms.clone();
        let frame_ms = self.frame_ms.clone();
        let last_frame = self.last_frame.clone();
        let paint_data = data.clone();
        let rel =
            move |position: Point<Pixels>| (position.x / px(1.) - bx, position.y / px(1.) - by);

        let mut view = div()
            .id("graph")
            .relative()
            .flex_1()
            .min_h_0()
            .h_full()
            .overflow_hidden()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, _| {
                    this.drag_last = Some(rel(event.position));
                    this.drag_moved = false;
                }),
            )
            .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
                let (x, y) = rel(event.position);
                if event.pressed_button == Some(MouseButton::Left)
                    && let Some((lx, ly)) = this.drag_last
                    && let Some(camera) = this.camera.as_mut()
                {
                    if (x - lx).abs() + (y - ly).abs() > 2.0 {
                        this.drag_moved = true;
                    }
                    camera.pan(x - lx, y - ly);
                    this.drag_last = Some((x, y));
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseUpEvent, _, cx| {
                    let moved = this.drag_moved;
                    this.drag_last = None;
                    // A click that did not drag opens the node under the cursor.
                    if !moved && let (Some(data), Some(camera)) = (this.graph.clone(), this.camera)
                    {
                        let (x, y) = rel(event.position);
                        let [_, _, w, h] = this.graph_bounds.get();
                        if let Some(index) = graph::pick(&data, &camera, w, h, x, y)
                            && Self::stress_nodes().is_none()
                        {
                            this.open_symbol(data.nodes[index].id.clone(), cx);
                        }
                    }
                }),
            )
            .on_scroll(cx.listener(move |this, event: &ScrollEvent, _, cx| {
                let delta = event.delta.pixel_delta(px(20.)).y / px(1.);
                let (x, y) = rel(event.position);
                let [_, _, w, h] = this.graph_bounds.get();
                if let Some(camera) = this.camera.as_mut() {
                    camera.zoom_at(x, y, 1.0015f32.powf(delta), w.max(1.0), h.max(1.0));
                    cx.notify();
                }
            }))
            .child(
                canvas(
                    |bounds, _, _| bounds,
                    move |_, bounds: Bounds<Pixels>, window, _| {
                        let started = std::time::Instant::now();
                        let (ox, oy) = (bounds.origin.x / px(1.), bounds.origin.y / px(1.));
                        let (w, h) = (bounds.size.width / px(1.), bounds.size.height / px(1.));
                        bounds_cell.set([ox, oy, w, h]);

                        // One stroked path carries every edge; edges are
                        // sampled when too many are in view.
                        let in_view = graph::visible_nodes(&paint_data, &camera, w, h);
                        // Zoomed out on a large graph, drawing every node costs
                        // more than a frame. Draw density cells instead; detail
                        // returns as the user zooms in.
                        let overview = in_view.len() > graph::DETAIL_LIMIT;
                        let edge_budget = if overview { 1200 } else { 2500 };
                        let edges = graph::visible_edges(&paint_data, &camera, w, h, edge_budget);
                        if !edges.is_empty() {
                            let mut builder = PathBuilder::stroke(px(1.));
                            for (ax, ay, bx, by) in edges {
                                builder.move_to(point(px(ox + ax), px(oy + ay)));
                                builder.line_to(point(px(ox + bx), px(oy + by)));
                            }
                            if let Ok(path) = builder.build() {
                                window.paint_path(path, edge_color);
                            }
                        }
                        // Shapes are batched into one filled path per color:
                        // a path with thousands of polygons costs far less
                        // than the same number of individual quads.
                        let mut batches: Vec<Option<PathBuilder>> =
                            (0..palette.len()).map(|_| None).collect();
                        let mut add = |color: usize, points: &[Point<Pixels>]| {
                            batches[color % palette.len()]
                                .get_or_insert_with(PathBuilder::fill)
                                .add_polygon(points, true);
                        };
                        if overview {
                            let side = graph::cell_size(w, h) - 1.0;
                            for cell in graph::density_cells(&paint_data, &camera, w, h) {
                                let (x, y) = (ox + cell.x + 0.5, oy + cell.y + 0.5);
                                add(
                                    cell.community,
                                    &[
                                        point(px(x), px(y)),
                                        point(px(x + side), px(y)),
                                        point(px(x + side), px(y + side)),
                                        point(px(x), px(y + side)),
                                    ],
                                );
                            }
                        } else {
                            for (index, x, y) in in_view {
                                let node = &paint_data.nodes[index];
                                let r = graph::radius(node.rank, paint_data.max_rank, camera.zoom);
                                let (cx, cy) = (ox + x, oy + y);
                                // An octagon reads as a dot at these sizes.
                                let k = r * 0.4142;
                                add(
                                    node.community,
                                    &[
                                        point(px(cx - k), px(cy - r)),
                                        point(px(cx + k), px(cy - r)),
                                        point(px(cx + r), px(cy - k)),
                                        point(px(cx + r), px(cy + k)),
                                        point(px(cx + k), px(cy + r)),
                                        point(px(cx - k), px(cy + r)),
                                        point(px(cx - r), px(cy + k)),
                                        point(px(cx - r), px(cy - k)),
                                    ],
                                );
                            }
                        }
                        for (color, batch) in batches.into_iter().enumerate() {
                            if let Some(builder) = batch
                                && let Ok(path) = builder.build()
                            {
                                let alpha = if overview { 0.8 } else { 1.0 };
                                window.paint_path(path, palette[color].opacity(alpha));
                            }
                        }
                        paint_ms.set(started.elapsed().as_secs_f32() * 1000.0);
                        let now = std::time::Instant::now();
                        if let Some(previous) = last_frame.replace(Some(now)) {
                            frame_ms.set((now - previous).as_secs_f32() * 1000.0);
                        }
                    },
                )
                .absolute()
                .size_full(),
            );
        for (index, x, y) in labels {
            view = view.child(
                div()
                    .absolute()
                    .left(px(x + 8.0))
                    .top(px(y - 8.0))
                    .text_xs()
                    .text_color(theme.foreground.opacity(0.85))
                    .child(data.nodes[index].name.clone()),
            );
        }
        view.child(
            div()
                .absolute()
                .left(px(10.))
                .top(px(8.))
                .px_2()
                .rounded(theme.radius)
                .bg(theme.background.opacity(0.8))
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(hud),
        )
        .into_any_element()
    }

    fn proposal_actions(&self, cx: &mut Context<Self>) -> Option<Div> {
        let Page::Proposal { id, .. } = &self.page else {
            return None;
        };
        let (approve_id, reject_id) = (id.clone(), id.clone());
        Some(
            h_flex()
                .gap_2()
                .p_2()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    Button::new("approve")
                        .label("Approve and write")
                        .small()
                        .primary()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.decide_proposal(approve_id.clone(), true, cx)
                        })),
                )
                .child(
                    Button::new("reject")
                        .label("Reject")
                        .small()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.decide_proposal(reject_id.clone(), false, cx)
                        })),
                ),
        )
    }

    fn details(&self, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let heading = move |text: &str| {
            div()
                .text_xs()
                .font_bold()
                .text_color(muted)
                .child(text.to_string())
        };
        let mut pane = v_flex()
            .w(px(DETAILS_WIDTH))
            .h_full()
            .gap_3()
            .p_3()
            .border_l_1()
            .border_color(glass::tinted_border(cx))
            .bg(self.glass.chrome(cx.theme().background))
            .overflow_hidden();
        match &self.page {
            Page::Note {
                path,
                text,
                sources,
                state,
                ..
            } => {
                pane = pane
                    .child(heading("NOTE"))
                    .child(div().text_xs().child(path.clone()));
                let label = match state.as_str() {
                    "generated" => "Generated by CodeWiki",
                    "human_edited" | "human_edited_stale" => "Edited by hand — protected",
                    "stale" => "Source changed — sync to refresh",
                    other => other,
                };
                pane = pane.child(div().text_xs().text_color(muted).child(label.to_string()));
                pane = pane.child(heading("OUTLINE"));
                for section in markdown::outline(text) {
                    pane = pane.child(div().text_sm().child(section));
                }
                if !sources.is_empty() {
                    pane = pane.child(heading("SOURCE"));
                    for source in sources {
                        pane = pane.child(div().text_xs().child(source.clone()));
                    }
                }
                if self.local_note_path().is_some() {
                    pane = pane.child(
                        Button::new("obsidian")
                            .label("Open in Obsidian")
                            .small()
                            .on_click(cx.listener(|this, _, _, cx| this.open_in_obsidian(cx))),
                    );
                }
            }
            Page::Generated { title, .. } => {
                pane = pane
                    .child(heading("LIVE VIEW"))
                    .child(div().text_sm().child(title.clone()))
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted)
                            .child("Served from the index, not stored in the vault."),
                    );
            }
            Page::Proposal { .. } => {
                pane = pane.child(heading("REVIEW QUEUE"));
                for (id, path) in &self.pending_proposals {
                    let id = id.clone();
                    pane = pane.child(
                        Button::new(SharedString::from(format!("proposal-{id}")))
                            .label(path.rsplit('/').next().unwrap_or(path).to_string())
                            .small()
                            .ghost()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_proposal(id.clone(), cx)
                            })),
                    );
                }
            }
            Page::Graph => {
                pane = pane.child(heading("GRAPH")).child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .child("Drag to pan, scroll to zoom, click a node to open it. Size shows how much depends on a symbol; color shows its cluster."),
                );
            }
            Page::Message(_) => {}
        }
        if self.repos.len() > 1 {
            pane = pane.child(heading("REPOSITORIES"));
            for repo in &self.repos {
                let target = repo.clone();
                pane = pane.child(
                    Button::new(SharedString::from(format!("repo-{}", repo.id)))
                        .label(repo.name.clone())
                        .small()
                        .ghost()
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.select_repo(target.clone(), cx)),
                        ),
                );
            }
        }
        pane
    }
}

fn parse_repos(value: &Value) -> Vec<Repo> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            Some(Repo {
                id: r["id"].as_str()?.to_string(),
                name: r["name"].as_str()?.to_string(),
                path: r["path"].as_str().unwrap_or("").to_string(),
            })
        })
        .collect()
}

fn parse_notes(value: &Value) -> Vec<NoteInfo> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|n| {
            Some(NoteInfo {
                path: n["path"].as_str()?.to_string(),
                title: n["title"].as_str()?.to_string(),
                state: n["state"].as_str().unwrap_or("").to_string(),
            })
        })
        .collect()
}

/// Short name for a tab.
fn page_title(page: &Page) -> String {
    match page {
        Page::Message(_) => "CodeWiki".into(),
        Page::Note { path, .. } => {
            let name = path.rsplit('/').next().unwrap_or(path);
            match name.trim_end_matches(".md") {
                "_HOME" => "Home".into(),
                "_index" => "Overview".into(),
                title => title.to_string(),
            }
        }
        Page::Generated { title, .. } => title.clone(),
        Page::Proposal { .. } => "Review".into(),
        Page::Graph => "Graph".into(),
    }
}

/// `01-Core` -> `Core`.
fn zone_label(zone: &str) -> String {
    match zone.split_once('-') {
        Some((number, rest)) if number.len() == 2 && number.chars().all(|c| c.is_ascii_digit()) => {
            rest.to_string()
        }
        _ => zone.to_string(),
    }
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = if cx.theme().is_dark() {
            cw_mermaid::Palette::Dark
        } else {
            cw_mermaid::Palette::Light
        };
        if palette != self.palette {
            self.palette = palette;
            self.prepare_page();
        }
        if let Some(query) = self.pending_palette.take() {
            self.toggle_palette(window, cx);
            self.palette_input
                .update(cx, |input, cx| input.set_value(query, window, cx));
        }
        if self.pending_edit && matches!(self.page, Page::Note { .. }) {
            self.pending_edit = false;
            self.start_edit(window, cx);
        }
        if let Some(command) = self.pending_command.take() {
            self.run_command(command, window, cx);
        }
        // Read once so unused-field lints stay honest about what the UI shows.
        let _ = self.notes.iter().filter(|n| n.state == "stale").count();
        let title = match &self.repo {
            Some(repo) => format!("CodeWiki — {}", repo.name),
            None => "CodeWiki".to_string(),
        };
        self.glass = glass::Levels::for_window(window, cx);
        self.pane_width =
            (window.viewport_size().width / px(1.) - SIDEBAR_WIDTH - DETAILS_WIDTH).max(0.0);
        let title_bar = TitleBar::new()
            .bg(self.glass.chrome(cx.theme().title_bar))
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(title),
            );
        v_flex()
            .id("root")
            .relative()
            .track_focus(&self.focus)
            .on_action(
                cx.listener(|this, _: &TogglePalette, window, cx| this.toggle_palette(window, cx)),
            )
            .on_action(cx.listener(|this, _: &NewTab, _, cx| this.new_tab(cx)))
            .on_action(cx.listener(|this, _: &CloseTab, _, cx| this.close_tab(this.active_tab, cx)))
            .on_action(cx.listener(|this, _: &NextTab, _, cx| this.next_tab(cx)))
            .on_action(cx.listener(|this, _: &GoBack, _, cx| this.go_back(cx)))
            .on_action(cx.listener(|this, _: &SaveNote, _, cx| this.save_edit(cx)))
            // The palette's input would otherwise use these keys itself.
            .capture_action(cx.listener(|this, _: &Escape, window, cx| {
                if this.palette_open {
                    this.toggle_palette(window, cx);
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(|this, _: &MoveDown, _, cx| {
                if this.palette_open {
                    let count = this.palette_matches(cx).len();
                    this.palette_selected =
                        (this.palette_selected + 1).min(count.saturating_sub(1));
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .capture_action(cx.listener(|this, _: &MoveUp, _, cx| {
                if this.palette_open {
                    this.palette_selected = this.palette_selected.saturating_sub(1);
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .size_full()
            .text_color(cx.theme().foreground)
            .when(self.palette_open, |root| root.child(self.palette_view(cx)))
            .child(title_bar)
            .child(glass::hairline(cx))
            .child(self.body(window, cx))
    }
}

impl AppView {
    /// Everything below the title bar: sidebar, toolbar, tabs, page, details.
    fn body(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        h_flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(self.sidebar(cx))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .child(self.toolbar(cx))
                    .child(self.tab_bar(cx))
                    .child(
                        h_flex()
                            .flex_1()
                            .min_h_0()
                            .child(
                                // The pane: denser glass than the chrome
                                // around it, because it carries the text.
                                h_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .h_full()
                                    .bg(self.glass.pane(cx.theme().background))
                                    .child(if matches!(self.page, Page::Graph) {
                                        self.graph_view(window, cx)
                                    } else {
                                        self.page_view(cx)
                                    }),
                            )
                            .child(self.details(cx)),
                    ),
            )
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(icons::AppAssets);
    app.run(|cx| {
        gpui_kit::init(cx);
        motion::init(cx);
        fonts::register(cx);
        themes::init(cx);
        cx.bind_keys([
            KeyBinding::new("ctrl-k", TogglePalette, None),
            KeyBinding::new("ctrl-t", NewTab, None),
            KeyBinding::new("ctrl-w", CloseTab, None),
            KeyBinding::new("ctrl-tab", NextTab, None),
            KeyBinding::new("alt-left", GoBack, None),
            KeyBinding::new("ctrl-s", SaveNote, None),
        ]);
        let (window, _) = gpui_kit::open_window(TitleBar::window_options(), cx, |window, cx| {
            window.set_window_title("CodeWiki");
            // Windows 11 backdrop; older systems ignore it.
            window.set_background_appearance(WindowBackgroundAppearance::MicaBackdrop);
            let app = cx.new(|cx| {
                let view = AppView::new(window, cx);
                // Shortcuts need a focused element to dispatch from.
                window.focus(&view.focus, cx);
                view
            });
            // The shell draws the drifting backdrop and holds the app over it.
            cx.new(|cx| backdrop::Shell::new(app, window, cx))
        })
        .expect("failed to open the main window");
        // gpui-kit paints the theme background over the whole window, which
        // hides the backdrop. Clear it; the app's own surfaces are the glass.
        window
            .update(cx, |_, window, cx| {
                gpui_kit::base::Root::update(window, cx, |root, _, cx| {
                    root.style().background = Some(gpui_kit::transparent_black().into());
                    cx.notify();
                });
            })
            .expect("the main window was just opened");
    });
}
