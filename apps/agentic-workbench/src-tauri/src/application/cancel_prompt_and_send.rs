use crate::{
    application::agent_run_errors::SendPromptError,
    ports::{
        event_sink::RunEventSink, session_handle::SessionHandle, session_registry::SessionRegistry,
    },
};

pub struct CancelPromptAndSendUseCase<R>
where
    R: SessionRegistry,
    R::Session: SessionHandle,
{
    registry: R,
}

impl<R> CancelPromptAndSendUseCase<R>
where
    R: SessionRegistry,
    R::Session: SessionHandle,
{
    pub fn new(registry: R) -> Self {
        Self { registry }
    }

    pub async fn execute<S>(
        self,
        sink: S,
        run_id: String,
        prompt: String,
    ) -> Result<(), SendPromptError>
    where
        S: RunEventSink,
    {
        let trimmed = prompt.trim().to_string();
        if trimmed.is_empty() {
            return Err(SendPromptError::EmptyPrompt);
        }

        let session = self
            .registry
            .active_session(&run_id)
            .await
            .ok_or(SendPromptError::RunNotActive)?;

        session
            .cancel_current_prompt_and_send(sink, trimmed)
            .await
            .map(|_| ())
            .map_err(|err| SendPromptError::DispatchFailed(err.to_string()))
    }
}
