use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite, Row};
use time::OffsetDateTime;
use ulid::Ulid;

pub type Db = Pool<Sqlite>;

#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    pub pw_hash: String,
    pub disabled: bool,
}

pub async fn connect_db(url: &str) -> anyhow::Result<Db> {
    Ok(SqlitePoolOptions::new().max_connections(10).connect(url).await?)
}

pub async fn migrate_db(pool: &Db) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

pub async fn create_user(pool: &Db, username: &str, pw_hash: &str) -> anyhow::Result<String> {
    let id = Ulid::new().to_string();
    let now = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query("INSERT INTO users(id, username, pw_hash, created_at, updated_at) VALUES(?,?,?,?,?)")
        .bind(&id).bind(username).bind(pw_hash).bind(now).bind(now)
        .execute(pool).await?;
    Ok(id)
}

pub async fn assign_role(pool: &Db, user_id: &str, role: &str) -> anyhow::Result<()> {
    sqlx::query("INSERT OR IGNORE INTO roles(name) VALUES(?)").bind(role).execute(pool).await?;
    sqlx::query("INSERT OR IGNORE INTO user_roles(user_id, role_name) VALUES(?,?)")
        .bind(user_id).bind(role).execute(pool).await?;
    Ok(())
}

pub async fn find_user_by_username(pool: &Db, username: &str) -> anyhow::Result<Option<User>> {
    let row = sqlx::query("SELECT id, username, pw_hash, disabled FROM users WHERE username=?")
        .bind(username).fetch_optional(pool).await?;
    Ok(row.map(|r| {
        let id: String = r.try_get(0).unwrap();
        let username: String = r.try_get(1).unwrap();
        let pw_hash: String = r.try_get(2).unwrap();
        let disabled_i: i64 = r.try_get(3).unwrap();
        User { id, username, pw_hash, disabled: disabled_i != 0 }
    }))
}

pub async fn roles_for_user(pool: &Db, user_id: &str) -> anyhow::Result<Vec<String>> {
    let rows = sqlx::query("SELECT role_name FROM user_roles WHERE user_id=?")
        .bind(user_id).fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| r.try_get::<String, _>(0).unwrap()).collect())
}

pub async fn create_session(pool: &Db, user_id: &str, ttl_secs: i64) -> anyhow::Result<String> {
    let id = Ulid::new().to_string();
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let exp = now + ttl_secs;
    sqlx::query("INSERT INTO sessions(id, user_id, created_at, expires_at) VALUES(?,?,?,?)")
        .bind(&id).bind(user_id).bind(now).bind(exp).execute(pool).await?;
    Ok(id)
}

pub struct SessionRecord { pub user_id: String, pub expires_at: i64, pub last_stepup: i64 }

pub async fn load_session(pool: &Db, sid: &str) -> anyhow::Result<Option<SessionRecord>> {
    let row = sqlx::query("SELECT user_id, expires_at, last_auth_stepup FROM sessions WHERE id=?")
        .bind(sid).fetch_optional(pool).await?;
    Ok(row.map(|r| SessionRecord {
        user_id: r.try_get::<String,_>(0).unwrap(),
        expires_at: r.try_get::<i64,_>(1).unwrap(),
        last_stepup: r.try_get::<i64,_>(2).unwrap(),
    }))
}

pub async fn touch_stepup(pool: &Db, sid: &str) -> anyhow::Result<()> {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query("UPDATE sessions SET last_auth_stepup=? WHERE id=?")
        .bind(now).bind(sid).execute(pool).await?;
    Ok(())
}

pub async fn delete_session(pool: &Db, sid: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM sessions WHERE id=?").bind(sid).execute(pool).await?;
    Ok(())
}



pub async fn record_login_attempt(pool: &Db, username: &str, ip: &str) -> anyhow::Result<()> {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query("INSERT INTO login_attempts(username, ts, ip) VALUES(?,?,?)")
        .bind(username).bind(now).bind(ip).execute(pool).await?;
    Ok(())
}

pub async fn login_counts(pool: &Db, username: &str, ip: &str, window_secs: i64) -> anyhow::Result<(i64, i64)> {
    let since = OffsetDateTime::now_utc().unix_timestamp() - window_secs;
    let cnt_user_ip: i64 = sqlx::query("SELECT COUNT(*) FROM login_attempts WHERE username=? AND ip=? AND ts>?")
        .bind(username).bind(ip).bind(since).fetch_one(pool).await?.try_get(0).unwrap();
    let cnt_ip: i64 = sqlx::query("SELECT COUNT(*) FROM login_attempts WHERE ip=? AND ts>?")
        .bind(ip).bind(since).fetch_one(pool).await?.try_get(0).unwrap();
    Ok((cnt_user_ip, cnt_ip))
}



pub async fn audit_record(pool: &Db, actor_user: &str, action: &str, target: &str, ip: &str, ua: &str, details_json: &str) -> anyhow::Result<()> {
    let id = Ulid::new().to_string();
    let ts = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query("INSERT INTO audit(id, ts, actor_user, action, target, ip, ua, details) VALUES(?,?,?,?,?,?,?,?)")
        .bind(id).bind(ts).bind(actor_user).bind(action).bind(target).bind(ip).bind(ua).bind(details_json)
        .execute(pool).await?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct AuditRow {
    pub ts: i64,
    pub actor_user: String,
    pub action: String,
    pub target: String,
    pub ip: String,
    pub ua: String,
    pub details: String,
}

pub async fn audit_list(pool: &Db, limit: i64, offset: i64) -> anyhow::Result<Vec<AuditRow>> {
    let limit = limit.clamp(1, 200);
    let rows = sqlx::query("SELECT ts, actor_user, action, target, ip, ua, details FROM audit ORDER BY ts DESC LIMIT ? OFFSET ?")
        .bind(limit).bind(offset).fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| AuditRow {
        ts: r.try_get(0).unwrap(),
        actor_user: r.try_get(1).unwrap(),
        action: r.try_get(2).unwrap(),
        target: r.try_get(3).unwrap(),
        ip: r.try_get(4).unwrap(),
        ua: r.try_get(5).unwrap(),
        details: r.try_get(6).unwrap(),
    }).collect())
}
