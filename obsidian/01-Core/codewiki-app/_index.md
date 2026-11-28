---
tags: [moc, core, codewiki-app]
---

# Core / codewiki-app — Map of Content

> Notes for `codewiki-app`: 58 concepts.

## Notes

- [[Action]] — what following a link or pressing a row does.
- [[Autostart Module]] — starting the app when the user signs in to Windows.
- [[Backdrop Module]] — the drifting backdrop: soft color under the glass.
- [[Block]] — A part of a page.
- [[Build Module]] — embeds the app's icon in the Windows program.
- [[Category]] — A group of settings, with a page of its own.
- [[codewiki-app]] — CodeWiki desktop app for Windows.
- [[Connection Module]] — when the app tries to reach its backend again, and how.
- [[Constellation Module]] — the constellation behind the glass: a made-up graph of notes and the links between them, which the backdrop (`backdrop.rs`) draws over its soft color.
- [[Contrast Module]] — WCAG contrast arithmetic for translucent surfaces.
- [[Diagram View Module]] — where an enlarged diagram sits in the diagram viewer.
- [[Diagrams Module]] — drawn Mermaid diagrams, kept so that a note opens without waiting.
- [[Doc]] — A generated page.
- [[Doc Module]] — what a generated page says, apart from how it is drawn.
- [[Doc View Module]] — draws a `Doc` with the `mui` components.
- [[Editor Module]] — text logic for the note editor: wikilink completion and line endings.
- [[FieldId]] — A setting the user types: each has one text input, which lives as long as the app does.
- [[Fonts Module]] — fonts bundled in the binary.
- [[Glass Module]] — glass: how see-through each surface is.
- [[Glyph]] — an icon of the app: the path its SVG is served under.
- [[Graph Module (codewiki-app)]] — geometry of the dependency graph view.
- [[Icons Module]] — icons, and the asset source that serves them.
- [[Identity Module]] — the name and the icon Windows shows above the app's notifications.
- [[Illustration Module]] — native renderer for the pictures of a note.
- [[Instance Module]] — one running app for each installed program.
- [[Live Module]] — the live pages: one symbol, one file, search results, hotspots, the theme picker, a proposal, the result of a sync, the notes that wait for a coding agent, and Home, the page the app starts on.
- [[Markdown Module]] — text preparation for the note viewer.
- [[Motion Module]] — motion: whether things move at all.
- [[MotionPref]] — what the user asked for.
- [[Mui Module]] — components drawn to Material UI's measurements.
- [[Outline Module]] — the outline in the details pane: the `##` headings of the note as its body draws them, which of their sections is being read, and the glide of the body to a heading the user clicks.
- [[Pages Module]] — the report pages: decisions, dead code, health, drift and the rest.
- [[Palette Module]] — the command palette's entries and matching.
- [[Panes Module]] — the two side panes: whether each is shown.
- [[Prefs Module]] — small per-user settings, one value per file under `%APPDATA%\CodeWiki` (`CodeWiki-Dev` for a development build, `cw_core::paths::APP_FOLDER`).
- [[Reading Module]] — typography of the reading pane.
- [[Ready Module]] — the lines the app writes to its log when a page has been drawn with its data.
- [[Review Module]] — the review queue as the app keeps it: the pending proposals, which of them are selected, which opens after a decision, and what the user is told.
- [[Review View Module]] — the review page as the app's view draws and runs it.
- [[Settings Layout Module]] — the measures of the Settings page that are a matter of taste.
- [[Settings Module]] — what the Settings page is made of, apart from how it is drawn: its categories, the fields a user types into, and the news shown beside one.
- [[Settings Pages Module]] — the pages of the Settings: one for each `Category`, built from what the app knows and from what the backend says about its own settings (`GET /v1/settings`).
- [[Settings View Module]] — the Settings page as the app's view draws and runs it.
- [[Span]] — A stretch of text within a line.
- [[Standard Module]] — standard levels for a theme the app did not ship.
- [[Startup Module]] — which repository the app opens when it starts.
- [[Status Module]] — what the app says about itself in the tray: what the backend is doing and whether an agent is using it.
- [[Suggest Module]] — what the search field suggests while it is typed in: the hits of its text, looked up at each pause in typing, in a list under the field.
- [[Suggestions]] — `Suggestions` struct defined in `codewiki_app::suggest` with 14 methods.
- [[Supervisor]] — watches the backend and decides when, and how, to try it again.
- [[Tab Ops Module]] — which tabs stay when some are closed: the logic of the tab menu, kept apart from the view so that it can be tested.
- [[Tab Prefs Module]] — the two tab settings: whether the app has tabs at all, and whether a note the user opens goes to a tab of its own.
- [[Themes Module]] — bundled color themes and theme switching.
- [[Toasts Module]] — short messages for the user: what to show and where it goes.
- [[Tray Module]] — the app's icon in the notification area, with its tooltip and its menu.
- [[Update Module]] — updating the app: when to look for a new release, and what the settings page says about it.
- [[Win32 Module]] — the Win32 calls GPUI has no way to make.
- [[Zoom Module]] — zoom: the whole app larger or smaller, as a browser zooms a page.

## Folders

- [[01-Core/codewiki-app/mui/_index|mui]] — 21 notes

## See also

- [[_HOME]]
- [[01-Core/_index|Core]]
