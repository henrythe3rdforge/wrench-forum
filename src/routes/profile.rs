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

#[derive(Deserialize, serde::Serialize)]
pub struct ProfileForm {
    pub bio: Option<String>,
    pub specialties: Option<String>,
    pub location: Option<String>,
    pub website: Option<String>,
}

pub async fn my_profile(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let html = format!(r#"<script>window.location.href = "/user/{}";</script>"#, user.username);
        return (jar, Html(html));
    }
    
    let html = r#"<script>window.location.href = "/login";</script>"#.to_string();
    (jar, Html(html))
}

pub async fn view_profile(
    jar: CookieJar,
    Path(username): Path<String>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    let (jar, current_user_id) = if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        let unread_count = db::get_unread_notification_count(&conn, user.id).unwrap_or(0);
        ctx.insert("current_user", &user);
        ctx.insert("unread_notifications", &unread_count);
        (jar, Some(user.id))
    } else {
        (jar, None)
    };
    
    let conn = db.lock().unwrap();
    
    match db::get_user_by_username(&conn, &username) {
        Ok(Some(profile_user)) => {
            let profile = db::get_user_profile(&conn, profile_user.id).ok().flatten();
            let stats = db::get_user_stats(&conn, profile_user.id).ok();
            let posts = db::get_posts_by_user(&conn, profile_user.id).unwrap_or_default();
            
            ctx.insert("profile_user", &profile_user);
            ctx.insert("profile", &profile);
            ctx.insert("stats", &stats);
            ctx.insert("posts", &posts);
            ctx.insert("is_own_profile", &(current_user_id == Some(profile_user.id)));
            ctx.insert("current_tab", &"posts");
            
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

pub async fn user_posts(
    jar: CookieJar,
    Path(username): Path<String>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, _)) = ensure_session(jar.clone(), &db) {
        ctx.insert("current_user", &user);
    }
    
    let conn = db.lock().unwrap();
    
    match db::get_user_by_username(&conn, &username) {
        Ok(Some(profile_user)) => {
            let posts = db::get_posts_by_user(&conn, profile_user.id).unwrap_or_default();
            ctx.insert("posts", &posts);
            
            let html = tera.render("partials/profile_posts.html", &ctx).unwrap_or_default();
            (jar, Html(html))
        }
        _ => (jar, Html("User not found".to_string()))
    }
}

pub async fn user_comments(
    jar: CookieJar,
    Path(username): Path<String>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, _)) = ensure_session(jar.clone(), &db) {
        ctx.insert("current_user", &user);
    }
    
    let conn = db.lock().unwrap();
    
    match db::get_user_by_username(&conn, &username) {
        Ok(Some(profile_user)) => {
            let comments = db::get_comments_by_user(&conn, profile_user.id).unwrap_or_default();
            ctx.insert("comments", &comments);
            
            let html = tera.render("partials/profile_comments.html", &ctx).unwrap_or_default();
            (jar, Html(html))
        }
        _ => (jar, Html("User not found".to_string()))
    }
}

pub async fn edit_profile_page(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        let profile = db::get_user_profile(&conn, user.id).ok().flatten();
        
        ctx.insert("user", &user);
        ctx.insert("profile", &profile);
        
        let html = tera.render("edit_profile.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
        return (jar, Html(html));
    }
    
    let html = r#"<script>window.location.href = "/login";</script>"#.to_string();
    (jar, Html(html))
}

pub async fn edit_profile_submit(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
    Form(form): Form<ProfileForm>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        
        // Validate website URL if provided
        if let Some(ref website) = form.website {
            if !website.is_empty() && !website.starts_with("http://") && !website.starts_with("https://") {
                ctx.insert("error", "Website must start with http:// or https://");
                ctx.insert("user", &user);
                ctx.insert("profile", &form);
                let html = tera.render("edit_profile.html", &ctx).unwrap();
                return (jar, Html(html));
            }
        }
        
        let _ = db::update_user_profile(
            &conn,
            user.id,
            form.bio.as_deref(),
            form.specialties.as_deref(),
            form.location.as_deref(),
            form.website.as_deref(),
        );
        
        let html = format!(r#"<script>window.location.href = "/user/{}";</script>"#, user.username);
        return (jar, Html(html));
    }
    
    let html = r#"<script>window.location.href = "/login";</script>"#.to_string();
    (jar, Html(html))
}
