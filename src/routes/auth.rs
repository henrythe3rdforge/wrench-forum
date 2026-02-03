use axum::{
    extract::State,
    response::Html,
    Form,
};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use std::sync::Arc;
use tera::{Context, Tera};

use crate::auth::{
    ensure_session, hash_password, verify_password, 
    create_session_token, session_expiry, set_session_cookie, clear_session_cookie,
    is_valid_email, is_valid_username, is_valid_password,
};
use crate::db::{self, Db};

#[derive(Deserialize)]
pub struct RegisterForm {
    pub email: String,
    pub username: String,
    pub password: String,
    pub password_confirm: String,
}

#[derive(Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
}

pub async fn register_page(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    // Redirect if already logged in
    if let Some((_, jar)) = ensure_session(jar.clone(), &db) {
        let html = r#"<script>window.location.href = "/";</script>"#.to_string();
        return (jar, Html(html));
    }
    
    let ctx = Context::new();
    let html = tera.render("register.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
    (jar, Html(html))
}

pub async fn register_submit(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
    Form(form): Form<RegisterForm>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    // Validation
    let mut errors = Vec::new();
    
    if !is_valid_email(&form.email) {
        errors.push("Invalid email address".to_string());
    }
    
    if !is_valid_username(&form.username) {
        errors.push("Username must be 3-20 characters, alphanumeric and underscores only".to_string());
    }
    
    if !is_valid_password(&form.password) {
        errors.push("Password must be at least 8 characters".to_string());
    }
    
    if form.password != form.password_confirm {
        errors.push("Passwords do not match".to_string());
    }
    
    if !errors.is_empty() {
        ctx.insert("errors", &errors);
        ctx.insert("email", &form.email);
        ctx.insert("username", &form.username);
        let html = tera.render("register.html", &ctx).unwrap();
        return (jar, Html(html));
    }
    
    let conn = db.lock().unwrap();
    
    // Check if email exists
    if db::get_user_by_email(&conn, &form.email).ok().flatten().is_some() {
        ctx.insert("errors", &vec!["Email already registered"]);
        ctx.insert("username", &form.username);
        let html = tera.render("register.html", &ctx).unwrap();
        return (jar, Html(html));
    }
    
    // Check if username exists
    if db::get_user_by_username(&conn, &form.username).ok().flatten().is_some() {
        ctx.insert("errors", &vec!["Username already taken"]);
        ctx.insert("email", &form.email);
        let html = tera.render("register.html", &ctx).unwrap();
        return (jar, Html(html));
    }
    
    // Create user
    let password_hash = match hash_password(&form.password) {
        Ok(h) => h,
        Err(_) => {
            ctx.insert("errors", &vec!["Failed to create account"]);
            let html = tera.render("register.html", &ctx).unwrap();
            return (jar, Html(html));
        }
    };
    
    let user_id = match db::create_user(&conn, &form.email, &password_hash, &form.username) {
        Ok(id) => id,
        Err(_) => {
            ctx.insert("errors", &vec!["Failed to create account"]);
            let html = tera.render("register.html", &ctx).unwrap();
            return (jar, Html(html));
        }
    };
    
    // Create session
    let token = create_session_token();
    let expiry = session_expiry();
    let _ = db::create_session(&conn, &token, user_id, &expiry);
    
    let jar = set_session_cookie(jar, &token);
    
    let html = r#"<script>window.location.href = "/verification";</script>"#.to_string();
    (jar, Html(html))
}

pub async fn login_page(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    // Redirect if already logged in
    if let Some((_, jar)) = ensure_session(jar.clone(), &db) {
        let html = r#"<script>window.location.href = "/";</script>"#.to_string();
        return (jar, Html(html));
    }
    
    let ctx = Context::new();
    let html = tera.render("login.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
    (jar, Html(html))
}

pub async fn login_submit(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
    Form(form): Form<LoginForm>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    let conn = db.lock().unwrap();
    
    // Find user
    let (user, password_hash) = match db::get_user_by_email(&conn, &form.email) {
        Ok(Some(data)) => data,
        _ => {
            ctx.insert("error", "Invalid email or password");
            ctx.insert("email", &form.email);
            let html = tera.render("login.html", &ctx).unwrap();
            return (jar, Html(html));
        }
    };
    
    // Verify password
    if !verify_password(&form.password, &password_hash) {
        ctx.insert("error", "Invalid email or password");
        ctx.insert("email", &form.email);
        let html = tera.render("login.html", &ctx).unwrap();
        return (jar, Html(html));
    }
    
    // Check if banned
    if user.banned {
        ctx.insert("error", "Your account has been banned");
        let html = tera.render("login.html", &ctx).unwrap();
        return (jar, Html(html));
    }
    
    // Create session
    let token = create_session_token();
    let expiry = session_expiry();
    let _ = db::create_session(&conn, &token, user.id, &expiry);
    
    // Log activity
    let _ = db::log_activity(&conn, user.id, "login", None, None, None, None);
    
    let jar = set_session_cookie(jar, &token);
    
    let html = r#"<script>window.location.href = "/";</script>"#.to_string();
    (jar, Html(html))
}

pub async fn logout(
    jar: CookieJar,
    State((db, _)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some(cookie) = jar.get("session") {
        let token = cookie.value().to_string();
        if let Ok(conn) = db.lock() {
            let _ = db::delete_session(&conn, &token);
        }
    }
    
    let jar = clear_session_cookie(jar);
    
    let html = r#"<script>window.location.href = "/";</script>"#.to_string();
    (jar, Html(html))
}
