//! `openapi/workbench.openapi.json`을 stdout으로 내보낸다. 결정적 출력(pretty JSON + 끝 개행).
//! 사용: `cargo run -q -p workbench-protocol --bin export_openapi > crates/workbench-protocol/openapi/workbench.openapi.json`

fn main() {
    print!("{}", workbench_protocol::openapi::render_openapi());
}
