use axum::{
    extract::State,
    response::Html,
};
use axum_extra::extract::CookieJar;
use std::sync::Arc;
use tera::{Context, Tera};

use crate::auth::ensure_session;
use crate::db::{self, Db};

pub async fn index(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    let user_jar = ensure_session(jar.clone(), &db);
    if let Some((user, jar)) = user_jar {
        ctx.insert("user", &user);
        
        let conn = db.lock().unwrap();
        let categories = db::get_categories(&conn).unwrap_or_default();
        let posts = db::get_posts(&conn, None, "hot", 20, 0).unwrap_or_default();
        ctx.insert("categories", &categories);
        ctx.insert("posts", &posts);
        
        let html = tera.render("home.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
        return (jar, Html(html));
    }
    
    let conn = db.lock().unwrap();
    let categories = db::get_categories(&conn).unwrap_or_default();
    let posts = db::get_posts(&conn, None, "hot", 20, 0).unwrap_or_default();
    ctx.insert("categories", &categories);
    ctx.insert("posts", &posts);
    
    let html = tera.render("home.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
    (jar, Html(html))
}
