//! The setup program's entry in Windows Settings > Apps.

/// The setup program's uninstall key under `HKEY_CURRENT_USER`.
pub const APPS_ENTRY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{7DEA7959-917F-46EC-80A8-625E2A9B24F5}_is1";

/// Write `version` as `DisplayVersion` of the Apps entry `key` under
/// `HKEY_CURRENT_USER`. The key is opened, never created: an installation made
/// by `install.cmd` has no entry and must not gain one. Returns whether the
/// entry existed and was updated.
#[cfg(windows)]
pub fn record_version(key: &str, version: &str) -> Result<bool, String> {
    use windows_registry::CURRENT_USER;

    let Ok(entry) = CURRENT_USER.options().read().write().open(key) else {
        return Ok(false);
    };
    entry
        .set_string("DisplayVersion", version)
        .map(|()| true)
        .map_err(|e| format!("could not write the version into Settings > Apps: {e}"))
}

/// There is no Apps entry outside Windows.
#[cfg(not(windows))]
pub fn record_version(_key: &str, _version: &str) -> Result<bool, String> {
    Ok(false)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use windows_registry::CURRENT_USER;

    fn test_key(name: &str) -> String {
        format!(r"Software\CodeWiki-test\{name}-{}", std::process::id())
    }

    fn clean_up(key: &str) {
        let _ = CURRENT_USER.remove_tree(key);
        if CURRENT_USER
            .open(r"Software\CodeWiki-test")
            .and_then(|key| key.keys().map(|mut keys| keys.next().is_none()))
            .unwrap_or(false)
        {
            let _ = CURRENT_USER.remove_tree(r"Software\CodeWiki-test");
        }
    }

    #[test]
    fn a_missing_entry_is_left_alone() {
        let key = test_key("apps-entry-missing");
        assert_eq!(record_version(&key, "0.3.1"), Ok(false));
        assert!(CURRENT_USER.open(&key).is_err());
        clean_up(&key);
    }

    #[test]
    fn an_existing_entry_gets_the_new_version() {
        let key = test_key("apps-entry-existing");
        let made = CURRENT_USER.create(&key).unwrap();
        made.set_string("DisplayVersion", "0.3.0").unwrap();
        made.set_string("DisplayName", "CodeWiki").unwrap();
        drop(made);

        assert_eq!(record_version(&key, "0.3.1"), Ok(true));

        let opened = CURRENT_USER.open(&key).unwrap();
        assert_eq!(opened.get_string("DisplayVersion").unwrap(), "0.3.1");
        assert_eq!(opened.get_string("DisplayName").unwrap(), "CodeWiki");
        drop(opened);
        clean_up(&key);
    }

    #[test]
    fn the_key_is_the_setup_programs() {
        assert!(APPS_ENTRY.starts_with(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\"));
        assert!(APPS_ENTRY.ends_with("{7DEA7959-917F-46EC-80A8-625E2A9B24F5}_is1"));
    }
}
