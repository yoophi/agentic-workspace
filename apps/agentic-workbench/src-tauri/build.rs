use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=../package.json");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-env-changed=COMMIT_SHA");
    println!("cargo:rerun-if-env-changed=GITHUB_REF_NAME");
    println!("cargo:rerun-if-env-changed=GITHUB_REF_TYPE");

    println!(
        "cargo:rustc-env=AGENTIC_WORKBENCH_PACKAGE_VERSION={}",
        package_json_version()
    );
    println!(
        "cargo:rustc-env=AGENTIC_WORKBENCH_GIT_COMMIT_HASH={}",
        build_commit_hash()
    );
    println!(
        "cargo:rustc-env=AGENTIC_WORKBENCH_GIT_COMMIT_TAG={}",
        build_commit_tag()
    );

    tauri_build::build();
}

fn package_json_version() -> String {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));
    let package_json_path = manifest_dir.join("../package.json");

    let Ok(package_json) = fs::read_to_string(package_json_path) else {
        return env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".into());
    };

    serde_json::from_str::<serde_json::Value>(&package_json)
        .ok()
        .and_then(|value| {
            value
                .get("version")
                .and_then(|version| version.as_str())
                .filter(|version| !version.trim().is_empty())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".into()))
}

fn build_commit_hash() -> String {
    env::var("GITHUB_SHA")
        .or_else(|_| env::var("COMMIT_SHA"))
        .ok()
        .map(|hash| hash.trim().to_owned())
        .filter(|hash| !hash.is_empty())
        .unwrap_or_else(git_commit_hash)
}

fn git_commit_hash() -> String {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));
    let repository_root = manifest_dir.join("../../..");

    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repository_root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|hash| hash.trim().to_owned())
        .filter(|hash| !hash.is_empty())
        .unwrap_or_else(|| "unknown".into())
}

fn build_commit_tag() -> String {
    github_tag_ref().unwrap_or_else(git_commit_tags)
}

fn github_tag_ref() -> Option<String> {
    let ref_type = env::var("GITHUB_REF_TYPE").ok()?;
    if ref_type.trim() != "tag" {
        return None;
    }

    env::var("GITHUB_REF_NAME")
        .ok()
        .map(|tag| tag.trim().to_owned())
        .filter(|tag| !tag.is_empty())
}

fn git_commit_tags() -> String {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));
    let repository_root = manifest_dir.join("../../..");

    Command::new("git")
        .args(["tag", "--points-at", "HEAD"])
        .current_dir(repository_root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|tags| {
            tags.lines()
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|tags| !tags.is_empty())
        .unwrap_or_else(|| "unknown".into())
}
