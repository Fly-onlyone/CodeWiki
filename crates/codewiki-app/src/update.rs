//! Updating the app: when to look for a new release, and what the settings
//! page says about it. The work itself is in `cw-update`.
//!
//! An installed app looks once a day unless the user turned that off: when
//! it starts, and, since it stays in the tray for weeks, every hour it looks
//! whether the day is over. The check's time is kept only when GitHub gave
//! an answer, so a check without network is tried again within the hour.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cw_update::{Check, Release, Version};

use crate::prefs;

/// The setting that turns the daily check off ("off") or on.
const AUTO_CHECK: &str = "update_check";
/// The setting that holds when GitHub last answered a check, in seconds since
/// 1970.
const CHECKED: &str = "update_checked";
const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Where the app stands with updates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// A development build or an unpacked package: it does not update itself.
    NotInstalled,
    /// Installed, and not checked since it started.
    Idle,
    Checking,
    UpToDate,
    Available(Release),
    /// The check got no answer; the text says why.
    Unavailable(String),
    Installing(Version),
    /// Installed, but this copy could not restart itself.
    Installed(Version),
}

impl State {
    /// What the check found.
    pub fn from_check(check: Check) -> Self {
        match check {
            Check::Newer(release) => Self::Available(release),
            Check::UpToDate => Self::UpToDate,
            Check::Unavailable(why) => Self::Unavailable(why),
        }
    }

    /// The release waiting to be installed.
    pub fn available(&self) -> Option<&Release> {
        match self {
            Self::Available(release) => Some(release),
            _ => None,
        }
    }

    /// Whether an update is being installed or waits for a restart. The
    /// backend is stopped then, so the app starts no new work.
    pub fn installing(&self) -> bool {
        matches!(self, Self::Installing(_) | Self::Installed(_))
    }

    /// Whether a check may start now.
    pub fn can_check(&self) -> bool {
        matches!(
            self,
            Self::Idle | Self::UpToDate | Self::Available(_) | Self::Unavailable(_)
        )
    }

    /// The value of the "Version" row on the settings page.
    pub fn describe(&self) -> String {
        let current = cw_core::VERSION;
        match self {
            Self::NotInstalled => {
                format!("{current}, a development build: only an installed copy updates itself")
            }
            Self::Idle => current.to_string(),
            Self::Checking => format!("{current}, looking for a newer version…"),
            Self::UpToDate => format!("{current}, the latest version"),
            Self::Available(release) => format!("{current}; {} is available", release.version),
            Self::Unavailable(why) => format!("{current}; could not check: {why}"),
            Self::Installing(version) => format!("{current}, installing {version}…"),
            Self::Installed(version) => {
                format!("{version} is installed; close CodeWiki and open it again to use it")
            }
        }
    }
}

/// Whether the app checks by itself when it starts.
pub fn auto_check() -> bool {
    prefs::read(AUTO_CHECK).as_deref() != Some("off")
}

pub fn set_auto_check(on: bool) {
    prefs::write(AUTO_CHECK, if on { "on" } else { "off" });
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Whether the daily check is due.
pub fn due() -> bool {
    due_after(prefs::read(CHECKED).and_then(|t| t.parse().ok()), now())
}

/// Due when it never ran, when a day has passed, or when the stored time is
/// in the future (the clock was set back).
fn due_after(last: Option<u64>, now: u64) -> bool {
    match last {
        None => true,
        Some(last) if last > now => true,
        Some(last) => now - last >= INTERVAL.as_secs(),
    }
}

/// Ask GitHub, and remember when it answered. Blocking.
pub fn check() -> Check {
    let check = cw_update::check(&cw_update::api_base(), cw_core::VERSION);
    if !matches!(check, Check::Unavailable(_)) {
        prefs::write(CHECKED, &now().to_string());
    }
    check
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: u64 = 24 * 60 * 60;

    #[test]
    fn the_check_is_due_once_a_day() {
        let now = 100 * DAY;
        assert!(due_after(None, now));
        assert!(!due_after(Some(now - DAY + 1), now));
        assert!(due_after(Some(now - DAY), now));
        // The clock went back: do not wait for it to catch up.
        assert!(due_after(Some(now + 60), now));
    }

    fn release() -> Release {
        Release {
            version: Version::new(9, 0, 0),
            zip_name: "CodeWiki-9.0.0-windows-x64.zip".into(),
            zip_url: "https://example.invalid/z".into(),
            sha256_url: "https://example.invalid/s".into(),
        }
    }

    #[test]
    fn describes_each_state_with_this_version() {
        let current = cw_core::VERSION;
        assert_eq!(State::Idle.describe(), current);
        assert!(State::UpToDate.describe().ends_with("the latest version"));
        assert_eq!(
            State::Available(release()).describe(),
            format!("{current}; 9.0.0 is available")
        );
        assert!(
            State::Unavailable("no network".into())
                .describe()
                .ends_with("could not check: no network")
        );
        assert!(State::NotInstalled.describe().contains("development build"));
    }

    #[test]
    fn a_check_does_not_start_while_one_runs_or_an_update_installs() {
        assert!(State::Idle.can_check());
        assert!(State::Available(release()).can_check());
        assert!(!State::Checking.can_check());
        assert!(!State::Installing(Version::new(9, 0, 0)).can_check());
        assert!(!State::Installed(Version::new(9, 0, 0)).can_check());
        assert!(State::Installed(Version::new(9, 0, 0)).installing());
        assert!(!State::NotInstalled.can_check());
    }
}
