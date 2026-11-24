//! Which waiting notes an agent holds.
//!
//! Several coding-agent sessions may explain notes at once. The backend lends
//! each note it lists to one agent for [`cw_core::CLAIM_MINUTES`] minutes, so
//! that two agents never get the same one. Nothing is written to disk: a new
//! backend starts with no claims, and an expired one frees itself.

use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Default)]
pub(crate) struct Claims {
    /// (repository id, note path in the vault) to (agent id, held until).
    held: HashMap<(String, String), (String, Instant)>,
}

fn period() -> Duration {
    Duration::from_secs(cw_core::CLAIM_MINUTES * 60)
}

impl Claims {
    /// Drop the claims that ran out. True when any went.
    pub fn expire(&mut self, now: Instant) -> bool {
        let before = self.held.len();
        self.held.retain(|_, (_, until)| *until > now);
        self.held.len() != before
    }

    /// The notes of `repo` held by an agent other than `agent`; with `None`,
    /// every held note.
    pub fn others(&self, repo: &str, agent: Option<&str>) -> Vec<String> {
        let mut paths: Vec<String> = self
            .held
            .iter()
            .filter(|((r, _), (holder, _))| r == repo && Some(holder.as_str()) != agent)
            .map(|((_, path), _)| path.clone())
            .collect();
        paths.sort();
        paths
    }

    /// Lend `notes` to `agent`. True when anything changed.
    pub fn claim(&mut self, repo: &str, agent: &str, notes: &[String], now: Instant) -> bool {
        let mut changed = false;
        for note in notes {
            let value = (agent.to_string(), now + period());
            let old = self.held.insert((repo.to_string(), note.clone()), value);
            changed |= old.is_none_or(|(holder, _)| holder != agent);
        }
        changed
    }

    /// Push every claim `agent` holds in `repo` to a full period from now.
    pub fn renew(&mut self, repo: &str, agent: &str, now: Instant) {
        for ((r, _), (holder, until)) in &mut self.held {
            if r == repo && holder == agent {
                *until = now + period();
            }
        }
    }

    /// Free one note. True when it was held.
    pub fn release(&mut self, repo: &str, note: &str) -> bool {
        self.held
            .remove(&(repo.to_string(), note.to_string()))
            .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notes(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn other_agents_notes_are_left_out_but_not_the_own() {
        let now = Instant::now();
        let mut claims = Claims::default();
        assert!(claims.claim("r", "a", &notes(&["one.md"]), now));
        assert!(claims.claim("r", "b", &notes(&["two.md"]), now));
        assert!(claims.claim("other", "b", &notes(&["three.md"]), now));
        assert_eq!(claims.others("r", Some("a")), ["two.md"]);
        assert_eq!(claims.others("r", Some("b")), ["one.md"]);
        assert_eq!(claims.others("r", None), ["one.md", "two.md"]);
        // Claiming again what the agent holds changes nothing.
        assert!(!claims.claim("r", "a", &notes(&["one.md"]), now));
    }

    #[test]
    fn a_claim_expires_after_the_period() {
        let now = Instant::now();
        let mut claims = Claims::default();
        claims.claim("r", "a", &notes(&["one.md"]), now);
        assert!(!claims.expire(now + period() - Duration::from_secs(1)));
        assert_eq!(claims.others("r", None), ["one.md"]);
        assert!(claims.expire(now + period()));
        assert!(claims.others("r", None).is_empty());
        assert!(!claims.expire(now + period()));
    }

    #[test]
    fn renewing_extends_only_the_agents_claims_in_that_repo() {
        let now = Instant::now();
        let mut claims = Claims::default();
        claims.claim("r", "a", &notes(&["one.md"]), now);
        claims.claim("r", "b", &notes(&["two.md"]), now);
        let later = now + period() - Duration::from_secs(1);
        claims.renew("r", "a", later);
        assert!(claims.expire(now + period()));
        assert_eq!(claims.others("r", None), ["one.md"]);
    }

    #[test]
    fn a_release_frees_the_note() {
        let now = Instant::now();
        let mut claims = Claims::default();
        claims.claim("r", "a", &notes(&["one.md"]), now);
        assert!(!claims.release("r", "two.md"));
        assert!(claims.release("r", "one.md"));
        assert!(claims.others("r", None).is_empty());
    }
}
