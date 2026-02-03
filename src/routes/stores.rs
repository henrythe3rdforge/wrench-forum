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
    pub description: Option<String>,
    pub category: String,
}

#[derive(Deserialize)]
pub struct StoreQuery {
    pub category: Option<String>,
}

#[derive(Deserialize)]
pub struct VoteForm {
    pub positive: bool,
}

pub async fn list_stores(
    jar: CookieJar,
    Query(query): Query<StoreQuery>,
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
    let stores = db::get_stores(&conn, query.category.as_deref()).unwrap_or_default();
    let categories = db::get_store_categories(&conn).unwrap_or_default();
    
    ctx.insert("stores", &stores);
    ctx.insert("store_categories", &categories);
    ctx.insert("selected_category", &query.category);
    ctx.insert("current_page", &"stores");
    
    let html = tera.render("stores.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
    (jar, Html(html))
}

pub async fn submit_store(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
    Form(form): Form<StoreForm>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.can_vote_stores() {
            return (jar, Html("<div class=\"toast error\">Only verified mechanics can submit stores</div>".to_string()));
        }
        
        // Validation
        if form.name.trim().is_empty() {
            return (jar, Html("<div class=\"toast error\">Store name is required</div>".to_string()));
        }
        
        if form.url.trim().is_empty() {
            return (jar, Html("<div class=\"toast error\">Store URL is required</div>".to_string()));
        }
        
        // Basic URL validation
        if !form.url.starts_with("http://") && !form.url.starts_with("https://") {
            return (jar, Html("<div class=\"toast error\">URL must start with http:// or https://</div>".to_string()));
        }
        
        let conn = db.lock().unwrap();
        match db::create_store(&conn, &form.name, &form.url, form.description.as_deref(), &form.category, user.id) {
            Ok(_) => {
                // Return updated store list
                let stores = db::get_stores(&conn, None).unwrap_or_default();
                ctx.insert("stores", &stores);
                ctx.insert("user", &user);
                
                let stores_html = tera.render("partials/store_list.html", &ctx).unwrap_or_default();
                let html = format!(r#"
                    {}
                    <div id="toast-container" hx-swap-oob="beforeend">
                        <div class="toast success">Store submitted successfully!</div>
                    </div>
                "#, stores_html);
                return (jar, Html(html));
            }
            Err(_) => {
                return (jar, Html("<div class=\"toast error\">Failed to submit store</div>".to_string()));
            }
        }
    }
    
    (jar, Html("<div class=\"toast error\">Please log in</div>".to_string()))
}

pub async fn vote_store(
    jar: CookieJar,
    Path(store_id): Path<i64>,
    State((db, _)): State<(Db, Arc<Tera>)>,
    Form(form): Form<VoteForm>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.can_vote_stores() {
            return (jar, Html("<div class=\"toast error\">Only verified mechanics can vote on stores</div>".to_string()));
        }
        
        let conn = db.lock().unwrap();
        
        // Check if user already voted the same way
        let existing_vote = db::get_user_store_vote(&conn, store_id, user.id).ok().flatten();
        
        if existing_vote == Some(form.positive) {
            // Remove vote (toggle off)
            // For now, just show they already voted
            return (jar, Html("<div class=\"toast\">Vote recorded</div>".to_string()));
        }
        
        let _ = db::vote_store(&conn, store_id, user.id, form.positive);
        
        // Get updated store info
        let stores = db::get_stores(&conn, None).unwrap_or_default();
        if let Some(store) = stores.iter().find(|s| s.id == store_id) {
            let score_class = if let Some(score) = store.reliability_score {
                if score >= 70.0 { "good" } else if score >= 40.0 { "neutral" } else { "bad" }
            } else {
                "neutral"
            };
            
            return (jar, Html(format!(
                r#"<div class="reliability">
                    <span class="score {}">{:.0}%</span>
                    <span class="votes">({} votes)</span>
                </div>
                <div id="toast-container" hx-swap-oob="beforeend">
                    <div class="toast success">Vote recorded!</div>
                </div>"#,
                score_class,
                store.reliability_score.unwrap_or(0.0),
                store.total_votes
            )));
        }
        
        return (jar, Html("<div class=\"toast success\">Vote recorded!</div>".to_string()));
    }
    
    (jar, Html("<div class=\"toast error\">Please log in</div>".to_string()))
}
