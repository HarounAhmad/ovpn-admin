use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use argon2::password_hash::{rand_core::OsRng, SaltString};

pub fn hash_password(pw: &str, pepper: &[u8]) -> anyhow::Result<String> {
    let params = Params::new(64 * 1024, 2, 1, None)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let a2 = Argon2::new_with_secret(pepper, Algorithm::Argon2id, Version::V0x13, params)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let salt = SaltString::generate(&mut OsRng);
    let phc = a2.hash_password(pw.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?
        .to_string();
    Ok(phc)
}

pub fn verify_password(pw: &str, phc: &str, pepper: &[u8]) -> bool {
    let params = Params::new(64 * 1024, 2, 1, None).unwrap_or_else(|_| Params::default());
    let a2 = match Argon2::new_with_secret(pepper, Algorithm::Argon2id, Version::V0x13, params) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let parsed = match PasswordHash::new(phc) {
        Ok(p) => p,
        Err(_) => return false,
    };
    a2.verify_password(pw.as_bytes(), &parsed).is_ok()
}