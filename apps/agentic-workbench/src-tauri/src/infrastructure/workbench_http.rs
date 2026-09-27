//! 042: Workbench HTTP/WS 어댑터(`workbench-server`) 조립. 허용 출처는 AW 창을 띄우는 WebView 출처뿐이다 —
//! 개발(`devUrl`), macOS·Linux 배포(`tauri://localhost`), Windows 배포(`http://tauri.localhost`).

use workbench_server::origin::OriginPolicy;

pub const WEBVIEW_ORIGINS: [&str; 3] = [
    "http://localhost:1420",
    "tauri://localhost",
    "http://tauri.localhost",
];

pub fn origin_policy() -> OriginPolicy {
    OriginPolicy::new(WEBVIEW_ORIGINS)
}
