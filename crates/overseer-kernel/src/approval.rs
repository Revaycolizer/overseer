//! Persistent approval ledger.
//!
//! Approval requests are stored in their own `approval_requests` table — never
//! in generic settings rows — so every human decision is an auditable record
//! with an immutable action payload.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::action::ActionRequest;
use crate::error::{KernelError, KernelResult};

/// Canonical schema for the approval ledger.
///
/// This is the single source of truth for the table shape. The Overseer runtime
/// migration (`migrations/20260715000000_kernel_approval_requests.sql`) contains
/// the identical `CREATE TABLE`; the kernel tests apply this constant to an
/// in-memory database so they exercise the real schema.
pub const APPROVAL_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS approval_requests (
    id            TEXT PRIMARY KEY,
    run_id        BIGINT NOT NULL,
    action_id     TEXT NOT NULL,
    action_kind   TEXT NOT NULL,
    tool_name     TEXT NOT NULL,
    action_payload TEXT NOT NULL,
    payload_hash  TEXT NOT NULL,
    summary       TEXT,
    status        TEXT NOT NULL DEFAULT 'pending',
    reason        TEXT,
    required_role TEXT,
    policy_version TEXT,
    policy_snapshot TEXT,
    requested_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    resolved_at   DATETIME,
    resolver      TEXT,
    resolution    TEXT
);
CREATE INDEX IF NOT EXISTS idx_approval_requests_run_status
    ON approval_requests(run_id, status);
"#;

/// Lifecycle status of an approval request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Cancelled,
    Expired,
}

impl ApprovalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApprovalStatus::Pending => "pending",
            ApprovalStatus::Approved => "approved",
            ApprovalStatus::Rejected => "rejected",
            ApprovalStatus::Cancelled => "cancelled",
            ApprovalStatus::Expired => "expired",
        }
    }
}

/// A persisted approval request row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApprovalRecord {
    pub id: String,
    pub run_id: i64,
    pub action_id: String,
    pub action_kind: String,
    pub tool_name: String,
    pub action_payload: String,
    pub payload_hash: String,
    pub summary: Option<String>,
    pub status: String,
    pub reason: Option<String>,
    pub required_role: Option<String>,
    pub policy_version: Option<String>,
    pub policy_snapshot: Option<String>,
    pub requested_at: chrono::NaiveDateTime,
    pub resolved_at: Option<chrono::NaiveDateTime>,
    pub resolver: Option<String>,
    pub resolution: Option<String>,
}

impl ApprovalRecord {
    /// Reconstruct the immutable [`ActionRequest`] that was approved. Executing
    /// this — rather than a freshly generated action — is what guarantees the
    /// approved payload is the one that runs.
    #[must_use]
    pub fn action(&self) -> KernelResult<ActionRequest> {
        Ok(serde_json::from_str(&self.action_payload)?)
    }
}

/// Stable, non-cryptographic content hash (FNV-1a 64) for payload integrity /
/// deduplication. Deterministic across processes (unlike std's `DefaultHasher`).
#[must_use]
pub fn payload_hash(payload: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in payload.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    format!("{:016x}", hash)
}

/// Repository over the `approval_requests` table.
#[derive(Clone)]
pub struct ApprovalStore {
    pool: SqlitePool,
}

impl ApprovalStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create the schema. Used by tests and any standalone/embedded runtime.
    pub async fn ensure_schema(&self) -> KernelResult<()> {
        for stmt in APPROVAL_SCHEMA.split(';') {
            let stmt = stmt.trim();
            if stmt.is_empty() {
                continue;
            }
            sqlx::query(stmt).execute(&self.pool).await?;
        }
        Ok(())
    }

    /// Persist a new pending approval for `req`. Returns the approval id.
    pub async fn create_pending(
        &self,
        req: &ActionRequest,
        reason: &str,
        required_role: Option<&str>,
        policy_version: &str,
        policy_snapshot: &str,
    ) -> KernelResult<String> {
        let approval_id = uuid::Uuid::new_v4().to_string();
        let payload = req.canonical_payload();
        let hash = payload_hash(&payload);
        sqlx::query(
            "INSERT INTO approval_requests
                (id, run_id, action_id, action_kind, tool_name, action_payload, payload_hash,
                 summary, status, reason, required_role, policy_version, policy_snapshot)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?, ?, ?)",
        )
        .bind(&approval_id)
        .bind(req.run_id)
        .bind(&req.id)
        .bind(req.kind.as_str())
        .bind(&req.tool_name)
        .bind(&payload)
        .bind(&hash)
        .bind(&req.summary)
        .bind(reason)
        .bind(required_role)
        .bind(policy_version)
        .bind(policy_snapshot)
        .execute(&self.pool)
        .await?;
        Ok(approval_id)
    }

    pub async fn get(&self, id: &str) -> KernelResult<Option<ApprovalRecord>> {
        let row =
            sqlx::query_as::<_, ApprovalRecord>("SELECT * FROM approval_requests WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row)
    }

    /// Transition a *pending* approval to a terminal status. Returns `true` only
    /// if this call performed the transition; a second call (duplicate resolve)
    /// returns `false`, giving idempotent resolution.
    pub async fn mark_resolved(
        &self,
        id: &str,
        status: ApprovalStatus,
        resolver: Option<&str>,
        resolution: Option<&str>,
    ) -> KernelResult<bool> {
        let affected = sqlx::query(
            "UPDATE approval_requests
                SET status = ?, resolver = ?, resolution = ?, resolved_at = CURRENT_TIMESTAMP
              WHERE id = ? AND status = 'pending'",
        )
        .bind(status.as_str())
        .bind(resolver)
        .bind(resolution)
        .bind(id)
        .execute(&self.pool)
        .await?
        .rows_affected();
        Ok(affected > 0)
    }

    pub async fn list_pending(&self, run_id: Option<i64>) -> KernelResult<Vec<ApprovalRecord>> {
        let rows = match run_id {
            Some(rid) => {
                sqlx::query_as::<_, ApprovalRecord>(
                    "SELECT * FROM approval_requests WHERE status = 'pending' AND run_id = ? ORDER BY requested_at ASC",
                )
                .bind(rid)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, ApprovalRecord>(
                    "SELECT * FROM approval_requests WHERE status = 'pending' ORDER BY requested_at ASC",
                )
                .fetch_all(&self.pool)
                .await?
            }
        };
        Ok(rows)
    }

    /// Restart recovery: mark every still-pending approval as cancelled. In-flight
    /// async waiters do not survive a process restart, so their ledger rows must
    /// not be left dangling as `pending`.
    pub async fn cancel_stale_pending(&self, reason: &str) -> KernelResult<u64> {
        let affected = sqlx::query(
            "UPDATE approval_requests
                SET status = 'cancelled', resolver = 'system:restart', resolution = ?, resolved_at = CURRENT_TIMESTAMP
              WHERE status = 'pending'",
        )
        .bind(reason)
        .execute(&self.pool)
        .await?
        .rows_affected();
        Ok(affected)
    }

    /// Fetch a record, erroring if it is not pending (used to reject duplicate/
    /// stale resolutions at the store layer).
    pub async fn require_pending(&self, id: &str) -> KernelResult<ApprovalRecord> {
        match self.get(id).await? {
            Some(rec) if rec.status == "pending" => Ok(rec),
            _ => Err(KernelError::ApprovalNotPending(id.to_string())),
        }
    }
}
