use axum::{
    extract::{Multipart, State},
    response::Html,
};
use axum_extra::extract::CookieJar;
use std::sync::Arc;
use tera::Tera;
use uuid::Uuid;

use crate::auth::ensure_session;
use crate::db::{self, Db};

const MAX_FILE_SIZE: usize = 5 * 1024 * 1024; // 5MB
const ALLOWED_TYPES: &[&str] = &["image/jpeg", "image/png", "image/gif", "image/webp"];

pub async fn upload_file(
    jar: CookieJar,
    State((db, _)): State<(Db, Arc<Tera>)>,
    mut multipart: Multipart,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        while let Some(field) = multipart.next_field().await.ok().flatten() {
            let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
            let original_name = field.file_name().unwrap_or("upload").to_string();
            
            // Check content type
            if !ALLOWED_TYPES.contains(&content_type.as_str()) {
                return (jar, Html(format!(
                    r#"{{"error": "Invalid file type. Allowed: JPEG, PNG, GIF, WebP"}}"#
                )));
            }
            
            let data = match field.bytes().await {
                Ok(d) => d,
                Err(_) => {
                    return (jar, Html(r#"{"error": "Failed to read file"}"#.to_string()));
                }
            };
            
            // Check file size
            if data.len() > MAX_FILE_SIZE {
                return (jar, Html(r#"{"error": "File too large. Maximum 5MB."}"#.to_string()));
            }
            
            // Generate unique filename
            let ext = match content_type.as_str() {
                "image/jpeg" => "jpg",
                "image/png" => "png",
                "image/gif" => "gif",
                "image/webp" => "webp",
                _ => "bin",
            };
            let filename = format!("{}.{}", Uuid::new_v4(), ext);
            let path = format!("static/uploads/{}", filename);
            
            // Write file
            if let Err(_) = std::fs::write(&path, &data) {
                return (jar, Html(r#"{"error": "Failed to save file"}"#.to_string()));
            }
            
            // Save to database
            let conn = db.lock().unwrap();
            match db::create_upload(&conn, user.id, &filename, &original_name, &path, &content_type, data.len() as i64) {
                Ok(upload_id) => {
                    let url = format!("/static/uploads/{}", filename);
                    return (jar, Html(format!(
                        r#"{{"success": true, "url": "{}", "id": {}}}"#,
                        url, upload_id
                    )));
                }
                Err(_) => {
                    // Clean up file on db error
                    let _ = std::fs::remove_file(&path);
                    return (jar, Html(r#"{"error": "Failed to save upload record"}"#.to_string()));
                }
            }
        }
        
        return (jar, Html(r#"{"error": "No file provided"}"#.to_string()));
    }
    
    (jar, Html(r#"{"error": "Please log in"}"#.to_string()))
}

pub async fn upload_avatar(
    jar: CookieJar,
    State((db, _)): State<(Db, Arc<Tera>)>,
    mut multipart: Multipart,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        while let Some(field) = multipart.next_field().await.ok().flatten() {
            let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
            
            if !ALLOWED_TYPES.contains(&content_type.as_str()) {
                return (jar, Html("<div class=\"toast error\">Invalid file type</div>".to_string()));
            }
            
            let data = match field.bytes().await {
                Ok(d) => d,
                Err(_) => {
                    return (jar, Html("<div class=\"toast error\">Failed to read file</div>".to_string()));
                }
            };
            
            // Smaller limit for avatars
            if data.len() > 2 * 1024 * 1024 {
                return (jar, Html("<div class=\"toast error\">Avatar must be under 2MB</div>".to_string()));
            }
            
            let ext = match content_type.as_str() {
                "image/jpeg" => "jpg",
                "image/png" => "png",
                "image/gif" => "gif",
                "image/webp" => "webp",
                _ => "bin",
            };
            let filename = format!("avatar_{}_{}.{}", user.id, Uuid::new_v4(), ext);
            let path = format!("static/uploads/{}", filename);
            
            if let Err(_) = std::fs::write(&path, &data) {
                return (jar, Html("<div class=\"toast error\">Failed to save avatar</div>".to_string()));
            }
            
            let conn = db.lock().unwrap();
            let avatar_url = format!("/static/uploads/{}", filename);
            
            // Delete old avatar if exists
            if let Ok(Some(profile)) = db::get_user_profile(&conn, user.id) {
                if let Some(old_path) = profile.avatar_path {
                    if old_path.starts_with("/static/uploads/") {
                        let old_file = old_path.trim_start_matches('/');
                        let _ = std::fs::remove_file(old_file);
                    }
                }
            }
            
            let _ = db::update_user_avatar(&conn, user.id, &avatar_url);
            
            return (jar, Html(format!(
                r#"<img src="{}" class="avatar-preview" alt="Avatar">
                <div id="toast-container" hx-swap-oob="beforeend">
                    <div class="toast success">Avatar updated!</div>
                </div>"#,
                avatar_url
            )));
        }
        
        return (jar, Html("<div class=\"toast error\">No file provided</div>".to_string()));
    }
    
    (jar, Html("<div class=\"toast error\">Please log in</div>".to_string()))
}
