//! `codewiki model` as a person runs it. Nothing here downloads a model.
#![cfg(windows)]

use std::path::Path;
use std::process::{Command, Output};

fn codewiki(data: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_codewiki"))
        .args(args)
        // Its own data folder, and nothing of the machine's settings.
        .env("CODEWIKI_DATA_DIR", data)
        .env_remove("CODEWIKI_EMBED")
        .env_remove("CODEWIKI_EMBED_MODEL")
        .env_remove("CODEWIKI_MODELS_URL")
        .output()
        .expect("the codewiki binary runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Put a model of the catalog under `data` as an install leaves it, with
/// empty files of the right sizes.
fn install_by_hand(data: &Path, id: &str) {
    let entry = cw_models::catalog::find(id).unwrap();
    let dir = cw_models::Store::new(data, None).dir(entry);
    std::fs::create_dir_all(&dir).unwrap();
    for file in &entry.files {
        std::fs::File::create(dir.join(&file.path))
            .unwrap()
            .set_len(file.size)
            .unwrap();
    }
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::json!({ "revision": entry.revision }).to_string(),
    )
    .unwrap();
}

#[test]
fn lists_the_catalog_and_what_is_installed() {
    let data = tempfile::tempdir().unwrap();
    let output = codewiki(data.path(), &["model", "list"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3, "{text}");
    assert!(lines[0].starts_with("granite-small-english-r2"), "{text}");
    assert!(lines[0].contains("94 MB") && lines[0].contains("Apache-2.0"));
    assert!(!text.contains("installed"), "{text}");

    install_by_hand(data.path(), "gte-modernbert-base");
    let output = codewiki(data.path(), &["model", "list", "--json"]);
    let models: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(models[0]["installed"], false);
    assert_eq!(models[1]["id"], "gte-modernbert-base");
    assert_eq!(models[1]["installed"], true);
    assert_eq!(models[1]["size"], 301_625_980u64);
}

#[test]
fn recommends_a_model_for_this_machine_with_its_reason() {
    let data = tempfile::tempdir().unwrap();
    let output = codewiki(data.path(), &["model", "recommend", "--json"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let answer: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert!(answer["hardware"]["ram_total"].as_u64().unwrap() > 0);
    let recommended = &answer["recommended"];
    assert!(!recommended["reason"].as_str().unwrap().is_empty());
    // Whatever the machine, what is recommended is in the catalog.
    if let Some(id) = recommended["model"].as_str() {
        assert!(cw_models::catalog::find(id).is_some(), "{id}");
    }
    // As a sentence too.
    let output = codewiki(data.path(), &["model", "recommend"]);
    assert!(output.status.success());
    assert!(stdout(&output).ends_with(".\n"), "{}", stdout(&output));
}

#[test]
fn refuses_a_model_the_catalog_does_not_have() {
    let data = tempfile::tempdir().unwrap();
    for action in ["install", "remove"] {
        let output = codewiki(data.path(), &["model", action, "bert"]);
        assert!(!output.status.success());
        let error = stderr(&output);
        assert!(
            error.contains("unknown model 'bert'") && error.contains("gte-modernbert-base"),
            "{error}"
        );
    }
    assert!(!data.path().join("models").exists());
}

#[test]
fn removes_an_installed_model_and_says_when_it_was_in_use() {
    let data = tempfile::tempdir().unwrap();
    let id = "granite-small-english-r2";
    install_by_hand(data.path(), id);
    // An install of what is there downloads nothing.
    let output = codewiki(data.path(), &["model", "install", id]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("is installed already"));

    let output = Command::new(env!("CARGO_BIN_EXE_codewiki"))
        .args(["model", "remove", id])
        .env("CODEWIKI_DATA_DIR", data.path())
        .env("CODEWIKI_EMBED", "builtin")
        .env("CODEWIKI_EMBED_MODEL", id)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("Removed Granite Embedding Small English R2."));
    assert!(
        stderr(&output).contains("the settings name this model"),
        "{}",
        stderr(&output)
    );
    assert!(!data.path().join("models").join(id).exists());
    // Removing what is not there is not an error, and warns of nothing.
    let output = codewiki(data.path(), &["model", "remove", id]);
    assert!(output.status.success());
    assert!(stderr(&output).is_empty(), "{}", stderr(&output));
}
