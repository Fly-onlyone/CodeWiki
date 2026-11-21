//! The built-in embedding engine is gone: no crate of it may be left in the lock file.

#[test]
fn the_workspace_carries_no_model_engine() {
    let lock =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.lock")).unwrap();
    for name in [
        "candle-core",
        "tokenizers",
        "nvml-wrapper",
        "cw-models",
        "cw-embed-engine",
    ] {
        assert!(
            !lock.contains(&format!("name = \"{name}\"")),
            "{name} is still in Cargo.lock"
        );
    }
}
