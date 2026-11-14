//! Unpacking a verified package and checking that its programs start.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use semver::Version;

use crate::PROGRAMS;

/// How long `codewiki --version` may take. It prints at once; a virus scan
/// of a program never run before can hold it for a few seconds.
const VERSION_TIMEOUT: Duration = Duration::from_secs(20);

/// Unpack `zip` into an empty `to` and check that it is a CodeWiki package of
/// `version` whose `codewiki.exe` starts on this machine.
pub fn stage(zip: &Path, to: &Path, version: &Version) -> Result<(), String> {
    if to.exists() {
        std::fs::remove_dir_all(to)
            .map_err(|e| format!("could not empty {}: {e}", to.display()))?;
    }
    std::fs::create_dir_all(to).map_err(|e| format!("could not create {}: {e}", to.display()))?;
    let file =
        std::fs::File::open(zip).map_err(|e| format!("could not open {}: {e}", zip.display()))?;
    zip::ZipArchive::new(file)
        .and_then(|mut archive| archive.extract(to))
        .map_err(|e| format!("could not unpack {}: {e}", zip.display()))?;
    for program in PROGRAMS {
        if !to.join(program).is_file() {
            return Err(format!("the package holds no {program}"));
        }
    }
    let found = program_version(&to.join("codewiki.exe"))?;
    if found != *version {
        return Err(format!(
            "the package's codewiki.exe is version {found}, not {version}"
        ));
    }
    Ok(())
}

/// The version `<exe> --version` prints. The program is started without a
/// console window and stopped if it does not answer in time.
pub fn program_version(exe: &Path) -> Result<Version, String> {
    let mut command = Command::new(exe);
    command
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    // Started from the desktop app, a console program would open a terminal
    // window (see `cw-client`).
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("could not start {}: {e}", exe.display()))?;
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < VERSION_TIMEOUT => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "{} did not answer within {} seconds",
                    exe.display(),
                    VERSION_TIMEOUT.as_secs()
                ));
            }
            Err(e) => return Err(format!("could not wait for {}: {e}", exe.display())),
        }
    };
    let mut output = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_string(&mut output);
    }
    if !status.success() {
        return Err(format!("{} --version failed ({status})", exe.display()));
    }
    parse_version_line(&output)
        .ok_or_else(|| format!("{} printed no version: {output:?}", exe.display()))
}

/// The version in clap's `--version` line, `codewiki 0.2.0`.
fn parse_version_line(text: &str) -> Option<Version> {
    Version::parse(text.split_whitespace().nth(1)?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_clap_version_lines() {
        assert_eq!(
            parse_version_line("codewiki 0.2.0\n"),
            Some(Version::new(0, 2, 0))
        );
        assert_eq!(parse_version_line("codewiki\n"), None);
        assert_eq!(parse_version_line(""), None);
    }

    #[test]
    fn a_zip_without_the_programs_is_not_a_package() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("p.zip");
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
        writer
            .start_file("LICENSE", zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut writer, b"Apache-2.0").unwrap();
        writer.finish().unwrap();

        let to = dir.path().join("stage");
        let error = stage(&zip_path, &to, &Version::new(0, 2, 0)).unwrap_err();
        assert!(error.contains("codewiki.exe"), "{error}");
        assert!(to.join("LICENSE").is_file());
    }

    #[test]
    fn a_program_that_does_not_exist_has_no_version() {
        let dir = tempfile::tempdir().unwrap();
        assert!(program_version(&dir.path().join("missing.exe")).is_err());
    }
}
