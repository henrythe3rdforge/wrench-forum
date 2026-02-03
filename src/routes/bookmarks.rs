use axum::{
    extract::{Path, State},
    response::Html,
};
use axum_extra::extract::CookieJar;
use std::sync::Arc;
use tera::{Context, Tera};

use crate::auth::ensure_session;
use crate::db::{self, Db};

pub async fn list_bookmarks(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        let unread_count = db::get_unread_notification_count(&conn, user.id).unwrap_or(0);
        ctx.insert("unread_notifications", &unread_count);
        
        let bookmarks = db::get_user_bookmarks(&conn, user.id).unwrap_or_default();
        ctx.insert("user", &user);
        ctx.insert("posts", &bookmarks);
        ctx.insert("current_page", &"bookmarks");
        
        let html = tera.render("bookmarks.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
        return (jar, Html(html));
    }
    
    let html = r#"<script>window.location.href = "/login";</script>"#.to_string();
    (jar, Html(html))
}

pub async fn toggle_bookmark(
    jar: CookieJar,
    Path(post_id): Path<i64>,
    State((db, _)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        
        let is_bookmarked = db::is_post_bookmarked(&conn, user.id, post_id).unwrap_or(false);
        
        if is_bookmarked {
            let _ = db::remove_bookmark(&conn, user.id, post_id);
            return (jar, Html(format!(
                r#"<button class="btn-icon bookmark-btn" 
                           hx-post="/post/{}/bookmark" 
                           hx-swap="outerHTML"
                           title="Add to bookmarks">
                    <span class="icon">🔖</span>
                </button>
                <div id="toast-container" hx-swap-oob="beforeend">
                    <div class="toast">Removed from bookmarks</div>
                </div>"#,
                post_id
            )));
        } else {
            let _ = db::add_bookmark(&conn, user.id, post_id);
            return (jar, Html(format!(
                r#"<button class="btn-icon bookmark-btn bookmarked" 
                           hx-post="/post/{}/bookmark" 
                           hx-swap="outerHTML"
                           title="Remove from bookmarks">
                    <span class="icon">🔖</span>
                </button>
                <div id="toast-container" hx-swap-oob="beforeend">
                    <div class="toast success">Added to bookmarks</div>
                </div>"#,
                post_id
            )));
        }
    }
    
    (jar, Html("<div class=\"toast error\">Please log in</div>".to_string()))
}
