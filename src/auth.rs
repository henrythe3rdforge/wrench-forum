use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::db::{self, Db};
use crate::models::User;

// Re-export time crate types for cookie duration
mod time {
    pub use ::cookie::time::Duration;
}

/// Hash a password using Argon2id
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

/// Generate a new session token
pub fn create_session_token() -> String {
    Uuid::new_v4().to_string()
}

/// Get session expiry timestamp (30 days from now)
pub fn session_expiry() -> String {
    (Utc::now() + Duration::days(30)).format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Check if a session is valid and return the user if so
pub fn ensure_session(jar: CookieJar, db: &Db) -> Option<(User, CookieJar)> {
    let token = jar.get("session")?.value().to_string();
    let conn = db.lock().ok()?;
    
    let session = db::get_session(&conn, &token).ok()??;
    
    // Check expiry
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    if session.expires_at < now {
        let _ = db::delete_session(&conn, &token);
        return None;
    }
    
    let user = db::get_user_by_id(&conn, session.user_id).ok()??;
    
    if user.banned {
        return None;
    }
    
    Some((user, jar))
}

/// Set the session cookie
pub fn set_session_cookie(jar: CookieJar, token: &str) -> CookieJar {
    let cookie = Cookie::build(("session", token.to_string()))
        .path("/")
        .http_only(true)
        .max_age(time::Duration::days(30));
    jar.add(cookie)
}

/// Clear the session cookie
pub fn clear_session_cookie(jar: CookieJar) -> CookieJar {
    jar.remove(Cookie::from("session"))
}

/// Validate email format (basic check)
pub fn is_valid_email(email: &str) -> bool {
    let email = email.trim();
    if email.is_empty() || email.len() > 254 {
        return false;
    }
    
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    
    let local = parts[0];
    let domain = parts[1];
    
    !local.is_empty() && !domain.is_empty() && domain.contains('.')
}

/// Validate username (alphanumeric, underscores, 3-20 chars)
pub fn is_valid_username(username: &str) -> bool {
    let username = username.trim();
    if username.len() < 3 || username.len() > 20 {
        return false;
    }
    
    username.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Validate password strength
pub fn is_valid_password(password: &str) -> bool {
    password.len() >= 8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_password() {
        let password = "my_secure_password_123";
        let hash = hash_password(password).expect("Failed to hash password");
        
        assert!(verify_password(password, &hash));
        assert!(!verify_password("wrong_password", &hash));
    }

    #[test]
    fn test_session_token_uniqueness() {
        let token1 = create_session_token();
        let token2 = create_session_token();
        
        assert_ne!(token1, token2);
        assert_eq!(token1.len(), 36); // UUID format
    }

    #[test]
    fn test_email_validation() {
        assert!(is_valid_email("test@example.com"));
        assert!(is_valid_email("user.name@sub.domain.org"));
        assert!(!is_valid_email(""));
        assert!(!is_valid_email("invalid"));
        assert!(!is_valid_email("@example.com"));
        assert!(!is_valid_email("test@"));
        assert!(!is_valid_email("test@nodot"));
    }

    #[test]
    fn test_username_validation() {
        assert!(is_valid_username("john_doe"));
        assert!(is_valid_username("User123"));
        assert!(is_valid_username("abc"));
        assert!(!is_valid_username("ab")); // too short
        assert!(!is_valid_username("a".repeat(21).as_str())); // too long
        assert!(!is_valid_username("user@name")); // invalid char
        assert!(!is_valid_username("user name")); // spaces not allowed
    }

    #[test]
    fn test_password_validation() {
        assert!(is_valid_password("password123"));
        assert!(is_valid_password("12345678"));
        assert!(!is_valid_password("short"));
        assert!(!is_valid_password("1234567")); // 7 chars
    }
}
