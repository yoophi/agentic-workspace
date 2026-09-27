//! 서버 제어 상태(044): 작업 관문, 임대 표, 조립이 넣는 서버 host port. `server.*`·`lease.*`·`desktop.*`·`bench.list`
//! handler가 이것을 쓴다.

use std::sync::{Arc, OnceLock};

use crate::{
    application::{bench_service::BenchServices, lease::LeaseTable, work_gate::WorkGate},
    ports::server_host::ServerHost,
};

pub struct ServerControl {
    work_gate: Arc<WorkGate>,
    leases: LeaseTable,
    host: OnceLock<Arc<dyn ServerHost>>,
    epoch: String,
    benches: Arc<BenchServices>,
}

impl ServerControl {
    pub fn new(work_gate: Arc<WorkGate>, epoch: String, benches: Arc<BenchServices>) -> Self {
        Self {
            work_gate,
            leases: LeaseTable::default(),
            host: OnceLock::new(),
            epoch,
            benches,
        }
    }

    pub fn work_gate(&self) -> &Arc<WorkGate> {
        &self.work_gate
    }

    pub fn leases(&self) -> &LeaseTable {
        &self.leases
    }

    pub fn epoch(&self) -> &str {
        &self.epoch
    }

    pub fn benches(&self) -> &Arc<BenchServices> {
        &self.benches
    }

    /// 조립이 한 번 넣는다. 두 번째는 무시하고 false.
    pub fn attach_host(&self, host: Arc<dyn ServerHost>) -> bool {
        self.host.set(host).is_ok()
    }

    pub fn host(&self) -> Option<&Arc<dyn ServerHost>> {
        self.host.get()
    }
}
