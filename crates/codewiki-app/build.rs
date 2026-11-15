//! Embeds the app's icon in the Windows program.

fn main() {
    println!("cargo:rerun-if-changed=resources/codewiki.rc");
    println!("cargo:rerun-if-changed=resources/codewiki.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile(
            "resources/codewiki.rc",
            embed_resource::ParamsIncludeDirs(["resources"]),
        )
        .manifest_optional()
        .expect("the icon resource compiles");
    }
}
