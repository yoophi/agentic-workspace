use std::{future::Future, pin::Pin};

use anyhow::Result;

use crate::{
    domain::acp_session::{AcpSessionListQuery, AcpSessionLookup, AcpSessionRecord},
    ports::acp_session_store::AcpSessionStore,
};

pub struct NoopAcpSessionStore;

impl AcpSessionStore for NoopAcpSessionStore {
    fn record_session<'a>(
        &'a self,
        _record: AcpSessionRecord,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }

    fn latest_session<'a>(
        &'a self,
        _lookup: AcpSessionLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<AcpSessionRecord>>> + Send + 'a>> {
        Box::pin(async { Ok(None) })
    }

    fn list_sessions<'a>(
        &'a self,
        _query: AcpSessionListQuery,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<AcpSessionRecord>>> + Send + 'a>> {
        Box::pin(async { Ok(Vec::new()) })
    }

    fn clear_session<'a>(
        &'a self,
        _run_id: String,
    ) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'a>> {
        Box::pin(async { Ok(false) })
    }
}
