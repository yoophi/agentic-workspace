//! 기록형 데스크톱 포트(040). 작업대 단위 전달·run 종료 후처리·`run.start` 보강 호출을 기록한다.

use std::sync::Mutex;

use acp_agent_core::domain::run::AgentRunRequest;
use workbench_core::ports::desktop_bridge::{
    DesktopBridge, DesktopDelivery, LaunchContext, RunLaunchDecorator, RunTerminalHook,
};

#[derive(Default)]
pub struct RecordingDesktop {
    pub deliveries: Mutex<Vec<DesktopDelivery>>,
    pub terminals: Mutex<Vec<String>>,
    pub launches: Mutex<Vec<LaunchContext>>,
}

impl RecordingDesktop {
    pub fn deliveries_for(&self, bench_id: &str) -> Vec<DesktopDelivery> {
        self.deliveries
            .lock()
            .unwrap()
            .iter()
            .filter(|delivery| match delivery {
                DesktopDelivery::Run { bench_id: id, .. }
                | DesktopDelivery::ExchangeRequested { bench_id: id, .. }
                | DesktopDelivery::ExchangeStatus { bench_id: id, .. }
                | DesktopDelivery::TitleRequested { bench_id: id, .. } => id == bench_id,
            })
            .cloned()
            .collect()
    }
}

impl DesktopBridge for RecordingDesktop {
    fn deliver(&self, delivery: DesktopDelivery) {
        self.deliveries.lock().unwrap().push(delivery);
    }
}

impl RunTerminalHook for RecordingDesktop {
    fn on_terminal(&self, run_id: &str) {
        self.terminals.lock().unwrap().push(run_id.to_owned());
    }
}

impl RunLaunchDecorator for RecordingDesktop {
    fn decorate(
        &self,
        _request: &mut AgentRunRequest,
        context: &LaunchContext,
    ) -> Result<(), String> {
        self.launches.lock().unwrap().push(context.clone());
        Ok(())
    }
}
