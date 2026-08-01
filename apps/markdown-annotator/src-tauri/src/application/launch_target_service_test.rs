use std::fs;

use super::*;
static TEMP_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn temporary_directory() -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "ma-launch-target-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        TEMP_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::create_dir_all(&directory).unwrap();
    directory
}

#[test]
fn resolves_directory_and_cwd_to_same_root() {
    let directory = temporary_directory();
    let explicit = LaunchTargetService::resolve(Some(&directory), &directory).unwrap();
    let cwd = LaunchTargetService::resolve(None, &directory).unwrap();
    assert_eq!(explicit.root.root_id, cwd.root.root_id);
    assert_eq!(explicit.selected_document, None);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn resolves_markdown_file_to_parent_root_and_selection() {
    let directory = temporary_directory();
    let file = directory.join("review.markdown");
    fs::write(&file, "# Review").unwrap();
    let target = LaunchTargetService::resolve(Some(&file), &directory).unwrap();
    assert_eq!(
        target.root.canonical_path,
        directory.canonicalize().unwrap()
    );
    assert_eq!(target.selected_document.as_deref(), Some("review.markdown"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn rejects_non_markdown_file() {
    let directory = temporary_directory();
    let file = directory.join("review.txt");
    fs::write(&file, "text").unwrap();
    assert_eq!(
        LaunchTargetService::resolve(Some(&file), &directory)
            .unwrap_err()
            .code,
        FileBrowserErrorCode::UnsupportedExtension
    );
    fs::remove_dir_all(directory).unwrap();
}
