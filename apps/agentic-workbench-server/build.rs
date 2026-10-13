fn main() {
    println!("cargo:rerun-if-env-changed=AGENTIC_WORKBENCH_RELEASE_VERSION");
    let version = std::env::var("AGENTIC_WORKBENCH_RELEASE_VERSION")
        .ok()
        .filter(|version| !version.trim().is_empty())
        .unwrap_or_else(|| std::env::var("CARGO_PKG_VERSION").unwrap());
    println!("cargo:rustc-env=AGENTIC_WORKBENCH_SERVER_VERSION={version}");
}
