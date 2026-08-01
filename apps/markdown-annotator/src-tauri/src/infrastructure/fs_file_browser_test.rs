use super::*;
static TEMP_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn temp_root() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ma-fs-browser-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        TEMP_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn scans_markdown_ancestors_and_exact_exclusions() {
    let root_path = temp_root();
    fs::create_dir_all(root_path.join("a")).unwrap();
    fs::create_dir_all(root_path.join("b/b1")).unwrap();
    fs::create_dir_all(root_path.join("c")).unwrap();
    fs::create_dir_all(root_path.join("node_modules/pkg")).unwrap();
    fs::write(root_path.join("a/file.md"), "a").unwrap();
    fs::write(root_path.join("b/b1/file2.markdown"), "b").unwrap();
    fs::write(root_path.join("c/file.txt"), "c").unwrap();
    fs::write(root_path.join("node_modules/pkg/hidden.md"), "hidden").unwrap();
    let browser = FsFileBrowser;
    let root = browser.canonical_root(&root_path).unwrap();
    let result = browser.scan_root(&root, &["node_modules".into()]).unwrap();
    let paths = result
        .entries
        .iter()
        .map(|entry| entry.relative_path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        ["a", "a/file.md", "b", "b/b1", "b/b1/file2.markdown"]
    );
    fs::remove_dir_all(root_path).unwrap();
}

#[test]
fn reads_utf8_bom_and_rejects_unsafe_documents() {
    let root_path = temp_root();
    fs::write(root_path.join("readme.md"), b"\xef\xbb\xbf# Hello").unwrap();
    fs::write(root_path.join("bad.md"), [0xff, 0xfe]).unwrap();
    fs::write(root_path.join("note.txt"), "text").unwrap();
    let browser = FsFileBrowser;
    let root = browser.canonical_root(&root_path).unwrap();
    let result = browser.read_document(&root, "readme.md").unwrap();
    assert_eq!(result.markdown_text, "# Hello");
    assert_eq!(result.identity.fingerprint.len(), 64);
    assert_eq!(
        browser
            .read_document(&root, "../secret.md")
            .unwrap_err()
            .code,
        FileBrowserErrorCode::InvalidRelativePath
    );
    assert_eq!(
        browser.read_document(&root, "note.txt").unwrap_err().code,
        FileBrowserErrorCode::UnsupportedExtension
    );
    assert_eq!(
        browser.read_document(&root, "bad.md").unwrap_err().code,
        FileBrowserErrorCode::InvalidUtf8
    );
    fs::remove_dir_all(root_path).unwrap();
}

#[cfg(unix)]
#[test]
fn never_follows_directory_symlinks() {
    use std::os::unix::fs::symlink;
    let root_path = temp_root();
    let outside = temp_root();
    fs::write(outside.join("secret.md"), "secret").unwrap();
    symlink(&outside, root_path.join("linked")).unwrap();
    let browser = FsFileBrowser;
    let root = browser.canonical_root(&root_path).unwrap();
    assert!(browser.scan_root(&root, &[]).unwrap().entries.is_empty());
    fs::remove_dir_all(root_path).unwrap();
    fs::remove_dir_all(outside).unwrap();
}
