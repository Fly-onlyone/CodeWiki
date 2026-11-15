//! Drawn Mermaid diagrams, kept so that a note opens without waiting.
//!
//! Drawing one diagram takes seconds in a debug build, and the first of a
//! session also reads every installed font. The view therefore never draws:
//! it asks here, shows a placeholder for what is missing, and has the missing
//! diagrams drawn off the UI thread. No view types, so it is unit-tested.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use cw_mermaid::Palette;
use gpui_kit::Image;

/// A diagram is known by the palette it is drawn in and its Mermaid source.
pub type Key = (Palette, String);

/// Drawn diagrams kept before the store starts over. Notes hold one diagram
/// each, so this covers every note a session is likely to visit.
const LIMIT: usize = 32;

/// What the store knows about one diagram.
pub enum Lookup {
    /// Drawn; `None` when the source is not a diagram that can be drawn.
    Ready(Option<Arc<Image>>),
    /// Not drawn yet. It has been queued.
    Pending,
}

#[derive(Default)]
pub struct Diagrams {
    done: HashMap<Key, Option<Arc<Image>>>,
    /// Asked for, and not yet handed out by [`Diagrams::take_queued`].
    queued: Vec<Key>,
    /// Handed out, and not yet stored.
    running: HashSet<Key>,
}

impl Diagrams {
    /// The drawn diagram, or `Pending` after queueing it once.
    pub fn lookup(&mut self, source: &str, palette: Palette) -> Lookup {
        let key = (palette, source.to_string());
        if let Some(image) = self.done.get(&key) {
            return Lookup::Ready(image.clone());
        }
        if !self.running.contains(&key) && !self.queued.contains(&key) {
            // Starting over only when something new is asked for means a page
            // whose diagrams are all drawn is never drawn again.
            if self.done.len() >= LIMIT {
                self.done.clear();
            }
            self.queued.push(key);
        }
        Lookup::Pending
    }

    /// The diagrams to draw now. The caller must [`Diagrams::store`] each.
    pub fn take_queued(&mut self) -> Vec<Key> {
        let keys = std::mem::take(&mut self.queued);
        self.running.extend(keys.iter().cloned());
        keys
    }

    /// Keep the result of drawing `key`; `None` when it could not be drawn.
    pub fn store(&mut self, key: Key, image: Option<Arc<Image>>) {
        self.running.remove(&key);
        self.done.insert(key, image);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::ImageFormat;

    const SOURCE: &str = "flowchart LR\n    A --> B\n";

    fn image() -> Option<Arc<Image>> {
        Some(Arc::new(Image::from_bytes(ImageFormat::Png, Vec::new())))
    }

    #[test]
    fn a_missing_diagram_is_queued_once_and_ready_after_it_is_stored() {
        let mut store = Diagrams::default();
        assert!(matches!(
            store.lookup(SOURCE, Palette::Dark),
            Lookup::Pending
        ));
        // Asked again before the job starts: still one job.
        assert!(matches!(
            store.lookup(SOURCE, Palette::Dark),
            Lookup::Pending
        ));
        let jobs = store.take_queued();
        assert_eq!(jobs, [(Palette::Dark, SOURCE.to_string())]);
        // Asked again while the job runs: no second job.
        assert!(matches!(
            store.lookup(SOURCE, Palette::Dark),
            Lookup::Pending
        ));
        assert!(store.take_queued().is_empty());

        store.store(jobs[0].clone(), image());
        assert!(matches!(
            store.lookup(SOURCE, Palette::Dark),
            Lookup::Ready(Some(_))
        ));
        assert!(store.take_queued().is_empty());
    }

    #[test]
    fn a_diagram_that_cannot_be_drawn_is_not_tried_again() {
        let mut store = Diagrams::default();
        store.lookup("broken\n", Palette::Dark);
        for key in store.take_queued() {
            store.store(key, None);
        }
        assert!(matches!(
            store.lookup("broken\n", Palette::Dark),
            Lookup::Ready(None)
        ));
        assert!(store.take_queued().is_empty());
    }

    #[test]
    fn each_palette_has_its_own_drawing() {
        let mut store = Diagrams::default();
        store.lookup(SOURCE, Palette::Dark);
        for key in store.take_queued() {
            store.store(key, image());
        }
        assert!(matches!(
            store.lookup(SOURCE, Palette::Light),
            Lookup::Pending
        ));
        assert_eq!(store.take_queued(), [(Palette::Light, SOURCE.to_string())]);
    }

    #[test]
    fn the_store_starts_over_at_its_limit_and_settles() {
        let mut store = Diagrams::default();
        // One page with more diagrams than the limit.
        let sources: Vec<String> = (0..LIMIT + 3)
            .map(|n| format!("flowchart LR\n    A{n}\n"))
            .collect();
        let mut rounds = 0;
        loop {
            for source in &sources {
                store.lookup(source, Palette::Dark);
            }
            let jobs = store.take_queued();
            if jobs.is_empty() {
                break;
            }
            for key in jobs {
                store.store(key, image());
            }
            rounds += 1;
            assert!(rounds < 4, "drawing the same page must come to an end");
        }
        assert_eq!(store.done.len(), sources.len());

        // The next new diagram empties the store.
        store.lookup(SOURCE, Palette::Dark);
        assert!(store.done.is_empty());
    }
}
