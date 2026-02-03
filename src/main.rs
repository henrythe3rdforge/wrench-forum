use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tera::Tera;
use tower_http::services::ServeDir;

mod auth;
mod db;
mod models;
mod routes;

#[tokio::main]
async fn main() {
    // Initialize database
    let db = db::init_db().expect("Failed to initialize database");
    
    // Initialize templates
    let tera = match Tera::new("templates/**/*.html") {
        Ok(t) => Arc::new(t),
        Err(e) => {
            eprintln!("Template parsing error: {}", e);
            std::process::exit(1);
        }
    };
    
    let state = (db, tera);
    
    // Build router
    let app = Router::new()
        // Home
        .route("/", get(routes::home::index))
        
        // Auth
        .route("/register", get(routes::auth::register_page))
        .route("/register", post(routes::auth::register_submit))
        .route("/login", get(routes::auth::login_page))
        .route("/login", post(routes::auth::login_submit))
        .route("/logout", get(routes::auth::logout))
        
        // Forum
        .route("/category/{slug}", get(routes::forum::category_posts))
        .route("/post/new", get(routes::forum::new_post_page))
        .route("/post/new", post(routes::forum::create_post))
        .route("/post/{id}", get(routes::forum::view_post))
        .route("/post/{id}/comment", post(routes::forum::add_comment))
        .route("/post/{id}/vote", post(routes::forum::vote_post))
        .route("/comment/{id}/vote", post(routes::forum::vote_comment))
        .route("/post/{id}/report", post(routes::forum::report_post))
        .route("/comment/{id}/report", post(routes::forum::report_comment))
        
        // Stores
        .route("/stores", get(routes::stores::list_stores))
        .route("/stores/submit", post(routes::stores::submit_store))
        .route("/store/{id}/vote", post(routes::stores::vote_store))
        
        // Profile
        .route("/profile", get(routes::profile::my_profile))
        .route("/user/{username}", get(routes::profile::view_profile))
        
        // Verification
        .route("/verification", get(routes::verification::verification_page))
        .route("/verification", post(routes::verification::submit_verification))
        
        // Admin
        .route("/admin", get(routes::admin::admin_panel))
        .route("/admin/verify/{id}/approve", post(routes::admin::approve_verification))
        .route("/admin/verify/{id}/deny", post(routes::admin::deny_verification))
        .route("/admin/user/{id}/role", post(routes::admin::update_user_role))
        
        // Moderation
        .route("/mod", get(routes::moderation::mod_queue))
        .route("/mod/post/{id}/remove", post(routes::moderation::remove_post))
        .route("/mod/comment/{id}/remove", post(routes::moderation::remove_comment))
        .route("/mod/user/{id}/ban", post(routes::moderation::ban_user))
        .route("/mod/user/{id}/unban", post(routes::moderation::unban_user))
        .route("/mod/report/{id}/resolve", post(routes::moderation::resolve_report))
        
        // Static files
        .nest_service("/static", ServeDir::new("static"))
        
        .with_state(state);
    
    println!("🔧 Wrench Forum running at http://localhost:3000");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
