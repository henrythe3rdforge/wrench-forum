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
pub struct BanForm {
    pub reason: Option<String>,
}

pub async fn mod_queue(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.can_moderate() {
            ctx.insert("error", "Moderator access required");
            let html = tera.render("error.html", &ctx).unwrap();
            return (jar, Html(html));
        }
        
        let conn = db.lock().unwrap();
        
        let reports = db::get_unresolved_reports(&conn).unwrap_or_default();
        let banned_users = db::get_banned_users(&conn).unwrap_or_default();
        let unread_count = db::get_unread_notification_count(&conn, user.id).unwrap_or(0);
        
        ctx.insert("user", &user);
        ctx.insert("reports", &reports);
        ctx.insert("banned_users", &banned_users);
        ctx.insert("unread_notifications", &unread_count);
        ctx.insert("current_page", &"mod");
        
        let html = tera.render("mod_queue.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
        return (jar, Html(html));
    }
    
    let html = r#"<script>window.location.href = "/login";</script>"#.to_string();
    (jar, Html(html))
}

pub async fn remove_post(
    jar: CookieJar,
    Path(id): Path<i64>,
    State((db, _)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.can_moderate() {
            return (jar, Html("Unauthorized".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let _ = db::remove_post(&conn, id);
        let _ = db::log_activity(&conn, user.id, "remove_post", Some("post"), Some(id), None, None);
        
        return (jar, Html(r#"
            <span class="removed-badge">Removed</span>
            <div id="toast-container" hx-swap-oob="beforeend">
                <div class="toast success">Post removed</div>
            </div>
        "#.to_string()));
    }
    
    (jar, Html("Unauthorized".to_string()))
}

pub async fn restore_post(
    jar: CookieJar,
    Path(id): Path<i64>,
    State((db, _)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.can_moderate() {
            return (jar, Html("Unauthorized".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let _ = db::restore_post(&conn, id);
        let _ = db::log_activity(&conn, user.id, "restore_post", Some("post"), Some(id), None, None);
        
        return (jar, Html(r#"
            <span class="restored-badge">Restored</span>
            <div id="toast-container" hx-swap-oob="beforeend">
                <div class="toast success">Post restored</div>
            </div>
        "#.to_string()));
    }
    
    (jar, Html("Unauthorized".to_string()))
}

pub async fn pin_post(
    jar: CookieJar,
    Path(id): Path<i64>,
    State((db, _)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.can_moderate() {
            return (jar, Html("Unauthorized".to_string()));
        }
        
        let conn = db.lock().unwrap();
        
        // Toggle pin status
        let current_pinned: Option<i64> = conn.query_row(
            "SELECT pinned FROM posts WHERE id = ?1",
            rusqlite::params![id],
            |r| r.get(0)
        ).ok();
        
        let new_pinned = current_pinned != Some(1);
        let _ = db::pin_post(&conn, id, new_pinned);
        let _ = db::log_activity(&conn, user.id, if new_pinned { "pin_post" } else { "unpin_post" }, Some("post"), Some(id), None, None);
        
        let message = if new_pinned { "Post pinned" } else { "Post unpinned" };
        return (jar, Html(format!(r#"
            <div id="toast-container" hx-swap-oob="beforeend">
                <div class="toast success">{}</div>
            </div>
        "#, message)));
    }
    
    (jar, Html("Unauthorized".to_string()))
}

pub async fn remove_comment(
    jar: CookieJar,
    Path(id): Path<i64>,
    State((db, _)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.can_moderate() {
            return (jar, Html("Unauthorized".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let _ = db::remove_comment(&conn, id);
        let _ = db::log_activity(&conn, user.id, "remove_comment", Some("comment"), Some(id), None, None);
        
        return (jar, Html(r#"
            <span class="removed-badge">Comment removed</span>
            <div id="toast-container" hx-swap-oob="beforeend">
                <div class="toast success">Comment removed</div>
            </div>
        "#.to_string()));
    }
    
    (jar, Html("Unauthorized".to_string()))
}

pub async fn ban_user(
    jar: CookieJar,
    Path(id): Path<i64>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
    Form(form): Form<BanForm>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.can_moderate() {
            return (jar, Html("Unauthorized".to_string()));
        }
        
        // Prevent self-ban
        if id == user.id {
            return (jar, Html("<div class=\"toast error\">Cannot ban yourself</div>".to_string()));
        }
        
        let conn = db.lock().unwrap();
        
        // Check if target is admin (can't ban admins)
        if let Ok(Some(target)) = db::get_user_by_id(&conn, id) {
            if target.role.is_admin() && !user.role.is_admin() {
                return (jar, Html("<div class=\"toast error\">Cannot ban an admin</div>".to_string()));
            }
        }
        
        let _ = db::set_user_banned(&conn, id, true);
        let _ = db::delete_user_sessions(&conn, id); // Force logout
        let _ = db::log_activity(&conn, user.id, "ban_user", Some("user"), Some(id), form.reason.as_deref(), None);
        
        // Return updated ban list
        let banned_users = db::get_banned_users(&conn).unwrap_or_default();
        let mut ctx = Context::new();
        ctx.insert("banned_users", &banned_users);
        
        let html = tera.render("partials/ban_list.html", &ctx).unwrap_or_default();
        return (jar, Html(format!(
            r#"{}
            <div id="toast-container" hx-swap-oob="beforeend">
                <div class="toast success">User banned</div>
            </div>"#,
            html
        )));
    }
    
    (jar, Html("Unauthorized".to_string()))
}

pub async fn unban_user(
    jar: CookieJar,
    Path(id): Path<i64>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.can_moderate() {
            return (jar, Html("Unauthorized".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let _ = db::set_user_banned(&conn, id, false);
        let _ = db::log_activity(&conn, user.id, "unban_user", Some("user"), Some(id), None, None);
        
        let banned_users = db::get_banned_users(&conn).unwrap_or_default();
        let mut ctx = Context::new();
        ctx.insert("banned_users", &banned_users);
        
        let html = tera.render("partials/ban_list.html", &ctx).unwrap_or_default();
        return (jar, Html(format!(
            r#"{}
            <div id="toast-container" hx-swap-oob="beforeend">
                <div class="toast success">User unbanned</div>
            </div>"#,
            html
        )));
    }
    
    (jar, Html("Unauthorized".to_string()))
}

pub async fn resolve_report(
    jar: CookieJar,
    Path(id): Path<i64>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.can_moderate() {
            return (jar, Html("Unauthorized".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let _ = db::resolve_report(&conn, id);
        let _ = db::log_activity(&conn, user.id, "resolve_report", Some("report"), Some(id), None, None);
        
        let reports = db::get_unresolved_reports(&conn).unwrap_or_default();
        let mut ctx = Context::new();
        ctx.insert("reports", &reports);
        
        let html = tera.render("partials/report_queue.html", &ctx).unwrap_or_default();
        return (jar, Html(format!(
            r#"{}
            <div id="toast-container" hx-swap-oob="beforeend">
                <div class="toast success">Report resolved</div>
            </div>"#,
            html
        )));
    }
    
    (jar, Html("Unauthorized".to_string()))
}
