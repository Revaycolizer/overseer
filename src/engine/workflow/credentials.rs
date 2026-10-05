use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use sha2::{Digest, Sha256};
use std::io::Write;

// Each installation owns its key. Publish a complete key atomically so concurrent
// daemon/TUI startups cannot read a partially written key or overwrite one another.
fn encryption_key() -> anyhow::Result<Key<Aes256Gcm>> {
    encryption_key_at(&crate::db::get_overseer_data_dir())
}

fn encryption_key_at(directory: &std::path::Path) -> anyhow::Result<Key<Aes256Gcm>> {
    std::fs::create_dir_all(&directory)?;
    let path = directory.join("credentials.key");
    if !path.exists() {
        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes)?;
        let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            temporary
                .as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        temporary.write_all(&bytes)?;
        temporary.as_file().sync_all()?;
        if let Err(error) = std::fs::hard_link(temporary.path(), &path) {
            if error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(error.into());
            }
        }
    }
    let bytes = std::fs::read(&path)?;
    anyhow::ensure!(
        bytes.len() == 32,
        "Invalid installation credential key; restore credentials.key from backup"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(Key::<Aes256Gcm>::from_slice(&bytes).to_owned())
}

fn encrypt_with_key(plaintext: &str, key: &Key<Aes256Gcm>) -> anyhow::Result<String> {
    let cipher = Aes256Gcm::new(key);
    let mut nonce_bytes = [0u8; 12];
    getrandom::getrandom(&mut nonce_bytes)?;
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_bytes())
        .map_err(|e| anyhow::anyhow!("encrypt: {e}"))?;
    let mut result = nonce_bytes.to_vec();
    result.extend(ciphertext);
    Ok(hex::encode(result))
}

pub fn encrypt(plaintext: &str) -> anyhow::Result<String> {
    Ok(format!(
        "v2:{}",
        encrypt_with_key(plaintext, &encryption_key()?)?
    ))
}

fn decrypt_with_key(encoded: &str, key: &Key<Aes256Gcm>) -> anyhow::Result<String> {
    let data = hex::decode(encoded)?;
    anyhow::ensure!(data.len() >= 28, "invalid ciphertext");
    let (nonce, ciphertext) = data.split_at(12);
    let plaintext = Aes256Gcm::new(key)
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|e| anyhow::anyhow!("decrypt: {e}"))?;
    Ok(String::from_utf8(plaintext)?)
}

pub fn decrypt(encoded: &str) -> anyhow::Result<String> {
    if let Some(data) = encoded.strip_prefix("v2:") {
        anyhow::ensure!(
            crate::db::get_overseer_data_dir()
                .join("credentials.key")
                .exists(),
            "Missing credential key; restore credentials.key from backup"
        );
        decrypt_with_key(data, &encryption_key()?)
    } else {
        // Compatibility only: new and updated credentials always use the local key.
        let legacy = Sha256::digest(b"overseer-local-cred-key-2026");
        decrypt_with_key(encoded, Key::<Aes256Gcm>::from_slice(&legacy))
    }
}

/// Upgrade existing installations without dropping their saved credentials.
pub async fn migrate_legacy(db: &crate::db::Database) -> anyhow::Result<()> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, encrypted_value FROM credentials WHERE encrypted_value NOT LIKE 'v2:%'",
    )
    .fetch_all(&db.pool)
    .await?;
    let mut transaction = db.pool.begin().await?;
    for (id, old) in rows {
        let upgraded = encrypt(&decrypt(&old)?)?;
        sqlx::query("UPDATE credentials SET encrypted_value=? WHERE id=? AND encrypted_value=?")
            .bind(upgraded)
            .bind(id)
            .bind(old)
            .execute(&mut *transaction)
            .await?;
    }
    let settings: Vec<(String, String)> = sqlx::query_as("SELECT key,value FROM settings WHERE key IN ('openai_api_key','anthropic_api_key') AND value!='' AND value NOT LIKE 'v2:%'")
        .fetch_all(&mut *transaction).await?;
    for (key, value) in settings {
        sqlx::query("UPDATE settings SET value=? WHERE key=?")
            .bind(encrypt(&value)?)
            .bind(key)
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encryption_rejects_wrong_keys_corruption_and_short_input() {
        let a = Key::<Aes256Gcm>::from_slice(&[1; 32]);
        let b = Key::<Aes256Gcm>::from_slice(&[2; 32]);
        let encoded = encrypt_with_key("test credential", a).unwrap();
        assert_eq!(decrypt_with_key(&encoded, a).unwrap(), "test credential");
        assert!(decrypt_with_key(&encoded, b).is_err());
        assert!(decrypt_with_key("abcd", a).is_err());
        let mut data = hex::decode(encoded).unwrap();
        data[15] ^= 1;
        assert!(decrypt_with_key(&hex::encode(data), a).is_err());
    }
    #[test]
    fn installation_keys_are_stable_private_and_distinct() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let key = encryption_key_at(first.path()).unwrap();
        assert_eq!(key, encryption_key_at(first.path()).unwrap());
        assert_ne!(key, encryption_key_at(second.path()).unwrap());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(first.path().join("credentials.key"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
    }
    #[test]
    fn legacy_credentials_remain_readable() {
        let key = Sha256::digest(b"overseer-local-cred-key-2026");
        let encoded = encrypt_with_key("legacy value", Key::<Aes256Gcm>::from_slice(&key)).unwrap();
        assert_eq!(decrypt(&encoded).unwrap(), "legacy value");
    }
}
