// src/db/backfill.rs — backfill_ids()
use super::Database;
use sqlx::Acquire;

impl Database {
    pub(super) async fn backfill_ids(&self) -> anyhow::Result<()> {
        let mut conn = self.pool.acquire().await?;

        // Disable foreign keys temporarily for ID migration
        sqlx::query("PRAGMA foreign_keys = OFF")
            .execute(&mut *conn)
            .await?;

        let mut tx = conn.begin().await?;

        // println!("🔄 Backfilling Missing Snowflake IDs...");

        // 1. Backfill settings
        let settings_to_backfill: Vec<(String,)> =
            sqlx::query_as("SELECT key FROM settings WHERE id IS NULL")
                .fetch_all(&mut *tx)
                .await?;
        for (key,) in settings_to_backfill {
            let id = self.next_id().await;
            sqlx::query("UPDATE settings SET id = ? WHERE key = ?")
                .bind(id)
                .bind(key)
                .execute(&mut *tx)
                .await?;
        }

        // 2. Backfill agent_tools
        let agent_tools_to_backfill: Vec<(i64, i64)> =
            sqlx::query_as("SELECT agent_id, tool_id FROM agent_tools WHERE id IS NULL")
                .fetch_all(&mut *tx)
                .await?;
        for (a_id, t_id) in agent_tools_to_backfill {
            let id = self.next_id().await;
            sqlx::query("UPDATE agent_tools SET id = ? WHERE agent_id = ? AND tool_id = ?")
                .bind(id)
                .bind(a_id)
                .bind(t_id)
                .execute(&mut *tx)
                .await?;
        }

        // 3. Backfill agents with sequential IDs (and update foreign keys)
        let agents_to_backfill: Vec<(i64,)> =
            sqlx::query_as("SELECT id FROM agents WHERE id < 1000000000000000000")
                .fetch_all(&mut *tx)
                .await?;

        for (old_id,) in agents_to_backfill {
            let new_id = self.next_id().await;

            // Update foreign keys in agent_tools
            sqlx::query("UPDATE agent_tools SET agent_id = ? WHERE agent_id = ?")
                .bind(new_id)
                .bind(old_id)
                .execute(&mut *tx)
                .await?;

            // Update main agent ID
            sqlx::query("UPDATE agents SET id = ? WHERE id = ?")
                .bind(new_id)
                .bind(old_id)
                .execute(&mut *tx)
                .await?;

            //    println!("✅ Migrated agent ID {} -> {}", old_id, new_id);
        }

        tx.commit().await?;

        // Re-enable foreign keys
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&mut *conn)
            .await?;
        Ok(())
    }
}
