#[path = "support/echo.rs"]
mod echo;

use local_mcp_server::{HostConfig, start};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let token = std::env::var("LOCAL_MCP_TOKEN")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| std::io::Error::other("set LOCAL_MCP_TOKEN before running the example"))?;
    let host = start(
        || Ok(echo::Echo("echo")),
        move |candidate| (candidate == token).then(|| "example-session".to_owned()),
        HostConfig::default(),
    )
    .await?;
    println!("MCP endpoint: {}", host.endpoint());
    tokio::signal::ctrl_c().await?;
    host.stop_accepting();
    // A writing application drains its accepted mutations here.
    host.shutdown().await
}
