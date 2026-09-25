//! Email/password auth: signup + login with argon2id password hashes.
//!
//! In-memory users (same MVP posture as the other stores — HashMap behind a
//! mutex, no persistence). Passwords are never stored nor logged in plaintext;
//! only the PHC hash string is kept. Successful signup/login mints a standard
//! backend session JWT (`provider: "email"`, `sub: "email:<addr>"`), accepted
//! by `require_auth` like the provider sessions.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Utc};
use rand_core::OsRng;
use serde::Serialize;

pub const MAX_EMAIL_LEN: usize = 254;
pub const MAX_PASSWORD_LEN: usize = 128;
pub const MIN_PASSWORD_LEN: usize = 8;
pub const MAX_NAME_LEN: usize = 64;

#[derive(Debug, Clone)]
struct User {
    email: String,
    name: Option<String>,
    password_hash: String,
    #[allow(dead_code)]
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EmailSession {
    pub provider: String,
    pub email: String,
    pub name: Option<String>,
    pub session_token: String,
    pub expires_in: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmailError {
    InvalidEmail,
    InvalidPassword,
    InvalidName,
    Exists,
    InvalidCredentials,
    Internal,
}

impl std::fmt::Display for EmailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Generic messages only — never echo emails, names, or password
        // material back to clients (prevents enumeration/oracle).
        match self {
            Self::InvalidEmail => write!(f, "invalid email"),
            Self::InvalidPassword => write!(
                f,
                "password must be {MIN_PASSWORD_LEN}..{MAX_PASSWORD_LEN} characters"
            ),
            Self::InvalidName => write!(f, "invalid name"),
            Self::Exists => write!(f, "account already exists"),
            Self::InvalidCredentials => write!(f, "invalid email or password"),
            Self::Internal => write!(f, "internal error"),
        }
    }
}
impl std::error::Error for EmailError {}

static USER_STORE: OnceLock<Mutex<HashMap<String, User>>> = OnceLock::new();

fn user_store() -> &'static Mutex<HashMap<String, User>> {
    USER_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_user_store() -> std::sync::MutexGuard<'static, HashMap<String, User>> {
    match user_store().lock() {
        Ok(g) => g,
        Err(poisoned) => {
            tracing::warn!("user store mutex poisoned; recovering inner");
            poisoned.into_inner()
        }
    }
}

/// Normalize for lookup + storage: trim + lowercase.
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

fn valid_email(email: &str) -> bool {
    if email.is_empty()
        || email.len() > MAX_EMAIL_LEN
        || email
            .bytes()
            .any(|b| b.is_ascii_whitespace() || b.is_ascii_control())
    {
        return false;
    }
    let mut parts = email.split('@');
    let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    if local.is_empty() || local.len() > 64 || domain.len() < 3 {
        return false;
    }
    // Bare-minimum domain shape: dot inside, no leading/trailing dot/dash.
    if !domain.contains('.') || domain.starts_with(['.', '-']) || domain.ends_with(['.', '-']) {
        return false;
    }
    true
}

fn clean_name(name: Option<&str>) -> Result<Option<String>, EmailError> {
    let Some(raw) = name else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.len() > MAX_NAME_LEN || trimmed.chars().any(|c| c.is_control()) {
        return Err(EmailError::InvalidName);
    }
    Ok(Some(trimmed.to_string()))
}

fn hash_password(password: &str) -> Result<String, EmailError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| {
            tracing::warn!("argon2 hash failed");
            EmailError::Internal
        })
}

fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

fn mint(email: &str, name: Option<&str>) -> EmailSession {
    let sub = format!("email:{email}");
    let session_token = crate::session::mint_session("email", &sub, name, Some(email));
    EmailSession {
        provider: "email".to_string(),
        email: email.to_string(),
        name: name.map(|s| s.to_string()),
        session_token,
        expires_in: crate::session::SESSION_TTL_SECS,
    }
}

/// Create an account. Fails closed: invalid input → 400, duplicate → 409.
pub fn signup(email: &str, password: &str, name: Option<&str>) -> Result<EmailSession, EmailError> {
    let normalized = normalize_email(email);
    if !valid_email(&normalized) {
        return Err(EmailError::InvalidEmail);
    }
    if password.len() < MIN_PASSWORD_LEN || password.len() > MAX_PASSWORD_LEN {
        return Err(EmailError::InvalidPassword);
    }
    let clean = clean_name(name)?;
    let mut store = lock_user_store();
    if store.contains_key(&normalized) {
        return Err(EmailError::Exists);
    }
    let password_hash = hash_password(password)?;
    store.insert(
        normalized.clone(),
        User {
            email: normalized.clone(),
            name: clean.clone(),
            password_hash,
            created_at: Utc::now(),
        },
    );
    Ok(mint(&normalized, clean.as_deref()))
}

/// Log in. Unknown email and wrong password map to the same error (no oracle).
pub fn login(email: &str, password: &str) -> Result<EmailSession, EmailError> {
    let normalized = normalize_email(email);
    if normalized.is_empty() || normalized.len() > MAX_EMAIL_LEN {
        return Err(EmailError::InvalidCredentials);
    }
    let store = lock_user_store();
    let Some(user) = store.get(&normalized) else {
        return Err(EmailError::InvalidCredentials);
    };
    if !verify_password(password, &user.password_hash) {
        return Err(EmailError::InvalidCredentials);
    }
    Ok(mint(&user.email, user.name.as_deref()))
}

/// Clear users (tests).
#[allow(dead_code)]
pub fn clear_user_store() {
    lock_user_store().clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_sync;

    fn before() -> impl Drop {
        let guard = test_sync::lock();
        clear_user_store();
        guard
    }

    #[test]
    fn signup_mints_verifiable_session() {
        let _guard = before();
        let s = signup("Ada@Example.com ", "correct-horse-01", Some("Ada"))
            .expect("signup should succeed");
        assert_eq!(s.provider, "email");
        assert_eq!(s.email, "ada@example.com");
        assert_eq!(s.name.as_deref(), Some("Ada"));
        assert_eq!(s.expires_in, crate::session::SESSION_TTL_SECS);
        let claims = crate::session::verify_session(&s.session_token).expect("session must verify");
        assert_eq!(claims.sub, "email:ada@example.com");
        assert_eq!(claims.provider, "email");
    }

    #[test]
    fn signup_duplicate_rejected_case_insensitive() {
        let _guard = before();
        signup("dup@example.com", "password-01", None).expect("first ok");
        let err = signup("DUP@example.com", "password-02", None).unwrap_err();
        assert_eq!(err, EmailError::Exists);
    }

    #[test]
    fn signup_rejects_bad_email_and_password() {
        let _guard = before();
        for bad in [
            "",
            "no-at",
            "a@b",
            "@example.com",
            "a@.com",
            "a b@example.com",
            "a@example",
        ] {
            assert_eq!(
                signup(bad, "password-01", None).unwrap_err(),
                EmailError::InvalidEmail,
                "email {bad:?}"
            );
        }
        assert_eq!(
            signup("ok@example.com", "short", None).unwrap_err(),
            EmailError::InvalidPassword
        );
        assert_eq!(
            signup(
                "ok2@example.com",
                "password-01",
                Some("x".repeat(65).as_str())
            )
            .unwrap_err(),
            EmailError::InvalidName
        );
    }

    #[test]
    fn blank_name_becomes_none() {
        let _guard = before();
        let s = signup("noname@example.com", "password-01", Some("   ")).expect("signup ok");
        assert_eq!(s.name, None);
    }

    #[test]
    fn login_ok_and_no_oracle() {
        let _guard = before();
        signup("user@example.com", "password-01", None).expect("signup ok");
        let s = login("USER@example.com", "password-01").expect("login ok");
        assert_eq!(s.email, "user@example.com");
        // Wrong password and unknown email look identical.
        assert_eq!(
            login("user@example.com", "wrong-password").unwrap_err(),
            EmailError::InvalidCredentials
        );
        assert_eq!(
            login("nobody@example.com", "password-01").unwrap_err(),
            EmailError::InvalidCredentials
        );
    }

    #[test]
    fn stored_hash_is_not_plaintext() {
        let _guard = before();
        signup("hash@example.com", "password-01", None).expect("signup ok");
        let store = lock_user_store();
        let user = store.get("hash@example.com").expect("user stored");
        assert!(user.password_hash.starts_with("$argon2"));
        assert!(!user.password_hash.contains("password-01"));
    }
}
