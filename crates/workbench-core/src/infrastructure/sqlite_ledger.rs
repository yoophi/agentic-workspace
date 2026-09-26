//! SQLite(WAL) operation ledger. 스키마·상태 전이는 `specs/037-workbench-seam/data-model.md` §3.
//!
//! 연결은 하나만 열고 `Mutex`로 감싼다. SQLite 3.51.3 미만의 WAL-reset 버그는 두 연결 이상이 동시에 쓸 때만
//! 발생하므로 단일 writer가 이중 방어다(research R2). 호출자는 `spawn_blocking` 안에서 부른다.

use std::{path::PathBuf, sync::Mutex, time::Duration};

use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, Connection, ErrorCode, OptionalExtension, Transaction};
use workbench_protocol::{IdempotencyKey, OperationId, PrincipalKind, RequestId};

use crate::{
    infrastructure::data_paths::DataPaths,
    ports::operation_ledger::{
        LedgerError, LedgerKey, LedgerRecord, LedgerResult, LedgerState, NewLedgerEntry,
        OperationLedger, ReconcileSummary,
    },
};

pub const SCHEMA_VERSION: i64 = 1;
/// 멱등성 결과 보존 기간. `pending`/`unknown`에는 적용하지 않는다.
pub const RESULT_TTL: chrono::Duration = chrono::Duration::hours(24);

const DDL_V1: &str = r#"
CREATE TABLE IF NOT EXISTS schema_version (
  version    INTEGER NOT NULL,
  applied_at TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS operation_ledger (
  execution_id         TEXT PRIMARY KEY,
  principal_kind       TEXT NOT NULL,
  operation            TEXT NOT NULL,
  contract_revision    INTEGER NOT NULL,
  idempotency_key      TEXT NOT NULL,
  input_fingerprint    TEXT NOT NULL,
  aggregate            TEXT NOT NULL,
  reserved_resource_id TEXT,
  state                TEXT NOT NULL,
  result_json          TEXT,
  revision             INTEGER,
  request_id           TEXT NOT NULL,
  created_at           TEXT NOT NULL,
  updated_at           TEXT NOT NULL,
  expires_at           TEXT,
  UNIQUE (principal_kind, operation, contract_revision, idempotency_key)
);
CREATE UNIQUE INDEX IF NOT EXISTS operation_ledger_reserved
  ON operation_ledger (aggregate, reserved_resource_id)
  WHERE reserved_resource_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS operation_ledger_expiry
  ON operation_ledger (expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS operation_ledger_pending
  ON operation_ledger (state) WHERE state IN ('pending','unknown');

CREATE TABLE IF NOT EXISTS aggregate_revision (
  aggregate  TEXT PRIMARY KEY,
  revision   INTEGER NOT NULL,
  updated_at TEXT    NOT NULL
);
"#;

pub struct SqliteOperationLedger {
    path: PathBuf,
    connection: Mutex<Connection>,
}

pub fn now_rfc3339() -> String {
    format_time(Utc::now())
}

fn format_time(time: DateTime<Utc>) -> String {
    time.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn storage(error: rusqlite::Error) -> LedgerError {
    LedgerError::Storage(error.to_string())
}

impl SqliteOperationLedger {
    pub fn open(paths: &DataPaths) -> LedgerResult<Self> {
        let path = paths.ledger_file();
        let connection = Connection::open(&path).map_err(storage)?;
        connection
            .busy_timeout(Duration::from_millis(5000))
            .map_err(storage)?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(storage)?;
        connection
            .pragma_update(None, "synchronous", "NORMAL")
            .map_err(storage)?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(storage)?;
        Ok(Self {
            path,
            connection: Mutex::new(connection),
        })
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    fn with_connection<R>(
        &self,
        f: impl FnOnce(&mut Connection) -> LedgerResult<R>,
    ) -> LedgerResult<R> {
        let mut guard = self
            .connection
            .lock()
            .map_err(|_| LedgerError::Storage("ledger connection mutex poisoned".into()))?;
        f(&mut guard)
    }

    /// 테스트 전용: TTL 검증을 위해 만료 시각을 임의로 바꾼다.
    #[cfg(any(test, feature = "test-hooks"))]
    pub fn set_expires_at_for_all(&self, expires_at: DateTime<Utc>) -> LedgerResult<usize> {
        self.with_connection(|conn| {
            conn.execute(
                "UPDATE operation_ledger SET expires_at = ?1 WHERE state IN ('applied','failed')",
                params![format_time(expires_at)],
            )
            .map_err(storage)
        })
    }

    /// 테스트·진단용: 상태별 건수.
    #[cfg(any(test, feature = "test-hooks"))]
    pub fn count_by_state(&self, state: LedgerState) -> LedgerResult<usize> {
        self.with_connection(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM operation_ledger WHERE state = ?1",
                params![state.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count as usize)
            .map_err(storage)
        })
    }

    /// 테스트·진단용: `applied` 항목이 예약한 resource id 목록.
    #[cfg(any(test, feature = "test-hooks"))]
    pub fn applied_resource_ids(&self, aggregate: &str) -> LedgerResult<Vec<String>> {
        self.with_connection(|conn| {
            let mut statement = conn
                .prepare(
                    "SELECT reserved_resource_id FROM operation_ledger
                     WHERE aggregate = ?1 AND state = 'applied' AND reserved_resource_id IS NOT NULL
                     ORDER BY revision",
                )
                .map_err(storage)?;
            let rows = statement
                .query_map(params![aggregate], |row| row.get::<_, String>(0))
                .map_err(storage)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(storage)
        })
    }
}

fn bump_revision(tx: &Transaction<'_>, aggregate: &str, now: &str) -> LedgerResult<u64> {
    tx.execute(
        "INSERT INTO aggregate_revision (aggregate, revision, updated_at) VALUES (?1, 1, ?2)
         ON CONFLICT(aggregate) DO UPDATE SET revision = revision + 1, updated_at = excluded.updated_at",
        params![aggregate, now],
    )
    .map_err(storage)?;
    tx.query_row(
        "SELECT revision FROM aggregate_revision WHERE aggregate = ?1",
        params![aggregate],
        |row| row.get::<_, i64>(0),
    )
    .map(|revision| revision as u64)
    .map_err(storage)
}

fn apply_row(
    tx: &Transaction<'_>,
    execution_id: &str,
    aggregate: &str,
    result: &serde_json::Value,
    now: &str,
) -> LedgerResult<u64> {
    let revision = bump_revision(tx, aggregate, now)?;
    let expires_at = format_time(Utc::now() + RESULT_TTL);
    tx.execute(
        "UPDATE operation_ledger
         SET state = 'applied', result_json = ?2, revision = ?3, updated_at = ?4, expires_at = ?5
         WHERE execution_id = ?1",
        params![
            execution_id,
            result.to_string(),
            revision as i64,
            now,
            expires_at
        ],
    )
    .map_err(storage)?;
    Ok(revision)
}

fn read_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<LedgerRecord> {
    let principal_kind: String = row.get("principal_kind")?;
    let operation: String = row.get("operation")?;
    let idempotency_key: String = row.get("idempotency_key")?;
    let state: String = row.get("state")?;
    let result_json: Option<String> = row.get("result_json")?;
    let request_id: String = row.get("request_id")?;
    let revision: Option<i64> = row.get("revision")?;
    let contract_revision: i64 = row.get("contract_revision")?;

    let invalid = |message: String| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                message,
            )),
        )
    };

    Ok(LedgerRecord {
        execution_id: row.get("execution_id")?,
        key: LedgerKey {
            principal_kind: match principal_kind.as_str() {
                "desktop" => PrincipalKind::Desktop,
                "test" => PrincipalKind::Test,
                other => return Err(invalid(format!("unknown principal kind {other}"))),
            },
            operation: OperationId::parse(&operation)
                .ok_or_else(|| invalid(format!("unknown operation {operation}")))?,
            contract_revision: contract_revision as u32,
            idempotency_key: IdempotencyKey::new(idempotency_key)
                .map_err(|error| invalid(error.to_string()))?,
        },
        input_fingerprint: row.get("input_fingerprint")?,
        aggregate: row.get("aggregate")?,
        reserved_resource_id: row.get("reserved_resource_id")?,
        state: LedgerState::parse(&state)
            .ok_or_else(|| invalid(format!("unknown state {state}")))?,
        result_json: result_json
            .map(|json| serde_json::from_str(&json))
            .transpose()
            .map_err(|error| invalid(error.to_string()))?,
        revision: revision.map(|value| value as u64),
        request_id: RequestId::new(request_id).map_err(|error| invalid(error.to_string()))?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        expires_at: row.get("expires_at")?,
    })
}

const SELECT_RECORD: &str =
    "SELECT execution_id, principal_kind, operation, contract_revision, idempotency_key,
    input_fingerprint, aggregate, reserved_resource_id, state, result_json, revision, request_id,
    created_at, updated_at, expires_at FROM operation_ledger";

impl OperationLedger for SqliteOperationLedger {
    fn migrate(&self) -> LedgerResult<()> {
        self.with_connection(|conn| {
            conn.execute_batch(DDL_V1).map_err(storage)?;
            let current: Option<i64> = conn
                .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                    row.get(0)
                })
                .map_err(storage)?;
            match current {
                None => {
                    conn.execute(
                        "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
                        params![SCHEMA_VERSION, now_rfc3339()],
                    )
                    .map_err(storage)?;
                    Ok(())
                }
                Some(found) if found == SCHEMA_VERSION => Ok(()),
                Some(found) => Err(LedgerError::UnsupportedSchema {
                    found,
                    supported: SCHEMA_VERSION,
                }),
            }
        })
    }

    fn find(&self, key: &LedgerKey) -> LedgerResult<Option<LedgerRecord>> {
        self.with_connection(|conn| {
            conn.query_row(
                &format!(
                    "{SELECT_RECORD} WHERE principal_kind = ?1 AND operation = ?2
                     AND contract_revision = ?3 AND idempotency_key = ?4"
                ),
                params![
                    key.principal_kind.as_str(),
                    key.operation.as_str(),
                    key.contract_revision as i64,
                    key.idempotency_key.as_str()
                ],
                read_record,
            )
            .optional()
            .map_err(storage)
        })
    }

    fn begin(&self, entry: NewLedgerEntry) -> LedgerResult<String> {
        let execution_id = format!("exec_{}", uuid::Uuid::new_v4().simple());
        let now = now_rfc3339();
        self.with_connection(|conn| {
            let result = conn.execute(
                "INSERT INTO operation_ledger (execution_id, principal_kind, operation, contract_revision,
                    idempotency_key, input_fingerprint, aggregate, reserved_resource_id, state,
                    request_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending', ?9, ?10, ?10)",
                params![
                    execution_id,
                    entry.key.principal_kind.as_str(),
                    entry.key.operation.as_str(),
                    entry.key.contract_revision as i64,
                    entry.key.idempotency_key.as_str(),
                    entry.input_fingerprint,
                    entry.aggregate,
                    entry.reserved_resource_id,
                    entry.request_id.as_str(),
                    now
                ],
            );
            match result {
                Ok(_) => Ok(execution_id.clone()),
                Err(rusqlite::Error::SqliteFailure(error, message))
                    if error.code == ErrorCode::ConstraintViolation =>
                {
                    let message = message.unwrap_or_default();
                    if message.contains("reserved_resource_id") {
                        Err(LedgerError::DuplicateReservation)
                    } else {
                        Err(LedgerError::DuplicateKey)
                    }
                }
                Err(error) => Err(storage(error)),
            }
        })
    }

    fn complete(&self, execution_id: &str, result: &serde_json::Value) -> LedgerResult<u64> {
        let now = now_rfc3339();
        self.with_connection(|conn| {
            let tx = conn.transaction().map_err(storage)?;
            let (state, aggregate): (String, String) = tx
                .query_row(
                    "SELECT state, aggregate FROM operation_ledger WHERE execution_id = ?1",
                    params![execution_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(storage)?
                .ok_or_else(|| LedgerError::NotFound(execution_id.to_owned()))?;
            if state != "pending" {
                return Err(LedgerError::InvalidState(
                    LedgerState::parse(&state)
                        .map(LedgerState::as_str)
                        .unwrap_or("?"),
                ));
            }
            let revision = apply_row(&tx, execution_id, &aggregate, result, &now)?;
            tx.commit().map_err(storage)?;
            Ok(revision)
        })
    }

    fn fail(&self, execution_id: &str, fault: &serde_json::Value) -> LedgerResult<()> {
        let now = now_rfc3339();
        let expires_at = format_time(Utc::now() + RESULT_TTL);
        self.with_connection(|conn| {
            let changed = conn
                .execute(
                    "UPDATE operation_ledger
                     SET state = 'failed', result_json = ?2, updated_at = ?3, expires_at = ?4
                     WHERE execution_id = ?1 AND state = 'pending'",
                    params![execution_id, fault.to_string(), now, expires_at],
                )
                .map_err(storage)?;
            if changed == 0 {
                return Err(LedgerError::NotFound(execution_id.to_owned()));
            }
            Ok(())
        })
    }

    fn reconcile_pending(
        &self,
        resolve: &mut dyn FnMut(&LedgerRecord) -> Option<serde_json::Value>,
    ) -> LedgerResult<ReconcileSummary> {
        let pending: Vec<LedgerRecord> = self.with_connection(|conn| {
            let mut statement = conn
                .prepare(&format!(
                    "{SELECT_RECORD} WHERE state = 'pending' ORDER BY created_at"
                ))
                .map_err(storage)?;
            let rows = statement.query_map([], read_record).map_err(storage)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(storage)
        })?;

        let mut summary = ReconcileSummary::default();
        for record in pending {
            let decision = resolve(&record);
            let now = now_rfc3339();
            self.with_connection(|conn| {
                let tx = conn.transaction().map_err(storage)?;
                match &decision {
                    Some(result) => {
                        apply_row(&tx, &record.execution_id, &record.aggregate, result, &now)?;
                        summary.applied += 1;
                    }
                    None => {
                        tx.execute(
                            "UPDATE operation_ledger SET state = 'unknown', updated_at = ?2
                             WHERE execution_id = ?1",
                            params![record.execution_id, now],
                        )
                        .map_err(storage)?;
                        summary.unknown += 1;
                    }
                }
                tx.commit().map_err(storage)
            })?;
        }
        Ok(summary)
    }

    fn gc_expired(&self, now: DateTime<Utc>) -> LedgerResult<usize> {
        self.with_connection(|conn| {
            conn.execute(
                "DELETE FROM operation_ledger
                 WHERE state IN ('applied','failed') AND expires_at IS NOT NULL AND expires_at < ?1",
                params![format_time(now)],
            )
            .map_err(storage)
        })
    }

    fn current_revision(&self, aggregate: &str) -> LedgerResult<u64> {
        self.with_connection(|conn| {
            let existing: Option<i64> = conn
                .query_row(
                    "SELECT revision FROM aggregate_revision WHERE aggregate = ?1",
                    params![aggregate],
                    |row| row.get(0),
                )
                .optional()
                .map_err(storage)?;
            match existing {
                Some(revision) => Ok(revision as u64),
                None => {
                    conn.execute(
                        "INSERT INTO aggregate_revision (aggregate, revision, updated_at) VALUES (?1, 0, ?2)",
                        params![aggregate, now_rfc3339()],
                    )
                    .map_err(storage)?;
                    Ok(0)
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use workbench_protocol::CONTRACT_REVISION;

    use super::*;

    fn ledger() -> (tempfile::TempDir, SqliteOperationLedger) {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::new(dir.path());
        paths.ensure_dirs().unwrap();
        let ledger = SqliteOperationLedger::open(&paths).unwrap();
        ledger.migrate().unwrap();
        (dir, ledger)
    }

    fn entry(key: &str, reserved: &str) -> NewLedgerEntry {
        NewLedgerEntry {
            key: LedgerKey {
                principal_kind: PrincipalKind::Desktop,
                operation: OperationId::ProjectCreate,
                contract_revision: CONTRACT_REVISION,
                idempotency_key: IdempotencyKey::new(key).unwrap(),
            },
            input_fingerprint: "fp".into(),
            aggregate: "projects".into(),
            reserved_resource_id: Some(reserved.into()),
            request_id: RequestId::new("r1").unwrap(),
        }
    }

    #[test]
    fn migrate_is_idempotent_and_records_version() {
        let (_dir, ledger) = ledger();
        ledger.migrate().unwrap();
        assert_eq!(ledger.current_revision("projects").unwrap(), 0);
    }

    #[test]
    fn begin_complete_transitions_and_bumps_revision() {
        let (_dir, ledger) = ledger();
        let exec = ledger.begin(entry("k1", "project-1")).unwrap();
        let record = ledger.find(&entry("k1", "x").key).unwrap().unwrap();
        assert_eq!(record.state, LedgerState::Pending);
        assert_eq!(record.expires_at, None);

        let revision = ledger.complete(&exec, &json!({"id": "project-1"})).unwrap();
        assert_eq!(revision, 1);
        let record = ledger.find(&entry("k1", "x").key).unwrap().unwrap();
        assert_eq!(record.state, LedgerState::Applied);
        assert_eq!(record.revision, Some(1));
        assert!(record.expires_at.is_some());
        assert_eq!(record.result_json, Some(json!({"id": "project-1"})));

        let exec2 = ledger.begin(entry("k2", "project-2")).unwrap();
        assert_eq!(ledger.complete(&exec2, &json!({})).unwrap(), 2);
        assert_eq!(ledger.current_revision("projects").unwrap(), 2);
        assert_eq!(
            ledger.complete(&exec2, &json!({})).unwrap_err(),
            LedgerError::InvalidState("applied")
        );
    }

    #[test]
    fn duplicate_key_and_reservation_are_distinguished() {
        let (_dir, ledger) = ledger();
        ledger.begin(entry("k1", "project-1")).unwrap();
        assert_eq!(
            ledger.begin(entry("k1", "project-9")).unwrap_err(),
            LedgerError::DuplicateKey
        );
        assert_eq!(
            ledger.begin(entry("k2", "project-1")).unwrap_err(),
            LedgerError::DuplicateReservation
        );
    }

    #[test]
    fn fail_marks_failed_and_only_from_pending() {
        let (_dir, ledger) = ledger();
        let exec = ledger.begin(entry("k1", "project-1")).unwrap();
        ledger.fail(&exec, &json!({"code": "unavailable"})).unwrap();
        let record = ledger.find(&entry("k1", "x").key).unwrap().unwrap();
        assert_eq!(record.state, LedgerState::Failed);
        assert!(record.expires_at.is_some());
        assert!(matches!(
            ledger.fail(&exec, &json!({})).unwrap_err(),
            LedgerError::NotFound(_)
        ));
        assert_eq!(ledger.current_revision("projects").unwrap(), 0);
    }

    #[test]
    fn reconcile_applies_or_marks_unknown_without_rerunning() {
        let (_dir, ledger) = ledger();
        ledger.begin(entry("k1", "project-1")).unwrap();
        ledger.begin(entry("k2", "project-2")).unwrap();
        let summary = ledger
            .reconcile_pending(&mut |record| {
                (record.reserved_resource_id.as_deref() == Some("project-1"))
                    .then(|| json!({"id": "project-1"}))
            })
            .unwrap();
        assert_eq!(
            summary,
            ReconcileSummary {
                applied: 1,
                unknown: 1
            }
        );
        assert_eq!(ledger.count_by_state(LedgerState::Applied).unwrap(), 1);
        assert_eq!(ledger.count_by_state(LedgerState::Unknown).unwrap(), 1);
        assert_eq!(ledger.current_revision("projects").unwrap(), 1);
        // 두 번째 reconcile은 pending이 없어 아무 것도 하지 않는다.
        assert_eq!(
            ledger.reconcile_pending(&mut |_| None).unwrap(),
            ReconcileSummary::default()
        );
    }

    #[test]
    fn gc_removes_expired_results_but_keeps_pending_unknown_and_revision() {
        let (_dir, ledger) = ledger();
        let exec = ledger.begin(entry("k1", "project-1")).unwrap();
        ledger.complete(&exec, &json!({})).unwrap();
        ledger.begin(entry("k2", "project-2")).unwrap();
        ledger
            .set_expires_at_for_all(Utc::now() - chrono::Duration::hours(1))
            .unwrap();

        assert_eq!(ledger.gc_expired(Utc::now()).unwrap(), 1);
        assert_eq!(ledger.count_by_state(LedgerState::Applied).unwrap(), 0);
        assert_eq!(ledger.count_by_state(LedgerState::Pending).unwrap(), 1);
        assert_eq!(ledger.current_revision("projects").unwrap(), 1);
    }
}
