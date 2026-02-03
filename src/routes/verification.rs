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
        if user.role.can_post() {
            ctx.insert("error", "You are already verified");
            ctx.insert("user", &user);
            let html = tera.render("error.html", &ctx).unwrap();
            return (jar, Html(html));
        }
        
        let conn = db.lock().unwrap();
        let has_pending = db::has_pending_verification(&conn, user.id).unwrap_or(false);
        
        if has_pending {
            ctx.insert("message", "Your verification request is pending review");
            ctx.insert("user", &user);
            let html = tera.render("verification_pending.html", &ctx).unwrap();
            return (jar, Html(html));
        }
        
        ctx.insert("user", &user);
        
        let proof_types = vec![
            ("ase_cert", "ASE Certification Number"),
            ("shop_employment", "Shop Employment Verification"),
            ("business_license", "Automotive Business License"),
            ("other", "Other Professional Credential"),
        ];
        ctx.insert("proof_types", &proof_types);
        
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
            ctx.insert("error", "You are already verified");
            ctx.insert("user", &user);
            let html = tera.render("error.html", &ctx).unwrap();
            return (jar, Html(html));
        }
        
        let conn = db.lock().unwrap();
        
        let has_pending = db::has_pending_verification(&conn, user.id).unwrap_or(false);
        if has_pending {
            ctx.insert("message", "You already have a pending verification request");
            ctx.insert("user", &user);
            let html = tera.render("verification_pending.html", &ctx).unwrap();
            return (jar, Html(html));
        }
        
        match db::create_verification_request(&conn, user.id, &form.proof_text, &form.proof_type) {
            Ok(_) => {
                ctx.insert("message", "Your verification request has been submitted and is pending review");
                ctx.insert("user", &user);
                let html = tera.render("verification_pending.html", &ctx).unwrap();
                return (jar, Html(html));
            }
            Err(_) => {
                ctx.insert("error", "Failed to submit verification request");
                ctx.insert("user", &user);
                let html = tera.render("error.html", &ctx).unwrap();
                return (jar, Html(html));
            }
        }
    }
    
    let html = r#"<script>window.location.href = "/login";</script>"#.to_string();
    (jar, Html(html))
}
