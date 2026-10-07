//! Scheduled / repeating autonomy.
//!
//! A durable table of goals with a next-run time. A background tick (spawned by
//! the daemon) finds due goals and runs each through the governed agent loop
//! (`agent_loop::run_agent`) — so scheduled work is still policy- and
//! approval-gated, exactly like a typed `/do`.
//!
//! The table is created lazily (no optional feature / migration needed). The
//! agent can manage its own schedule via the `schedule_goal` / `list_schedules`
//! loop tools.

use crate::db::Database;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How often the daemon tick checks for due goals.
const TICK_SECS: u64 = 30;

pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub async fn ensure_table(db: &Database) {
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS scheduled_goals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            conversation_id INTEGER NOT NULL,
            goal TEXT NOT NULL,
            interval_secs INTEGER NOT NULL DEFAULT 0,
            next_run_at INTEGER NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            last_run_at INTEGER,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(&db.pool)
    .await;
}

/// Add a schedule. `interval_secs == 0` means run once (at `first_run_at`).
pub async fn add_schedule(
    db: &Database,
    conversation_id: i64,
    goal: &str,
    interval_secs: i64,
    first_run_at: i64,
) -> anyhow::Result<i64> {
    ensure_table(db).await;
    let id = sqlx::query(
        "INSERT INTO scheduled_goals (conversation_id, goal, interval_secs, next_run_at) VALUES (?, ?, ?, ?)",
    )
    .bind(conversation_id)
    .bind(goal)
    .bind(interval_secs.max(0))
    .bind(first_run_at)
    .execute(&db.pool)
    .await?
    .last_insert_rowid();
    Ok(id)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Schedule {
    pub id: i64,
    pub conversation_id: i64,
    pub goal: String,
    pub interval_secs: i64,
    pub next_run_at: i64,
}

/// Goals whose `next_run_at <= now` and are enabled.
pub async fn due_schedules(db: &Database, now: i64) -> Vec<Schedule> {
    ensure_table(db).await;
    sqlx::query_as::<_, (i64, i64, String, i64, i64)>(
        "SELECT id, conversation_id, goal, interval_secs, next_run_at
         FROM scheduled_goals WHERE enabled = 1 AND next_run_at <= ? ORDER BY next_run_at ASC",
    )
    .bind(now)
    .fetch_all(&db.pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(
        |(id, conversation_id, goal, interval_secs, next_run_at)| Schedule {
            id,
            conversation_id,
            goal,
            interval_secs,
            next_run_at,
        },
    )
    .collect()
}

/// After running: reschedule by interval, or disable a one-shot.
pub async fn mark_ran(db: &Database, sched: &Schedule, now: i64) {
    if sched.interval_secs > 0 {
        let next = now + sched.interval_secs;
        let _ =
            sqlx::query("UPDATE scheduled_goals SET last_run_at = ?, next_run_at = ? WHERE id = ?")
                .bind(now)
                .bind(next)
                .bind(sched.id)
                .execute(&db.pool)
                .await;
    } else {
        let _ = sqlx::query("UPDATE scheduled_goals SET last_run_at = ?, enabled = 0 WHERE id = ?")
            .bind(now)
            .bind(sched.id)
            .execute(&db.pool)
            .await;
    }
}

pub async fn list_schedules(db: &Database) -> Vec<Schedule> {
    ensure_table(db).await;
    sqlx::query_as::<_, (i64, i64, String, i64, i64)>(
        "SELECT id, conversation_id, goal, interval_secs, next_run_at
         FROM scheduled_goals WHERE enabled = 1 ORDER BY next_run_at ASC",
    )
    .fetch_all(&db.pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(
        |(id, conversation_id, goal, interval_secs, next_run_at)| Schedule {
            id,
            conversation_id,
            goal,
            interval_secs,
            next_run_at,
        },
    )
    .collect()
}

/// The daemon's scheduler loop. Ticks every `TICK_SECS`, runs each due goal
/// through the governed agent loop (spawned so a run awaiting approval never
/// blocks the tick), then reschedules.
pub async fn run_scheduler(db: Arc<Database>) {
    tracing::info!("scheduler service started (tick {TICK_SECS}s)");
    loop {
        let now = now_unix();
        let due = due_schedules(&db, now).await;
        for sched in due {
            // Reschedule immediately so a long/approval-pending run isn't repeated.
            mark_ran(&db, &sched, now).await;
            let db2 = db.clone();
            let s = sched.clone();
            tokio::spawn(async move {
                tracing::info!(id = s.id, goal = %s.goal, "scheduler firing goal");
                let _ =
                    crate::engine::agent_loop::run_agent(&db2, s.conversation_id, &s.goal, None)
                        .await;
            });
        }
        tokio::time::sleep(Duration::from_secs(TICK_SECS)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn due_and_reschedule_logic() {
        let db = crate::db::Database::new_in_memory().await.unwrap();
        let conv = crate::backend::add_conversation(&db, "t").await.unwrap();

        // A goal due in the past (repeating hourly) and one due far in the future.
        let past = add_schedule(&db, conv, "morning brief", 3600, now_unix() - 10)
            .await
            .unwrap();
        let _future = add_schedule(&db, conv, "later", 0, now_unix() + 10_000)
            .await
            .unwrap();

        let now = now_unix();
        let due = due_schedules(&db, now).await;
        assert_eq!(due.len(), 1, "only the past-due goal is due");
        assert_eq!(due[0].id, past);

        // Reschedule the repeating goal → it moves into the future.
        mark_ran(&db, &due[0], now).await;
        assert!(
            due_schedules(&db, now).await.is_empty(),
            "no longer due after reschedule"
        );
        let all = list_schedules(&db).await;
        assert_eq!(
            all.len(),
            2,
            "both still enabled (one repeating, one pending)"
        );
    }

    #[tokio::test]
    async fn one_shot_disables_after_running() {
        let db = crate::db::Database::new_in_memory().await.unwrap();
        let conv = crate::backend::add_conversation(&db, "t").await.unwrap();
        let _ = add_schedule(&db, conv, "once", 0, now_unix() - 1)
            .await
            .unwrap();
        let due = due_schedules(&db, now_unix()).await;
        assert_eq!(due.len(), 1);
        mark_ran(&db, &due[0], now_unix()).await;
        assert!(
            list_schedules(&db).await.is_empty(),
            "one-shot is disabled after running"
        );
    }
}
