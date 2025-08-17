use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};

pub type Db = Pool<Sqlite>;

pub async fn connect_db(url: &str) -> anyhow::Result<Db> {
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect(url)
        .await?;
    Ok(pool)
}

pub async fn migrate_db(pool: &Db) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}
