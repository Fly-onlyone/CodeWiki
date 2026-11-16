//! CodeWiki desktop app.
//!
//! A native viewer for the vault and the live index. All data comes from the
//! backend over HTTP; this binary links none of the engine crates. The one
//! thing it does outside the backend is update its own installation
//! (`cw-update`).

// Release builds are a GUI app: no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
mod backdrop;
mod contrast;
mod diagrams;
mod doc;
mod doc_view;
mod editor;
mod fonts;
mod glass;
mod graph;
mod icons;
mod illustration;
mod instance;
mod live;
mod markdown;
mod motion;
mod mui;
mod pages;
mod palette;
mod panes;
mod prefs;
mod reading;
mod settings;
mod settings_layout;
mod settings_pages;
mod settings_view;
mod standard;
mod startup;
mod status;
mod themes;
mod tray;
mod update;
mod win32;

use std::collections::BTreeMap;
use std::rc::Rc;

use cw_client::{Backend, encode};
use gpui_kit::base::TextView;
use gpui_kit::base::motion::{Presence, Transition};
use gpui_kit::component::input::{
    Editor, EditorState, Escape, InputEvent, InputState, MoveDown, MoveUp,
};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, StyledExt as _, h_flex, v_flex};
use gpui_kit::component::{TITLE_BAR_HEIGHT, TitleBar};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde_json::{Value, json};

use doc::{Action, Doc};
use markdown::NOTE_SCHEME;
use mui::button::{Button, IconButton};
use mui::feedback::{self, Tone};
use mui::list::{self, ListFolder, ListItemButton};
use mui::menu::{AnchorBounds, Menu, MenuItem, MenuSection};
use mui::motion::{Enter, EnterExt as _};
use mui::surfaces;
use mui::tabs::{Tab as TabItem, Tabs};
use mui::text_field::TextField;
use mui::tokens::Variant;
use palette::Command;
use panes::Pane;

actions!(
    codewiki,
    [
        TogglePalette,
        NewTab,
        CloseTab,
        NextTab,
        GoBack,
        SaveNote,
        ToggleSidebar,
        ToggleDetails,
        OpenSettings
    ]
);

/// Most entries the palette lists at once.
const PALETTE_ROWS: usize = 14;

/// Width of the theme menu: an icon, the longest bundled name, four color
/// samples and the mark of the theme in use.
const THEME_MENU_WIDTH: f32 = 340.0;
/// The mark of the theme in use, and the place kept for it in every row.
const THEME_MARK: f32 = 18.0;
/// Between the window's top right corner and a menu that hangs from it.
const MENU_CORNER_GAP: f32 = 12.0;
/// How often, and how long apart, the app looks again for what an update
/// left behind and could not delete at once.
const CLEANUP_RETRIES: usize = 3;
const CLEANUP_PAUSE: std::time::Duration = std::time::Duration::from_secs(5);
/// How often the app asks the backend what it is doing, for the tray icon.
const STATUS_PAUSE: std::time::Duration = std::time::Duration::from_secs(5);
/// The same, of a backend too old to say: it will not learn to.
const OLD_BACKEND_PAUSE: std::time::Duration = std::time::Duration::from_secs(60);
/// How often a running app looks whether the daily check for a newer release
/// is due. An app in the tray runs for weeks; one long timer would not
/// survive the computer's sleep.
const RECHECK_PAUSE: std::time::Duration = std::time::Duration::from_secs(60 * 60);
/// The setting that remembers that the user was told, once, that closing the
/// window leaves the app running in the tray.
const TRAY_HINT: &str = "tray_hint";
/// What Windows knows the app as when it shows a notification.
const APP_IDENTITY: &str = "CodeWiki.App";
/// The smallest window: the sidebar, the details pane, and a reading pane
/// about as wide as the two together.
const MIN_WINDOW: (f32, f32) = (960.0, 600.0);
/// Kept free around the first window on a screen it would otherwise fill. A
/// window as large as the free area has its unseen frame, and with it a few
/// pixels of the title bar, outside the screen.
const SCREEN_MARGIN: f32 = 16.0;

/// The vault's map note, which the app calls "Wiki map".
const WIKI_MAP: &str = "_HOME";

/// Where the app goes once a repository's notes have loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Landing {
    /// Stay on the current page.
    Stay,
    /// The app has just started: the view the second argument names, or
    /// Home.
    Startup,
    /// The user picked the repository: its wiki map.
    WikiMap,
}

/// What reaches the app from outside GPUI: from another thread, or from
/// code that runs with no way to the app's state. Each is sent through a
/// channel and carried out by one task of the app's view (`AppView::listen`),
/// so that none of it ever runs while GPUI is in the middle of something.
enum Event {
    /// A second launch of the app handed over what it was asked to open.
    Handover(instance::Request),
    /// The window was asked to close: hide it, the app stays in the tray.
    Hide,
    /// The tray icon was clicked: show the window.
    Show,
    /// A row of the tray menu was chosen.
    Menu(tray::Command),
}

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
    /// A message with something to press, such as "Try again". Like a
    /// message, it is not kept in the history and claims no repository data.
    Notice(Rc<Doc>),
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
        doc: Rc<Doc>,
    },
    Proposal {
        id: String,
        doc: Rc<Doc>,
    },
    /// The dependency graph canvas.
    Graph,
    /// The page the app starts on: Open folder and the registered
    /// repositories. Its page is built from the app's state on every frame,
    /// so going back to it never shows an old list.
    Home,
    /// The settings, at one of their categories. Built on every frame too,
    /// and drawn by this view itself (`settings_view.rs`).
    Settings(settings::Category),
}

struct AppView {
    backend: Option<Backend>,
    /// The registered repositories whose folders are there.
    repos: Vec<Repo>,
    repo: Option<Repo>,
    notes: Vec<NoteInfo>,
    /// Whether `notes` holds the open repository's notes yet.
    notes_loaded: bool,
    pending_proposals: Vec<(String, String)>,
    /// Folders of the repositories opened in the app, newest first.
    recent: Vec<String>,
    /// The registered repositories whose folders are not found. Home lists
    /// them apart, each with a way to remove it.
    missing: Vec<Repo>,
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
    /// The note or view to open on once a repository has loaded: what the
    /// launch named, or the last launch that handed over to this app.
    requested_view: Option<String>,
    /// What a second launch asked for before the backend was reached; carried
    /// out as soon as it is.
    pending_request: Option<instance::Request>,
    /// The icon in the notification area. `None` when it could not be made;
    /// closing the window then ends the app, as it did before there was one.
    tray: Option<tray::Tray>,
    /// What the backend last said it is doing; asked every few seconds.
    backend_status: status::Backend,
    /// The backend did not answer the last time it was asked.
    backend_silent: bool,
    focus: FocusHandle,
    theme_menu_open: bool,
    /// The row of the theme menu that the keys act on.
    theme_menu_highlight: usize,
    /// Holds the focus while the theme menu is open.
    theme_menu_focus: FocusHandle,
    /// Where the toolbar's theme button is, for the menu to hang under.
    theme_anchor: AnchorBounds,
    /// Set while the current note is open in the editor.
    editing: Option<Editing>,
    busy: Option<String>,
    error: Option<String>,
    /// Where the app stands with updates.
    update: update::State,
    search: Entity<InputState>,
    /// Bumped on every page change so the viewer rebuilds its state.
    page_serial: usize,
    /// The generated page. The shell draws it, in the place this view
    /// leaves empty and reports through `page_slot`.
    doc_page: Entity<doc_view::DocPage>,
    page_slot: Rc<doc_view::PageSlot>,
    /// Whether this frame left a place for the page.
    page_placed: std::cell::Cell<bool>,
    /// Markdown of the current page, ready for the viewer.
    rendered: String,
    /// Diagrams of the current page, addressed by index from `rendered`.
    diagrams: Vec<std::sync::Arc<Image>>,
    /// Every diagram drawn so far, and those still to draw.
    diagram_store: diagrams::Diagrams,
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
    /// Whether the sidebar and the details pane are shown; the user hides
    /// and shows each, and `panes` remembers it.
    sidebar_open: bool,
    details_open: bool,
    /// How much of each shows in this frame, from 0 to 1: between the two
    /// while a pane slides in or out.
    sidebar_shown: f32,
    details_shown: f32,
    /// What the Settings page remembers: the backend's settings, the text
    /// inputs, the news beside each field.
    settings: settings_view::State,
    _subscriptions: Vec<Subscription>,
}

/// Width of the sidebar and of the details pane, in pixels.
const SIDEBAR_WIDTH: f32 = 280.0;
const DETAILS_WIDTH: f32 = 260.0;
/// Diameter of the colored dot before each outline entry, in pixels.
const OUTLINE_DOT: f32 = 6.0;

impl AppView {
    fn new(
        page_slot: Rc<doc_view::PageSlot>,
        events: (async_channel::Sender<Event>, async_channel::Receiver<Event>),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // What a link, row or button of a generated page asks for comes back
        // here.
        let view = cx.entity().downgrade();
        let on_action: doc_view::OnAction = Rc::new(move |action, _, cx| {
            let action = action.clone();
            _ = view.update(cx, |this, cx| this.run_action(&action, cx));
        });
        let doc_page = cx.new(|_| doc_view::DocPage::new(on_action));
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
        // Tab moves the focus on from the theme menu, which then closes.
        let theme_menu_focus = cx.focus_handle();
        let theme_menu_blur = cx.on_blur(&theme_menu_focus, window, |this, window, cx| {
            this.close_theme_menu(false, window, cx);
        });
        // Whatever changes in the app may change what the tray says of it.
        let tray_follows = cx.observe_self(|this, _| this.refresh_tray());
        let sidebar_open = panes::load(Pane::Sidebar);
        let details_open = panes::load(Pane::Details);
        let (settings, settings_subscriptions) = settings_view::State::new(window, cx);
        let mut this = Self {
            backend: None,
            repos: Vec::new(),
            repo: None,
            notes: Vec::new(),
            notes_loaded: false,
            pending_proposals: Vec::new(),
            recent: startup::recent(),
            missing: Vec::new(),
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
            requested_view: startup::launch().view.clone(),
            pending_request: None,
            tray: None,
            backend_status: status::Backend::default(),
            backend_silent: false,
            focus: cx.focus_handle(),
            theme_menu_open: false,
            theme_menu_highlight: 0,
            theme_menu_focus,
            theme_anchor: AnchorBounds::default(),
            editing: None,
            busy: Some("Connecting".into()),
            error: None,
            update: update::State::NotInstalled,
            search,
            page_serial: 0,
            doc_page,
            page_slot,
            page_placed: std::cell::Cell::new(false),
            rendered: String::new(),
            diagrams: Vec::new(),
            diagram_store: diagrams::Diagrams::default(),
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
            sidebar_open,
            details_open,
            sidebar_shown: if sidebar_open { 1.0 } else { 0.0 },
            details_shown: if details_open { 1.0 } else { 0.0 },
            settings,
            _subscriptions: vec![
                subscription,
                palette_subscription,
                appearance,
                theme_menu_blur,
                tray_follows,
            ],
        };
        let (to_app, events) = events;
        this._subscriptions.extend(settings_subscriptions);
        this.connect(cx);
        this.start_updates(cx);
        this.start_tray(to_app, window);
        this.start_status_poll(cx);
        this.listen(events, window, cx);
        this
    }

    /// Put the app's icon in the notification area. Without one (Windows
    /// refused, or the window has no name to hide it by) the app is as it
    /// was before: closing the window ends it.
    fn start_tray(&mut self, to_app: async_channel::Sender<Event>, window: &Window) {
        if win32::hwnd(window).is_none() {
            log::warn!("no tray icon: the window cannot be hidden");
            return;
        }
        let (show, menu) = (to_app.clone(), to_app.clone());
        let made = tray::Tray::new(
            &self.tray_snapshot(),
            win32::small_icon_size(),
            move || _ = show.try_send(Event::Show),
            move |command| _ = menu.try_send(Event::Menu(command)),
        );
        match made {
            Ok(tray) => self.tray = Some(tray),
            Err(e) => {
                log::warn!("no tray icon ({e}): closing the window ends the app");
                // Started for the tray, the app would be running with nothing
                // of it on screen and no way to reach it.
                if startup::launch().tray {
                    _ = to_app.try_send(Event::Show);
                }
            }
        }
    }

    /// What the tray shows of the app right now.
    fn tray_snapshot(&self) -> tray::Snapshot {
        let current = self.repo.as_ref().map(|repo| repo.id.as_str());
        // Another repository is named, as the user knows it; the open one is
        // matched by its id.
        let updating: Vec<String> = self
            .backend_status
            .updating
            .iter()
            .map(|id| {
                self.repos
                    .iter()
                    .find(|repo| &repo.id == id && Some(id.as_str()) != current)
                    .map_or_else(|| id.clone(), |repo| repo.name.clone())
            })
            .collect();
        let connected = self.backend.is_some() && !self.backend_silent;
        tray::Snapshot {
            repo: self.repo.as_ref().map(|repo| repo.name.clone()),
            state: status::state_text(connected, self.busy.as_deref(), &updating, current),
            mcp: status::mcp_text(self.backend_status.bridge),
            can_sync: connected
                && self.repo.is_some()
                && self.busy.is_none()
                && updating.is_empty()
                && !self.update.installing(),
            recent: self.recent.clone(),
            update: self
                .update
                .available()
                .map(|release| release.version.to_string()),
        }
    }

    fn refresh_tray(&mut self) {
        if self.tray.is_some() {
            let snapshot = self.tray_snapshot();
            if let Some(tray) = &mut self.tray {
                tray.refresh(&snapshot);
            }
        }
    }

    /// Ask the backend every few seconds what it is doing, for as long as
    /// the app runs: the tray icon says it, also while the window is hidden.
    fn start_status_poll(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let executor = cx.background_executor().clone();
            loop {
                // Not while an update is installed: it stops the backend.
                let Ok(backend) = this.update(cx, |this, _| {
                    this.backend.clone().filter(|_| !this.update.installing())
                }) else {
                    return;
                };
                let answer = match backend {
                    Some(backend) => Some(executor.spawn(async move { backend.status() }).await),
                    None => None,
                };
                let pause = match &answer {
                    Some(Ok(None)) => OLD_BACKEND_PAUSE,
                    _ => STATUS_PAUSE,
                };
                if this.update(cx, |this, _| this.take_status(answer)).is_err() {
                    return;
                }
                executor.timer(pause).await;
            }
        })
        .detach();
    }

    /// Keep what the backend answered to `GET /v1/status`; `None` when it
    /// was not asked. The window shows none of it, so nothing is redrawn.
    fn take_status(&mut self, answer: Option<Result<Option<Value>, String>>) {
        match answer {
            Some(Ok(Some(body))) => {
                self.backend_status = status::parse(&body);
                self.backend_silent = false;
            }
            Some(Ok(None)) => {
                self.backend_status = status::Backend {
                    updating: Vec::new(),
                    bridge: status::Bridge::OldBackend,
                };
                self.backend_silent = false;
            }
            Some(Err(e)) => {
                if !self.backend_silent {
                    log::info!("the backend did not say what it is doing: {e}");
                }
                self.backend_status = status::Backend::default();
                self.backend_silent = true;
            }
            None => {
                self.backend_status = status::Backend::default();
                self.backend_silent = false;
            }
        }
        self.refresh_tray();
    }

    /// Carry out what reaches the app from outside GPUI, one event at a
    /// time, for as long as the window lives.
    fn listen(
        &mut self,
        events: async_channel::Receiver<Event>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let hwnd = win32::hwnd(window);
        // Started for the tray, the window has never been on screen. GPUI
        // puts it in its place when it is first activated; showing it before
        // that would show it where Windows happens to put a new window.
        let mut placed = !startup::launch().tray;
        cx.spawn_in(window, async move |this, cx| {
            while let Ok(event) = events.recv().await {
                // Hiding and showing make Windows call into GPUI before they
                // return, so both are done here, between two updates of the
                // app, where GPUI is in the middle of nothing.
                if matches!(event, Event::Hide)
                    && let Some(hwnd) = hwnd
                {
                    win32::hide(hwnd);
                }
                let Ok(show) = this.update_in(cx, |this, _, cx| this.handle_event(event, cx))
                else {
                    return;
                };
                if !show {
                    continue;
                }
                if let Some(hwnd) = hwnd
                    && placed
                    && !win32::is_visible(hwnd)
                {
                    win32::show(hwnd);
                }
                placed = true;
                // Restores a minimized window and brings it to the front.
                if this
                    .update_in(cx, |_, window, _| window.activate_window())
                    .is_err()
                {
                    return;
                }
            }
        })
        .detach();
    }

    /// Carry out one event. Returns whether the window is to be shown and
    /// brought to the front afterwards.
    fn handle_event(&mut self, event: Event, cx: &mut Context<Self>) -> bool {
        match event {
            Event::Handover(request) => {
                log::info!(
                    "a second launch handed over: folder {:?}, view {:?}",
                    request.repo,
                    request.view
                );
                // Started for the tray, it wants no window; the app runs.
                let show = !request.tray;
                self.open_request(request, cx);
                show
            }
            Event::Hide => {
                self.after_hide(cx);
                false
            }
            Event::Show => true,
            Event::Menu(command) => self.run_tray_command(command, cx),
        }
    }

    /// The window has just been hidden. The first time ever, say that the
    /// app is still there: a window that closes and a program that goes on
    /// is not what closing means elsewhere.
    fn after_hide(&mut self, cx: &mut Context<Self>) {
        log::info!("the window is hidden; the app stays in the tray");
        if prefs::read(TRAY_HINT).is_some() {
            return;
        }
        prefs::write(TRAY_HINT, "shown");
        // Windows shows a notification only of an app it knows by name. The
        // name is given here, not at start: it also decides which taskbar
        // button the window belongs to, and this way that is touched once in
        // the life of an installation instead of on every start.
        cx.set_app_identity(APP_IDENTITY, "CodeWiki");
        cx.show_system_notification(SystemNotification {
            tag: TRAY_HINT.into(),
            title: "CodeWiki is still running".into(),
            body: "Click its icon in the notification area to open it again, right-click the icon to quit."
                .into(),
            actions: Vec::new(),
        });
    }

    /// Carry out a row of the tray menu. Returns whether the window is to be
    /// shown.
    fn run_tray_command(&mut self, command: tray::Command, cx: &mut Context<Self>) -> bool {
        log::info!("tray menu: {command:?}");
        match command {
            tray::Command::Open => true,
            // Without the window: the tray says when it is done.
            tray::Command::Sync => {
                self.sync(cx);
                false
            }
            tray::Command::Recent(path) => {
                self.open_repo_folder(path, Landing::WikiMap, cx);
                true
            }
            tray::Command::Settings => {
                self.open_settings(None, cx);
                true
            }
            tray::Command::CheckUpdates => {
                // The result shows on the settings' Updates page.
                self.open_settings(Some(settings::Category::Updates), cx);
                self.check_for_update(cx);
                true
            }
            // Home offers the newer release.
            tray::Command::ShowUpdate => {
                self.open_home(cx);
                true
            }
            tray::Command::Quit => self.quit(cx),
        }
    }

    /// End the app, and with it its backend (`on_app_quit` in `main`). Not
    /// with unsaved edits, which would be lost without a word: the app then
    /// stays, says so, and returns true, for its window to be shown.
    fn quit(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.can_leave() {
            cx.notify();
            return true;
        }
        log::info!("quitting, as asked");
        // Takes the icon away at once; Windows would otherwise keep it until
        // the pointer passes over it.
        self.tray = None;
        cx.quit();
        false
    }

    /// Open what a second launch asked for: its folder, on its view, or the
    /// view alone in the repository that is open. A launch that named
    /// neither only brought the window up.
    fn open_request(&mut self, request: instance::Request, cx: &mut Context<Self>) {
        if self.backend.is_none() {
            // Still connecting. `connect` comes back to it.
            self.pending_request = Some(request);
            return;
        }
        match request.repo {
            Some(path) => {
                self.requested_view = request.view;
                self.open_repo_folder(path, Landing::Startup, cx);
            }
            None if request.view.is_some() && self.repo.is_some() => {
                self.requested_view = request.view;
                self.open_startup_view(cx);
                cx.notify();
            }
            None => {}
        }
    }

    /// Run blocking backend work off the UI thread, then apply the result.
    /// Nothing starts while an update is installed: it stops the backend.
    fn run<T: Send + 'static>(
        &mut self,
        label: &str,
        cx: &mut Context<Self>,
        work: impl FnOnce(&Backend) -> Result<T, String> + Send + 'static,
        done: impl FnOnce(&mut Self, T, &mut Context<Self>) + 'static,
    ) {
        if self.update.installing() {
            return;
        }
        let Some(backend) = self.backend.clone() else {
            return;
        };
        self.busy = Some(label.to_string());
        self.error = None;
        cx.notify();
        let label = label.to_string();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { work(&backend) })
                .await;
            _ = this.update(cx, |this, cx| {
                this.busy = None;
                match result {
                    Ok(value) => done(this, value, cx),
                    Err(message) => {
                        log::warn!("{label}: {message}");
                        this.error = Some(message);
                    }
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
                    let opened = startup::repo().and_then(|path| backend.repo_id(&path).ok());
                    let (repos, missing) = registered_repos(&backend)?;
                    Ok::<_, String>((backend, repos, missing, opened))
                })
                .await;
            _ = this.update(cx, |this, cx| {
                this.busy = None;
                match result {
                    Ok((backend, repos, missing, opened)) => {
                        this.backend = Some(backend);
                        this.take_repos(repos, missing);
                        let first = opened
                            .and_then(|id| this.repos.iter().find(|r| r.id == id).cloned())
                            .or_else(|| this.newest_recent());
                        match first {
                            Some(repo) => this.select_repo(repo, Landing::Startup, cx),
                            None => this.set_page(Page::Home, false),
                        }
                        if let Some(request) = this.pending_request.take() {
                            this.open_request(request, cx);
                        }
                    }
                    Err(message) => {
                        log::error!("the backend is not reachable: {message}");
                        this.error = Some(message);
                        let log = cw_core::logs::path("backend").display().to_string();
                        let logs = cw_core::logs::dir().display().to_string();
                        // The backend saved in the settings, when it is the
                        // one that was tried: the variable wins over it.
                        let remote = cw_client::saved_remote()
                            .filter(|remote| remote.in_use && cw_client::env_remote().is_none())
                            .map(|remote| remote.url);
                        this.set_page(
                            Page::Notice(Rc::new(live::offline_page(
                                &log,
                                &logs,
                                remote.as_deref(),
                            ))),
                            false,
                        );
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Try the backend again after it could not be reached. Does nothing
    /// while connected or while another attempt runs.
    fn reconnect(&mut self, cx: &mut Context<Self>) {
        if self.backend.is_some() || self.busy.is_some() {
            return;
        }
        self.busy = Some("Connecting".into());
        self.error = None;
        self.set_page(Page::Message(String::new()), false);
        cx.notify();
        self.connect(cx);
    }

    /// At start: undo an update that did not finish or clear what a finished
    /// one left, then look for a new release if the daily check is due. Only
    /// an installed copy does either.
    fn start_updates(&mut self, cx: &mut Context<Self>) {
        let Some(install) = cw_update::installed_dir() else {
            return;
        };
        let check = update::auto_check() && update::due();
        self.update = if check {
            update::State::Checking
        } else {
            update::State::Idle
        };
        cx.spawn(async move |this, cx| {
            let executor = cx.background_executor().clone();
            let found = executor
                .spawn({
                    let install = install.clone();
                    async move {
                        for line in cw_update::recover_and_clean(&install) {
                            log::info!("update: {line}");
                        }
                        check.then(update::check)
                    }
                })
                .await;
            if let Some(found) = found {
                _ = this.update(cx, |this, cx| this.show_check(found, cx));
            }
            // Right after an update, the app this one replaced is still
            // ending, and Windows does not delete a program that runs. Clean
            // up once more when it has had time to go.
            for _ in 0..CLEANUP_RETRIES {
                if cw_update::leftovers(&install).is_empty() {
                    break;
                }
                executor.timer(CLEANUP_PAUSE).await;
                let install = install.clone();
                executor
                    .spawn(async move {
                        for line in cw_update::recover_and_clean(&install) {
                            log::info!("update: {line}");
                        }
                    })
                    .await;
            }
            // The app stays in the tray, for weeks perhaps: look again
            // whenever the daily check is due. It only ever offers.
            loop {
                executor.timer(RECHECK_PAUSE).await;
                let Ok(due) = this.update(cx, |this, _| {
                    this.update.can_check() && update::auto_check() && update::due()
                }) else {
                    return;
                };
                if !due {
                    continue;
                }
                let found = executor.spawn(async { update::check() }).await;
                if this
                    .update(cx, |this, cx| this.show_check(found, cx))
                    .is_err()
                {
                    return;
                }
            }
        })
        .detach();
    }

    /// Look for a newer release now.
    fn check_for_update(&mut self, cx: &mut Context<Self>) {
        if !self.update.can_check() {
            return;
        }
        self.update = update::State::Checking;
        self.refresh_settings(cx);
        cx.spawn(async move |this, cx| {
            let found = cx
                .background_executor()
                .spawn(async { update::check() })
                .await;
            _ = this.update(cx, |this, cx| this.show_check(found, cx));
        })
        .detach();
    }

    fn show_check(&mut self, found: cw_update::Check, cx: &mut Context<Self>) {
        match &found {
            cw_update::Check::Newer(release) => {
                log::info!("update: {} is available", release.version);
            }
            cw_update::Check::UpToDate => log::info!("update: this is the latest version"),
            cw_update::Check::Unavailable(why) => log::warn!("update: could not check: {why}"),
        }
        self.update = update::State::from_check(found);
        self.refresh_settings(cx);
    }

    /// Show what changed on the settings page, when it is open. The page is
    /// built from the app's state on every frame, so a new frame is all it
    /// takes.
    fn refresh_settings(&mut self, cx: &mut Context<Self>) {
        cx.notify();
    }

    /// Download the release the last check found, put it in place of this
    /// installation and restart. The backend is stopped first: it would keep
    /// serving the old code.
    fn install_update(&mut self, cx: &mut Context<Self>) {
        let Some(release) = self.update.available().cloned() else {
            return;
        };
        if let Some(busy) = &self.busy {
            self.error = Some(format!(
                "Wait for “{busy}” to finish, then install the update."
            ));
            cx.notify();
            return;
        }
        // A restart would lose unsaved edits.
        if !self.can_leave() {
            cx.notify();
            return;
        }
        let Some(install) = cw_update::installed_dir() else {
            return;
        };
        let version = release.version.clone();
        log::info!("update: installing {version} into {}", install.display());
        self.update = update::State::Installing(version.clone());
        self.busy = Some(format!("Updating to {version}"));
        self.error = None;
        self.refresh_settings(cx);
        // A backend named by CODEWIKI_URL runs elsewhere; it is not ours.
        let backend = self.backend.clone().filter(|_| !cw_client::uses_remote());
        // The new app opens the same repository; without one, the one used
        // last.
        let args: Vec<std::ffi::OsString> = self
            .repo
            .as_ref()
            .map(|repo| repo.path.clone())
            .filter(|path| std::path::Path::new(path).is_dir())
            .map(Into::into)
            .into_iter()
            .collect();
        cx.spawn(async move |this, cx| {
            let (outcome, backend_answers) = cx
                .background_executor()
                .spawn(async move {
                    let installed = cw_update::install(&install, &release, || match &backend {
                        Some(backend) if !backend.stop() => Err(
                            "CodeWiki's backend is still busy with other work; try again when it finishes"
                                .to_string(),
                        ),
                        _ => Ok(()),
                    });
                    let outcome = match installed {
                        Ok(how) => {
                            log::info!("update: {how:?}; restarting");
                            // The new app must become the running one, not
                            // hand over to this one, which is about to end.
                            instance::release();
                            match cw_update::relaunch(&install, &args) {
                                Ok(()) => Ok(true),
                                Err(e) => {
                                    log::error!("update: {e}");
                                    Ok(false)
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!("update: {e}");
                            Err((release, e))
                        }
                    };
                    // A backend that was not ours to stop is still there.
                    let answers = backend.as_ref().is_none_or(Backend::answers);
                    (outcome, answers)
                })
                .await;
            _ = this.update(cx, |this, cx| {
                this.busy = None;
                if !backend_answers {
                    this.backend = None;
                }
                match outcome {
                    Ok(true) => {
                        cx.quit();
                        return;
                    }
                    Ok(false) => {
                        this.update = update::State::Installed(version.clone());
                        this.error = Some(format!(
                            "CodeWiki {version} is installed, but it could not restart itself. Close CodeWiki and open it again."
                        ));
                    }
                    Err((release, e)) => {
                        this.update = update::State::Available(release);
                        this.error = Some(format!("The update to {version} failed: {e}."));
                        // The swap failed after the backend was stopped. The
                        // old programs are back in place: start it again.
                        if this.backend.is_none() {
                            this.restart_backend(cx);
                        }
                    }
                }
                this.refresh_settings(cx);
            });
        })
            .detach();
    }

    /// Find or start a backend again without leaving the page or clearing
    /// the message shown. On failure the palette offers "Reconnect".
    fn restart_backend(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async { Backend::connect() })
                .await;
            _ = this.update(cx, |this, cx| {
                match result {
                    Ok(backend) => this.backend = Some(backend),
                    Err(message) => log::warn!("the backend did not start again: {message}"),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Ask for a folder, then open it as a repository.
    fn open_folder(&mut self, cx: &mut Context<Self>) {
        if self.backend.is_none() {
            return;
        }
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Open".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = chosen.await else {
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let path = path.to_string_lossy().into_owned();
            _ = this.update(cx, |this, cx| {
                this.open_repo_folder(path, Landing::WikiMap, cx);
            });
        })
        .detach();
    }

    /// Register `path` with the backend, if it is new, and open it: on its
    /// wiki map when the user picked the folder, on the view a launch named
    /// when `landing` is [`Landing::Startup`].
    fn open_repo_folder(&mut self, path: String, landing: Landing, cx: &mut Context<Self>) {
        self.run(
            "Opening",
            cx,
            move |b| {
                let id = b.repo_id(&path)?;
                let (repos, missing) = registered_repos(b)?;
                Ok((id, repos, missing))
            },
            move |this, (id, repos, missing), cx| {
                this.take_repos(repos, missing);
                if landing == Landing::Startup {
                    this.open_launched_repo(&id, cx);
                } else {
                    this.open_repo(&id, cx);
                }
            },
        );
    }

    /// Take the repository `id` off the backend's list, as the user asked on
    /// Home for one whose folder is not found. Nothing on disk is deleted,
    /// and opening the folder again registers it under the same id.
    fn forget_repo(&mut self, id: String, cx: &mut Context<Self>) {
        self.run(
            "Removing",
            cx,
            move |b| {
                b.delete(&format!("/v1/repos/{id}"))?;
                log::info!("forgot repository {id}, as asked");
                registered_repos(b)
            },
            |this, (repos, missing), cx| {
                this.take_repos(repos, missing);
                cx.notify();
            },
        );
    }

    /// Keep the repositories the backend listed: those whose folders are
    /// there, and those whose folders are not found. The recent list loses
    /// every folder that is no longer registered, and keeps a missing one,
    /// so that it is in its old place when its folder is back.
    fn take_repos(&mut self, repos: Vec<Repo>, missing: Vec<Repo>) {
        let paths: Vec<&str> = repos
            .iter()
            .chain(&missing)
            .map(|repo| repo.path.as_str())
            .collect();
        self.recent = startup::keep_recent(&paths);
        self.repos = repos;
        self.missing = missing;
    }

    /// The registered repository opened most recently, if any.
    fn newest_recent(&self) -> Option<Repo> {
        self.recent
            .iter()
            .find_map(|path| self.repos.iter().find(|repo| &repo.path == path))
            .cloned()
    }

    /// Show Home, unless it is on screen already.
    fn open_home(&mut self, cx: &mut Context<Self>) {
        if self.backend.is_none() || matches!(self.page, Page::Home) {
            return;
        }
        self.set_page(Page::Home, true);
        cx.notify();
    }

    /// Open the registered repository `id`, as the user picked it: its wiki
    /// map, or Home while it has no wiki. The open repository keeps its tabs.
    fn open_repo(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.repo.as_ref().is_some_and(|repo| repo.id == id) {
            if self.notes_loaded && self.notes.is_empty() {
                self.open_home(cx);
            } else {
                self.open_note(WIKI_MAP.into(), true, cx);
            }
            return;
        }
        if let Some(repo) = self.repos.iter().find(|repo| repo.id == id).cloned() {
            self.select_repo(repo, Landing::WikiMap, cx);
        }
    }

    /// Open the registered repository `id` as a launch that handed over
    /// asked: on the view it named, as a start of the app would. A launch
    /// that named only the repository that is open already changes nothing;
    /// its window came up, and the page stays where the user left it.
    fn open_launched_repo(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.repo.as_ref().is_some_and(|repo| repo.id == id) {
            if self.requested_view.is_some() {
                self.open_startup_view(cx);
                cx.notify();
            }
            return;
        }
        if let Some(repo) = self.repos.iter().find(|repo| repo.id == id).cloned() {
            self.select_repo(repo, Landing::Startup, cx);
        }
    }

    fn select_repo(&mut self, repo: Repo, landing: Landing, cx: &mut Context<Self>) {
        if !self.can_leave() {
            cx.notify();
            return;
        }
        self.recent = startup::remember(&repo.path);
        log::info!("repository: {} ({})", repo.name, repo.path);
        let from_home = matches!(self.page, Page::Home);
        self.repo = Some(repo);
        // Nothing of the repository left behind may show, on Home least of
        // all, while the new one loads.
        self.notes.clear();
        self.notes_loaded = false;
        self.pending_proposals.clear();
        self.history.clear();
        if from_home && landing == Landing::WikiMap {
            // Back leads to Home, where the repository was picked.
            self.history.push(Page::Home);
        }
        self.tabs.truncate(1);
        self.active_tab = 0;
        self.refresh(landing, cx);
    }

    /// Reload the note list and proposals, then go where `landing` says. A
    /// repository with no wiki lands on Home, which offers to sync it.
    fn refresh(&mut self, landing: Landing, cx: &mut Context<Self>) {
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
                this.notes_loaded = true;
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
                match landing {
                    Landing::Stay => {}
                    _ if this.notes.is_empty() => {
                        this.history.clear();
                        this.set_page(Page::Home, false);
                    }
                    Landing::WikiMap => this.open_note(WIKI_MAP.into(), false, cx),
                    Landing::Startup => this.open_startup_view(cx),
                }
            },
        );
    }

    /// Show the view the launch named (its second argument, or that of the
    /// last launch that handed over), so that other tools can open the app
    /// on a note or a view; Home when there is none.
    fn open_startup_view(&mut self, cx: &mut Context<Self>) {
        let first = self.requested_view.clone().unwrap_or_default();
        if first.is_empty() || first == "home:" {
            self.set_page(Page::Home, false);
            return;
        }
        if first == "graph:" {
            self.open_graph(cx);
            return;
        }
        // Forms used to open the app on a view from a script.
        if let Some(note) = first.strip_prefix("edit:") {
            self.pending_edit = true;
            self.open_note(note.to_string(), false, cx);
            return;
        }
        if let Some(query) = first.strip_prefix("palette:") {
            self.pending_palette = Some(query.to_string());
            self.set_page(Page::Home, false);
            cx.notify();
            return;
        }
        // `settings:` opens the settings; `settings:ai` one of their
        // categories, by its slug.
        if let Some(slug) = first.strip_prefix("settings:") {
            let category = settings::Category::from_slug(slug);
            self.pending_command = Some(Command::Settings(Some(category)));
            cx.notify();
            return;
        }
        let command = match first.as_str() {
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
            // The theme menu opens over a page, not in place of one, and it
            // hangs from the toolbar, which Home does not have.
            let over_map = command == Command::Themes;
            self.pending_command = Some(command);
            if over_map {
                self.open_note(WIKI_MAP.into(), false, cx);
            }
            cx.notify();
            return;
        }
        match first.strip_prefix("file:") {
            Some(path) => self.open_file(path.to_string(), cx),
            None => self.open_note(first, false, cx),
        }
    }

    /// The backend to stop when the app ends: the one it uses, whoever
    /// started it. Not one named by `CODEWIKI_URL`, which is not this app's
    /// to stop, and none while an update is installed, which has stopped it
    /// already and may have started the next version's.
    fn backend_to_stop(&self) -> Option<Backend> {
        if stops_backend_on_quit(cw_client::uses_remote(), self.update.installing()) {
            self.backend.clone()
        } else {
            None
        }
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
                this.refresh(Landing::Stay, cx);
                this.open_note(reopen, false, cx);
            },
        );
    }

    fn set_page(&mut self, page: Page, remember: bool) {
        if !self.can_leave() {
            return;
        }
        let previous = std::mem::replace(&mut self.page, page);
        if remember && !matches!(previous, Page::Message(_) | Page::Notice(_)) {
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
        self.close_theme_menu(false, window, cx);
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

    /// Hide the sidebar or the details pane, or show it again, and remember
    /// the choice.
    fn toggle_pane(&mut self, pane: Pane, cx: &mut Context<Self>) {
        let open = match pane {
            Pane::Sidebar => &mut self.sidebar_open,
            Pane::Details => &mut self.details_open,
        };
        *open = !*open;
        panes::store(pane, *open);
        cx.notify();
    }

    /// The width the sidebar and the details pane take of this frame, in
    /// pixels.
    fn side_width(&self) -> f32 {
        SIDEBAR_WIDTH * self.sidebar_shown + DETAILS_WIDTH * self.details_shown
    }

    /// Entries the palette lists for what is typed now.
    fn palette_matches(&self, cx: &App) -> Vec<palette::Entry> {
        let mut entries = palette::actions();
        if self.backend.is_none() && self.busy.is_none() {
            entries.insert(
                0,
                palette::Entry::new("Reconnect to the backend", "Command", Command::Reconnect),
            );
        }
        // Only an installed copy can start with Windows; the entry says
        // what choosing it does.
        if let Some(on) = autostart::enabled() {
            entries.push(palette::Entry::new(
                if on {
                    "Start with Windows: turn off"
                } else {
                    "Start with Windows: turn on"
                },
                "Setting",
                Command::Autostart(!on),
            ));
        }
        if let Some(release) = self.update.available() {
            entries.insert(
                0,
                palette::Entry::new(
                    // "Update" in the label: it is what someone types.
                    format!("Update CodeWiki to {} and restart", release.version),
                    "Command",
                    Command::InstallUpdate,
                ),
            );
        }
        for note in &self.notes {
            let label = match note.title.as_str() {
                WIKI_MAP => "Wiki map".to_string(),
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
        for theme in themes::entries(cx) {
            entries.push(palette::Entry::new(
                format!("Theme: {}", theme.name),
                "Theme",
                Command::Theme(theme.name.to_string()),
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
            Command::Home => self.open_home(cx),
            Command::Note(path) => self.open_note(path, true, cx),
            Command::Theme(name) => self.select_theme(&name, cx),
            Command::Repo(id) => self.open_repo(&id, cx),
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
            Command::Themes => self.open_theme_menu(window, cx),
            Command::Settings(category) => self.open_settings(category, cx),
            Command::Motion(pref) => {
                motion::set(pref, cx);
                cx.notify();
            }
            Command::NewTab => self.new_tab(cx),
            Command::CloseTab => self.close_tab(self.active_tab, cx),
            Command::ToggleSidebar => self.toggle_pane(Pane::Sidebar, cx),
            Command::ToggleDetails => self.toggle_pane(Pane::Details, cx),
            Command::OpenInObsidian => self.open_in_obsidian(cx),
            Command::Reconnect => self.reconnect(cx),
            Command::OpenFolder => self.open_folder(cx),
            Command::OpenLogs => self.open_logs(cx),
            Command::CheckUpdates => {
                // The result shows on the settings page.
                self.open_settings(Some(settings::Category::Updates), cx);
                self.check_for_update(cx);
            }
            Command::InstallUpdate => self.install_update(cx),
            Command::Quit => {
                // The window is open, so a refusal shows where it stands.
                self.quit(cx);
            }
            // The page with the switch then shows what Windows has.
            Command::Autostart(on) => {
                self.run_action(&Action::Autostart(on), cx);
                self.open_settings(Some(settings::Category::Appearance), cx);
            }
        }
    }

    /// Fetch one analysis from the backend and show it as a generated page.
    fn open_report(
        &mut self,
        title: &'static str,
        route: &'static str,
        render: fn(&Value) -> Doc,
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
                        doc: Rc::new(render(&data)),
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
        render: fn(&Value) -> Doc,
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
                        doc: Rc::new(render(&data)),
                    },
                    true,
                );
            },
        );
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

    /// Build the viewer's Markdown for the current page. Nothing is drawn
    /// here: a diagram comes from the store, and one the store does not have
    /// yet leaves a line in its place until `draw_queued_diagrams` has it.
    fn prepare_page(&mut self) {
        self.diagrams.clear();
        let palette = self.palette;
        self.rendered = match &self.page {
            Page::Note { text, .. } => {
                let diagrams = &mut self.diagrams;
                let store = &mut self.diagram_store;
                markdown::prepare_note_with(text, |source| match store.lookup(source, palette) {
                    diagrams::Lookup::Ready(Some(image)) => {
                        diagrams.push(image);
                        markdown::Diagram::Image(format!(
                            "{}{}",
                            markdown::DIAGRAM_SCHEME,
                            diagrams.len() - 1
                        ))
                    }
                    diagrams::Lookup::Ready(None) => markdown::Diagram::Source,
                    diagrams::Lookup::Pending => markdown::Diagram::Drawing,
                })
            }
            // Drawn from their own model, not from Markdown.
            Page::Message(_)
            | Page::Notice(_)
            | Page::Generated { .. }
            | Page::Proposal { .. }
            | Page::Graph
            | Page::Home
            | Page::Settings(_) => String::new(),
        };
    }

    /// Draw the diagrams `prepare_page` found missing, off the UI thread, then
    /// show them. Drawing one takes seconds in a debug build, and the first of
    /// a session also reads every installed font.
    fn draw_queued_diagrams(&mut self, cx: &mut Context<Self>) {
        let jobs = self.diagram_store.take_queued();
        if jobs.is_empty() {
            return;
        }
        cx.spawn(async move |this, cx| {
            let drawn = cx
                .background_executor()
                .spawn(async move {
                    jobs.into_iter()
                        .map(|(palette, source)| {
                            let started = std::time::Instant::now();
                            let image = cw_mermaid::render_png(&source, palette).ok().map(|png| {
                                std::sync::Arc::new(Image::from_bytes(ImageFormat::Png, png))
                            });
                            log::debug!("diagram drawn in {} ms", started.elapsed().as_millis());
                            ((palette, source), image)
                        })
                        .collect::<Vec<_>>()
                })
                .await;
            _ = this.update(cx, |this, cx| {
                for (key, image) in drawn {
                    this.diagram_store.store(key, image);
                }
                // The page may have changed meanwhile; the store answers for
                // whichever page is shown now. The page's serial stays, so
                // its scroll position and selection do too.
                this.prepare_page();
                cx.notify();
            });
        })
        .detach();
    }

    fn go_back(&mut self, cx: &mut Context<Self>) {
        // The settings fill the window: with no page before them (the app
        // was started on them) Back must still lead somewhere.
        if self.history.is_empty() && self.on_settings_page() {
            self.leave_settings(cx);
            return;
        }
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
                // `--illustration` opens the first note on its illustration.
                this.show_figure = this.figure.is_some()
                    && this.history.is_empty()
                    && startup::launch().illustration;
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
                        doc: Rc::new(live::symbol_page(&detail)),
                    },
                    true,
                );
            },
        );
    }

    /// Open the theme menu under the toolbar's theme button, with the theme
    /// in use highlighted, or close it when it is open.
    fn open_theme_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.theme_menu_open {
            self.close_theme_menu(true, window, cx);
            return;
        }
        if self.palette_open {
            self.toggle_palette(window, cx);
        }
        let active = cx.theme().theme_name().clone();
        self.theme_menu_highlight = themes::entries(cx)
            .iter()
            .position(|entry| entry.name == active)
            .unwrap_or(0);
        self.theme_menu_open = true;
        window.focus(&self.theme_menu_focus, cx);
        cx.notify();
    }

    /// Close the theme menu. `restore_focus` is false when the focus has
    /// already gone where it should be.
    fn close_theme_menu(
        &mut self,
        restore_focus: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.theme_menu_open {
            return;
        }
        self.theme_menu_open = false;
        if restore_focus {
            window.focus(&self.focus, cx);
        }
        cx.notify();
    }

    /// Apply a theme and remember it. An open theme menu stays open, so that
    /// several can be tried in a row.
    fn select_theme(&mut self, name: &str, cx: &mut Context<Self>) {
        themes::select(name, cx);
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
                        doc: Rc::new(live::file_page(&context, &risk, &radius)),
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
                        doc: Rc::new(live::hotspots_page(&rows)),
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
                        doc: Rc::new(live::search_page(&shown, &hits)),
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
                this.set_page(
                    Page::Generated {
                        title: "Sync".into(),
                        doc: Rc::new(live::sync_page(&report)),
                    },
                    true,
                );
                this.refresh(Landing::Stay, cx);
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
                        doc: Rc::new(live::proposal_page(&detail)),
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
                this.refresh(Landing::Stay, cx);
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

    /// Follow a link clicked in a note.
    fn follow_link(&mut self, url: &str, cx: &mut Context<Self>) {
        if let Some(target) = url.strip_prefix(NOTE_SCHEME) {
            self.run_action(&Action::Note(markdown::decode(target)), cx);
        } else if url.starts_with("https://") || url.starts_with("http://") {
            cx.open_url(url);
        }
    }

    /// Do what a link, row or button of a generated page asks for.
    fn run_action(&mut self, action: &Action, cx: &mut Context<Self>) {
        match action {
            Action::Note(target) => self.open_note(target.clone(), true, cx),
            Action::Symbol(qualified) => self.open_symbol(qualified.clone(), cx),
            Action::File(path) => self.open_file(path.clone(), cx),
            Action::Motion(pref) => {
                motion::set(*pref, cx);
                cx.notify();
            }
            Action::Reconnect => self.reconnect(cx),
            Action::OpenFolder => self.open_folder(cx),
            Action::CheckUpdates => self.check_for_update(cx),
            Action::InstallUpdate => self.install_update(cx),
            Action::AutoUpdate(on) => {
                update::set_auto_check(*on);
                cx.notify();
            }
            Action::Autostart(on) => {
                match autostart::set(*on) {
                    Ok(()) => log::info!("start with Windows: {}", if *on { "on" } else { "off" }),
                    Err(e) => {
                        log::warn!("start with Windows: {e}");
                        self.error = Some(e);
                    }
                }
                // The page reads the setting back from where Windows keeps
                // it, each time it is drawn.
                cx.notify();
            }
            Action::OpenRepo(id) => self.open_repo(id, cx),
            Action::ForgetRepo(id) => self.forget_repo(id.clone(), cx),
            Action::Sync => self.sync(cx),
            Action::OpenLogs => self.open_logs(cx),
            // The rest belong to the Settings page.
            other => {
                self.run_settings_action(other, cx);
            }
        }
    }

    /// Show the folder of the log files in Explorer, so that a user can
    /// attach `app.log` and `backend.log` to a bug report.
    fn open_logs(&self, cx: &mut Context<Self>) {
        let dir = cw_core::logs::dir();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            log::warn!("could not create the logs folder {}: {e}", dir.display());
        }
        log::info!("opening the logs folder {}", dir.display());
        cx.open_with_system(&dir);
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> Div {
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
        let item = |note: &NoteInfo, level: usize, cx: &mut Context<Self>| {
            let path = note.path.clone();
            let label = match note.title.as_str() {
                "_index" => "Overview".to_string(),
                WIKI_MAP => "Wiki map".to_string(),
                title => title.to_string(),
            };
            let icon = match note.title.as_str() {
                "_index" => icons::OVERVIEW,
                WIKI_MAP => icons::MAP,
                _ => icons::NOTE,
            };
            ListItemButton::new(SharedString::from(format!("note:{}", note.path)), label)
                .icon(icon)
                .tint(icons::tint(icon, cx))
                .level(level)
                .selected(note.path == current)
                .on_click(cx.listener(move |this, _, _, cx| this.open_note(path.clone(), true, cx)))
        };

        let mut list = v_flex().pb_2();
        for (zone, folders) in &zones {
            let label = if zone.is_empty() {
                "Vault".to_string()
            } else {
                zone_label(zone)
            };
            list = list.child(list::subheader(label, cx));
            for (folder, notes) in folders {
                let mut notes = notes.clone();
                // Map notes first, then the rest by title.
                notes.sort_by_key(|n| (!n.title.starts_with('_'), n.title.to_lowercase()));
                if folder.is_empty() {
                    for note in notes {
                        list = list.child(item(note, 0, cx));
                    }
                } else {
                    let open = notes.iter().any(|n| n.path == current);
                    let rows: Vec<ListItemButton> = notes.iter().map(|n| item(n, 1, cx)).collect();
                    list = list.child(
                        ListFolder::new(
                            format!("folder:{zone}/{folder}"),
                            folder.clone(),
                            icons::FOLDER,
                            icons::EXPAND,
                        )
                        .tint(icons::tint(icons::FOLDER, cx))
                        .open_at_first(open)
                        .rows(rows),
                    );
                }
            }
        }

        let theme = cx.theme();
        v_flex()
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .flex_shrink_0()
            .border_r_1()
            .border_color(glass::tinted_border(cx))
            .bg(self.glass.chrome(glass::tinted(theme.sidebar, cx)))
            .text_color(theme.sidebar_foreground)
            .child(
                v_flex()
                    .px_4()
                    .py_3()
                    .child(div().font_bold().child("CodeWiki"))
                    .child(
                        div().text_xs().text_color(theme.muted_foreground).child(
                            self.repo
                                .as_ref()
                                .map(|r| r.name.clone())
                                .unwrap_or_default(),
                        ),
                    ),
            )
            .child(
                div()
                    .id("vault")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(list),
            )
            // The way into the settings stays in view under the notes,
            // however long their list is.
            .child(glass::hairline(cx))
            .child(self.settings_entry(cx))
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
            .bg(self.glass.chrome(glass::tinted(cx.theme().background, cx)))
            .child(
                IconButton::new("toggle-sidebar", icons::SIDEBAR)
                    .tooltip(if self.sidebar_open {
                        "Hide sidebar (Ctrl+B)"
                    } else {
                        "Show sidebar (Ctrl+B)"
                    })
                    .small()
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_pane(Pane::Sidebar, cx))),
            )
            .child(
                IconButton::new("home", icons::HOME)
                    .tint(icons::tint(icons::HOME, cx))
                    .tooltip("Home")
                    .small()
                    .disabled(self.backend.is_none())
                    .on_click(cx.listener(|this, _, _, cx| this.open_home(cx))),
            )
            .child(
                IconButton::new("back", icons::BACK)
                    .tooltip("Back (Alt+Left)")
                    .small()
                    .disabled(self.history.is_empty())
                    .on_click(cx.listener(|this, _, _, cx| this.go_back(cx))),
            )
            .child(
                div()
                    .flex_1()
                    .px_1()
                    .child(TextField::new("search", &self.search).icon(icons::SEARCH)),
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
                        .contained()
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
            .when_some(
                self.update.available().map(|r| r.version.to_string()),
                |this, version| {
                    this.child(
                        Button::new("update")
                            .icon(icons::UPDATE)
                            .label(format!("Update to {version}"))
                            .tooltip("Download it, install it and restart CodeWiki")
                            .small()
                            .contained()
                            .disabled(self.busy.is_some())
                            .on_click(cx.listener(|this, _, _, cx| this.install_update(cx))),
                    )
                },
            )
            .child(
                Button::new("commands")
                    .icon(icons::COMMANDS)
                    .label("Ctrl+K")
                    .tooltip("Commands")
                    .small()
                    .text()
                    .on_click(cx.listener(|this, _, window, cx| this.toggle_palette(window, cx))),
            )
            .child(
                IconButton::new("graph", icons::GRAPH)
                    .tint(icons::tint(icons::GRAPH, cx))
                    .tooltip("Graph")
                    .small()
                    .on_click(cx.listener(|this, _, _, cx| this.open_graph(cx))),
            )
            .child(
                IconButton::new("hotspots", icons::HOTSPOTS)
                    .tint(icons::tint(icons::HOTSPOTS, cx))
                    .tooltip("Hotspots")
                    .small()
                    .disabled(self.repo.is_none())
                    .on_click(cx.listener(|this, _, _, cx| this.open_hotspots(cx))),
            )
            .child({
                let view = cx.entity_id();
                self.theme_anchor.track(
                    IconButton::new("theme", icons::THEME)
                        .tint(icons::tint(icons::THEME, cx))
                        .tooltip(format!("Theme: {}", cx.theme().theme_name()))
                        .small()
                        .on_click(
                            cx.listener(|this, _, window, cx| this.open_theme_menu(window, cx)),
                        ),
                    // An open menu follows its button when the window is resized.
                    move |_, cx| doc_view::notify_after_draw(view, cx),
                )
            })
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
                    .when(pending > 0, |b| b.contained())
                    .disabled(pending == 0)
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some((id, _)) = this.pending_proposals.first().cloned() {
                            this.open_proposal(id, cx);
                        }
                    })),
            )
            .child(
                IconButton::new("toggle-details", icons::DETAILS)
                    .tooltip(if self.details_open {
                        "Hide details (Ctrl+Alt+B)"
                    } else {
                        "Show details (Ctrl+Alt+B)"
                    })
                    .small()
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_pane(Pane::Details, cx))),
            )
    }

    /// The row of tabs under the toolbar.
    fn tab_bar(&self, cx: &mut Context<Self>) -> Div {
        let closable = self.tabs.len() > 1;
        let tabs = (0..self.tabs.len()).map(|index| {
            let title = page_title(if index == self.active_tab {
                &self.page
            } else {
                &self.tabs[index].page
            });
            TabItem::new(title).when(closable, |tab| {
                tab.suffix(
                    IconButton::new(("close-tab", index), icons::CLOSE)
                        .tooltip("Close tab (Ctrl+W)")
                        .small()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.close_tab(index, cx);
                        })),
                )
            })
        });
        let tabs: Vec<TabItem> = tabs.collect();
        let view = cx.entity().downgrade();
        h_flex()
            .w_full()
            .px_3()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(glass::tinted_border(cx))
            .bg(self.glass.chrome(glass::tinted(cx.theme().background, cx)))
            .child(
                div().flex_1().min_w_0().overflow_hidden().child(
                    Tabs::new("tabs")
                        .tabs(tabs)
                        .selected(self.active_tab)
                        .on_select(move |index, _, cx| {
                            _ = view.update(cx, |this, cx| this.activate_tab(index, cx));
                        }),
                ),
            )
            .child(
                IconButton::new("new-tab", icons::NEW_TAB)
                    .tint(icons::tint(icons::NEW_TAB, cx))
                    .tooltip("New tab (Ctrl+T)")
                    .small()
                    .on_click(cx.listener(|this, _, _, cx| this.new_tab(cx))),
            )
    }

    /// The command palette, drawn over everything else. `shown` runs from 0
    /// to 1 as it opens and back as it closes; while it closes, the palette
    /// is still drawn but no longer takes input.
    fn palette_view(&self, shown: f32, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let open = self.palette_open;
        let entries = self.palette_matches(cx);
        let selected = self.palette_selected.min(entries.len().saturating_sub(1));
        let (muted, backdrop) = {
            let theme = cx.theme();
            (theme.muted_foreground, theme.background.opacity(0.6))
        };
        let mut list = v_flex();
        for (index, entry) in entries.into_iter().enumerate() {
            let command = entry.command;
            list = list.child(
                ListItemButton::new(("palette-entry", index), entry.label)
                    .icon(icons::for_command(&command))
                    .selected(index == selected)
                    .trailing(surfaces::text(Variant::Caption, entry.hint).text_color(muted))
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
                .opacity(shown.clamp(0.0, 1.0))
                .bg(backdrop)
                .when(open, |scrim| {
                    scrim.occlude().on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, window, cx| {
                            if this.palette_open {
                                this.toggle_palette(window, cx);
                            }
                        }),
                    )
                })
                .child(
                    h_flex().w_full().justify_center().pt(px(90.)).child(
                        surfaces::paper(24, cx)
                            .id("palette")
                            .w(px(560.))
                            .overflow_hidden()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(glass::hairline(cx))
                            .child(
                                div().p_2().child(
                                    TextField::new("palette-input", &self.palette_input)
                                        .icon(icons::SEARCH),
                                ),
                            )
                            .child(list.pb_2())
                            // Drops into place as it fades in.
                            .enter("palette-in", Enter::DIALOG),
                    ),
                ),
        )
        .with_priority(1)
    }

    /// The theme menu, hung under the toolbar's theme button. `shown` runs
    /// from 0 to 1 as for the palette. `None` until the button has been
    /// drawn once, which tells where it is.
    ///
    /// A page that fills the window has no toolbar; the palette can still
    /// ask for the menu there, and it then hangs from the window's top right
    /// corner, under the title bar.
    fn theme_menu_view(&self, shown: f32, window: &Window, cx: &mut Context<Self>) -> Option<Menu> {
        let anchor = if self.fills_window() {
            let right = window.viewport_size().width - px(MENU_CORNER_GAP);
            Bounds::new(
                point(right, TITLE_BAR_HEIGHT + px(MENU_CORNER_GAP)),
                size(px(0.), px(0.)),
            )
        } else {
            self.theme_anchor.get()?
        };
        let active = cx.theme().theme_name().clone();
        let entries = themes::entries(cx);
        let mut menu = Menu::new("theme-menu", anchor, &self.theme_menu_focus)
            .width(THEME_MENU_WIDTH)
            .highlighted(self.theme_menu_highlight)
            .open(self.theme_menu_open)
            .shown(shown);
        for (dark, label) in [(true, "Dark"), (false, "Light")] {
            let items: Vec<MenuItem> = entries
                .iter()
                .filter(|entry| entry.dark == dark)
                .map(|entry| theme_menu_item(entry, entry.name == active, cx))
                .collect();
            if !items.is_empty() {
                menu = menu.section(MenuSection::new(label).items(items));
            }
        }
        let (select, highlight, close) = (
            cx.entity().downgrade(),
            cx.entity().downgrade(),
            cx.entity().downgrade(),
        );
        Some(
            menu.on_select(move |index, _, cx| {
                _ = select.update(cx, |this, cx| {
                    this.theme_menu_highlight = index;
                    let name = themes::entries(cx).get(index).map(|e| e.name.clone());
                    if let Some(name) = name {
                        this.select_theme(&name, cx);
                    }
                });
            })
            .on_highlight(move |index, _, cx| {
                _ = highlight.update(cx, |this, cx| {
                    this.theme_menu_highlight = index;
                    cx.notify();
                });
            })
            .on_close(move |window, cx| {
                _ = close.update(cx, |this, cx| this.close_theme_menu(true, window, cx));
            }),
        )
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
        let preview = markdown::prepare_note_with(&value, |_| markdown::Diagram::Source);
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
                    .small()
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
    fn page_view(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        match &self.editing {
            Some(editing) => v_flex()
                .flex_1()
                .min_w_0()
                .h_full()
                .when_some(self.error.clone(), |this, error| {
                    this.child(div().p_2().child(feedback::alert(Tone::Error, error, cx)))
                })
                .child(self.edit_view(editing, cx))
                .into_any_element(),
            None => self.viewer(window, cx).into_any_element(),
        }
    }

    fn viewer(&self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let source = self.rendered.clone();
        let diagrams = self.diagrams.clone();
        let view = cx.entity().downgrade();
        // Notes are prose; everything else is mostly tables and needs room.
        let measure = match self.page {
            Page::Note { .. } => reading::NOTE_MEASURE,
            _ => reading::WIDE_MEASURE,
        };
        let showing_figure = self.show_figure && self.figure.is_some();
        let figure = self
            .figure
            .as_ref()
            .filter(|_| self.show_figure)
            .map(|figure| illustration::render(figure, "figure", window, cx));
        let page = (!showing_figure).then(|| {
            let content = match &self.page {
                Page::Note { .. } => div()
                    .size_full()
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
                    .into_any_element(),
                Page::Generated { doc, .. } | Page::Proposal { doc, .. } | Page::Notice(doc) => {
                    return self.doc_element(doc, cx);
                }
                Page::Message(message) => {
                    return self.doc_element(&Rc::new(live::message_page(message)), cx);
                }
                Page::Home => return self.doc_element(&Rc::new(self.home_doc()), cx),
                // Each has a view of its own (`graph_view`, `settings_frame`).
                Page::Graph | Page::Settings(_) => div().into_any_element(),
            };
            // A new note rises into place. The id changes with the page, so
            // this runs once per page and not on a redraw.
            div()
                .flex_1()
                .min_h_0()
                .child(content)
                .enter(("page-in", self.page_serial), Enter::PAGE)
                .into_any_element()
        });
        v_flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .when_some(self.error.clone(), |this, error| {
                this.child(div().p_2().child(feedback::alert(Tone::Error, error, cx)))
            })
            .when_some(self.proposal_actions(cx), |this, actions| {
                this.child(actions)
            })
            .when_some(figure, |this, figure| {
                this.child(
                    div()
                        .id("figure")
                        .flex_1()
                        .min_h_0()
                        .p_6()
                        .overflow_y_scrollbar()
                        .child(figure),
                )
            })
            .when_some(page, |this, page| this.child(page))
    }

    /// Home, from the registered repositories and the open one.
    fn home_doc(&self) -> Doc {
        fn listed(repos: &[Repo]) -> Vec<live::Project<'_>> {
            repos
                .iter()
                .map(|repo| live::Project {
                    id: &repo.id,
                    name: &repo.name,
                    path: &repo.path,
                })
                .collect()
        }
        let projects = listed(&self.repos);
        let missing = listed(&self.missing);
        let update = self.update.available().map(|r| r.version.to_string());
        let no_wiki = self
            .repo
            .as_ref()
            .filter(|_| self.notes_loaded && self.notes.is_empty())
            .map(|repo| repo.name.as_str());
        live::home_page(&live::HomeInput {
            projects: &projects,
            recent: &self.recent,
            current: self.repo.as_ref().map(|repo| repo.id.as_str()),
            no_wiki,
            missing: &missing,
            update: update.as_deref(),
        })
    }

    /// A generated page. The shell draws it, so that this view and the page
    /// are not redrawn for each other's sake; here is only the empty place
    /// it goes in, which reports where it is.
    fn doc_element(&self, doc: &Rc<Doc>, cx: &mut Context<Self>) -> AnyElement {
        let (serial, width) = (self.page_serial, self.pane_width);
        self.doc_page
            .update(cx, |page, cx| page.show(doc, serial, width, cx));
        self.page_placed.set(true);
        let slot = self.page_slot.clone();
        div()
            .flex_1()
            .min_h_0()
            .child(
                canvas(
                    move |bounds, _, cx| slot.set(Some(bounds), cx),
                    |_, _, _, _| {},
                )
                .size_full(),
            )
            .into_any_element()
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
                (viewport.width / px(1.) - self.side_width()).max(200.0),
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
        let mut hud = format!(
            "{} nodes · {} edges · {} in view · paint {:.1} ms",
            data.nodes.len(),
            data.edges.len(),
            visible.len(),
            self.paint_ms.get(),
        );
        // The time between two paints. It is a frame rate only while the
        // stress test redraws on every frame; otherwise it is how long the
        // view sat idle, which read as a slow first frame.
        if stress {
            hud.push_str(&format!(" · frame {:.1} ms", self.frame_ms.get()));
        }

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
            div().absolute().left(px(10.)).top(px(8.)).child(
                feedback::chip(hud, Tone::Neutral, true, cx)
                    .bg(theme.background.opacity(0.8))
                    .text_color(theme.muted_foreground),
            ),
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
                        .contained()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.decide_proposal(approve_id.clone(), true, cx)
                        })),
                )
                .child(
                    Button::new("reject")
                        .label("Reject")
                        .small()
                        .error()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.decide_proposal(reject_id.clone(), false, cx)
                        })),
                ),
        )
    }

    fn details(&self, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let accent = cx.theme().link;
        let heading = move |text: &str| {
            surfaces::text(Variant::Overline, text.to_string()).text_color(accent)
        };
        let mut pane = v_flex()
            .w(px(DETAILS_WIDTH))
            .h_full()
            .flex_shrink_0()
            .gap_3()
            .p_3()
            .border_l_1()
            .border_color(glass::tinted_border(cx))
            .bg(self.glass.chrome(glass::tinted(cx.theme().background, cx)))
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
                // Each section gets a dot in the next chart color, so the
                // outline carries the theme's palette.
                let markers = {
                    let theme = cx.theme();
                    [
                        theme.chart_1,
                        theme.chart_2,
                        theme.chart_3,
                        theme.chart_4,
                        theme.chart_5,
                    ]
                };
                for (index, section) in markdown::outline(text).into_iter().enumerate() {
                    pane = pane.child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .size(px(OUTLINE_DOT))
                                    .flex_shrink_0()
                                    .rounded_full()
                                    .bg(markers[index % markers.len()]),
                            )
                            .child(div().flex_1().min_w_0().text_sm().child(section)),
                    );
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
                            .text()
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
            Page::Message(_) | Page::Notice(_) | Page::Home | Page::Settings(_) => {}
        }
        // Home lists the repositories itself.
        if self.repos.len() > 1 && !matches!(self.page, Page::Home) {
            pane = pane.child(heading("REPOSITORIES"));
            for repo in &self.repos {
                let target = repo.id.clone();
                pane = pane.child(
                    Button::new(SharedString::from(format!("repo-{}", repo.id)))
                        .label(repo.name.clone())
                        .small()
                        .text()
                        .on_click(cx.listener(move |this, _, _, cx| this.open_repo(&target, cx))),
                );
            }
        }
        pane
    }
}

/// The registered repositories, split into those whose folders are there and
/// those whose folders are gone. A backend from before the list said so
/// counts every folder as there.
fn parse_repos(value: &Value) -> (Vec<Repo>, Vec<Repo>) {
    let (present, missing): (Vec<_>, Vec<_>) = value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            let repo = Repo {
                id: r["id"].as_str()?.to_string(),
                name: r["name"].as_str()?.to_string(),
                path: r["path"].as_str().unwrap_or("").to_string(),
            };
            Some((repo, r["present"].as_bool().unwrap_or(true)))
        })
        .partition(|(_, present)| *present);
    let repos = |list: Vec<(Repo, bool)>| list.into_iter().map(|(repo, _)| repo).collect();
    (repos(present), repos(missing))
}

/// The registered repositories whose folders are there, and those whose
/// folders are not found. Nothing is forgotten here: a folder can be away
/// for a while. Blocks; run it off the UI thread.
fn registered_repos(backend: &Backend) -> Result<(Vec<Repo>, Vec<Repo>), String> {
    Ok(parse_repos(&backend.get("/v1/repos")?))
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

/// A theme as a row of a menu: its icon in a color of its own, its name, a
/// sample of its four colors, and a mark on the one in use. The toolbar's
/// theme menu and the theme select of the settings list themes with it, so
/// that a theme looks the same in both.
fn theme_menu_item(entry: &themes::Entry, in_use: bool, cx: &App) -> MenuItem {
    let mark = cx.theme().primary;
    MenuItem::new(entry.name.clone())
        .icon(icons::for_theme(&entry.name))
        .tint(Some(themes::glyph_tint(entry.swatches[1], cx)))
        .selected(in_use)
        .trailing(
            h_flex()
                .flex_shrink_0()
                .gap_2()
                .items_center()
                .child(feedback::dots(entry.swatches, cx))
                .child(div().size(px(THEME_MARK)).when(in_use, |slot| {
                    slot.child(
                        Icon::new(icons::CHECK)
                            .with_size(px(THEME_MARK))
                            .text_color(mark),
                    )
                })),
        )
}

/// Short name for a tab.
fn page_title(page: &Page) -> String {
    match page {
        Page::Message(_) | Page::Notice(_) => "CodeWiki".into(),
        Page::Home => "Home".into(),
        Page::Note { path, .. } => {
            let name = path.rsplit('/').next().unwrap_or(path);
            match name.trim_end_matches(".md") {
                WIKI_MAP => "Wiki map".into(),
                "_index" => "Overview".into(),
                title => title.to_string(),
            }
        }
        Page::Generated { title, .. } => title.clone(),
        Page::Proposal { .. } => "Review".into(),
        Page::Graph => "Graph".into(),
        Page::Settings(_) => "Settings".into(),
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
        self.draw_queued_diagrams(cx);
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
        // The inputs of the settings take the values that arrived since the
        // last frame; that needs the window.
        self.sync_settings_fields(window, cx);
        // Read once so unused-field lints stay honest about what the UI shows.
        let _ = self.notes.iter().filter(|n| n.state == "stale").count();
        self.page_placed.set(false);
        self.glass = glass::Levels::for_window(window, cx);
        // Components that draw their own surfaces read the levels from here.
        if cx.try_global::<glass::Levels>() != Some(&self.glass) {
            cx.set_global(self.glass);
            // The page is drawn cached and reads the levels; draw it again.
            doc_view::notify_after_draw(self.doc_page.entity_id(), cx);
        }
        // A pane the user hides or shows slides out or in; the page takes
        // the width that is left, frame by frame.
        self.sidebar_shown = mui::motion::drawer("sidebar-shown", self.sidebar_open, window, cx);
        self.details_shown = mui::motion::drawer("details-shown", self.details_open, window, cx);
        let chrome = if self.fills_window() {
            0.0
        } else {
            self.side_width()
        };
        self.pane_width = (window.viewport_size().width / px(1.) - chrome).max(0.0);
        // The palette stays drawn for a moment after it closes, to fade out.
        let palette = Presence::new("palette-presence", self.palette_open)
            .transition(Transition::new(mui::motion::scaled(mui::motion::EXIT)))
            .sample(window, cx);
        let theme_menu = Presence::new("theme-menu-presence", self.theme_menu_open)
            .transition(Transition::new(mui::motion::scaled(mui::motion::EXIT)))
            .sample(window, cx);
        let root = v_flex()
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
            .on_action(
                cx.listener(|this, _: &ToggleSidebar, _, cx| this.toggle_pane(Pane::Sidebar, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ToggleDetails, _, cx| this.toggle_pane(Pane::Details, cx)),
            )
            .on_action(cx.listener(|this, _: &OpenSettings, _, cx| this.open_settings(None, cx)))
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
            .when(palette.should_render(), |root| {
                root.child(self.palette_view(palette.progress, cx))
            })
            .when(theme_menu.should_render(), |root| {
                root.children(self.theme_menu_view(theme_menu.progress, window, cx))
            })
            // The list of a select of the Settings page, while one is open.
            .children(self.settings_menu_view(cx))
            // The place of the title bar, which the shell draws.
            .child(div().h(TITLE_BAR_HEIGHT).w_full().flex_none())
            .child(glass::hairline(cx))
            .child(self.body(window, cx));
        // With no generated page in this frame, the shell has none to draw.
        if !self.page_placed.get() {
            self.page_slot.set(None, cx);
        }
        root
    }
}

/// How the app's window opens: at GPUI's default size, cut to what the screen
/// has free of the taskbar, in the middle of that. GPUI itself measures
/// against the whole screen, so on a screen less tall than its default the
/// bottom of the window lay under the taskbar.
fn window_options(cx: &App) -> WindowOptions {
    let mut options = TitleBar::window_options();
    options.window_min_size = Some(size(px(MIN_WINDOW.0), px(MIN_WINDOW.1)));
    // Started for the tray (`--tray`, as Windows starts the app at sign-in):
    // the window is made, so that the app is whole, but not shown. A click
    // on the tray icon shows it.
    let in_tray = startup::launch().tray;
    options.show = !in_tray;
    options.focus = !in_tray;
    if let Some(display) = cx.primary_display() {
        options.window_bounds = Some(WindowBounds::Windowed(first_bounds(
            display.visible_bounds(),
        )));
    }
    options
}

/// The first window's place on a screen whose free area is `free`: GPUI's
/// default size, or what the free area holds with a margin around it.
fn first_bounds(free: Bounds<Pixels>) -> Bounds<Pixels> {
    let room = size(
        (free.size.width - px(2.0 * SCREEN_MARGIN)).max(px(MIN_WINDOW.0)),
        (free.size.height - px(2.0 * SCREEN_MARGIN)).max(px(MIN_WINDOW.1)),
    );
    let fitted = DEFAULT_WINDOW_SIZE.min(&room);
    let centre = free.center();
    Bounds::new(
        point(
            centre.x - fitted.width / 2.0,
            centre.y - fitted.height / 2.0,
        ),
        fitted,
    )
}

/// Whether ending the app also stops the backend. `remote` says that the
/// backend is the one `CODEWIKI_URL` names; `installing` that an update is
/// being installed or waits for a restart.
fn stops_backend_on_quit(remote: bool, installing: bool) -> bool {
    !remote && !installing
}

/// The window's title bar. The shell draws it, not the app's view: the
/// buttons of a title bar that is part of a cached view stop answering (see
/// `backdrop`).
fn title_bar(app: &Entity<AppView>, window: &mut Window, cx: &mut App) -> AnyElement {
    let title = match &app.read(cx).repo {
        Some(repo) => format!("CodeWiki — {}", repo.name),
        None => "CodeWiki".to_string(),
    };
    let levels = glass::Levels::for_window(window, cx);
    TitleBar::new()
        .bg(levels.chrome(glass::tinted(cx.theme().title_bar, cx)))
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(img(icons::LOGO).size(px(16.)).flex_none())
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(title),
                ),
        )
        .into_any_element()
}

impl AppView {
    /// Whether the page takes the whole window, with no sidebar, toolbar,
    /// tabs or details: Home, the settings, and what shows before a
    /// repository has loaded, so the app does not flash its chrome on the
    /// way to Home.
    fn fills_window(&self) -> bool {
        match self.page {
            Page::Home | Page::Settings(_) => true,
            Page::Message(_) | Page::Notice(_) => !self.notes_loaded,
            _ => false,
        }
    }

    /// Everything below the title bar: sidebar, toolbar, tabs, page, details.
    fn body(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        if self.fills_window() {
            let column = v_flex()
                .flex_1()
                .min_h_0()
                .w_full()
                .bg(self
                    .glass
                    .pane(glass::tinted_pane(cx.theme().background, cx)))
                .when(self.busy.is_some(), |column| {
                    column.child(feedback::linear_progress("busy", cx))
                });
            if let Page::Settings(category) = self.page {
                return column.child(self.settings_frame(category, window, cx));
            }
            // Home has no sidebar to hold the way into the settings, so it
            // is at Home's own foot. The pages shown before a repository
            // has loaded have none: there is nothing to set up yet.
            let home = matches!(self.page, Page::Home);
            return column
                .child(
                    h_flex()
                        .flex_1()
                        .min_h_0()
                        .w_full()
                        .child(self.page_view(window, cx)),
                )
                .when(home, |column| {
                    column
                        .child(glass::hairline(cx))
                        .child(h_flex().child(self.settings_entry(cx)))
                });
        }
        h_flex()
            .flex_1()
            .min_h_0()
            .w_full()
            // A pane keeps its width inside a frame that narrows, so its
            // text does not wrap anew while it slides. The sidebar is held
            // to the frame's right edge and so leaves to the left. Hidden
            // and at rest, a pane is not built at all.
            .when(self.sidebar_shown > 0.0, |row| {
                row.child(
                    div()
                        .flex()
                        .w(px(SIDEBAR_WIDTH * self.sidebar_shown))
                        .h_full()
                        .flex_none()
                        .justify_end()
                        .overflow_hidden()
                        .child(self.sidebar(cx)),
                )
            })
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .child(self.toolbar(cx))
                    .when(self.busy.is_some(), |column| {
                        column.child(feedback::linear_progress("busy", cx))
                    })
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
                                    .bg(self
                                        .glass
                                        .pane(glass::tinted_pane(cx.theme().background, cx)))
                                    .child(if matches!(self.page, Page::Graph) {
                                        self.graph_view(window, cx)
                                    } else {
                                        self.page_view(window, cx)
                                    }),
                            )
                            .when(self.details_shown > 0.0, |row| {
                                row.child(
                                    div()
                                        .flex()
                                        .w(px(DETAILS_WIDTH * self.details_shown))
                                        .h_full()
                                        .flex_none()
                                        .overflow_hidden()
                                        .child(self.details(cx)),
                                )
                            }),
                    ),
            )
    }
}

fn main() {
    // A release build has no console, so nothing the app or GPUI reports
    // would be kept anywhere: every record at Info and above goes to
    // `logs\app.log`, GPUI's own included (it names the graphics adapter it
    // picked), and so does every panic.
    // The level is the one chosen in the settings, unless `CODEWIKI_LOG`
    // names another for this run.
    let level = prefs::read(settings_view::LOG_LEVEL_PREF)
        .and_then(|name| cw_core::logs::level_named(&name))
        .unwrap_or(log::LevelFilter::Info);
    cw_core::logs::init("app", level);
    // Before anything of GPUI: a launch that hands over to the running app
    // ends here, with no window of its own.
    let (to_app, inbox) = async_channel::unbounded();
    let deliver = {
        let to_app = to_app.clone();
        move |request| to_app.try_send(Event::Handover(request)).is_ok()
    };
    match instance::acquire(startup::launch(), deliver) {
        instance::Acquired::HandedOver => return,
        instance::Acquired::Primary(held) => instance::hold(held),
        instance::Acquired::Separate => {}
    }
    let app = gpui_kit::application().with_assets(icons::AppAssets);
    app.run(move |cx| {
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
            KeyBinding::new("ctrl-b", ToggleSidebar, None),
            KeyBinding::new("ctrl-alt-b", ToggleDetails, None),
            KeyBinding::new("ctrl-,", OpenSettings, None),
        ]);
        let (window, _) = gpui_kit::open_window(window_options(cx), cx, |window, cx| {
            window.set_window_title("CodeWiki");
            // Windows 11 backdrop; older systems ignore it.
            window.set_background_appearance(WindowBackgroundAppearance::MicaBackdrop);
            let slot = Rc::new(doc_view::PageSlot::default());
            let app = cx.new(|cx| {
                let view = AppView::new(slot.clone(), (to_app.clone(), inbox), window, cx);
                // Shortcuts need a focused element to dispatch from.
                window.focus(&view.focus, cx);
                view
            });
            let page = app.read(cx).doc_page.clone();
            // Closing the window hides it; the app stays in the tray, where
            // Quit ends it. The window is never really closed, so nothing in
            // it is lost: not the page, not unsaved edits. Without a tray
            // icon closing ends the app as it used to, and then unsaved edits
            // keep the window open, with a word, as leaving the page does.
            window.on_window_should_close(cx, {
                let app = app.clone();
                let to_app = to_app.clone();
                move |_, cx| {
                    if app.read(cx).tray.is_some() {
                        // Not here: GPUI is in the middle of this message.
                        return to_app.try_send(Event::Hide).is_err();
                    }
                    app.update(cx, |app, cx| {
                        let leave = app.can_leave();
                        if !leave {
                            cx.notify();
                        }
                        leave
                    })
                }
            });
            // A click on the one notification the app shows, that it is
            // still running, opens the window.
            cx.on_system_notification_response({
                let to_app = to_app.clone();
                move |_, _| _ = to_app.try_send(Event::Show)
            });
            // The backend ends with the app. It finishes what it is doing
            // first (an update of the wiki, say) and then exits by itself;
            // the app does not wait for that.
            cx.on_app_quit({
                let app = app.clone();
                move |cx| {
                    // A launch from now on is the app itself.
                    instance::release();
                    let backend = app.read(cx).backend_to_stop();
                    cx.background_executor().spawn(async move {
                        let Some(backend) = backend else {
                            return;
                        };
                        match backend.shutdown() {
                            Ok(()) => log::info!("asked the backend to stop"),
                            Err(e) => log::warn!("could not ask the backend to stop: {e}"),
                        }
                    })
                }
            })
            .detach();
            // The shell draws the drifting backdrop, the app over it, and
            // the generated page in the place the app leaves for it.
            let bar = {
                let app = app.clone();
                Box::new(move |window: &mut Window, cx: &mut App| title_bar(&app, window, cx))
            };
            cx.new(|cx| backdrop::Shell::new(app, bar, page, slot, window, cx))
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

#[cfg(test)]
mod tests {
    // Not `super::*`: that brings in GPUI's `test` attribute in place of the
    // standard one.
    use serde_json::json;

    use gpui_kit::{Bounds, DEFAULT_WINDOW_SIZE, point, px, size};

    use super::{Repo, first_bounds, parse_repos, stops_backend_on_quit};

    #[test]
    fn the_first_window_stays_inside_the_free_part_of_the_screen() {
        // A 1600x900 screen with a 48 px taskbar at the bottom: the window
        // is as wide as GPUI's default, and less tall than what is free by a
        // margin above and below.
        let free = Bounds::new(point(px(0.), px(0.)), size(px(1600.), px(852.)));
        let window = first_bounds(free);
        assert_eq!(window.size, size(DEFAULT_WINDOW_SIZE.width, px(820.)));
        assert_eq!(window.origin, point(px(32.), px(16.)));

        // A taskbar on the left moves the free area, and the window with it.
        let free = Bounds::new(point(px(60.), px(0.)), size(px(3780.), px(2160.)));
        let window = first_bounds(free);
        assert_eq!(window.size, DEFAULT_WINDOW_SIZE);
        assert!(window.origin.x >= px(60.));
        assert_eq!(
            window.origin.x + window.size.width / 2.0,
            px(60.) + px(3780.) / 2.0
        );
    }

    #[test]
    fn the_repository_list_splits_present_from_missing() {
        let listed = json!([
            {"id": "a1", "name": "alpha", "path": "C:/alpha", "present": true},
            {"id": "z9", "name": "zzzbot", "path": "C:/zzzbot", "present": false},
            // A backend from before `present` was listed.
            {"id": "b2", "name": "beta", "path": "C:/beta"},
            {"name": "no id"}
        ]);
        let (present, missing) = parse_repos(&listed);
        let names = |repos: &[Repo]| repos.iter().map(|r| r.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&present), ["alpha", "beta"]);
        assert_eq!(names(&missing), ["zzzbot"]);
        assert_eq!(missing[0].id, "z9");
    }

    #[test]
    fn closing_stops_a_local_backend_only_and_not_during_an_update() {
        assert!(stops_backend_on_quit(false, false));
        // One named by CODEWIKI_URL is not this app's to stop.
        assert!(!stops_backend_on_quit(true, false));
        // An update has stopped it, and may have started the next one.
        assert!(!stops_backend_on_quit(false, true));
    }
}
