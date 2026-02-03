use axum::{
    extract::{Path, State},
    response::Html,
};
use axum_extra::extract::CookieJar;
use std::sync::Arc;
use tera::{Context, Tera};

use crate::auth::ensure_session;
use crate::db::{self, Db};

pub async fn view_profile(
    jar: CookieJar,
    Path(username): Path<String>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    let jar = if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        ctx.insert("user", &user);
        jar
    } else {
        jar
    };
    
    let conn = db.lock().unwrap();
    
    match db::get_user_by_username(&conn, &username) {
        Ok(Some(profile_user)) => {
            let posts = db::get_posts_by_user(&conn, profile_user.id).unwrap_or_default();
            
            ctx.insert("profile_user", &profile_user);
            ctx.insert("posts", &posts);
            
            let html = tera.render("profile.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
            (jar, Html(html))
        }
        _ => {
            ctx.insert("error", "User not found");
            let html = tera.render("error.html", &ctx).unwrap();
            (jar, Html(html))
        }
    }
}

pub async fn my_profile(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let mut ctx = Context::new();
        ctx.insert("user", &user);
        
        let conn = db.lock().unwrap();
        let posts = db::get_posts_by_user(&conn, user.id).unwrap_or_default();
        let has_pending = db::has_pending_verification(&conn, user.id).unwrap_or(false);
        
        ctx.insert("profile_user", &user);
        ctx.insert("posts", &posts);
        ctx.insert("is_own_profile", &true);
        ctx.insert("has_pending_verification", &has_pending);
        
        let html = tera.render("profile.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
        return (jar, Html(html));
    }
    
    let html = r#"<script>window.location.href = "/login";</script>"#.to_string();
    (jar, Html(html))
}
