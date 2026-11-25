//! How many proposals wait in each repository, for `GET /v1/status`.
//!
//! Counting reads every proposal of every registered repository, and one on
//! a network share or on a disk that sleeps can take seconds. The app asks
//! for the status every 5 s and takes an answer slower than 3 s for a lost
//! backend, so the status waits for a count at most [`WAIT`]. A count that
//! takes longer goes on in the background and is kept for the answers after
//! it; until it is over, an answer gives what it has counted so far and, for
//! the rest, what the last count that finished found. One count runs at a
//! time: an answer asked for meanwhile waits for the count that runs.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use tokio::sync::watch;

/// Pending proposals by repository id.
pub(crate) type Counts = BTreeMap<String, usize>;

/// What a count found for one repository: how many proposals wait there, or
/// `None` when it is left out (its folder is gone, its vault cannot be read).
pub(crate) type Found = Option<usize>;

/// How long the status waits for a count.
pub(crate) const WAIT: Duration = Duration::from_secs(1);

/// The counts the status gives, and the count that runs.
#[derive(Clone, Default)]
pub(crate) struct ProposalCounts(Arc<Mutex<State>>);

#[derive(Default)]
struct State {
    /// What the last count that finished found.
    last: Option<Counts>,
    /// The count that runs, if one does.
    running: Option<Arc<Count>>,
}

/// A count under way.
struct Count {
    /// What it has found so far, by repository id.
    found: Mutex<BTreeMap<String, Found>>,
    /// `None` while it runs; then whether it went through every repository.
    over: watch::Sender<Option<bool>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl ProposalCounts {
    /// The counts, within `wait`. `count` counts every repository and tells
    /// each one it found; it runs on a thread of its own, and only when no
    /// count runs yet.
    pub async fn get<F>(&self, wait: Duration, count: F) -> Counts
    where
        F: FnOnce(&mut dyn FnMut(String, Found)) + Send + 'static,
    {
        let (running, mut over) = self.start(count);
        let finished = tokio::time::timeout(wait, async {
            over.wait_for(Option::is_some)
                .await
                .is_ok_and(|over| *over == Some(true))
        })
        .await
        .unwrap_or(false);
        let found = lock(&running.found).clone();
        if finished {
            return counted(&found);
        }
        merge(lock(&self.0).last.as_ref(), &found)
    }

    /// The count that runs, or a new one when none does.
    fn start<F>(&self, count: F) -> (Arc<Count>, watch::Receiver<Option<bool>>)
    where
        F: FnOnce(&mut dyn FnMut(String, Found)) + Send + 'static,
    {
        let mut state = lock(&self.0);
        if let Some(running) = &state.running {
            return (running.clone(), running.over.subscribe());
        }
        let (over, watch) = watch::channel(None);
        let running = Arc::new(Count {
            found: Mutex::default(),
            over,
        });
        state.running = Some(running.clone());
        drop(state);
        let mut end = End {
            counts: self.clone(),
            running: running.clone(),
            finished: false,
        };
        tokio::task::spawn_blocking(move || {
            count(&mut |id, found| {
                lock(&end.running.found).insert(id, found);
            });
            end.went_through();
        });
        (running, watch)
    }
}

/// Ends a count when it is dropped, a panic included: keeps what it found
/// when it went through every repository, and lets the next count run.
struct End {
    counts: ProposalCounts,
    running: Arc<Count>,
    finished: bool,
}

impl End {
    /// The count went through every repository.
    fn went_through(&mut self) {
        self.finished = true;
    }
}

impl Drop for End {
    fn drop(&mut self) {
        let found = self.finished.then(|| counted(&lock(&self.running.found)));
        let mut state = lock(&self.counts.0);
        if let Some(found) = found {
            state.last = Some(found);
        }
        state.running = None;
        drop(state);
        self.running.over.send_replace(Some(self.finished));
    }
}

/// The repositories a count found, less those it left out.
fn counted(found: &BTreeMap<String, Found>) -> Counts {
    found
        .iter()
        .filter_map(|(id, count)| Some((id.clone(), (*count)?)))
        .collect()
}

/// The counts while a count runs: what it found so far, over what the last
/// count that finished found.
fn merge(last: Option<&Counts>, found: &BTreeMap<String, Found>) -> Counts {
    let mut counts = last.cloned().unwrap_or_default();
    for (id, count) in found {
        match count {
            Some(count) => counts.insert(id.clone(), *count),
            None => counts.remove(id),
        };
    }
    counts
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;

    use super::*;

    fn counts(pairs: &[(&str, usize)]) -> Counts {
        pairs.iter().map(|(id, n)| (id.to_string(), *n)).collect()
    }

    /// A count that finds `pairs` at once.
    fn quick(
        pairs: &'static [(&'static str, Found)],
    ) -> impl FnOnce(&mut dyn FnMut(String, Found)) + Send + 'static {
        move |found| {
            for (id, n) in pairs {
                found(id.to_string(), *n);
            }
        }
    }

    /// A count that finds `pairs`, then stalls until the sender it returns
    /// is used or dropped.
    #[allow(clippy::type_complexity)]
    fn stalled(
        pairs: &'static [(&'static str, Found)],
    ) -> (
        mpsc::Sender<()>,
        impl FnOnce(&mut dyn FnMut(String, Found)) + Send + 'static,
    ) {
        let (go, wait) = mpsc::channel::<()>();
        (go, move |found: &mut dyn FnMut(String, Found)| {
            for (id, n) in pairs {
                found(id.to_string(), *n);
            }
            let _ = wait.recv();
        })
    }

    /// Wait until no count runs.
    async fn settled(counts: &ProposalCounts) {
        for _ in 0..500 {
            if lock(&counts.0).running.is_none() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("the count never ended");
    }

    #[test]
    fn what_a_count_found_so_far_goes_over_the_last_counts() {
        let last = counts(&[("a", 1), ("b", 2), ("c", 3)]);
        let found = BTreeMap::from([
            ("a".to_string(), Some(5)),
            // Its folder went away.
            ("b".to_string(), None),
            ("d".to_string(), Some(4)),
        ]);
        assert_eq!(
            merge(Some(&last), &found),
            counts(&[("a", 5), ("c", 3), ("d", 4)])
        );
        // Before any count finished, what was found is all there is.
        assert_eq!(merge(None, &found), counts(&[("a", 5), ("d", 4)]));
        assert_eq!(merge(Some(&last), &BTreeMap::new()), last);
    }

    #[tokio::test]
    async fn a_count_in_time_gives_what_it_found() {
        let pending = ProposalCounts::default();
        let answer = pending
            .get(WAIT, quick(&[("a", Some(1)), ("gone", None)]))
            .await;
        assert_eq!(answer, counts(&[("a", 1)]));
        let answer = pending.get(WAIT, quick(&[("a", Some(2))])).await;
        assert_eq!(answer, counts(&[("a", 2)]));
    }

    #[tokio::test]
    async fn a_slow_count_gives_the_last_counts_and_finishes_in_the_background() {
        let pending = ProposalCounts::default();
        pending
            .get(WAIT, quick(&[("a", Some(1)), ("b", Some(2))]))
            .await;

        // The count stalls after the first repository.
        let (go, count) = stalled(&[("a", Some(5))]);
        let short = Duration::from_millis(50);
        assert_eq!(
            pending.get(short, count).await,
            counts(&[("a", 5), ("b", 2)])
        );

        // An answer asked for meanwhile starts no second count.
        let started = Arc::new(AtomicBool::new(false));
        let flag = started.clone();
        let second = move |_: &mut dyn FnMut(String, Found)| flag.store(true, Ordering::SeqCst);
        assert_eq!(
            pending.get(short, second).await,
            counts(&[("a", 5), ("b", 2)])
        );
        assert!(!started.load(Ordering::SeqCst));

        // The stalled count finishes on its own and is kept.
        go.send(()).unwrap();
        settled(&pending).await;
        let (go, count) = stalled(&[]);
        assert_eq!(pending.get(short, count).await, counts(&[("a", 5)]));
        drop(go);
        settled(&pending).await;
    }

    #[tokio::test]
    async fn a_count_that_panics_lets_the_next_one_run() {
        let pending = ProposalCounts::default();
        pending.get(WAIT, quick(&[("a", Some(1))])).await;
        let broken = |found: &mut dyn FnMut(String, Found)| {
            found("a".to_string(), Some(9));
            panic!("a vault that could not be read");
        };
        // What it found before it failed is shown; it is not kept.
        assert_eq!(pending.get(WAIT, broken).await, counts(&[("a", 9)]));
        settled(&pending).await;
        assert_eq!(
            pending.get(WAIT, quick(&[("a", Some(3))])).await,
            counts(&[("a", 3)])
        );
    }
}
