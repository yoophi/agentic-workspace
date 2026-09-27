//! run 이벤트 발행 포트(039). AW의 run sink가 의존하는 좁은 인터페이스이고 `WorkbenchRuntime`이 구현한다.

use acp_agent_core::domain::events::RunEvent;
use workbench_protocol::EventEnvelope;

pub trait RunEventPublisher: Send + Sync {
    /// 순번을 부여해 보관·구독자 전달을 하고 `deliver(&envelope)`를 부른다. `deliver`는 **같은 스트림 lock 안에서**
    /// 불린다(research R6): 서로 다른 sink가 같은 run에 동시에 발행해도 전달 순서 = 순번 순서. 막히지 않아야 하고
    /// hub를 다시 호출하면 안 된다. 보관 한도로 제거된 run이면 버리고 `None`.
    fn publish_run(
        &self,
        run_id: &str,
        event: &RunEvent,
        terminal: bool,
        deliver: &mut dyn FnMut(&EventEnvelope),
    ) -> Option<EventEnvelope>;
}
