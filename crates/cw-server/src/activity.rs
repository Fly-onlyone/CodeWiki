//! What the backend is doing right now, for `GET /v1/status`.
//!
//! Two things a client cannot see otherwise: which wiki updates are running,
//! whoever started them (the app, an agent's bridge, a git webhook), and when
//! an agent's MCP bridge last called. Nothing here is written to disk: it
//! describes this process, and a new backend starts with none of it.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

/// Seconds since the Unix epoch, on this machine's clock.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default()
}

struct RunningUpdate {
    /// Tells apart two updates of one repository, one of which waits.
    key: u64,
    repo: String,
    started: u64,
}

#[derive(Default)]
pub(crate) struct Activity {
    /// When a bridge last called, in seconds since the epoch; 0 for never.
    last_bridge_call: AtomicU64,
    updates: Mutex<Vec<RunningUpdate>>,
    next_key: AtomicU64,
}

/// Marks an update as running until it is dropped, a panic included.
pub(crate) struct UpdateGuard<'a> {
    activity: &'a Activity,
    key: u64,
}

impl Drop for UpdateGuard<'_> {
    fn drop(&mut self) {
        self.activity
            .updates
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|update| update.key != self.key);
    }
}

impl Activity {
    /// Record that an agent's MCP bridge called just now.
    pub fn bridge_called(&self) {
        // Never 0, which means "never", even on a clock set to the epoch.
        self.last_bridge_call.store(now().max(1), Ordering::Relaxed);
    }

    /// Record an update of `repo` as running, from now until the guard is
    /// dropped. An update that waits for another one counts as running.
    pub fn begin_update(&self, repo: &str) -> UpdateGuard<'_> {
        let key = self.next_key.fetch_add(1, Ordering::Relaxed);
        self.updates
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(RunningUpdate {
                key,
                repo: repo.to_string(),
                started: now(),
            });
        UpdateGuard {
            activity: self,
            key,
        }
    }

    /// The body of `GET /v1/status`. `now` is this machine's clock, so that a
    /// client works with ages and its own clock does not matter.
    pub fn report(&self) -> Value {
        let updates: Vec<Value> = self
            .updates
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .map(|update| json!({ "repo": update.repo, "started": update.started }))
            .collect();
        let last_call = match self.last_bridge_call.load(Ordering::Relaxed) {
            0 => Value::Null,
            at => json!(at),
        };
        json!({
            "version": cw_core::VERSION,
            "now": now(),
            "updates": updates,
            "bridge": { "last_call": last_call },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_update_is_running_until_its_guard_is_dropped() {
        let activity = Activity::default();
        assert_eq!(activity.report()["updates"], json!([]));

        let first = activity.begin_update("alpha");
        let second = activity.begin_update("alpha");
        let third = activity.begin_update("beta");
        let repos = |activity: &Activity| -> Vec<String> {
            activity.report()["updates"]
                .as_array()
                .unwrap()
                .iter()
                .map(|update| update["repo"].as_str().unwrap().to_string())
                .collect()
        };
        assert_eq!(repos(&activity), ["alpha", "alpha", "beta"]);

        // Only the one that ended goes, not the other update of the same
        // repository.
        drop(first);
        assert_eq!(repos(&activity), ["alpha", "beta"]);
        drop(third);
        drop(second);
        assert!(repos(&activity).is_empty());
    }

    #[test]
    fn a_bridge_call_is_remembered_with_its_time() {
        let activity = Activity::default();
        assert_eq!(activity.report()["bridge"]["last_call"], Value::Null);

        activity.bridge_called();
        let report = activity.report();
        let last_call = report["bridge"]["last_call"].as_u64().unwrap();
        assert!(last_call > 0);
        assert!(last_call <= report["now"].as_u64().unwrap());
        assert_eq!(report["version"], cw_core::VERSION);
    }
}
