use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use sha2::{Sha256, Digest};

fn encryption_key() -> Key<Aes256Gcm> {
    let hash = Sha256::digest(b"overseer-local-cred-key-2026");
    Key::<Aes256Gcm>::from_slice(&hash).clone()
}

pub fn encrypt(plaintext: &str) -> anyhow::Result<String> {
    let cipher = Aes256Gcm::new(&encryption_key());
    let mut nonce_bytes = [0u8; 12];
    getrandom::getrandom(&mut nonce_bytes)?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| anyhow::anyhow!("encrypt: {e}"))?;
    let mut result = nonce_bytes.to_vec();
    result.extend(ciphertext);
    Ok(hex::encode(result))
}

pub fn decrypt(hex_data: &str) -> anyhow::Result<String> {
    let data = hex::decode(hex_data)?;
    if data.len() < 12 { anyhow::bail!("invalid ciphertext"); }
    let (nonce_bytes, ciphertext) = data.split_at(12);
    let cipher = Aes256Gcm::new(&encryption_key());
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("decrypt: {e}"))?;
    Ok(String::from_utf8(plaintext)?)
}
