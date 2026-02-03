use axum::{
    extract::{Path, State},
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
pub struct RoleForm {
    pub role: String,
}

pub async fn admin_panel(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.is_admin() {
            ctx.insert("error", "Admin access required");
            ctx.insert("user", &user);
            let html = tera.render("error.html", &ctx).unwrap();
            return (jar, Html(html));
        }
        
        ctx.insert("user", &user);
        
        let conn = db.lock().unwrap();
        let pending = db::get_pending_verification_requests(&conn).unwrap_or_default();
        let users = db::get_all_users(&conn).unwrap_or_default();
        
        ctx.insert("pending_verifications", &pending);
        ctx.insert("all_users", &users);
        
        let html = tera.render("admin.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
        return (jar, Html(html));
    }
    
    let html = r#"<script>window.location.href = "/login";</script>"#.to_string();
    (jar, Html(html))
}

pub async fn approve_verification(
    jar: CookieJar,
    Path(request_id): Path<i64>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.is_admin() {
            return (jar, Html("Admin access required".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let _ = db::approve_verification(&conn, request_id, user.id);
        
        // Return updated list
        let pending = db::get_pending_verification_requests(&conn).unwrap_or_default();
        ctx.insert("pending_verifications", &pending);
        
        let html = tera.render("partials/verification_queue.html", &ctx).unwrap_or_default();
        return (jar, Html(html));
    }
    
    (jar, Html("Login required".to_string()))
}

pub async fn deny_verification(
    jar: CookieJar,
    Path(request_id): Path<i64>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.is_admin() {
            return (jar, Html("Admin access required".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let _ = db::deny_verification(&conn, request_id, user.id);
        
        let pending = db::get_pending_verification_requests(&conn).unwrap_or_default();
        ctx.insert("pending_verifications", &pending);
        
        let html = tera.render("partials/verification_queue.html", &ctx).unwrap_or_default();
        return (jar, Html(html));
    }
    
    (jar, Html("Login required".to_string()))
}

pub async fn update_user_role(
    jar: CookieJar,
    Path(user_id): Path<i64>,
    State((db, _)): State<(Db, Arc<Tera>)>,
    Form(form): Form<RoleForm>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.is_admin() {
            return (jar, Html("Admin access required".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let _ = db::update_user_role(&conn, user_id, &form.role);
        
        return (jar, Html(format!(r#"<span class="role-badge {}">{}</span>"#, form.role, form.role)));
    }
    
    (jar, Html("Login required".to_string()))
}
