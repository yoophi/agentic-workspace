//! AW shell Adapter: lifecycle implementation is shared with other desktop HTTP consumers.
pub use workbench_desktop_client::{
    DesktopConnection as ExternalServer, DesktopConnectionConfig, WindowIdentity,
};

pub fn discover_server_executable() -> std::path::PathBuf {
    workbench_desktop_client::discover_server_executable(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../target"),
    )
}
