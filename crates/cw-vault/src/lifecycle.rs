//! Note lifecycle: what may be done to a note given its state on disk.

use serde::{Deserialize, Serialize};

use crate::manifest::{NoteEntry, Origin, body_hash};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteState {
    /// In the manifest but the file is gone.
    Missing,
    /// As CodeWiki wrote it, sources unchanged.
    Generated,
    /// As CodeWiki wrote it, but a cited source changed.
    Stale,
    /// A human wrote or changed it; sources unchanged since last review.
    HumanEdited,
    /// A human wrote or changed it and a cited source changed.
    HumanEditedStale,
}

/// What the updater should do with a note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Nothing to do.
    Keep,
    /// Write fresh content directly.
    Regenerate,
    /// Write fresh content as a proposal for human review.
    Propose,
}

/// Classify a note. `disk` is the current file text, if the file exists.
/// `sources_changed` is true when any cited source hash differs from the
/// manifest.
pub fn state(entry: &NoteEntry, disk: Option<&str>, sources_changed: bool) -> NoteState {
    let Some(text) = disk else {
        return NoteState::Missing;
    };
    let edited = entry.origin == Origin::Adopted || body_hash(text) != entry.body_hash;
    match (edited, sources_changed) {
        (false, false) => NoteState::Generated,
        (false, true) => NoteState::Stale,
        (true, false) => NoteState::HumanEdited,
        (true, true) => NoteState::HumanEditedStale,
    }
}

/// Decide the action. `new_input_hash` is the hash of the inputs a
/// regeneration would use now.
pub fn action(entry: &NoteEntry, state: NoteState, new_input_hash: &str) -> Action {
    match state {
        NoteState::Missing => Action::Regenerate,
        NoteState::Generated | NoteState::Stale => {
            if entry.input_hash == new_input_hash {
                Action::Keep
            } else {
                Action::Regenerate
            }
        }
        NoteState::HumanEdited => Action::Keep,
        NoteState::HumanEditedStale => {
            if entry.input_hash == new_input_hash
                || entry.rejected_inputs.iter().any(|r| r == new_input_hash)
            {
                Action::Keep
            } else {
                Action::Propose
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::note::NoteKind;

    fn entry(origin: Origin, text: &str) -> NoteEntry {
        NoteEntry {
            id: "n".into(),
            path: "01-Core/N.md".into(),
            kind: NoteKind::Concept,
            origin,
            sources: vec![],
            body_hash: body_hash(text),
            input_hash: "in-1".into(),
            rejected_inputs: vec!["in-rejected".into()],
            commit: None,
            model: None,
            illustration: None,
        }
    }

    #[test]
    fn classifies_states() {
        let e = entry(Origin::Generated, "# N\n");
        assert_eq!(state(&e, None, false), NoteState::Missing);
        assert_eq!(state(&e, Some("# N\r\n"), false), NoteState::Generated);
        assert_eq!(state(&e, Some("# N\n"), true), NoteState::Stale);
        assert_eq!(
            state(&e, Some("# N edited\n"), false),
            NoteState::HumanEdited
        );
        assert_eq!(
            state(&e, Some("# N edited\n"), true),
            NoteState::HumanEditedStale
        );
        // Adopted notes are human-owned even when untouched since adoption.
        let adopted = entry(Origin::Adopted, "# N\n");
        assert_eq!(
            state(&adopted, Some("# N\n"), true),
            NoteState::HumanEditedStale
        );
    }

    #[test]
    fn human_edited_notes_are_never_regenerated() {
        let e = entry(Origin::Generated, "# N\n");
        for s in [NoteState::HumanEdited, NoteState::HumanEditedStale] {
            for input in ["in-1", "in-2", "in-rejected"] {
                assert_ne!(action(&e, s, input), Action::Regenerate);
            }
        }
    }

    #[test]
    fn decides_actions() {
        let e = entry(Origin::Generated, "# N\n");
        assert_eq!(action(&e, NoteState::Missing, "in-1"), Action::Regenerate);
        assert_eq!(action(&e, NoteState::Stale, "in-1"), Action::Keep);
        assert_eq!(action(&e, NoteState::Stale, "in-2"), Action::Regenerate);
        assert_eq!(action(&e, NoteState::Generated, "in-2"), Action::Regenerate);
        assert_eq!(
            action(&e, NoteState::HumanEditedStale, "in-2"),
            Action::Propose
        );
        assert_eq!(
            action(&e, NoteState::HumanEditedStale, "in-rejected"),
            Action::Keep
        );
        assert_eq!(action(&e, NoteState::HumanEdited, "in-2"), Action::Keep);
    }
}
