//! How many proposals wait in each repository, for `GET /v1/status`.
//!
//! Counting reads every proposal of every registered repository, and one on
//! a network share or on a disk that sleeps can take seconds. The app asks
//! for the status every 5 s and takes an answer slower than 3 s for a lost
//! backend, so the status waits for a count at most [`WAIT`]. A count that
//! takes longer goes on in the background. One count runs at a time: a
//! question asked meanwhile joins the count that runs.
//!
//! An answer gives only the repositories counted while it waited. A count
//! made before the question may be older than what the app has loaded since,
//! and the app would take the difference for news; a repository left out of
//! an answer is only compared at the next one.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use tokio::sync::watch;

/// Pending proposals by repository id.
pub(crate) type Counts = BTreeMap<String, usize>;

/// What a count found for one repository: how many proposals wait there, or
/// `None` when it is left out (its folder is gone, its vault cannot be read).
pub(crate) type Found = Option<usize>;

/// How long the status waits for a count.
pub(crate) const WAIT: Duration = Duration::from_secs(1);

/// The count that runs, if one does.
#[derive(Clone, Default)]
pub(crate) struct ProposalCounts(Arc<Mutex<Option<Arc<Count>>>>);

/// A count under way.
struct Count {
    /// What it has found so far, by repository id, with when.
    found: Mutex<BTreeMap<String, (Found, Instant)>>,
    /// Set once it is over, finished or not.
    over: watch::Sender<bool>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl ProposalCounts {
    /// The repositories counted within `wait` from now. `count` counts every
    /// repository and tells each one it found; it runs on a thread of its
    /// own, and only when no count runs yet.
    pub async fn get<F>(&self, wait: Duration, count: F) -> Counts
    where
        F: FnOnce(&mut dyn FnMut(String, Found)) + Send + 'static,
    {
        let asked = Instant::now();
        let (running, mut over) = self.start(count);
        // Either way, the answer is what was counted by now.
        let _ = tokio::time::timeout(wait, over.wait_for(|over| *over)).await;
        fresh(&lock(&running.found), asked)
    }

    /// The count that runs, or a new one when none does.
    fn start<F>(&self, count: F) -> (Arc<Count>, watch::Receiver<bool>)
    where
        F: FnOnce(&mut dyn FnMut(String, Found)) + Send + 'static,
    {
        let mut slot = lock(&self.0);
        if let Some(running) = &*slot {
            return (running.clone(), running.over.subscribe());
        }
        let (over, watch) = watch::channel(false);
        let running = Arc::new(Count {
            found: Mutex::default(),
            over,
        });
        *slot = Some(running.clone());
        drop(slot);
        let end = End {
            counts: self.clone(),
            running: running.clone(),
        };
        tokio::task::spawn_blocking(move || {
            count(&mut |id, found| {
                lock(&end.running.found).insert(id, (found, Instant::now()));
            });
        });
        (running, watch)
    }
}

/// Ends a count when it is dropped, a panic included, and lets the next one
/// run.
struct End {
    counts: ProposalCounts,
    running: Arc<Count>,
}

impl Drop for End {
    fn drop(&mut self) {
        *lock(&self.counts.0) = None;
        self.running.over.send_replace(true);
    }
}

/// What a count found at or after `asked`, less the repositories it left out.
fn fresh(found: &BTreeMap<String, (Found, Instant)>, asked: Instant) -> Counts {
    found
        .iter()
        .filter(|(_, (_, at))| *at >= asked)
        .filter_map(|(id, (count, _))| Some((id.clone(), (*count)?)))
        .collect()
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

    /// A count that finds each repository it is fed, until the feed ends.
    #[allow(clippy::type_complexity)]
    fn fed() -> (
        mpsc::Sender<(&'static str, Found)>,
        impl FnOnce(&mut dyn FnMut(String, Found)) + Send + 'static,
    ) {
        let (feed, items) = mpsc::channel::<(&'static str, Found)>();
        (feed, move |found: &mut dyn FnMut(String, Found)| {
            for (id, n) in items {
                found(id.to_string(), n);
            }
        })
    }

    #[test]
    fn only_what_was_counted_after_the_question_is_answered() {
        let asked = Instant::now();
        let before = asked - Duration::from_millis(5);
        let found = BTreeMap::from([
            ("old".to_string(), (Some(1), before)),
            ("new".to_string(), (Some(2), asked)),
            // Its folder went away.
            ("gone".to_string(), (None, asked)),
        ]);
        assert_eq!(fresh(&found, asked), counts(&[("new", 2)]));
        assert_eq!(fresh(&found, before), counts(&[("new", 2), ("old", 1)]));
    }

    #[tokio::test]
    async fn a_slow_count_answers_only_what_it_counted_while_the_question_waited() {
        let pending = ProposalCounts::default();
        pending
            .get(WAIT, quick(&[("a", Some(1)), ("b", Some(2))]))
            .await;

        let (feed, count) = fed();
        feed.send(("a", Some(5))).unwrap();
        let short = Duration::from_millis(100);
        // Not the old count of `b`, which it has not reached: that may be
        // older than what the app loaded since.
        assert_eq!(pending.get(short, count).await, counts(&[("a", 5)]));

        // A question asked meanwhile joins that count, starts none, and gets
        // only what it counts from then on.
        let started = Arc::new(AtomicBool::new(false));
        let flag = started.clone();
        let second = move |_: &mut dyn FnMut(String, Found)| flag.store(true, Ordering::SeqCst);
        let later = feed.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            later.send(("b", Some(7))).unwrap();
        });
        assert_eq!(pending.get(short, second).await, counts(&[("b", 7)]));
        assert!(!started.load(Ordering::SeqCst));

        // Once it ends, the next question counts anew.
        drop(feed);
        settled(&pending).await;
        assert_eq!(
            pending
                .get(WAIT, quick(&[("a", Some(5)), ("b", Some(7))]))
                .await,
            counts(&[("a", 5), ("b", 7)])
        );
    }

    /// Wait until no count runs.
    async fn settled(counts: &ProposalCounts) {
        for _ in 0..500 {
            if lock(&counts.0).is_none() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("the count never ended");
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
    async fn a_count_that_panics_lets_the_next_one_run() {
        let pending = ProposalCounts::default();
        pending.get(WAIT, quick(&[("a", Some(1))])).await;
        let broken = |found: &mut dyn FnMut(String, Found)| {
            found("a".to_string(), Some(9));
            panic!("a vault that could not be read");
        };
        // What it found before it failed is shown.
        assert_eq!(pending.get(WAIT, broken).await, counts(&[("a", 9)]));
        settled(&pending).await;
        assert_eq!(
            pending.get(WAIT, quick(&[("a", Some(3))])).await,
            counts(&[("a", 3)])
        );
    }
}
