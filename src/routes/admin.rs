use axum::{
    extract::{Path, Query, State},
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

#[derive(Deserialize)]
pub struct FlairForm {
    pub flair: String,
}

#[derive(Deserialize)]
pub struct AnnouncementForm {
    pub title: String,
    pub content: String,
    pub announcement_type: Option<String>,
    pub expires_days: Option<i64>,
}

#[derive(Deserialize)]
pub struct PaginationQuery {
    pub page: Option<i64>,
}

pub async fn admin_panel(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.is_admin() {
            ctx.insert("error", "Admin access required");
            let html = tera.render("error.html", &ctx).unwrap();
            return (jar, Html(html));
        }
        
        let conn = db.lock().unwrap();
        
        let users = db::get_all_users(&conn).unwrap_or_default();
        let pending_verifications = db::get_pending_verification_requests(&conn).unwrap_or_default();
        let announcements = db::get_active_announcements(&conn).unwrap_or_default();
        let stats = db::get_forum_stats(&conn).unwrap_or_default();
        let recent_activity = db::get_recent_activity(&conn, 20).unwrap_or_default();
        let unread_count = db::get_unread_notification_count(&conn, user.id).unwrap_or(0);
        
        ctx.insert("user", &user);
        ctx.insert("users", &users);
        ctx.insert("pending_verifications", &pending_verifications);
        ctx.insert("announcements", &announcements);
        ctx.insert("stats", &stats);
        ctx.insert("recent_activity", &recent_activity);
        ctx.insert("unread_notifications", &unread_count);
        ctx.insert("current_page", &"admin");
        
        let html = tera.render("admin.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
        return (jar, Html(html));
    }
    
    let html = r#"<script>window.location.href = "/login";</script>"#.to_string();
    (jar, Html(html))
}

pub async fn approve_verification(
    jar: CookieJar,
    Path(id): Path<i64>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.is_admin() {
            return (jar, Html("Unauthorized".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let _ = db::approve_verification(&conn, id, user.id);
        let _ = db::log_activity(&conn, user.id, "approve_verification", Some("verification"), Some(id), None, None);
        
        // Return updated verification queue
        let pending = db::get_pending_verification_requests(&conn).unwrap_or_default();
        let mut ctx = Context::new();
        ctx.insert("pending_verifications", &pending);
        
        let html = tera.render("partials/verification_queue.html", &ctx).unwrap_or_default();
        return (jar, Html(format!(
            r#"{}
            <div id="toast-container" hx-swap-oob="beforeend">
                <div class="toast success">Verification approved!</div>
            </div>"#,
            html
        )));
    }
    
    (jar, Html("Unauthorized".to_string()))
}

pub async fn deny_verification(
    jar: CookieJar,
    Path(id): Path<i64>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.is_admin() {
            return (jar, Html("Unauthorized".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let _ = db::deny_verification(&conn, id, user.id);
        let _ = db::log_activity(&conn, user.id, "deny_verification", Some("verification"), Some(id), None, None);
        
        let pending = db::get_pending_verification_requests(&conn).unwrap_or_default();
        let mut ctx = Context::new();
        ctx.insert("pending_verifications", &pending);
        
        let html = tera.render("partials/verification_queue.html", &ctx).unwrap_or_default();
        return (jar, Html(format!(
            r#"{}
            <div id="toast-container" hx-swap-oob="beforeend">
                <div class="toast">Verification denied</div>
            </div>"#,
            html
        )));
    }
    
    (jar, Html("Unauthorized".to_string()))
}

pub async fn update_user_role(
    jar: CookieJar,
    Path(user_id): Path<i64>,
    State((db, _)): State<(Db, Arc<Tera>)>,
    Form(form): Form<RoleForm>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.is_admin() {
            return (jar, Html("Unauthorized".to_string()));
        }
        
        // Prevent changing own role
        if user_id == user.id {
            return (jar, Html("<div class=\"toast error\">Cannot change your own role</div>".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let _ = db::update_user_role(&conn, user_id, &form.role);
        let _ = db::log_activity(&conn, user.id, "change_role", Some("user"), Some(user_id), Some(&form.role), None);
        
        return (jar, Html("<div class=\"toast success\">Role updated!</div>".to_string()));
    }
    
    (jar, Html("Unauthorized".to_string()))
}

pub async fn update_user_flair(
    jar: CookieJar,
    Path(user_id): Path<i64>,
    State((db, _)): State<(Db, Arc<Tera>)>,
    Form(form): Form<FlairForm>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.is_admin() {
            return (jar, Html("Unauthorized".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let _ = db::update_user_flair(&conn, user_id, &form.flair);
        let _ = db::log_activity(&conn, user.id, "change_flair", Some("user"), Some(user_id), Some(&form.flair), None);
        
        return (jar, Html("<div class=\"toast success\">Flair updated!</div>".to_string()));
    }
    
    (jar, Html("Unauthorized".to_string()))
}

pub async fn create_announcement(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
    Form(form): Form<AnnouncementForm>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.is_admin() {
            return (jar, Html("Unauthorized".to_string()));
        }
        
        if form.title.trim().is_empty() || form.content.trim().is_empty() {
            return (jar, Html("<div class=\"toast error\">Title and content are required</div>".to_string()));
        }
        
        let expires_at = form.expires_days.map(|days| {
            (chrono::Utc::now() + chrono::Duration::days(days)).format("%Y-%m-%d %H:%M:%S").to_string()
        });
        
        let conn = db.lock().unwrap();
        let announcement_type = form.announcement_type.unwrap_or_else(|| "info".to_string());
        let _ = db::create_announcement(&conn, &form.title, &form.content, &announcement_type, user.id, expires_at.as_deref());
        let _ = db::log_activity(&conn, user.id, "create_announcement", None, None, Some(&form.title), None);
        
        let announcements = db::get_active_announcements(&conn).unwrap_or_default();
        let mut ctx = Context::new();
        ctx.insert("announcements", &announcements);
        
        let html = tera.render("partials/announcement_list.html", &ctx).unwrap_or_default();
        return (jar, Html(format!(
            r#"{}
            <div id="toast-container" hx-swap-oob="beforeend">
                <div class="toast success">Announcement created!</div>
            </div>"#,
            html
        )));
    }
    
    (jar, Html("Unauthorized".to_string()))
}

pub async fn deactivate_announcement(
    jar: CookieJar,
    Path(id): Path<i64>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.is_admin() {
            return (jar, Html("Unauthorized".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let _ = db::deactivate_announcement(&conn, id);
        let _ = db::log_activity(&conn, user.id, "deactivate_announcement", Some("announcement"), Some(id), None, None);
        
        let announcements = db::get_active_announcements(&conn).unwrap_or_default();
        let mut ctx = Context::new();
        ctx.insert("announcements", &announcements);
        
        let html = tera.render("partials/announcement_list.html", &ctx).unwrap_or_default();
        return (jar, Html(format!(
            r#"{}
            <div id="toast-container" hx-swap-oob="beforeend">
                <div class="toast">Announcement deactivated</div>
            </div>"#,
            html
        )));
    }
    
    (jar, Html("Unauthorized".to_string()))
}

pub async fn forum_stats(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.is_admin() {
            return (jar, Html("Unauthorized".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let stats = db::get_forum_stats(&conn).unwrap_or_default();
        
        let mut ctx = Context::new();
        ctx.insert("stats", &stats);
        
        let html = tera.render("partials/forum_stats.html", &ctx).unwrap_or_default();
        return (jar, Html(html));
    }
    
    (jar, Html("Unauthorized".to_string()))
}

pub async fn activity_logs(
    jar: CookieJar,
    Query(query): Query<PaginationQuery>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.is_admin() {
            return (jar, Html("Unauthorized".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let limit = 50;
        let activity = db::get_recent_activity(&conn, limit).unwrap_or_default();
        
        let mut ctx = Context::new();
        ctx.insert("activity", &activity);
        
        let html = tera.render("partials/activity_logs.html", &ctx).unwrap_or_default();
        return (jar, Html(html));
    }
    
    (jar, Html("Unauthorized".to_string()))
}
