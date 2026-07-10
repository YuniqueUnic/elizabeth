use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use std::sync::Arc;

use crate::db::DbPool;
use crate::models::room::row_utils::format_naive_datetime;
use crate::models::{RoomRefreshToken, RoomToken};

#[derive(Clone)]
pub struct RoomAccessRepository {
    pool: Arc<DbPool>,
}

impl RoomAccessRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    /// Atomically consumes one new-session grant and persists its access token.
    pub async fn grant_new_session(
        &self,
        room_id: i64,
        token: &RoomToken,
        refresh_token: Option<&RoomRefreshToken>,
        now: NaiveDateTime,
    ) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        let now = format_naive_datetime(now);
        let updated = sqlx::query(
            r#"
            UPDATE rooms
            SET current_times_entered = current_times_entered + 1,
                updated_at = $1
            WHERE id = $2
              AND status = 0
              AND current_times_entered < max_times_entered
              AND (expire_at IS NULL OR CAST(expire_at AS TEXT) > $1)
            "#,
        )
        .bind(&now)
        .bind(room_id)
        .execute(&mut *tx)
        .await
        .context("failed to consume room entry grant")?;

        if updated.rows_affected() == 0 {
            tx.rollback().await?;
            return Ok(false);
        }

        insert_token(&mut tx, token, &now).await?;
        if let Some(refresh_token) = refresh_token {
            insert_refresh_token(&mut tx, refresh_token).await?;
        }
        tx.commit().await?;
        Ok(true)
    }

    /// Rotates an existing access token without consuming another entry grant.
    pub async fn rotate_access_token(
        &self,
        room_id: i64,
        previous_jti: &str,
        token: &RoomToken,
        refresh_token: Option<&RoomRefreshToken>,
        now: NaiveDateTime,
    ) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        let now = format_naive_datetime(now);
        let room_available: i64 = sqlx::query_scalar(
            r#"
            SELECT CASE WHEN EXISTS(
                SELECT 1 FROM rooms
                WHERE id = $1
                  AND status = 0
                  AND (expire_at IS NULL OR CAST(expire_at AS TEXT) > $2)
            ) THEN 1 ELSE 0 END
            "#,
        )
        .bind(room_id)
        .bind(&now)
        .fetch_one(&mut *tx)
        .await?;

        if room_available == 0 {
            tx.rollback().await?;
            return Ok(false);
        }

        insert_token(&mut tx, token, &now).await?;
        if let Some(refresh_token) = refresh_token {
            insert_refresh_token(&mut tx, refresh_token).await?;
        }
        sqlx::query(
            "UPDATE room_tokens SET revoked_at = $1 WHERE room_id = $2 AND jti = $3 AND revoked_at IS NULL",
        )
        .bind(&now)
        .bind(room_id)
        .bind(previous_jti)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(true)
    }

    /// Rotates a refresh-backed session against the live room policy.
    pub async fn rotate_refresh_session(
        &self,
        room_id: i64,
        previous_access_jti: &str,
        previous_refresh_id: i64,
        new_access_token: &RoomToken,
        new_refresh_token: Option<&RoomRefreshToken>,
        now: NaiveDateTime,
    ) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        let now = format_naive_datetime(now);
        let available: i64 = sqlx::query_scalar(
            r#"
            SELECT CASE WHEN EXISTS(
                SELECT 1 FROM rooms
                WHERE id = $1
                  AND status = 0
                  AND (expire_at IS NULL OR expire_at > $2)
            ) THEN 1 ELSE 0 END
            "#,
        )
        .bind(room_id)
        .bind(&now)
        .fetch_one(&mut *tx)
        .await?;

        if available == 0 {
            tx.rollback().await?;
            return Ok(false);
        }

        insert_token(&mut tx, new_access_token, &now).await?;
        sqlx::query(
            "UPDATE room_tokens SET revoked_at = $1 WHERE room_id = $2 AND jti = $3 AND revoked_at IS NULL",
        )
        .bind(&now)
        .bind(room_id)
        .bind(previous_access_jti)
        .execute(&mut *tx)
        .await?;

        if let Some(new_refresh_token) = new_refresh_token {
            sqlx::query(
                "UPDATE room_refresh_tokens SET is_revoked = TRUE, last_used_at = $1 WHERE id = $2 AND room_id = $3 AND is_revoked = FALSE",
            )
            .bind(&now)
            .bind(previous_refresh_id)
            .bind(room_id)
            .execute(&mut *tx)
            .await?;
            insert_refresh_token(&mut tx, new_refresh_token).await?;
        } else {
            sqlx::query(
                "UPDATE room_refresh_tokens SET access_token_jti = $1, last_used_at = $2 WHERE id = $3 AND room_id = $4 AND is_revoked = FALSE",
            )
            .bind(&new_access_token.jti)
            .bind(&now)
            .bind(previous_refresh_id)
            .bind(room_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(true)
    }
}

async fn insert_token(
    tx: &mut sqlx::Transaction<'_, sqlx::Any>,
    token: &RoomToken,
    created_at: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO room_tokens (room_id, jti, expires_at, revoked_at, created_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(token.room_id)
    .bind(&token.jti)
    .bind(format_naive_datetime(token.expires_at))
    .bind(token.revoked_at.map(format_naive_datetime))
    .bind(created_at)
    .execute(&mut **tx)
    .await
    .context("failed to persist granted room token")?;
    Ok(())
}

async fn insert_refresh_token(
    tx: &mut sqlx::Transaction<'_, sqlx::Any>,
    token: &RoomRefreshToken,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO room_refresh_tokens (
            room_id, access_token_jti, token_hash, expires_at,
            created_at, last_used_at, is_revoked
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(token.room_id)
    .bind(&token.access_token_jti)
    .bind(&token.token_hash)
    .bind(format_naive_datetime(token.expires_at))
    .bind(format_naive_datetime(token.created_at))
    .bind(token.last_used_at.map(format_naive_datetime))
    .bind(token.is_revoked)
    .execute(&mut **tx)
    .await
    .context("failed to persist room refresh token")?;
    Ok(())
}
