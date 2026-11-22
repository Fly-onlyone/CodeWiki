//! `codewiki agents`: keep coding agents out of chosen folders.

use std::path::Path;
use std::process::{Command, Output};

/// Run the binary in `dir` against its own data folder `data`.
fn codewiki(dir: &Path, data: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_codewiki"))
        .current_dir(dir)
        .args(args)
        .env("CODEWIKI_DATA_DIR", data)
        .output()
        .expect("the codewiki binary runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn off_lists_a_folder_and_status_sees_it() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let folder = dir.path().to_str().unwrap();

    let off = codewiki(dir.path(), data.path(), &["agents", "off"]);
    assert_eq!(off.status.code(), Some(0), "{}", stderr(&off));
    let text = stdout(&off);
    assert!(
        text.starts_with("kept from coding agents: ") && text.trim_end().len() > 25,
        "{text}"
    );

    let status = codewiki(dir.path(), data.path(), &["agents", "status"]);
    assert_eq!(status.status.code(), Some(0));
    assert!(
        stdout(&status).starts_with("kept from coding agents by "),
        "{}",
        stdout(&status)
    );

    let list = codewiki(dir.path(), data.path(), &["agents", "list"]);
    assert_eq!(stdout(&list).lines().count(), 1, "{}", stdout(&list));

    let json = codewiki(dir.path(), data.path(), &["--json", "agents", "list"]);
    let value: serde_json::Value = serde_json::from_str(&stdout(&json)).unwrap();
    assert_eq!(value["folders"].as_array().unwrap().len(), 1);
    let json = codewiki(dir.path(), data.path(), &["--json", "agents", "status"]);
    let value: serde_json::Value = serde_json::from_str(&stdout(&json)).unwrap();
    assert!(value["covered_by"].is_string());

    let other = tempfile::tempdir().unwrap();
    let open = codewiki(other.path(), data.path(), &["agents", "status"]);
    assert_eq!(stdout(&open).trim_end(), "open to coding agents");
    let open = codewiki(
        other.path(),
        data.path(),
        &["--json", "agents", "status", folder],
    );
    assert!(stdout(&open).contains("covered_by"));
    let open = codewiki(other.path(), data.path(), &["--json", "agents", "status"]);
    let value: serde_json::Value = serde_json::from_str(&stdout(&open)).unwrap();
    assert!(value["covered_by"].is_null());
}

#[test]
fn off_twice_names_the_entry() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    codewiki(dir.path(), data.path(), &["agents", "off"]);
    let again = codewiki(dir.path(), data.path(), &["agents", "off"]);
    assert_eq!(again.status.code(), Some(0));
    assert!(
        stdout(&again).starts_with("already kept from coding agents by "),
        "{}",
        stdout(&again)
    );
}

#[test]
fn on_removes_it() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    codewiki(dir.path(), data.path(), &["agents", "off"]);
    let on = codewiki(dir.path(), data.path(), &["agents", "on"]);
    assert_eq!(on.status.code(), Some(0));
    assert!(
        stdout(&on).starts_with("coding agents allowed again: "),
        "{}",
        stdout(&on)
    );
    let list = codewiki(dir.path(), data.path(), &["agents", "list"]);
    assert_eq!(
        stdout(&list).trim_end(),
        "no folder is kept from coding agents"
    );
    let on = codewiki(dir.path(), data.path(), &["agents", "on"]);
    assert_eq!(on.status.code(), Some(0));
    assert!(stdout(&on).starts_with("not listed: "), "{}", stdout(&on));
}

#[test]
fn on_a_child_of_a_parent_entry_exits_1() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let child = dir.path().join("child");
    std::fs::create_dir(&child).unwrap();
    codewiki(dir.path(), data.path(), &["agents", "off"]);
    let on = codewiki(&child, data.path(), &["agents", "on"]);
    assert_eq!(on.status.code(), Some(1));
    let text = stdout(&on);
    assert!(
        text.starts_with("kept from coding agents by ")
            && text
                .trim_end()
                .ends_with("; remove that entry to let agents in"),
        "{text}"
    );
}

#[test]
fn list_is_empty_at_first() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let list = codewiki(dir.path(), data.path(), &["agents", "list"]);
    assert_eq!(list.status.code(), Some(0));
    assert_eq!(
        stdout(&list).trim_end(),
        "no folder is kept from coding agents"
    );
}

#[test]
fn a_broken_list_is_reported_and_left_alone() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let file = data.path().join("agents-off.json");
    std::fs::write(&file, "not json").unwrap();
    for args in [
        &["agents", "off"][..],
        &["agents", "on"],
        &["agents", "list"],
        &["agents", "status"],
    ] {
        let output = codewiki(dir.path(), data.path(), args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        let text = stderr(&output);
        assert!(
            text.contains(&cw_core::agents_off::unreadable(&file)),
            "{text}"
        );
    }
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "not json");
}
