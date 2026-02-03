use axum::{
    extract::State,
    response::Html,
    Form,
};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use std::sync::Arc;
use tera::{Context, Tera};

use crate::auth::ensure_session;
use crate::db::{self, Db};

#[derive(Deserialize)]
pub struct VerificationForm {
    pub proof_type: String,
    pub proof_text: String,
}

pub async fn verification_page(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        // Check if already verified
        if user.role.can_post() {
            ctx.insert("user", &user);
            ctx.insert("already_verified", &true);
            let html = tera.render("verification.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
            return (jar, Html(html));
        }
        
        // Check for pending request
        let conn = db.lock().unwrap();
        let has_pending = db::has_pending_verification(&conn, user.id).unwrap_or(false);
        
        ctx.insert("user", &user);
        ctx.insert("has_pending", &has_pending);
        
        let html = tera.render("verification.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
        return (jar, Html(html));
    }
    
    let html = r#"<script>window.location.href = "/login";</script>"#.to_string();
    (jar, Html(html))
}

pub async fn submit_verification(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
    Form(form): Form<VerificationForm>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if user.role.can_post() {
            let html = r#"<script>window.location.href = "/";</script>"#.to_string();
            return (jar, Html(html));
        }
        
        // Validation
        if form.proof_text.trim().is_empty() {
            ctx.insert("user", &user);
            ctx.insert("error", "Please provide verification details");
            let html = tera.render("verification.html", &ctx).unwrap();
            return (jar, Html(html));
        }
        
        if form.proof_text.len() < 50 {
            ctx.insert("user", &user);
            ctx.insert("error", "Please provide more detail (at least 50 characters)");
            let html = tera.render("verification.html", &ctx).unwrap();
            return (jar, Html(html));
        }
        
        let conn = db.lock().unwrap();
        
        // Check for existing pending request
        if db::has_pending_verification(&conn, user.id).unwrap_or(false) {
            ctx.insert("user", &user);
            ctx.insert("has_pending", &true);
            ctx.insert("error", "You already have a pending verification request");
            let html = tera.render("verification.html", &ctx).unwrap();
            return (jar, Html(html));
        }
        
        match db::create_verification_request(&conn, user.id, &form.proof_text, &form.proof_type) {
            Ok(_) => {
                let _ = db::log_activity(&conn, user.id, "submit_verification", None, None, None, None);
                let html = r#"<script>window.location.href = "/verification";</script>"#.to_string();
                return (jar, Html(html));
            }
            Err(_) => {
                ctx.insert("user", &user);
                ctx.insert("error", "Failed to submit verification request");
                let html = tera.render("verification.html", &ctx).unwrap();
                return (jar, Html(html));
            }
        }
    }
    
    let html = r#"<script>window.location.href = "/login";</script>"#.to_string();
    (jar, Html(html))
}
