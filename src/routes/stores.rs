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
pub struct StoreForm {
    pub name: String,
    pub url: String,
    pub category: String,
}

#[derive(Deserialize)]
pub struct StoreVoteForm {
    pub positive: bool,
}

#[derive(Deserialize)]
pub struct StoreQuery {
    pub category: Option<String>,
}

pub async fn list_stores(
    jar: CookieJar,
    Query(query): Query<StoreQuery>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    let jar = if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        ctx.insert("user", &user);
        ctx.insert("can_vote", &user.role.can_vote_stores());
        jar
    } else {
        ctx.insert("can_vote", &false);
        jar
    };
    
    let conn = db.lock().unwrap();
    let stores = db::get_stores(&conn, query.category.as_deref()).unwrap_or_default();
    let categories = db::get_store_categories(&conn).unwrap_or_default();
    
    ctx.insert("stores", &stores);
    ctx.insert("store_categories", &categories);
    ctx.insert("current_category", &query.category);
    
    // Predefined categories
    let default_categories = vec![
        "OEM Parts",
        "Aftermarket Parts",
        "Tools",
        "Fluids & Chemicals",
        "Electronics",
        "General",
    ];
    ctx.insert("default_categories", &default_categories);
    
    let html = tera.render("stores.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
    (jar, Html(html))
}

pub async fn submit_store(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
    Form(form): Form<StoreForm>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        let _ = db::create_store(&conn, &form.name, &form.url, &form.category, user.id);
        
        // Return updated store list
        let stores = db::get_stores(&conn, None).unwrap_or_default();
        let categories = db::get_store_categories(&conn).unwrap_or_default();
        
        let mut ctx = Context::new();
        ctx.insert("stores", &stores);
        ctx.insert("store_categories", &categories);
        ctx.insert("user", &user);
        ctx.insert("can_vote", &user.role.can_vote_stores());
        
        let html = tera.render("partials/store_list.html", &ctx).unwrap_or_default();
        return (jar, Html(html));
    }
    
    (jar, Html("Login required".to_string()))
}

pub async fn vote_store(
    jar: CookieJar,
    Path(store_id): Path<i64>,
    State((db, _)): State<(Db, Arc<Tera>)>,
    Form(form): Form<StoreVoteForm>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.can_vote_stores() {
            return (jar, Html("Only verified mechanics can vote on stores".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let _ = db::vote_store(&conn, store_id, user.id, form.positive);
        
        // Get updated store info
        let stores = db::get_stores(&conn, None).unwrap_or_default();
        if let Some(store) = stores.iter().find(|s| s.id == store_id) {
            let score = store.reliability_score.map(|s| format!("{:.0}%", s)).unwrap_or_else(|| "N/A".to_string());
            let html = format!(
                r#"<span class="reliability">{} ({}/{})</span>"#,
                score, store.positive_votes, store.total_votes
            );
            return (jar, Html(html));
        }
        
        return (jar, Html("Updated".to_string()));
    }
    
    (jar, Html("Login required".to_string()))
}
