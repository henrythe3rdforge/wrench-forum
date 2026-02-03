use axum::{
    extract::{Query, State},
    response::{Html, Json},
};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tera::{Context, Tera};

use crate::auth::ensure_session;
use crate::db::{self, Db};

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub category: Option<String>,
    pub sort: Option<String>,
    pub time: Option<String>,
}

#[derive(Serialize)]
pub struct SearchSuggestion {
    pub text: String,
    pub url: String,
    pub result_type: String,
}

pub async fn search_page(
    jar: CookieJar,
    Query(query): Query<SearchQuery>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
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
    let categories = db::get_categories(&conn).unwrap_or_default();
    ctx.insert("categories", &categories);
    
    if let Some(ref q) = query.q {
        if !q.trim().is_empty() {
            // Search posts
            let posts = db::search_posts(&conn, q, query.category.as_deref(), 50).unwrap_or_default();
            ctx.insert("posts", &posts);
            
            // Search stores
            let stores = db::search_stores(&conn, q).unwrap_or_default();
            ctx.insert("stores", &stores);
            
            ctx.insert("query", q);
            ctx.insert("result_count", &(posts.len() + stores.len()));
        }
    }
    
    ctx.insert("selected_category", &query.category);
    ctx.insert("sort", &query.sort.unwrap_or_else(|| "relevance".to_string()));
    ctx.insert("time", &query.time.unwrap_or_else(|| "all".to_string()));
    
    let html = tera.render("search.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
    (jar, Html(html))
}

pub async fn search_api(
    jar: CookieJar,
    Query(query): Query<SearchQuery>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, _)) = ensure_session(jar.clone(), &db) {
        ctx.insert("user", &user);
    }
    
    let conn = db.lock().unwrap();
    
    if let Some(ref q) = query.q {
        if !q.trim().is_empty() {
            let posts = db::search_posts(&conn, q, query.category.as_deref(), 20).unwrap_or_default();
            ctx.insert("posts", &posts);
            ctx.insert("query", q);
        }
    }
    
    let html = tera.render("partials/search_results.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
    (jar, Html(html))
}

pub async fn search_suggestions(
    Query(query): Query<SearchQuery>,
    State((db, _)): State<(Db, Arc<Tera>)>,
) -> Json<Vec<SearchSuggestion>> {
    if let Some(q) = query.q {
        if q.len() >= 2 {
            let conn = db.lock().unwrap();
            let results = db::global_search(&conn, &q, 5).unwrap_or_default();
            
            let suggestions: Vec<SearchSuggestion> = results.into_iter().map(|r| {
                SearchSuggestion {
                    text: r.title,
                    url: r.url,
                    result_type: r.result_type,
                }
            }).collect();
            
            return Json(suggestions);
        }
    }
    
    Json(vec![])
}
