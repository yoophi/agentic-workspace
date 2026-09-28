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

/// v2(038, research R16): 자원 예약 unique index를 `pending` 행에만 적용한다. 종료 상태(applied/failed/unknown)는
/// 예약을 해제하되 `reserved_resource_id` 값은 재시작 판정 증거로 남긴다.
pub const SCHEMA_VERSION: i64 = 2;
/// 멱등성 결과 보존 기간. `pending`/`unknown`에는 적용하지 않는다.
pub const RESULT_TTL: chrono::Duration = chrono::Duration::hours(24);

/// 버전과 무관한 테이블·index. 예약 index는 버전마다 다르므로 여기 없다(v1은 `DDL_V1`, v2는 `MIGRATION_V2`).
const DDL_BASE: &str = r#"
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

/// 037이 만든 v1 파일의 전체 DDL. 운영 코드는 더 이상 실행하지 않고, 승격 경로(v1 → v2) 테스트가 v1 파일을
/// 이 텍스트로 재현한다. v1 예약 index는 상태와 무관하게 배타적이었다(research R16의 결함).
#[cfg(test)]
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

/// v1 → v2: 상태와 무관하던 예약 unique index를 `state = 'pending'`으로 한정한다. 행 데이터는 바뀌지 않는다.
const MIGRATION_V2: &str = r#"
DROP INDEX IF EXISTS operation_ledger_reserved;
CREATE UNIQUE INDEX IF NOT EXISTS operation_ledger_reserved_pending
  ON operation_ledger (aggregate, reserved_resource_id)
  WHERE reserved_resource_id IS NOT NULL AND state = 'pending';
"#;

pub struct SqliteOperationLedger {
    path: PathBuf,
    connection: Mutex<Connection>,
    /// 시험: 켜져 있으면 상태별 수 읽기(`count_by_state`)가 저장소 오류로 끝난다(Codex r8).
    count_fault: std::sync::atomic::AtomicBool,
}

pub fn now_rfc3339() -> String {
    format_time(Utc::now())
}

fn format_time(time: DateTime<Utc>) -> String {
    time.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// 기존 ledger 파일의 저장 형식 버전을 **읽기 전용**으로 읽는다(044: 독립 서버가 모르는 형식을 데이터를 건드리기
/// 전에 거절한다). 파일이나 버전 표가 없으면 `None`.
pub fn read_schema_version(path: &std::path::Path) -> Result<Option<i64>, LedgerError> {
    if !path.exists() {
        return Ok(None);
    }
    let conn = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(storage)?;
    let has_table: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_version')",
            [],
            |row| row.get(0),
        )
        .map_err(storage)?;
    if !has_table {
        return Ok(None);
    }
    conn.query_row("SELECT MAX(version) FROM schema_version", [], |row| {
        row.get(0)
    })
    .map_err(storage)
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
            count_fault: std::sync::atomic::AtomicBool::new(false),
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

    /// 상태별 건수. `server.status`가 `pendingOperations`(`pending`)·`unresolvedOperations`(`unknown`)를 파생한다(044).
    /// 시험: 상태별 수 읽기 오류를 켜고 끈다(Codex r8 — 활동을 모르는 정지 판정).
    #[cfg(feature = "test-hooks")]
    pub fn set_count_fault(&self, failing: bool) {
        self.count_fault
            .store(failing, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn count_by_state(&self, state: LedgerState) -> LedgerResult<usize> {
        if self.count_fault.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(LedgerError::Storage("injected ledger read failure".into()));
        }
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
            principal_kind: PrincipalKind::parse(&principal_kind)
                .ok_or_else(|| invalid(format!("unknown principal kind {principal_kind}")))?,
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
            conn.execute_batch(DDL_BASE).map_err(storage)?;
            let current: Option<i64> = conn
                .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                    row.get(0)
                })
                .map_err(storage)?;
            match current {
                // 새 파일(None) 또는 037의 v1 파일: v2 index로 교체하고 버전을 기록한다.
                None | Some(1) => {
                    let tx = conn.transaction().map_err(storage)?;
                    tx.execute_batch(MIGRATION_V2).map_err(storage)?;
                    tx.execute(
                        "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
                        params![SCHEMA_VERSION, now_rfc3339()],
                    )
                    .map_err(storage)?;
                    tx.commit().map_err(storage)
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

    fn schema_version(ledger: &SqliteOperationLedger) -> i64 {
        ledger
            .with_connection(|conn| {
                conn.query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                    row.get(0)
                })
                .map_err(storage)
            })
            .unwrap()
    }

    #[test]
    fn migrate_is_idempotent_and_records_version() {
        let (_dir, ledger) = ledger();
        ledger.migrate().unwrap();
        assert_eq!(ledger.current_revision("projects").unwrap(), 0);
        assert_eq!(schema_version(&ledger), SCHEMA_VERSION);
    }

    /// research R16 (a): 037이 만든 v1 파일에 `applied` 예약이 남아 있어도, v2로 승격되면 같은 자원을 다시 예약할 수 있다.
    #[test]
    fn v1_file_upgrades_to_v2_and_releases_terminal_reservations() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::new(dir.path());
        paths.ensure_dirs().unwrap();
        {
            let conn = Connection::open(paths.ledger_file()).unwrap();
            conn.execute_batch(DDL_V1).unwrap();
            conn.execute(
                "INSERT INTO schema_version (version, applied_at) VALUES (1, ?1)",
                params![now_rfc3339()],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO operation_ledger (execution_id, principal_kind, operation, contract_revision,
                    idempotency_key, input_fingerprint, aggregate, reserved_resource_id, state,
                    result_json, revision, request_id, created_at, updated_at, expires_at)
                 VALUES ('exec_old', 'desktop', 'project.create', 1, 'old-key', 'fp', 'git-worktrees:/repo',
                    '/repo-worktrees/a', 'applied', 'null', 1, 'r0', ?1, ?1, ?1)",
                params![now_rfc3339()],
            )
            .unwrap();
        }

        let ledger = SqliteOperationLedger::open(&paths).unwrap();
        ledger.migrate().unwrap();
        assert_eq!(schema_version(&ledger), 2);
        let indexes: Vec<String> = ledger
            .with_connection(|conn| {
                let mut statement = conn
                    .prepare("SELECT name FROM sqlite_master WHERE type = 'index' ORDER BY name")
                    .map_err(storage)?;
                let rows = statement
                    .query_map([], |row| row.get::<_, String>(0))
                    .map_err(storage)?;
                rows.collect::<Result<Vec<_>, _>>().map_err(storage)
            })
            .unwrap();
        assert!(
            !indexes
                .iter()
                .any(|name| name == "operation_ledger_reserved"),
            "v1 index must be dropped: {indexes:?}"
        );
        assert!(
            indexes
                .iter()
                .any(|name| name == "operation_ledger_reserved_pending"),
            "v2 index must exist: {indexes:?}"
        );

        // v1에서는 DuplicateReservation이던 조합이 v2에서는 통과한다.
        let mut entry = entry("new-key", "/repo-worktrees/a");
        entry.aggregate = "git-worktrees:/repo".into();
        let exec = ledger.begin(entry).unwrap();
        assert!(exec.starts_with("exec_"));
        // 다시 migrate해도 멱등 — v1 예약 index를 다시 만들지 않는다(applied+pending이 같은 자원을 가진 지금
        // 그 index를 다시 만들면 constraint 위반으로 기동이 실패한다).
        ledger.migrate().unwrap();
        assert_eq!(schema_version(&ledger), 2);
        ledger
            .begin(entry_for(
                "k-after",
                "git-worktrees:/repo",
                "/repo-worktrees/b",
            ))
            .unwrap();
    }

    fn entry_for(key: &str, aggregate: &str, reserved: &str) -> NewLedgerEntry {
        let mut entry = entry(key, reserved);
        entry.aggregate = aggregate.into();
        entry
    }

    /// research R16 표: `applied`(및 failed/unknown) 전이는 예약을 해제하고, `pending`만 배타다.
    #[test]
    fn applied_transition_releases_reservation_but_pending_still_conflicts() {
        let (_dir, ledger) = ledger();
        let exec = ledger.begin(entry("k1", "/wt/a")).unwrap();
        assert_eq!(
            ledger.begin(entry("k2", "/wt/a")).unwrap_err(),
            LedgerError::DuplicateReservation
        );
        ledger.complete(&exec, &json!(null)).unwrap();
        let exec2 = ledger.begin(entry("k2", "/wt/a")).unwrap();
        ledger.fail(&exec2, &json!({"code": "internal"})).unwrap();
        let exec3 = ledger.begin(entry("k3", "/wt/a")).unwrap();
        // unknown으로 닫혀도 해제된다.
        ledger.reconcile_pending(&mut |_| None).unwrap();
        assert_eq!(ledger.count_by_state(LedgerState::Unknown).unwrap(), 1);
        let _ = exec3;
        ledger.begin(entry("k4", "/wt/a")).unwrap();
    }

    #[test]
    fn unsupported_future_schema_is_rejected() {
        let (_dir, ledger) = ledger();
        ledger
            .with_connection(|conn| {
                conn.execute(
                    "INSERT INTO schema_version (version, applied_at) VALUES (3, ?1)",
                    params![now_rfc3339()],
                )
                .map_err(storage)
            })
            .unwrap();
        assert_eq!(
            ledger.migrate().unwrap_err(),
            LedgerError::UnsupportedSchema {
                found: 3,
                supported: SCHEMA_VERSION
            }
        );
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
