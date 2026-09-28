use std::collections::HashSet;

use crate::ports::process_publication_store::{
    ProcessPublicationStore, PublicationEvent, PublicationStoreError, PublicationStoreResult,
    PublishOutcome, PublishRequest, ReserveOutcome, WithdrawOutcome,
};

#[derive(Debug, thiserror::Error)]
pub enum PublicationDeliveryError {
    #[error(transparent)]
    Store(#[from] PublicationStoreError),
    #[error("publication delivery failed: {0}")]
    Send(String),
}

pub struct ProcessPublicationService<'a, S> {
    store: &'a S,
}

impl<'a, S: ProcessPublicationStore> ProcessPublicationService<'a, S> {
    pub fn new(store: &'a S) -> Self {
        Self { store }
    }

    pub fn reserve(&self, attempt_id: &str) -> PublicationStoreResult<ReserveOutcome> {
        self.store.reserve(attempt_id)
    }

    pub fn publish(&self, request: &PublishRequest) -> PublicationStoreResult<PublishOutcome> {
        self.store.publish(request)
    }

    pub fn withdraw(&self, attempt_id: &str) -> PublicationStoreResult<WithdrawOutcome> {
        self.store.withdraw(attempt_id)
    }

    pub fn deliver_pending(
        &self,
        limit: usize,
        mut deliver: impl FnMut(&PublicationEvent) -> Result<(), String>,
    ) -> Result<usize, PublicationDeliveryError> {
        let events = self.store.pending_events(limit)?;
        let mut delivered = 0;
        for event in events {
            deliver(&event).map_err(PublicationDeliveryError::Send)?;
            self.store.acknowledge_event(&event.event_id)?;
            delivered += 1;
        }
        Ok(delivered)
    }
}

/// In-memory duplicate suppression for one live projection instance. Durable
/// client reconnect/restart dedupe remains the responsibility of the client
/// projection contract (T020); this type is not evidence for that boundary.
#[derive(Default)]
pub struct PublicationProjection {
    applied_event_ids: HashSet<String>,
}

impl PublicationProjection {
    pub fn apply_once(&mut self, event: &PublicationEvent) -> bool {
        self.applied_event_ids.insert(event.event_id.clone())
    }
}
