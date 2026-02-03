use axum::{
    extract::{Path, State},
    response::Html,
};
use axum_extra::extract::CookieJar;
use std::sync::Arc;
use tera::{Context, Tera};

use crate::auth::ensure_session;
use crate::db::{self, Db};

pub async fn mod_queue(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.can_moderate() {
            ctx.insert("error", "Moderator access required");
            ctx.insert("user", &user);
            let html = tera.render("error.html", &ctx).unwrap();
            return (jar, Html(html));
        }
        
        ctx.insert("user", &user);
        
        let conn = db.lock().unwrap();
        let reports = db::get_unresolved_reports(&conn).unwrap_or_default();
        let banned = db::get_banned_users(&conn).unwrap_or_default();
        
        ctx.insert("reports", &reports);
        ctx.insert("banned_users", &banned);
        
        let html = tera.render("mod_queue.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
        return (jar, Html(html));
    }
    
    let html = r#"<script>window.location.href = "/login";</script>"#.to_string();
    (jar, Html(html))
}

pub async fn remove_post(
    jar: CookieJar,
    Path(post_id): Path<i64>,
    State((db, _)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.can_moderate() {
            return (jar, Html("Moderator access required".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let _ = db::remove_post(&conn, post_id);
        
        return (jar, Html(r#"<span class="removed">Post removed ✓</span>"#.to_string()));
    }
    
    (jar, Html("Login required".to_string()))
}

pub async fn remove_comment(
    jar: CookieJar,
    Path(comment_id): Path<i64>,
    State((db, _)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.can_moderate() {
            return (jar, Html("Moderator access required".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let _ = db::remove_comment(&conn, comment_id);
        
        return (jar, Html(r#"<span class="removed">Comment removed ✓</span>"#.to_string()));
    }
    
    (jar, Html("Login required".to_string()))
}

pub async fn ban_user(
    jar: CookieJar,
    Path(user_id): Path<i64>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.can_moderate() {
            return (jar, Html("Moderator access required".to_string()));
        }
        
        // Can't ban yourself
        if user.id == user_id {
            return (jar, Html("Cannot ban yourself".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let _ = db::set_user_banned(&conn, user_id, true);
        
        let banned = db::get_banned_users(&conn).unwrap_or_default();
        ctx.insert("banned_users", &banned);
        
        let html = tera.render("partials/ban_list.html", &ctx).unwrap_or_default();
        return (jar, Html(html));
    }
    
    (jar, Html("Login required".to_string()))
}

pub async fn unban_user(
    jar: CookieJar,
    Path(user_id): Path<i64>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.can_moderate() {
            return (jar, Html("Moderator access required".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let _ = db::set_user_banned(&conn, user_id, false);
        
        let banned = db::get_banned_users(&conn).unwrap_or_default();
        ctx.insert("banned_users", &banned);
        
        let html = tera.render("partials/ban_list.html", &ctx).unwrap_or_default();
        return (jar, Html(html));
    }
    
    (jar, Html("Login required".to_string()))
}

pub async fn resolve_report(
    jar: CookieJar,
    Path(report_id): Path<i64>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.can_moderate() {
            return (jar, Html("Moderator access required".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let _ = db::resolve_report(&conn, report_id);
        
        let reports = db::get_unresolved_reports(&conn).unwrap_or_default();
        ctx.insert("reports", &reports);
        
        let html = tera.render("partials/report_queue.html", &ctx).unwrap_or_default();
        return (jar, Html(html));
    }
    
    (jar, Html("Login required".to_string()))
}
