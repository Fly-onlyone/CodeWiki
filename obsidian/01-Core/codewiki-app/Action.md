---
tags: [core, codewiki-app]
---

# Action

> What following a link or pressing a row does.

## What it does

`Action` names what should happen when the user follows a link, presses a row or clicks a button on a generated page. It is plain data: a page function builds a [[Doc]] that holds actions, and the drawing code hands the chosen one back to the app, which runs it.

This split keeps pages free of window code. A page says "open this note" or "turn autostart on" and does not know how. A test can read the actions of a page without a window.

## Source

- `crates/codewiki-app/src/doc.rs` — definition of `Action`

## How it works

The variants fall into a few groups.

- Navigation: `Note`, `Symbol` and `File` open a note, a symbol page or a source file page. `Settings` opens the settings at a [[Category]]. `OpenRepo`, `OpenReview` and `OpenFolder` move between repositories.
- Switches and choices of the app: `Motion`, `Zoom`, `Theme`, `Autostart`, `AutoUpdate`, `AutoExplain`, `OpenLast`, `Backdrop`, `ErrorReports` and `LogLevel`. The `bool` variants carry the new state the click asks for, not a toggle.
- Agent folders: `KeepAgentsOut`, `LetAgentsIn`, `PickAgentsFolder`, `ResetAgentsList`.
- Backend and secrets: `SetSetting`, `ReplaceSecret`, `KeepSecret`, `RemoveSecret` (each names a [[FieldId]]), `TestEmbedder`, `RefreshEmbedModels`, `ConnectRemote`, `UseLocalBackend`, `ForgetRemote`, `CancelEmbedJob`.
- Housekeeping: `Reconnect`, `Sync`, `CheckUpdates`, `InstallUpdate`, `OpenLogs`, `OpenDataFolder`, `ForgetRepo`.

An action sits in a [[Span]] link, a row, a button, a choice, a switch or an alert button. The doc view returns it to the app, and `run_action` and `run_settings_action` in [[codewiki-app]] match on it.

## Items

Declared as `pub enum Action`.

## Depends on

- [[MotionPref]] — uses `MotionPref`
- [[Category]] — uses `Category`
- [[FieldId]] — uses `FieldId`
- [[Count]] — uses `Count`

## Used by

- [[Doc View Module]] — from `LinkedLine`, `OnAction`, `choices_row`, `line`
- [[Doc Module]] — from `Choice`, `Press`, `Row`, `aside`
- [[codewiki-app]] — from `run_action`, `run_action_with`, `run_settings_action`
- [[Doc]] — from `actions`, `of_line`
- [[Live Module]] — from `HitTarget`
- [[Toasts Module]] — from `Notice`
- [[Block]] — from `Block`
- [[Span]] — from `Span`

## Gotchas

- `Action` derives `PartialEq`, so tests compare whole actions, as the page tests do with `Doc::actions`.
- A new variant needs handling where the app runs actions, in `run_action` or `run_settings_action`. Check both.
- `SetSetting` with no value means "put the setting back to its default", not "set it to empty".
- The `bool` variants carry the state the click asks for, so a switch row builds the opposite of what it shows.
- `ForgetRepo` takes a repository off the list only. Its folder and wiki stay.
- `KeepAgentsOut` covers the folder and everything below it.

## See also

- [[_index]]
- [[Doc Module]]
