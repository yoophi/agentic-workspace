use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationState {
    Pending,
    Published,
    Withdrawn,
}

impl PublicationState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Published => "published",
            Self::Withdrawn => "withdrawn",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "published" => Some(Self::Published),
            "withdrawn" => Some(Self::Withdrawn),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PublicationRecord {
    pub attempt_id: String,
    pub state: PublicationState,
    pub result: Option<Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PublicationEvent {
    pub event_id: String,
    pub attempt_id: String,
    pub event_kind: String,
    pub payload: Value,
    pub created_at: String,
    pub delivered_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PublishRequest {
    pub attempt_id: String,
    pub event_id: String,
    pub event_kind: String,
    pub result: Value,
    pub payload: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReserveOutcome {
    Reserved,
    Existing(PublicationRecord),
}

#[derive(Clone, Debug, PartialEq)]
pub enum PublishOutcome {
    Published(PublicationRecord),
    Replayed(PublicationRecord),
    Lost(PublicationState),
}

#[derive(Clone, Debug, PartialEq)]
pub enum WithdrawOutcome {
    Withdrawn,
    AlreadyWithdrawn,
    Lost(PublicationState),
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PublicationStoreError {
    #[error("publication storage unavailable: {0}")]
    Storage(String),
    #[error("publication attempt not found: {0}")]
    NotFound(String),
    #[error("publication replay payload differs for attempt: {0}")]
    ReplayMismatch(String),
}

pub type PublicationStoreResult<T> = Result<T, PublicationStoreError>;

pub trait ProcessPublicationStore: Send + Sync {
    fn reserve(&self, attempt_id: &str) -> PublicationStoreResult<ReserveOutcome>;
    fn find_publication(
        &self,
        attempt_id: &str,
    ) -> PublicationStoreResult<Option<PublicationRecord>>;
    fn publish(&self, request: &PublishRequest) -> PublicationStoreResult<PublishOutcome>;
    fn withdraw(&self, attempt_id: &str) -> PublicationStoreResult<WithdrawOutcome>;
    fn pending_events(&self, limit: usize) -> PublicationStoreResult<Vec<PublicationEvent>>;
    fn acknowledge_event(&self, event_id: &str) -> PublicationStoreResult<()>;
}
