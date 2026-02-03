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

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

pub fn create_session_token() -> String {
    Uuid::new_v4().to_string()
}

pub fn session_expiry() -> String {
    (Utc::now() + Duration::days(30)).format("%Y-%m-%d %H:%M:%S").to_string()
}

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

pub fn set_session_cookie(jar: CookieJar, token: &str) -> CookieJar {
    let cookie = Cookie::build(("session", token.to_string()))
        .path("/")
        .http_only(true)
        .max_age(time::Duration::days(30));
    jar.add(cookie)
}

pub fn clear_session_cookie(jar: CookieJar) -> CookieJar {
    jar.remove(Cookie::from("session"))
}
