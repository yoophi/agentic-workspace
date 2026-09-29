//! One immutable operation and its explicit, epoch-bound attempts.
use std::fmt;

use workbench_protocol::{
    operations::spec_for, CallReply, CallRequest, OperationId, OperationKind, Outcome,
    WorkbenchFault, PROTOCOL_VERSION,
};

/// Identity only; possession does not prove an endpoint or confer authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointIdentity {
    instance: String,
    epoch: String,
}

impl EndpointIdentity {
    pub fn new(
        instance: impl Into<String>,
        epoch: impl Into<String>,
    ) -> Result<Self, AttemptError> {
        let value = Self {
            instance: instance.into(),
            epoch: epoch.into(),
        };
        if value.instance.is_empty() || value.epoch.is_empty() {
            return Err(AttemptError::InvalidIdentity);
        }
        Ok(value)
    }
    pub fn instance(&self) -> &str {
        &self.instance
    }
    pub fn epoch(&self) -> &str {
        &self.epoch
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Generation(u64);

pub enum AttemptState {
    Prepared,
    Submitted,
    Unknown,
    Complete(CallReply),
    Fault(WorkbenchFault),
}

// Reply output, fault details and human messages can contain private inputs.
impl fmt::Debug for AttemptState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Prepared => "Prepared",
            Self::Submitted => "Submitted",
            Self::Unknown => "Unknown",
            Self::Complete(_) => "Complete([redacted])",
            Self::Fault(_) => "Fault([redacted])",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AttemptError {
    #[error("endpoint identity is incomplete")]
    InvalidIdentity,
    #[error("request protocol, operation or idempotency contract is invalid")]
    InvalidRequest,
    #[error("endpoint instance or epoch changed; original outcome remains unresolved")]
    EndpointChanged,
    #[error("attempt cannot transition from its current state")]
    InvalidState,
    #[error("completion belongs to an older attempt generation")]
    StaleGeneration,
    #[error("reply request identity does not match")]
    ReplyIdentity,
    #[error("attempt generation exhausted")]
    GenerationExhausted,
}

pub struct Attempt {
    request: CallRequest,
    endpoint: EndpointIdentity,
    generation: Generation,
    state: AttemptState,
}

impl fmt::Debug for Attempt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Attempt")
            .field("generation", &self.generation)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl Attempt {
    pub fn new(request: CallRequest, endpoint: EndpointIdentity) -> Result<Self, AttemptError> {
        let operation =
            OperationId::parse(&request.operation).ok_or(AttemptError::InvalidRequest)?;
        if request.protocol_version != PROTOCOL_VERSION
            || (spec_for(operation).kind == OperationKind::Command
                && request.idempotency_key.is_none())
        {
            return Err(AttemptError::InvalidRequest);
        }
        Ok(Self {
            request,
            endpoint,
            generation: Generation(0),
            state: AttemptState::Prepared,
        })
    }

    pub fn request(&self) -> &CallRequest {
        &self.request
    }
    pub fn endpoint(&self) -> &EndpointIdentity {
        &self.endpoint
    }
    pub fn generation(&self) -> Generation {
        self.generation
    }

    pub fn state(&self) -> &AttemptState {
        &self.state
    }

    /// Called only by an explicit submission/retry, never by a transport retry loop.
    pub fn begin(&mut self, endpoint: &EndpointIdentity) -> Result<Generation, AttemptError> {
        if endpoint != &self.endpoint {
            return Err(AttemptError::EndpointChanged);
        }
        let allowed = match &self.state {
            AttemptState::Prepared | AttemptState::Unknown => true,
            AttemptState::Fault(fault) => fault_allows_explicit_retry(fault),
            _ => false,
        };
        if !allowed {
            return Err(AttemptError::InvalidState);
        }
        self.generation = Generation(
            self.generation
                .0
                .checked_add(1)
                .ok_or(AttemptError::GenerationExhausted)?,
        );
        self.state = AttemptState::Submitted;
        Ok(self.generation)
    }

    fn check_completion(&self, generation: Generation) -> Result<(), AttemptError> {
        if generation != self.generation {
            return Err(AttemptError::StaleGeneration);
        }
        if !matches!(self.state, AttemptState::Submitted) {
            return Err(AttemptError::InvalidState);
        }
        Ok(())
    }

    pub fn mark_unknown(&mut self, generation: Generation) -> Result<(), AttemptError> {
        self.check_completion(generation)?;
        self.state = AttemptState::Unknown;
        Ok(())
    }

    /// HTTP envelope identity and shape validation must precede this transition.
    pub fn complete(
        &mut self,
        generation: Generation,
        reply: CallReply,
    ) -> Result<(), AttemptError> {
        self.check_completion(generation)?;
        self.state = AttemptState::Complete(reply);
        Ok(())
    }

    pub fn fail(
        &mut self,
        generation: Generation,
        fault: WorkbenchFault,
    ) -> Result<(), AttemptError> {
        self.check_completion(generation)?;
        if fault.request_id != self.request.request_id {
            return Err(AttemptError::ReplyIdentity);
        }
        self.state = AttemptState::Fault(fault);
        Ok(())
    }

    pub fn outcome(&self) -> Outcome {
        match &self.state {
            AttemptState::Prepared => Outcome::NotApplied,
            AttemptState::Complete(CallReply::Complete { .. }) => Outcome::Applied,
            AttemptState::Fault(fault) => fault.outcome,
            // Acceptance acknowledges an execution, not its final effect.
            _ => Outcome::Unknown,
        }
    }
}

/// Shared explicit retry policy for in-memory and durable attempts.
pub fn fault_allows_explicit_retry(fault: &WorkbenchFault) -> bool {
    fault.retryable && fault.outcome != Outcome::Applied
}
