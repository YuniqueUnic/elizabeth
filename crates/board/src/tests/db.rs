use crate::{
    constants::database::BUSY_TIMEOUT_SECS,
    db::{DbPoolSettings, backup_sqlite_before_migrations, init_db},
};

#[tokio::test]
async fn sqlite_pool_initializes() {
    let pool = DbPoolSettings::new("sqlite::memory:")
        .with_max_connections(1)
        .with_min_connections(1)
        .create_pool()
        .await;

    assert!(pool.is_ok());
}

#[tokio::test]
async fn sqlite_pool_applies_busy_timeout_to_every_connection() -> anyhow::Result<()> {
    let database = tempfile::NamedTempFile::new()?;
    let url = format!("sqlite://{}?mode=rwc", database.path().display());
    let pool = init_db(
        &DbPoolSettings::new(url)
            .with_max_connections(3)
            .with_min_connections(3),
    )
    .await?;

    let mut connections = Vec::new();
    for _ in 0..3 {
        connections.push(pool.acquire().await?);
    }

    for connection in &mut connections {
        let timeout_ms: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
            .fetch_one(&mut **connection)
            .await?;
        assert_eq!(timeout_ms, (BUSY_TIMEOUT_SECS * 1_000) as i64);
        let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&mut **connection)
            .await?;
        assert_eq!(foreign_keys, 1);
    }

    Ok(())
}

#[tokio::test]
async fn sqlite_migration_backups_keep_the_latest_three_iterations() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("app.db");
    let url = format!("sqlite://{}?mode=rwc", database.display());
    let pool = init_db(
        &DbPoolSettings::new(url.clone())
            .with_max_connections(1)
            .with_min_connections(1),
    )
    .await?;
    sqlx::query("CREATE TABLE data (id INTEGER PRIMARY KEY, value TEXT NOT NULL)")
        .execute(&pool)
        .await?;

    for iteration in 1..=4 {
        sqlx::query("INSERT INTO data (value) VALUES ($1)")
            .bind(format!("iteration-{iteration}"))
            .execute(&pool)
            .await?;
        backup_sqlite_before_migrations(&pool, &url)
            .await?
            .expect("existing database should be backed up");
    }

    assert!(!directory.path().join("app.db.bkp1").exists());
    for iteration in 2..=4 {
        assert!(
            directory
                .path()
                .join(format!("app.db.bkp{iteration}"))
                .exists()
        );
    }
    Ok(())
}
