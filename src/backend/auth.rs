use crate::db::Database;
use anyhow::Context;
use argon2::Argon2;

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    getrandom::getrandom(&mut buf).expect("getrandom failed");
    buf
}

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = random_bytes::<16>();
    let mut hash = [0u8; 32];
    Argon2::default()
        .hash_password_into(password.as_bytes(), &salt, &mut hash)
        .map_err(|e| anyhow::anyhow!("argon2: {e}"))?;
    Ok(format!("{}:{}", hex::encode(salt), hex::encode(hash)))
}

pub fn verify_password(password: &str, stored: &str) -> anyhow::Result<bool> {
    let parts: Vec<&str> = stored.split(':').collect();
    if parts.len() != 2 {
        return Ok(false);
    }
    let salt = hex::decode(parts[0])?;
    let expected = hex::decode(parts[1])?;
    let mut hash = [0u8; 32];
    Argon2::default()
        .hash_password_into(password.as_bytes(), &salt, &mut hash)
        .map_err(|e| anyhow::anyhow!("argon2: {e}"))?;
    Ok(hash.as_slice() == expected.as_slice())
}

pub fn generate_session_token() -> String {
    hex::encode(random_bytes::<32>())
}

pub async fn create_session(db: &Database, user_id: i64) -> anyhow::Result<String> {
    let token = generate_session_token();
    let id = db.next_id().await;
    let expires = chrono::Utc::now() + chrono::Duration::days(30);
    sqlx::query("INSERT INTO user_sessions (id, token, user_id, expires_at) VALUES (?, ?, ?, ?)")
        .bind(id)
        .bind(&token)
        .bind(user_id)
        .bind(expires.to_rfc3339())
        .execute(&db.pool)
        .await?;
    Ok(token)
}
