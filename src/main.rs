use axum::{
    routing::{get, post, delete},
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
    // Create uploads directory if it doesn't exist
    std::fs::create_dir_all("static/uploads").ok();
    
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
        // ============ Home ============
        .route("/", get(routes::home::index))
        
        // ============ Auth ============
        .route("/register", get(routes::auth::register_page))
        .route("/register", post(routes::auth::register_submit))
        .route("/login", get(routes::auth::login_page))
        .route("/login", post(routes::auth::login_submit))
        .route("/logout", get(routes::auth::logout))
        
        // ============ Forum ============
        .route("/category/{slug}", get(routes::forum::category_posts))
        .route("/post/new", get(routes::forum::new_post_page))
        .route("/post/new", post(routes::forum::create_post))
        .route("/post/{id}", get(routes::forum::view_post))
        .route("/post/{id}/edit", get(routes::forum::edit_post_page))
        .route("/post/{id}/edit", post(routes::forum::edit_post_submit))
        .route("/post/{id}/delete", post(routes::forum::delete_post))
        .route("/post/{id}/comment", post(routes::forum::add_comment))
        .route("/post/{id}/vote", post(routes::forum::vote_post))
        .route("/post/{id}/best-answer/{comment_id}", post(routes::forum::set_best_answer))
        .route("/comment/{id}/vote", post(routes::forum::vote_comment))
        .route("/comment/{id}/edit", post(routes::forum::edit_comment))
        .route("/comment/{id}/delete", post(routes::forum::delete_comment))
        .route("/post/{id}/report", post(routes::forum::report_post))
        .route("/comment/{id}/report", post(routes::forum::report_comment))
        
        // ============ Search ============
        .route("/search", get(routes::search::search_page))
        .route("/api/search", get(routes::search::search_api))
        .route("/api/search/suggestions", get(routes::search::search_suggestions))
        
        // ============ Stores ============
        .route("/stores", get(routes::stores::list_stores))
        .route("/stores/submit", post(routes::stores::submit_store))
        .route("/store/{id}/vote", post(routes::stores::vote_store))
        
        // ============ Profile ============
        .route("/profile", get(routes::profile::my_profile))
        .route("/profile/edit", get(routes::profile::edit_profile_page))
        .route("/profile/edit", post(routes::profile::edit_profile_submit))
        .route("/user/{username}", get(routes::profile::view_profile))
        .route("/user/{username}/posts", get(routes::profile::user_posts))
        .route("/user/{username}/comments", get(routes::profile::user_comments))
        
        // ============ Bookmarks ============
        .route("/bookmarks", get(routes::bookmarks::list_bookmarks))
        .route("/post/{id}/bookmark", post(routes::bookmarks::toggle_bookmark))
        
        // ============ Notifications ============
        .route("/notifications", get(routes::notifications::list_notifications))
        .route("/notifications/count", get(routes::notifications::notification_count))
        .route("/notifications/{id}/read", post(routes::notifications::mark_read))
        .route("/notifications/read-all", post(routes::notifications::mark_all_read))
        
        // ============ Verification ============
        .route("/verification", get(routes::verification::verification_page))
        .route("/verification", post(routes::verification::submit_verification))
        
        // ============ Admin ============
        .route("/admin", get(routes::admin::admin_panel))
        .route("/admin/verify/{id}/approve", post(routes::admin::approve_verification))
        .route("/admin/verify/{id}/deny", post(routes::admin::deny_verification))
        .route("/admin/user/{id}/role", post(routes::admin::update_user_role))
        .route("/admin/user/{id}/flair", post(routes::admin::update_user_flair))
        .route("/admin/announcement", post(routes::admin::create_announcement))
        .route("/admin/announcement/{id}/deactivate", post(routes::admin::deactivate_announcement))
        .route("/admin/stats", get(routes::admin::forum_stats))
        .route("/admin/activity", get(routes::admin::activity_logs))
        
        // ============ Moderation ============
        .route("/mod", get(routes::moderation::mod_queue))
        .route("/mod/post/{id}/remove", post(routes::moderation::remove_post))
        .route("/mod/post/{id}/restore", post(routes::moderation::restore_post))
        .route("/mod/post/{id}/pin", post(routes::moderation::pin_post))
        .route("/mod/comment/{id}/remove", post(routes::moderation::remove_comment))
        .route("/mod/user/{id}/ban", post(routes::moderation::ban_user))
        .route("/mod/user/{id}/unban", post(routes::moderation::unban_user))
        .route("/mod/report/{id}/resolve", post(routes::moderation::resolve_report))
        
        // ============ Uploads ============
        .route("/upload", post(routes::uploads::upload_file))
        .route("/upload/avatar", post(routes::uploads::upload_avatar))
        
        // ============ Static Files ============
        .nest_service("/static", ServeDir::new("static"))
        
        .with_state(state);
    
    println!("🔧 Wrench Forum running at http://localhost:3000");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
