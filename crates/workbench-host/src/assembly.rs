//! 데스크톱과 무관한 Workbench 조립(044 T014, research R1). 런타임 → MCP 서버 → HTTP/WS 어댑터 순서로 띄운다. MCP
//! 연결 주입(`McpLaunchDecorator`)은 창과 무관하게 모든 run 시작에 붙는다(R2). 독립 서버와 AW embedded 모드가 이 조립을
//! 함께 쓴다. 데스크톱 표현(창 삽입 전달)은 호출자가 `RuntimeAdapters::desktop`으로 넣는다(없으면 전달 없음).

use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use workbench_core::{
    application::workbench_runtime::{RuntimeAdapters, WorkbenchRuntime},
    infrastructure::data_paths::DataPaths,
    ports::desktop_bridge::{DesktopBridge, DesktopDelivery},
};
use workbench_protocol::Workbench;

use crate::{
    http::{AwServerInfo, HttpAssembly, WorkbenchHttpState, drain_for_exit},
    launch::McpLaunchDecorator,
    lifecycle::identity::OwnerIdentity,
    mcp::McpServerState,
};

/// HTTP 어댑터 기동 방식. AW debug 빌드는 기동 실패를 주입해 호환 부팅을 확인한다(043 T052).
#[derive(Debug, Clone)]
pub enum HttpStart {
    Start,
    Fail(String),
}

pub struct HostOptions {
    pub data_dir: PathBuf,
    pub adapters: RuntimeAdapters,
    pub server_version: String,
    /// MCP·HTTP 서버를 띄울 tokio 런타임(AW는 Tauri의 런타임 핸들을 넘긴다).
    pub spawner: tokio::runtime::Handle,
    pub drain_warn_after: Duration,
    pub http: HttpStart,
    /// 독립 서버의 소유자 신원(044). embedded 모드는 T028에서 넣는다.
    pub owner: Option<OwnerIdentity>,
}

impl HostOptions {
    pub fn new(
        data_dir: PathBuf,
        adapters: RuntimeAdapters,
        server_version: impl Into<String>,
        spawner: tokio::runtime::Handle,
    ) -> Self {
        Self {
            data_dir,
            adapters,
            server_version: server_version.into(),
            spawner,
            drain_warn_after: crate::http::default_drain_warn_after(),
            http: HttpStart::Start,
            owner: None,
        }
    }
}

/// 조립 결과. HTTP 어댑터 기동이 실패해도 런타임·MCP는 쓸 수 있다(042 FR-016). 이유는 `http_start_error`에 남는다.
pub struct HostAssembly {
    pub runtime: Arc<WorkbenchRuntime>,
    pub mcp: McpServerState,
    pub http: Option<Arc<WorkbenchHttpState>>,
    pub http_start_error: Option<String>,
}

pub fn assemble(options: HostOptions) -> Result<HostAssembly> {
    let HostOptions {
        data_dir,
        adapters,
        server_version,
        spawner,
        drain_warn_after,
        http,
        owner,
    } = options;
    let (runtime, mcp) = assemble_core(data_dir, adapters, &spawner)?;
    let instance_id = owner.as_ref().map(|owner| owner.instance_id().to_owned());
    let started = match http {
        HttpStart::Fail(reason) => Err(anyhow::anyhow!(reason)),
        HttpStart::Start => WorkbenchHttpState::start(
            HttpAssembly {
                workbench: runtime.clone() as Arc<dyn Workbench>,
                mcp_registry: mcp.capability_registry(),
                server_info: AwServerInfo {
                    version: server_version,
                    epoch: runtime.epoch().to_owned(),
                },
                drain_warn_after,
                owner,
            },
            &spawner,
        ),
    };
    let (http, http_start_error) = match started {
        Ok(state) => (Some(Arc::new(state)), None),
        Err(error) => (None, Some(format!("{error:#}"))),
    };
    // 044 T026: `desktop.*`·`server.status`가 이 어댑터의 발급기·이벤트 표·호출 수를 쓴다.
    if let Some(state) = &http {
        runtime.attach_server_host(state.server_host(instance_id, mcp.detached_calls()));
    }
    Ok(HostAssembly {
        runtime,
        mcp,
        http,
        http_start_error,
    })
}

/// 런타임과 MCP 서버(창과 무관한 MCP 주입 포함)만 띄운다. HTTP 설정을 따로 쓰는 시험 host가 이것을 쓴다.
pub fn assemble_core(
    data_dir: PathBuf,
    mut adapters: RuntimeAdapters,
    spawner: &tokio::runtime::Handle,
) -> Result<(Arc<WorkbenchRuntime>, McpServerState)> {
    let decorator = McpLaunchDecorator::new();
    adapters.launch_decorator = Some(decorator.clone());
    let runtime = WorkbenchRuntime::bootstrap_with(DataPaths::new(data_dir), adapters)
        .map_err(|error| anyhow::anyhow!(error.to_string()))
        .context("failed to bootstrap the Workbench runtime")?;
    let mcp = McpServerState::start(runtime.clone(), spawner)?;
    decorator.bind(mcp.clone());
    Ok((runtime, mcp))
}

impl HostAssembly {
    /// 종료(042 T033 순서): 수락 닫기 → 열린 작업대 닫기(run 취소·권한 대기 해제) → HTTP drain → MCP drain.
    pub async fn shutdown(&self) {
        let runtime = self.runtime.clone();
        drain_for_exit(self.http.clone(), self.mcp.detached_calls(), async move {
            runtime.close_all_benches().await;
        })
        .await;
    }
}

/// 창 삽입 전달이 없는 데스크톱 포트(외부 서버 모드·독립 서버). 서버는 창을 모른다(R3).
pub struct NoopDesktopBridge;

impl DesktopBridge for NoopDesktopBridge {
    fn deliver(&self, _delivery: DesktopDelivery) {}
}
