use axum::{
    extract::State,
    response::{Html, Redirect},
    Form,
};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use std::sync::Arc;
use tera::{Context, Tera};

use crate::auth::{
    clear_session_cookie, create_session_token, ensure_session, hash_password, session_expiry,
    set_session_cookie, verify_password,
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
    if form.password != form.password_confirm {
        ctx.insert("error", "Passwords do not match");
        let html = tera.render("register.html", &ctx).unwrap();
        return (jar, Html(html));
    }
    
    if form.password.len() < 6 {
        ctx.insert("error", "Password must be at least 6 characters");
        let html = tera.render("register.html", &ctx).unwrap();
        return (jar, Html(html));
    }
    
    if form.username.len() < 3 {
        ctx.insert("error", "Username must be at least 3 characters");
        let html = tera.render("register.html", &ctx).unwrap();
        return (jar, Html(html));
    }
    
    let password_hash = match hash_password(&form.password) {
        Ok(h) => h,
        Err(_) => {
            ctx.insert("error", "Failed to hash password");
            let html = tera.render("register.html", &ctx).unwrap();
            return (jar, Html(html));
        }
    };
    
    let conn = db.lock().unwrap();
    
    match db::create_user(&conn, &form.email, &password_hash, &form.username) {
        Ok(user_id) => {
            // Create session
            let token = create_session_token();
            let expires = session_expiry();
            let _ = db::create_session(&conn, &token, user_id, &expires);
            
            let jar = set_session_cookie(jar, &token);
            let html = r#"<script>window.location.href = "/";</script>"#.to_string();
            (jar, Html(html))
        }
        Err(e) => {
            let error = if e.to_string().contains("UNIQUE") {
                "Email or username already taken"
            } else {
                "Registration failed"
            };
            ctx.insert("error", error);
            let html = tera.render("register.html", &ctx).unwrap();
            (jar, Html(html))
        }
    }
}

pub async fn login_page(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
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
    
    match db::get_user_by_email(&conn, &form.email) {
        Ok(Some((user, password_hash))) => {
            if user.banned {
                ctx.insert("error", "This account has been banned");
                let html = tera.render("login.html", &ctx).unwrap();
                return (jar, Html(html));
            }
            
            if verify_password(&form.password, &password_hash) {
                let token = create_session_token();
                let expires = session_expiry();
                let _ = db::create_session(&conn, &token, user.id, &expires);
                
                let jar = set_session_cookie(jar, &token);
                let html = r#"<script>window.location.href = "/";</script>"#.to_string();
                (jar, Html(html))
            } else {
                ctx.insert("error", "Invalid email or password");
                let html = tera.render("login.html", &ctx).unwrap();
                (jar, Html(html))
            }
        }
        _ => {
            ctx.insert("error", "Invalid email or password");
            let html = tera.render("login.html", &ctx).unwrap();
            (jar, Html(html))
        }
    }
}

pub async fn logout(
    jar: CookieJar,
    State((db, _)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Redirect) {
    if let Some(cookie) = jar.get("session") {
        let conn = db.lock().unwrap();
        let _ = db::delete_session(&conn, cookie.value());
    }
    
    let jar = clear_session_cookie(jar);
    (jar, Redirect::to("/"))
}
