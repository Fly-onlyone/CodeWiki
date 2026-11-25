//! The name and the icon Windows shows above the app's notifications.
//!
//! An app without a package is known to Windows by the id it gives its
//! process (`APP_IDENTITY` in `main.rs`). The header of its notifications
//! shows what the user's registry holds for that id, under
//! `Software\Classes\AppUserModelId\<id>`: `DisplayName`, and `IconUri`, an
//! image file named by its full path. GPUI writes the name alone, at its
//! first notification, so the header had no icon. This module writes the
//! app's logo to `notification-icon.png` in the data folder, and both values,
//! just before the app's first Windows notification. Each is written only
//! when it is missing or different. `uninstall.ps1` removes the key and the
//! file.
//!
//! The values are those of Microsoft's "Send a local app notification from
//! other types of unpackaged apps" and of the Windows App SDK, which
//! registers an unpackaged app the same way (`RegisterAssets` in
//! `dev/AppNotifications/AppNotificationUtility.cpp`). `IconBackgroundColor`
//! is optional, and the SDK leaves it out too.

use std::path::{Path, PathBuf};

use windows_registry::{CURRENT_USER, Key, Type};

/// The file the logo is written to, in the data folder.
pub const ICON_FILE: &str = "notification-icon.png";

/// A string value of the key, as the registry holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    pub text: String,
    /// Whether it is a `REG_EXPAND_SZ` rather than a `REG_SZ`.
    pub expand: bool,
}

/// The values the key must hold for an app named `name` whose icon is the
/// file `icon`.
pub fn wanted(name: &str, icon: &Path) -> [(&'static str, Text); 2] {
    [
        (
            "DisplayName",
            // A `REG_SZ`, as GPUI writes it, so that the two never take turns.
            Text {
                text: name.to_string(),
                expand: false,
            },
        ),
        (
            "IconUri",
            // A `REG_EXPAND_SZ`, as Microsoft's example and the SDK write it.
            Text {
                text: icon.display().to_string(),
                expand: true,
            },
        ),
    ]
}

/// The values of `wanted` the key does not hold as they are: missing, or
/// with other text, or of the other type. `current` reads a value by name.
pub fn stale<'a>(
    wanted: &'a [(&'static str, Text)],
    current: impl Fn(&str) -> Option<Text>,
) -> Vec<&'a (&'static str, Text)> {
    wanted
        .iter()
        .filter(|(name, value)| current(name).as_ref() != Some(value))
        .collect()
}

/// Have Windows show `name` and `logo` above the notifications of the app
/// `id`. Returns what was written: the icon file, the names of the values,
/// or nothing when all was in place.
pub fn register(id: &str, name: &str, logo: &[u8]) -> Result<Vec<&'static str>, String> {
    let (icon, wrote_icon) = write_icon(&cw_core::paths::data_dir(), logo)?;
    let key = CURRENT_USER
        .create(format!(r"Software\Classes\AppUserModelId\{id}"))
        .map_err(|e| format!("could not open the registry key of {id}: {e}"))?;
    let mut written = Vec::new();
    if wrote_icon {
        written.push(ICON_FILE);
    }
    written.extend(update(&key, &wanted(name, &icon))?);
    Ok(written)
}

/// Write `logo` to [`ICON_FILE`] in `dir`, unless the file already holds it.
/// Returns the file's full path, which `IconUri` needs, and whether it was
/// written.
fn write_icon(dir: &Path, logo: &[u8]) -> Result<(PathBuf, bool), String> {
    let path = std::path::absolute(dir.join(ICON_FILE))
        .map_err(|e| format!("could not name the notification icon's file: {e}"))?;
    if std::fs::read(&path).is_ok_and(|bytes| bytes == logo) {
        return Ok((path, false));
    }
    std::fs::create_dir_all(dir)
        .and_then(|()| std::fs::write(&path, logo))
        .map_err(|e| format!("could not write {}: {e}", path.display()))?;
    Ok((path, true))
}

/// Write the values of `wanted` that `key` does not hold as they are.
/// Returns their names.
fn update(key: &Key, wanted: &[(&'static str, Text)]) -> Result<Vec<&'static str>, String> {
    let stale = stale(wanted, |name| read(key, name));
    for (name, value) in &stale {
        let result = if value.expand {
            key.set_expand_string(name, &value.text)
        } else {
            key.set_string(name, &value.text)
        };
        result.map_err(|e| format!("could not write {name}: {e}"))?;
    }
    Ok(stale.iter().map(|(name, _)| *name).collect())
}

/// The string value `name` of `key`; `None` when it is missing or not a
/// string.
fn read(key: &Key, name: &str) -> Option<Text> {
    let expand = match key.get_type(name).ok()? {
        Type::String => false,
        Type::ExpandString => true,
        _ => return None,
    };
    let text = key.get_string(name).ok()?;
    Some(Text { text, expand })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(text: &str, expand: bool) -> Option<Text> {
        Some(Text {
            text: text.into(),
            expand,
        })
    }

    fn names(stale: &[&(&'static str, Text)]) -> Vec<&'static str> {
        stale.iter().map(|(name, _)| *name).collect()
    }

    #[test]
    fn the_key_names_the_app_and_its_icon_by_full_path() {
        let [name, icon] = wanted("CodeWiki", Path::new(r"C:\Data\notification-icon.png"));
        assert_eq!(name, ("DisplayName", text("CodeWiki", false).unwrap()));
        assert_eq!(
            icon,
            (
                "IconUri",
                text(r"C:\Data\notification-icon.png", true).unwrap()
            )
        );
    }

    #[test]
    fn only_what_is_missing_or_different_is_written() {
        let wanted = wanted("CodeWiki", Path::new(r"C:\Data\notification-icon.png"));
        // A key GPUI made: the name, and no icon.
        let gpui = |name: &str| (name == "DisplayName").then(|| text("CodeWiki", false).unwrap());
        assert_eq!(names(&stale(&wanted, gpui)), ["IconUri"]);
        // Nothing yet.
        assert_eq!(names(&stale(&wanted, |_| None)), ["DisplayName", "IconUri"]);
        // All in place.
        let done = |name: &str| match name {
            "DisplayName" => text("CodeWiki", false),
            "IconUri" => text(r"C:\Data\notification-icon.png", true),
            _ => None,
        };
        assert!(stale(&wanted, done).is_empty());
        // The icon of another data folder, and a value of the other type.
        let other = |name: &str| match name {
            "DisplayName" => text("CodeWiki", true),
            "IconUri" => text(r"D:\Elsewhere\notification-icon.png", true),
            _ => None,
        };
        assert_eq!(names(&stale(&wanted, other)), ["DisplayName", "IconUri"]);
    }

    #[test]
    fn the_values_are_written_once_and_read_back() {
        // A key of the test's own, never the one Windows reads.
        let path = format!(r"Software\CodeWiki-test-identity-{}", std::process::id());
        let key = CURRENT_USER.create(&path).unwrap();
        let wanted = wanted("CodeWiki", Path::new(r"C:\Data\notification-icon.png"));
        let first = update(&key, &wanted);
        let second = update(&key, &wanted);
        let back = (read(&key, "DisplayName"), read(&key, "IconUri"));
        CURRENT_USER.remove_tree(&path).unwrap();
        assert_eq!(first, Ok(vec!["DisplayName", "IconUri"]));
        assert_eq!(second, Ok(vec![]));
        assert_eq!(back, (Some(wanted[0].1.clone()), Some(wanted[1].1.clone())));
    }

    #[test]
    fn the_icon_file_is_written_only_when_it_differs() {
        let dir = std::env::temp_dir().join(format!("codewiki-identity-{}", std::process::id()));
        let (path, first) = write_icon(&dir, b"logo").unwrap();
        let (_, second) = write_icon(&dir, b"logo").unwrap();
        let (_, changed) = write_icon(&dir, b"new logo").unwrap();
        let bytes = std::fs::read(&path).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(path.is_absolute());
        assert_eq!(path, dir.join(ICON_FILE));
        assert_eq!((first, second, changed), (true, false, true));
        assert_eq!(bytes, b"new logo");
    }
}
