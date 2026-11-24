//! `codewiki hook session-start` as an agent host runs it: it briefs, and it
//! never fails.

use std::path::Path;
use std::process::{Command, Output};

/// Run the binary in `dir` against its own data folder `data`, so no test
/// reads the developer's real settings or list of closed folders.
fn codewiki(dir: &Path, data: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_codewiki"))
        .current_dir(dir)
        .args(args)
        .env("CODEWIKI_DATA_DIR", data)
        // The host's project folder must not leak into the test.
        .env_remove("CLAUDE_PROJECT_DIR")
        .output()
        .expect("the codewiki binary runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn briefs_an_agent_in_a_repository_without_a_wiki() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let output = codewiki(dir.path(), data.path(), &["hook", "session-start"]);
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("no wiki has been generated"), "{text}");
    assert!(text.contains("`get_overview`") && text.contains("`get_context`"));
    assert!(
        !dir.path().join("obsidian").exists(),
        "a hook writes nothing"
    );
}

#[test]
fn briefs_from_a_subfolder_of_a_repository_with_a_wiki() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\n",
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "//! Demo crate.\npub fn run() {}\n",
    )
    .unwrap();
    let data = tempfile::tempdir().unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codewiki"))
        .current_dir(dir.path())
        .arg("init")
        .env("CODEWIKI_DATA_DIR", data.path())
        // No model and no embedding service: the structural wiki is enough.
        .env_remove("CODEWIKI_LLM")
        .env_remove("CODEWIKI_EMBED")
        .output()
        .unwrap();
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );

    let output = codewiki(
        &dir.path().join("src"),
        data.path(),
        &["hook", "session-start"],
    );
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(
        text.contains("Its home note is `obsidian/_HOME.md`."),
        "{text}"
    );
    assert!(text.contains("Pending proposals: 0."), "{text}");
}

#[test]
fn never_fails_and_prints_nothing_on_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    for args in [
        &["--repo", "does/not/exist", "hook", "session-start"][..],
        &["hook", "no-such-event"],
        &["hook"],
        &["hook", "session-start", "--unknown-flag"],
        &["hook", "session-start", "--agent", "bogus"],
    ] {
        let output = codewiki(dir.path(), data.path(), args);
        assert!(output.status.success(), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}: {}", stdout(&output));
    }
    // A broken manifest is an error like any other.
    std::fs::create_dir_all(dir.path().join("obsidian/.codewiki")).unwrap();
    std::fs::write(
        dir.path().join("obsidian/.codewiki/manifest.json"),
        "{ nope",
    )
    .unwrap();
    let output = codewiki(dir.path(), data.path(), &["hook", "session-start"]);
    assert!(output.status.success());
    assert!(output.stdout.is_empty(), "{}", stdout(&output));
}

/// A project with a wiki in `src`'s parent, as the briefing test above makes
/// one; `data` is the data folder the run uses.
fn project_with_wiki(data: &Path) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\n",
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "//! Demo crate.\npub fn run() {}\n",
    )
    .unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codewiki"))
        .current_dir(dir.path())
        .arg("init")
        .env("CODEWIKI_DATA_DIR", data)
        .env_remove("CODEWIKI_LLM")
        .env_remove("CODEWIKI_EMBED")
        .output()
        .unwrap();
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    dir
}

/// Write the list of folders closed to coding agents into `data`.
fn close_folders(data: &Path, folders: &[&Path]) {
    let folders: Vec<String> = folders
        .iter()
        .map(|folder| folder.canonicalize().unwrap().display().to_string())
        .collect();
    let document = serde_json::json!({ "version": 1, "folders": folders });
    std::fs::write(data.join("agents-off.json"), document.to_string()).unwrap();
}

#[test]
fn a_listed_project_gets_no_briefing() {
    let data = tempfile::tempdir().unwrap();
    let project = project_with_wiki(data.path());
    close_folders(data.path(), &[project.path()]);
    let output = codewiki(project.path(), data.path(), &["hook", "session-start"]);
    assert!(output.status.success());
    assert!(output.stdout.is_empty(), "{}", stdout(&output));
}

#[test]
fn a_listed_parent_silences_a_subfolder() {
    let data = tempfile::tempdir().unwrap();
    let project = project_with_wiki(data.path());
    close_folders(data.path(), &[project.path()]);
    let output = codewiki(
        &project.path().join("src"),
        data.path(),
        &["hook", "session-start"],
    );
    assert!(output.status.success());
    assert!(output.stdout.is_empty(), "{}", stdout(&output));
}

#[test]
fn a_listed_vault_root_silences_the_hook() {
    let data = tempfile::tempdir().unwrap();
    let project = project_with_wiki(data.path());
    std::fs::create_dir(project.path().join("src/inner")).unwrap();
    close_folders(data.path(), &[project.path()]);
    let output = codewiki(
        &project.path().join("src/inner"),
        data.path(),
        &["hook", "session-start"],
    );
    assert!(output.status.success());
    assert!(output.stdout.is_empty(), "{}", stdout(&output));
}

#[test]
fn a_project_that_holds_a_listed_folder_gets_no_briefing() {
    let data = tempfile::tempdir().unwrap();
    let project = project_with_wiki(data.path());
    close_folders(data.path(), &[&project.path().join("src")]);
    let output = codewiki(project.path(), data.path(), &["hook", "session-start"]);
    assert!(output.status.success());
    assert!(output.stdout.is_empty(), "{}", stdout(&output));
}

#[test]
fn a_vault_root_that_holds_a_listed_folder_silences_the_hook() {
    let data = tempfile::tempdir().unwrap();
    let project = project_with_wiki(data.path());
    // The session starts in `docs`; the vault root above it holds `src`.
    std::fs::create_dir(project.path().join("docs")).unwrap();
    close_folders(data.path(), &[&project.path().join("src")]);
    let output = codewiki(
        &project.path().join("docs"),
        data.path(),
        &["hook", "session-start"],
    );
    assert!(output.status.success());
    assert!(output.stdout.is_empty(), "{}", stdout(&output));
}

#[test]
fn a_broken_list_silences_the_hook() {
    let data = tempfile::tempdir().unwrap();
    let project = project_with_wiki(data.path());
    std::fs::write(data.path().join("agents-off.json"), "{").unwrap();
    let output = codewiki(project.path(), data.path(), &["hook", "session-start"]);
    assert!(output.status.success());
    assert!(output.stdout.is_empty(), "{}", stdout(&output));
}

#[test]
fn an_unlisted_project_still_gets_its_briefing() {
    let data = tempfile::tempdir().unwrap();
    let project = project_with_wiki(data.path());
    let other = tempfile::tempdir().unwrap();
    close_folders(data.path(), &[other.path()]);
    let output = codewiki(project.path(), data.path(), &["hook", "session-start"]);
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("Pending proposals: 0."), "{text}");
}

const FACTS: &str = "write notes when the user asks";

/// Run the hook with `stdin` piped in, then closed.
fn hook_with_stdin(dir: &Path, data: &Path, args: &[&str], stdin: &str) -> Output {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new(env!("CARGO_BIN_EXE_codewiki"))
        .current_dir(dir)
        .args(args)
        .env("CODEWIKI_DATA_DIR", data)
        .env_remove("CLAUDE_PROJECT_DIR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the codewiki binary runs");
    // The hook may exit before it reads; a broken pipe is not a failure.
    let _ = child.stdin.take().unwrap().write_all(stdin.as_bytes());
    child.wait_with_output().unwrap()
}

fn start(source: &str) -> String {
    format!(r#"{{"source":"{source}","permission_mode":"default"}}"#)
}

#[test]
fn a_fresh_claude_code_session_is_ordered_to_launch_the_agent() {
    let data = tempfile::tempdir().unwrap();
    let project = project_with_wiki(data.path());
    for source in ["startup", "resume", "clear"] {
        let output = hook_with_stdin(
            project.path(),
            data.path(),
            &["hook", "session-start"],
            &start(source),
        );
        assert!(output.status.success());
        let text = stdout(&output);
        assert!(text.contains("`codewiki:note-writer`"), "{source}: {text}");
        assert!(!text.contains(FACTS), "{text}");
    }
}

#[test]
fn a_fresh_codex_session_is_ordered_to_use_the_skill() {
    let data = tempfile::tempdir().unwrap();
    let project = project_with_wiki(data.path());
    let output = hook_with_stdin(
        project.path(),
        data.path(),
        &["hook", "session-start", "--agent", "codex"],
        &start("startup"),
    );
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("with the `write-notes` skill"), "{text}");
    assert!(!text.contains("note-writer"), "{text}");
}

#[test]
fn other_sessions_get_the_facts() {
    let data = tempfile::tempdir().unwrap();
    let project = project_with_wiki(data.path());
    let cases = [
        start("compact"),
        start("fork"),
        r#"{"source":"startup","permission_mode":"plan"}"#.to_string(),
        "{ not json".to_string(),
        String::new(),
    ];
    for stdin in cases {
        let output = hook_with_stdin(
            project.path(),
            data.path(),
            &["hook", "session-start"],
            &stdin,
        );
        assert!(output.status.success(), "{stdin}");
        let text = stdout(&output);
        assert!(text.contains(FACTS), "{stdin}: {text}");
    }
}

#[test]
fn the_switch_off_gives_the_facts() {
    let data = tempfile::tempdir().unwrap();
    let project = project_with_wiki(data.path());
    std::fs::write(data.path().join("auto-explain.txt"), "off").unwrap();
    let output = hook_with_stdin(
        project.path(),
        data.path(),
        &["hook", "session-start"],
        &start("startup"),
    );
    let text = stdout(&output);
    assert!(
        text.contains(FACTS) && !text.contains("note-writer"),
        "{text}"
    );
}

#[test]
fn a_linked_worktree_gets_the_facts() {
    let data = tempfile::tempdir().unwrap();
    let project = project_with_wiki(data.path());
    let git = |dir: &Path, args: &[&str]| {
        let output = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["-c", "user.name=Ann", "-c", "user.email=dev@example.com"])
            .args(["-c", "commit.gpgsign=false", "-c", "core.autocrlf=false"])
            .args(args)
            .output()
            .expect("git is installed");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(project.path(), &["init", "-q", "-b", "main"]);
    git(project.path(), &["add", "-A", "-f"]);
    git(project.path(), &["commit", "-q", "-m", "first"]);
    // The main checkout is ordered; its linked worktree is not.
    let main = hook_with_stdin(
        project.path(),
        data.path(),
        &["hook", "session-start"],
        &start("startup"),
    );
    assert!(stdout(&main).contains("note-writer"), "{}", stdout(&main));
    let holder = tempfile::tempdir().unwrap();
    let linked = holder.path().join("linked");
    git(
        project.path(),
        &[
            "worktree",
            "add",
            "-q",
            linked.to_str().unwrap(),
            "-b",
            "side",
        ],
    );
    let output = hook_with_stdin(
        &linked,
        data.path(),
        &["hook", "session-start"],
        &start("startup"),
    );
    let text = stdout(&output);
    assert!(
        text.contains(FACTS) && !text.contains("note-writer"),
        "{text}"
    );
}

#[test]
fn an_open_stdin_does_not_hold_the_session() {
    use std::process::Stdio;
    let data = tempfile::tempdir().unwrap();
    let project = project_with_wiki(data.path());
    let mut child = Command::new(env!("CARGO_BIN_EXE_codewiki"))
        .current_dir(project.path())
        .args(["hook", "session-start"])
        .env("CODEWIKI_DATA_DIR", data.path())
        .env_remove("CLAUDE_PROJECT_DIR")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    // Keep the pipe: the host never closes it.
    let _stdin = child.stdin.take();
    let started = std::time::Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success());
            return;
        }
        if started.elapsed() > std::time::Duration::from_secs(3) {
            let _ = child.kill();
            panic!("the hook waited for stdin");
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
