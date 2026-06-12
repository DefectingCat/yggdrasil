//! 密码哈希与校验（Argon2）。
//!
//! 使用随机 salt 生成 PHC 字符串格式哈希，并通过 Argon2 验证。
//! `#[allow(dead_code)]` 用于避免 WASM 构建中服务端函数体被剥离后的未使用警告。

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::rngs::OsRng;

#[allow(dead_code)]
/// 使用 Argon2 对明文密码进行哈希。
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(password_hash.to_string())
}

#[allow(dead_code)]
/// 校验明文密码是否与已存储的哈希匹配。
pub fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(hash)?;
    let argon2 = Argon2::default();
    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_roundtrip() {
        let hash = hash_password("mypassword123").unwrap();
        assert!(verify_password("mypassword123", &hash).unwrap());
    }

    #[test]
    fn verify_wrong_password_returns_false() {
        let hash = hash_password("correctpassword").unwrap();
        assert!(!verify_password("wrongpassword", &hash).unwrap());
    }

    #[test]
    fn different_hashes_for_same_password() {
        let hash1 = hash_password("samepassword").unwrap();
        let hash2 = hash_password("samepassword").unwrap();
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn verify_invalid_hash_returns_error() {
        let result = verify_password("password", "not-a-valid-hash");
        assert!(result.is_err());
    }

    #[test]
    fn hash_empty_password() {
        let hash = hash_password("").unwrap();
        assert!(verify_password("", &hash).unwrap());
    }

    #[test]
    fn hash_long_password() {
        let long_pw = "a".repeat(1000);
        let hash = hash_password(&long_pw).unwrap();
        assert!(verify_password(&long_pw, &hash).unwrap());
    }
}
