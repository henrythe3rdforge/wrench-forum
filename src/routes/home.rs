use axum::{
    extract::{Query, State},
    response::Html,
};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use std::sync::Arc;
use tera::{Context, Tera};

use crate::auth::ensure_session;
use crate::db::{self, Db};

#[derive(Deserialize)]
pub struct HomeQuery {
    pub sort: Option<String>,
    pub page: Option<i64>,
}

pub async fn index(
    jar: CookieJar,
    Query(query): Query<HomeQuery>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    let sort = query.sort.unwrap_or_else(|| "hot".to_string());
    let page = query.page.unwrap_or(1);
    
    let jar = if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        let unread_count = db::get_unread_notification_count(&conn, user.id).unwrap_or(0);
        ctx.insert("user", &user);
        ctx.insert("unread_notifications", &unread_count);
        jar
    } else {
        jar
    };
    
    let conn = db.lock().unwrap();
    
    // Get categories
    let categories = db::get_categories(&conn).unwrap_or_default();
    ctx.insert("categories", &categories);
    
    // Get posts with pagination
    let (posts, pagination) = db::get_posts_paginated(&conn, None, &sort, page, 25).unwrap_or_default();
    ctx.insert("posts", &posts);
    ctx.insert("pagination", &pagination);
    
    // Get trending posts for sidebar
    let trending = db::get_trending_posts(&conn, 5).unwrap_or_default();
    ctx.insert("trending", &trending);
    
    // Get announcements
    let announcements = db::get_active_announcements(&conn).unwrap_or_default();
    ctx.insert("announcements", &announcements);
    
    // Get forum stats
    let stats = db::get_forum_stats(&conn).unwrap_or_default();
    ctx.insert("stats", &stats);
    
    // Get all tags for filtering
    let tags = db::get_all_tags(&conn).unwrap_or_default();
    ctx.insert("tags", &tags);
    
    ctx.insert("sort", &sort);
    ctx.insert("current_page", &"home");
    
    let html = tera.render("home.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
    (jar, Html(html))
}
