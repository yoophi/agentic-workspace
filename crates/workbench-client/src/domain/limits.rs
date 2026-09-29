//! Validated allocation and time budgets. Checks happen before retaining input.
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resource {
    Input,
    Body,
    RetryState,
    Frame,
    Message,
    QueueBytes,
    QueueItems,
}

impl Resource {
    pub const ALL: [Self; 7] = [
        Self::Input,
        Self::Body,
        Self::RetryState,
        Self::Frame,
        Self::Message,
        Self::QueueBytes,
        Self::QueueItems,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LimitError {
    #[error("resource limit must be nonzero: {0:?}")]
    Zero(Resource),
    #[error("resource budget exceeded: {0:?}")]
    Exceeded(Resource),
    #[error("duration must be representable as nonzero milliseconds: {0}")]
    Duration(&'static str),
    #[error("recovery attempts must be nonzero and backoff bounds ordered")]
    Recovery,
    #[error(
        "retry-state budget must reserve input, response and bounded metadata without overflow"
    )]
    RetryStateBudget,
}

#[derive(Debug, Clone)]
pub struct LimitConfig {
    pub input_bytes: usize,
    pub body_bytes: usize,
    pub retry_state_bytes: usize,
    pub frame_bytes: usize,
    pub message_bytes: usize,
    pub queue_bytes: usize,
    pub queue_items: usize,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub recovery_attempts: u32,
    pub backoff_min: Duration,
    pub backoff_max: Duration,
}

impl Default for LimitConfig {
    fn default() -> Self {
        Self {
            input_bytes: 1024 * 1024,
            body_bytes: 8 * 1024 * 1024,
            retry_state_bytes: 256 * 1024 * 1024,
            frame_bytes: 1024 * 1024,
            message_bytes: 1024 * 1024,
            queue_bytes: 8 * 1024 * 1024,
            queue_items: 256,
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(15),
            recovery_attempts: 5,
            backoff_min: Duration::from_millis(250),
            backoff_max: Duration::from_secs(10),
        }
    }
}

/// The validated configuration cannot be changed in place by an adapter.
#[derive(Debug, Clone)]
pub struct Limits(LimitConfig);

impl Default for Limits {
    fn default() -> Self {
        Self::new(LimitConfig::default()).expect("constant default limits are valid")
    }
}

impl Limits {
    pub fn new(config: LimitConfig) -> Result<Self, LimitError> {
        let limits = Self(config);
        for resource in Resource::ALL {
            if limits.maximum(resource) == 0 {
                return Err(LimitError::Zero(resource));
            }
        }
        let minimum_state = limits
            .0
            .input_bytes
            .checked_add(
                limits
                    .0
                    .body_bytes
                    .checked_mul(24)
                    .ok_or(LimitError::RetryStateBudget)?,
            )
            .and_then(|bytes| bytes.checked_add(128 * 1024))
            .ok_or(LimitError::RetryStateBudget)?;
        if limits.0.retry_state_bytes < minimum_state {
            return Err(LimitError::RetryStateBudget);
        }
        for (name, duration) in [
            ("connect_timeout", limits.0.connect_timeout),
            ("request_timeout", limits.0.request_timeout),
            ("backoff_min", limits.0.backoff_min),
            ("backoff_max", limits.0.backoff_max),
        ] {
            let millis = duration.as_millis();
            if millis == 0 || millis > u64::MAX as u128 {
                return Err(LimitError::Duration(name));
            }
        }
        if limits.0.recovery_attempts == 0 || limits.0.backoff_min > limits.0.backoff_max {
            return Err(LimitError::Recovery);
        }
        Ok(limits)
    }

    pub fn config(&self) -> &LimitConfig {
        &self.0
    }

    pub fn maximum(&self, resource: Resource) -> usize {
        match resource {
            Resource::Input => self.0.input_bytes,
            Resource::Body => self.0.body_bytes,
            Resource::RetryState => self.0.retry_state_bytes,
            Resource::Frame => self.0.frame_bytes,
            Resource::Message => self.0.message_bytes,
            Resource::QueueBytes => self.0.queue_bytes,
            Resource::QueueItems => self.0.queue_items,
        }
    }

    /// Returns the new retained amount, never a truncated successful result.
    pub fn check_add(
        &self,
        resource: Resource,
        retained: usize,
        incoming: usize,
    ) -> Result<usize, LimitError> {
        retained
            .checked_add(incoming)
            .filter(|total| *total <= self.maximum(resource))
            .ok_or(LimitError::Exceeded(resource))
    }
}
