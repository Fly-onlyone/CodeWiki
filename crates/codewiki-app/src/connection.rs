//! When the app tries to reach its backend again, and how.
//!
//! Nothing here touches GPUI or the network. After each status poll
//! `main.rs` hands [`Supervisor`] what the poll found, asks it whether an
//! attempt is due, runs that attempt in a task of its own and hands the result
//! back. A sequence number drops the result of an attempt that the user
//! overtook by switching backends or pressing "Try again".

use std::time::{Duration, Instant};

/// The first wait before the backend is tried again, and the longest.
pub const BACKEND_FIRST: Duration = Duration::from_secs(5);
pub const BACKEND_CAP: Duration = Duration::from_secs(30);

/// Status polls in a row that must go unanswered before the backend counts
/// as lost. One alone can fail on a busy machine.
pub const SILENT_POLLS: u32 = 2;

/// Waits that double from a first one up to a cap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backoff {
    first: Duration,
    cap: Duration,
    next: Duration,
}

impl Backoff {
    pub fn new(first: Duration, cap: Duration) -> Self {
        Self {
            first,
            cap,
            next: first.min(cap),
        }
    }

    /// The wait before the next attempt. The one after is twice as long, up
    /// to the cap.
    pub fn next_delay(&mut self) -> Duration {
        let delay = self.next;
        self.next = delay.saturating_mul(2).min(self.cap);
        delay
    }

    /// Start again from the first wait.
    pub fn reset(&mut self) {
        self.next = self.first.min(self.cap);
    }
}

/// Numbers the attempts. An attempt takes the number current when it
/// starts; its result counts only while that number is still current, so a
/// bump drops every result of the attempts made before it.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Sequence(u64);

impl Sequence {
    /// The number an attempt that starts now takes.
    pub fn current(self) -> u64 {
        self.0
    }

    /// Drop the results of every attempt made so far, and return the number
    /// of the next.
    pub fn bump(&mut self) -> u64 {
        self.0 += 1;
        self.0
    }

    /// Whether the result of attempt `seq` still counts.
    pub fn is_current(self, seq: u64) -> bool {
        self.0 == seq
    }
}

/// What an attempt may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attempt {
    /// Find a backend, or start the one on this machine
    /// (`cw_client::Backend::connect`).
    Full,
    /// Only look for one that answers (`cw_client::Backend::find`).
    ProbeOnly,
}

/// When the backend was lost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lost {
    /// The app has not reached one since it started, or since the user
    /// switched backends: the offline page shows.
    Startup,
    /// The backend in use stopped answering.
    MidSession,
}

/// What a status poll of the backend in use found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Poll {
    Answered,
    Silent,
    /// Not asked: there is no backend, or an update is being installed.
    NotAsked,
}

/// A backend that is lost, and the attempts to reach one again.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Loss {
    lost: Lost,
    /// Whether the backend lost was named rather than found or started on
    /// this machine.
    remote: bool,
    /// Whether the one attempt that may start a backend is still to come.
    full_left: bool,
    /// When the next attempt is due; `None` while one runs.
    due: Option<Instant>,
}

/// Watches the backend and decides when, and how, to try it again.
#[derive(Debug, Clone)]
pub struct Supervisor {
    seq: Sequence,
    /// Status polls in a row that went unanswered.
    silent: u32,
    backoff: Backoff,
    loss: Option<Loss>,
}

impl Default for Supervisor {
    fn default() -> Self {
        Self {
            seq: Sequence::default(),
            silent: 0,
            backoff: Backoff::new(BACKEND_FIRST, BACKEND_CAP),
            loss: None,
        }
    }
}

impl Supervisor {
    /// The number of an attempt that starts now, such as the user's connect.
    pub fn seq(&self) -> u64 {
        self.seq.current()
    }

    /// Whether the result of attempt `seq` still counts.
    pub fn is_current(&self, seq: u64) -> bool {
        self.seq.is_current(seq)
    }

    /// What is lost, while it is.
    pub fn lost(&self) -> Option<Lost> {
        self.loss.as_ref().map(|loss| loss.lost)
    }

    /// The user switched backends or pressed "Try again", and a connect
    /// follows. The results of the attempts made before are dropped, and the
    /// waits start again from the first. Returns the connect's number.
    pub fn restart(&mut self) -> u64 {
        self.loss = None;
        self.silent = 0;
        self.backoff.reset();
        self.seq.bump()
    }

    /// The connect failed and the offline page shows. Look for a backend
    /// after each wait from now on; these attempts start none.
    pub fn offline(&mut self, now: Instant) {
        self.silent = 0;
        self.loss = Some(Loss {
            lost: Lost::Startup,
            remote: false,
            full_left: false,
            due: Some(now + self.backoff.next_delay()),
        });
    }

    /// Take what a status poll of the backend in use found. Returns true when
    /// this poll makes the backend lost. `remote` says whether that backend
    /// was named rather than found or started on this machine; only one on
    /// this machine may be started again.
    pub fn polled(&mut self, poll: Poll, remote: bool, now: Instant) -> bool {
        if let Some(loss) = &mut self.loss {
            // Back under the same address and token, as a backend on another
            // machine comes back: no need to sit out the wait.
            if poll == Poll::Answered
                && let Some(due) = &mut loss.due
            {
                *due = (*due).min(now);
            }
            return false;
        }
        match poll {
            Poll::Silent => self.silent += 1,
            Poll::Answered | Poll::NotAsked => self.silent = 0,
        }
        if self.silent < SILENT_POLLS {
            return false;
        }
        self.silent = 0;
        // Two polls have gone unanswered already: the first attempt is now.
        self.loss = Some(Loss {
            lost: Lost::MidSession,
            remote,
            full_left: !remote,
            due: Some(now),
        });
        true
    }

    /// The user asked to reconnect while the backend in use is lost: drop the
    /// attempt under way, and make the next one now, as the first of a loss.
    /// Returns false when nothing is lost mid-session; the offline page's
    /// "Try again" is a connect of its own ([`restart`](Self::restart)).
    pub fn retry_now(&mut self, now: Instant) -> bool {
        let Some(loss) = &mut self.loss else {
            return false;
        };
        if loss.lost != Lost::MidSession {
            return false;
        }
        loss.full_left = !loss.remote;
        loss.due = Some(now);
        self.backoff.reset();
        self.seq.bump();
        true
    }

    /// The backend in use is gone and could not be started again, as after
    /// a failed update: lost mid-session, though no poll said so. It was just
    /// tried, so the first attempt waits. A loss under way goes on, its next
    /// attempt allowed to start a backend on this machine again: the update
    /// may have refused the one it had.
    pub fn lose(&mut self, remote: bool, now: Instant) {
        if let Some(loss) = &mut self.loss {
            if loss.lost == Lost::MidSession && !remote {
                loss.full_left = true;
            }
            return;
        }
        self.silent = 0;
        self.loss = Some(Loss {
            lost: Lost::MidSession,
            remote,
            full_left: !remote,
            due: Some(now + self.backoff.next_delay()),
        });
    }

    /// The attempt to make now, with its number, when one is due. It runs
    /// until [`failed`](Self::failed) or [`succeeded`](Self::succeeded) is
    /// given its number, and no other is due meanwhile.
    pub fn due(&mut self, now: Instant) -> Option<(Attempt, u64)> {
        let loss = self.loss.as_mut()?;
        if loss.due.is_none_or(|due| due > now) {
            return None;
        }
        loss.due = None;
        let attempt = if std::mem::take(&mut loss.full_left) {
            Attempt::Full
        } else {
            Attempt::ProbeOnly
        };
        Some((attempt, self.seq.current()))
    }

    /// How long until the next attempt is due. `None` while none waits.
    pub fn until_due(&self, now: Instant) -> Option<Duration> {
        let due = self.loss.as_ref()?.due?;
        Some(due.saturating_duration_since(now))
    }

    /// Attempt `seq` reached no backend: try again after the next wait.
    /// Returns false for an attempt whose result no longer counts.
    pub fn failed(&mut self, seq: u64, now: Instant) -> bool {
        if !self.seq.is_current(seq) {
            return false;
        }
        let Some(loss) = &mut self.loss else {
            return false;
        };
        loss.due = Some(now + self.backoff.next_delay());
        true
    }

    /// Attempt `seq` reached a backend. Returns what had been lost, or `None`
    /// when the result no longer counts and the backend it found is dropped.
    pub fn succeeded(&mut self, seq: u64) -> Option<Lost> {
        if !self.seq.is_current(seq) {
            return None;
        }
        let loss = self.loss.take()?;
        self.silent = 0;
        self.backoff.reset();
        Some(loss.lost)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[test]
    fn the_waits_double_up_to_the_cap() {
        let mut backend = Backoff::new(BACKEND_FIRST, BACKEND_CAP);
        let waits: Vec<u64> = (0..6).map(|_| backend.next_delay().as_secs()).collect();
        assert_eq!(waits, [5, 10, 20, 30, 30, 30]);
        backend.reset();
        assert_eq!(backend.next_delay(), secs(5));
        assert_eq!(backend.next_delay(), secs(10));

        // The embedding server's waits.
        let mut server = Backoff::new(secs(3), secs(30));
        let waits: Vec<u64> = (0..6).map(|_| server.next_delay().as_secs()).collect();
        assert_eq!(waits, [3, 6, 12, 24, 30, 30]);

        // A first wait beyond the cap is the cap.
        let mut odd = Backoff::new(secs(40), secs(30));
        assert_eq!(odd.next_delay(), secs(30));
    }

    #[test]
    fn a_bump_drops_the_results_of_earlier_attempts() {
        let mut seq = Sequence::default();
        let first = seq.current();
        assert!(seq.is_current(first));
        let second = seq.bump();
        assert_ne!(first, second);
        assert!(!seq.is_current(first));
        assert!(seq.is_current(second));
        assert_eq!(seq.current(), second);
    }

    #[test]
    fn the_backend_is_lost_after_two_silent_polls_in_a_row() {
        let now = Instant::now();
        let mut watch = Supervisor::default();
        assert!(!watch.polled(Poll::Silent, false, now));
        // An answer in between starts the count again.
        assert!(!watch.polled(Poll::Answered, false, now));
        assert!(!watch.polled(Poll::Silent, false, now));
        // So does a poll that was not made.
        assert!(!watch.polled(Poll::NotAsked, false, now));
        assert!(!watch.polled(Poll::Silent, false, now));
        assert_eq!(watch.lost(), None);
        assert!(watch.polled(Poll::Silent, false, now));
        assert_eq!(watch.lost(), Some(Lost::MidSession));
        // Lost already: more silence starts no new loss.
        assert!(!watch.polled(Poll::Silent, false, now));
    }

    #[test]
    fn a_lost_local_backend_gets_one_full_attempt_then_probes() {
        let t0 = Instant::now();
        let mut watch = Supervisor::default();
        watch.polled(Poll::Silent, false, t0);
        watch.polled(Poll::Silent, false, t0);
        // The first attempt is due at once, and may start a backend.
        assert_eq!(watch.until_due(t0), Some(Duration::ZERO));
        let (attempt, seq) = watch.due(t0).unwrap();
        assert_eq!(attempt, Attempt::Full);
        assert!(watch.failed(seq, t0));
        // The next waits the first wait, and only looks.
        assert_eq!(watch.due(t0 + secs(4)), None);
        assert_eq!(watch.until_due(t0 + secs(4)), Some(secs(1)));
        let (attempt, seq) = watch.due(t0 + secs(5)).unwrap();
        assert_eq!(attempt, Attempt::ProbeOnly);
        let t1 = t0 + secs(6);
        assert!(watch.failed(seq, t1));
        assert_eq!(watch.due(t1 + secs(9)), None);
        let (attempt, seq) = watch.due(t1 + secs(10)).unwrap();
        assert_eq!(attempt, Attempt::ProbeOnly);

        // Found: the loss is over, and was one of the session.
        assert_eq!(watch.succeeded(seq), Some(Lost::MidSession));
        assert_eq!(watch.lost(), None);
        assert_eq!(watch.until_due(t1), None);

        // The next loss starts afresh: one full attempt, then the first wait.
        let t2 = t1 + secs(60);
        watch.polled(Poll::Silent, false, t2);
        watch.polled(Poll::Silent, false, t2);
        let (attempt, seq) = watch.due(t2).unwrap();
        assert_eq!(attempt, Attempt::Full);
        assert!(watch.failed(seq, t2));
        assert_eq!(watch.until_due(t2), Some(secs(5)));
    }

    #[test]
    fn a_lost_remote_backend_is_only_looked_for() {
        let t0 = Instant::now();
        let mut watch = Supervisor::default();
        watch.polled(Poll::Silent, true, t0);
        assert!(watch.polled(Poll::Silent, true, t0));
        let (attempt, seq) = watch.due(t0).unwrap();
        assert_eq!(attempt, Attempt::ProbeOnly);
        assert!(watch.failed(seq, t0));
        let (attempt, _) = watch.due(t0 + secs(5)).unwrap();
        assert_eq!(attempt, Attempt::ProbeOnly);
    }

    #[test]
    fn the_offline_page_looks_for_a_backend_after_each_wait() {
        let t0 = Instant::now();
        let mut watch = Supervisor::default();
        watch.offline(t0);
        assert_eq!(watch.lost(), Some(Lost::Startup));
        // The connect has just failed: the first look waits.
        assert_eq!(watch.due(t0), None);
        let mut now = t0;
        let mut waits = Vec::new();
        for _ in 0..5 {
            let wait = watch.until_due(now).unwrap();
            waits.push(wait.as_secs());
            now += wait;
            let (attempt, seq) = watch.due(now).unwrap();
            assert_eq!(attempt, Attempt::ProbeOnly);
            assert!(watch.failed(seq, now));
        }
        assert_eq!(waits, [5, 10, 20, 30, 30]);
        let (_, seq) = watch.due(now + secs(30)).unwrap();
        assert_eq!(watch.succeeded(seq), Some(Lost::Startup));
    }

    #[test]
    fn one_attempt_runs_at_a_time() {
        let t0 = Instant::now();
        let mut watch = Supervisor::default();
        watch.polled(Poll::Silent, false, t0);
        watch.polled(Poll::Silent, false, t0);
        let (_, seq) = watch.due(t0).unwrap();
        // A start can take half a minute; the polls go on meanwhile.
        assert_eq!(watch.due(t0 + secs(30)), None);
        assert_eq!(watch.until_due(t0 + secs(30)), None);
        assert!(watch.failed(seq, t0 + secs(30)));
        assert!(watch.due(t0 + secs(35)).is_some());
    }

    #[test]
    fn a_restart_drops_the_results_of_earlier_attempts() {
        let t0 = Instant::now();
        let mut watch = Supervisor::default();
        watch.offline(t0);
        let (_, old) = watch.due(t0 + secs(5)).unwrap();
        // "Try again", or another backend chosen in Settings.
        let connect = watch.restart();
        assert!(watch.is_current(connect));
        assert_eq!(watch.seq(), connect);
        assert!(!watch.is_current(old));
        assert_eq!(watch.lost(), None);
        // The old attempt ends after it: neither its failure nor its success
        // counts.
        assert!(!watch.failed(old, t0 + secs(6)));
        assert_eq!(watch.until_due(t0 + secs(6)), None);
        assert_eq!(watch.succeeded(old), None);
    }

    #[test]
    fn try_again_starts_the_waits_from_the_first() {
        let t0 = Instant::now();
        let mut watch = Supervisor::default();
        watch.offline(t0);
        let mut now = t0;
        for _ in 0..3 {
            now += watch.until_due(now).unwrap();
            let (_, seq) = watch.due(now).unwrap();
            watch.failed(seq, now);
        }
        assert_eq!(watch.until_due(now), Some(secs(30)));
        watch.restart();
        // The user's connect failed too.
        watch.offline(now);
        assert_eq!(watch.until_due(now), Some(secs(5)));
    }

    #[test]
    fn asking_during_a_loss_tries_at_once_and_may_start_a_backend_again() {
        let t0 = Instant::now();
        let mut watch = Supervisor::default();
        // Nothing is lost: "Reconnect" is the connect after the offline page.
        assert!(!watch.retry_now(t0));
        watch.offline(t0);
        assert!(!watch.retry_now(t0));

        let mut watch = Supervisor::default();
        watch.polled(Poll::Silent, false, t0);
        watch.polled(Poll::Silent, false, t0);
        let (attempt, seq) = watch.due(t0).unwrap();
        assert_eq!(attempt, Attempt::Full);
        assert!(watch.failed(seq, t0));
        let (_, probing) = watch.due(t0 + secs(5)).unwrap();
        // The user asks while a look is under way: that look no longer
        // counts, and the next attempt may start a backend, now.
        let t1 = t0 + secs(6);
        assert!(watch.retry_now(t1));
        assert!(!watch.failed(probing, t1));
        assert_eq!(watch.lost(), Some(Lost::MidSession));
        let (attempt, seq) = watch.due(t1).unwrap();
        assert_eq!(attempt, Attempt::Full);
        // The waits start over.
        assert!(watch.failed(seq, t1));
        assert_eq!(watch.until_due(t1), Some(secs(5)));

        // A backend on another machine is still only looked for.
        let mut watch = Supervisor::default();
        watch.polled(Poll::Silent, true, t0);
        watch.polled(Poll::Silent, true, t0);
        assert!(watch.retry_now(t0));
        let (attempt, _) = watch.due(t0).unwrap();
        assert_eq!(attempt, Attempt::ProbeOnly);
    }

    #[test]
    fn a_backend_that_could_not_be_started_again_is_lost() {
        let t0 = Instant::now();
        let mut watch = Supervisor::default();
        watch.lose(false, t0);
        assert_eq!(watch.lost(), Some(Lost::MidSession));
        // It was just tried: the first attempt waits, and may start one.
        assert_eq!(watch.due(t0), None);
        let (attempt, seq) = watch.due(t0 + secs(5)).unwrap();
        assert_eq!(attempt, Attempt::Full);
        assert_eq!(watch.succeeded(seq), Some(Lost::MidSession));
    }

    #[test]
    fn a_failed_restart_during_a_loss_may_start_a_backend_again() {
        let t0 = Instant::now();
        let mut watch = Supervisor::default();
        watch.polled(Poll::Silent, false, t0);
        watch.polled(Poll::Silent, false, t0);
        // The one attempt that may start a backend is refused, as while an
        // update installs.
        let (attempt, seq) = watch.due(t0).unwrap();
        assert_eq!(attempt, Attempt::Full);
        assert!(watch.failed(seq, t0));
        // The update failed and so did the restart after it: the loss goes
        // on, and its next attempt may start one again, on the same waits.
        watch.lose(false, t0 + secs(1));
        assert_eq!(watch.lost(), Some(Lost::MidSession));
        assert_eq!(watch.until_due(t0), Some(secs(5)));
        let (attempt, _) = watch.due(t0 + secs(5)).unwrap();
        assert_eq!(attempt, Attempt::Full);

        // A backend on another machine is still only looked for.
        let mut watch = Supervisor::default();
        watch.polled(Poll::Silent, true, t0);
        watch.polled(Poll::Silent, true, t0);
        let (_, seq) = watch.due(t0).unwrap();
        watch.failed(seq, t0);
        watch.lose(true, t0);
        let (attempt, _) = watch.due(t0 + secs(5)).unwrap();
        assert_eq!(attempt, Attempt::ProbeOnly);

        // The offline page's looks stay looks.
        let mut watch = Supervisor::default();
        watch.offline(t0);
        watch.lose(false, t0);
        assert_eq!(watch.lost(), Some(Lost::Startup));
        let (attempt, _) = watch.due(t0 + secs(5)).unwrap();
        assert_eq!(attempt, Attempt::ProbeOnly);
    }

    #[test]
    fn an_answer_during_a_loss_makes_the_next_attempt_due_now() {
        let t0 = Instant::now();
        let mut watch = Supervisor::default();
        watch.polled(Poll::Silent, true, t0);
        watch.polled(Poll::Silent, true, t0);
        let (_, seq) = watch.due(t0).unwrap();
        watch.failed(seq, t0);
        // A backend on another machine came back under the same address.
        let t1 = t0 + secs(2);
        assert!(!watch.polled(Poll::Answered, true, t1));
        assert_eq!(watch.until_due(t1), Some(Duration::ZERO));
        assert!(watch.due(t1).is_some());
    }
}
