use super::*;
use crate::ports::native_shell::NativeShell;
#[test]
fn validates_canonical_path_and_rejects_traversal() {
    let root = std::env::temp_dir().join(format!("ma-shell-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("a.md"), "a").unwrap();
    assert!(
        MacOsNativeShell
            .validated_display_path(&root, "a.md")
            .unwrap()
            .ends_with("a.md")
    );
    assert!(
        MacOsNativeShell
            .validated_display_path(&root, "../a.md")
            .is_err()
    );
    std::fs::remove_dir_all(root).unwrap();
}
