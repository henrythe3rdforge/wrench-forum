use axum::{
    extract::{Path, State},
    response::Html,
};
use axum_extra::extract::CookieJar;
use std::sync::Arc;
use tera::{Context, Tera};

use crate::auth::ensure_session;
use crate::db::{self, Db};

pub async fn list_notifications(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        
        let notifications = db::get_user_notifications(&conn, user.id, 50).unwrap_or_default();
        let unread_count = db::get_unread_notification_count(&conn, user.id).unwrap_or(0);
        
        ctx.insert("user", &user);
        ctx.insert("notifications", &notifications);
        ctx.insert("unread_notifications", &unread_count);
        ctx.insert("current_page", &"notifications");
        
        let html = tera.render("notifications.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
        return (jar, Html(html));
    }
    
    let html = r#"<script>window.location.href = "/login";</script>"#.to_string();
    (jar, Html(html))
}

pub async fn notification_count(
    jar: CookieJar,
    State((db, _)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        let count = db::get_unread_notification_count(&conn, user.id).unwrap_or(0);
        
        if count > 0 {
            return (jar, Html(format!(
                r#"<span class="notification-badge">{}</span>"#,
                if count > 99 { "99+".to_string() } else { count.to_string() }
            )));
        } else {
            return (jar, Html(String::new()));
        }
    }
    
    (jar, Html(String::new()))
}

pub async fn mark_read(
    jar: CookieJar,
    Path(id): Path<i64>,
    State((db, _)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        let _ = db::mark_notification_read(&conn, id);
        
        let count = db::get_unread_notification_count(&conn, user.id).unwrap_or(0);
        
        return (jar, Html(format!(
            r#"<span id="notification-count" hx-swap-oob="true">
                {}
            </span>"#,
            if count > 0 {
                format!(r#"<span class="notification-badge">{}</span>"#, count)
            } else {
                String::new()
            }
        )));
    }
    
    (jar, Html(String::new()))
}

pub async fn mark_all_read(
    jar: CookieJar,
    State((db, _)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        let _ = db::mark_all_notifications_read(&conn, user.id);
        
        return (jar, Html(r#"
            <span id="notification-count" hx-swap-oob="true"></span>
            <div id="toast-container" hx-swap-oob="beforeend">
                <div class="toast success">All notifications marked as read</div>
            </div>
        "#.to_string()));
    }
    
    (jar, Html(String::new()))
}
