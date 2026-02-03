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
use crate::models::Comment;

#[derive(Deserialize)]
pub struct PostForm {
    pub category_id: i64,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub tags: Vec<i64>,
}

#[derive(Deserialize)]
pub struct EditPostForm {
    pub title: String,
    pub body: String,
}

#[derive(Deserialize)]
pub struct CommentForm {
    pub body: String,
    pub parent_id: Option<i64>,
}

#[derive(Deserialize)]
pub struct EditCommentForm {
    pub body: String,
}

#[derive(Deserialize)]
pub struct VoteForm {
    pub value: i64,
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub sort: Option<String>,
    pub page: Option<i64>,
}

#[derive(Deserialize)]
pub struct ReportForm {
    pub reason: String,
}

pub async fn category_posts(
    jar: CookieJar,
    Path(slug): Path<String>,
    Query(query): Query<ListQuery>,
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
    let category = db::get_category_by_slug(&conn, &slug);
    let categories = db::get_categories(&conn).unwrap_or_default();
    let (posts, pagination) = db::get_posts_paginated(&conn, Some(&slug), &sort, page, 25).unwrap_or_default();
    
    ctx.insert("categories", &categories);
    ctx.insert("category", &category.ok().flatten());
    ctx.insert("posts", &posts);
    ctx.insert("pagination", &pagination);
    ctx.insert("sort", &sort);
    ctx.insert("current_slug", &slug);
    
    let html = tera.render("category.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
    (jar, Html(html))
}

pub async fn new_post_page(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.can_post() {
            ctx.insert("error", "Only verified mechanics can create posts");
            ctx.insert("error_details", "Please submit your credentials for verification first.");
            let html = tera.render("error.html", &ctx).unwrap();
            return (jar, Html(html));
        }
        
        ctx.insert("user", &user);
        let conn = db.lock().unwrap();
        let categories = db::get_categories(&conn).unwrap_or_default();
        let tags = db::get_all_tags(&conn).unwrap_or_default();
        ctx.insert("categories", &categories);
        ctx.insert("tags", &tags);
        
        let html = tera.render("new_post.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
        return (jar, Html(html));
    }
    
    let html = r#"<script>window.location.href = "/login";</script>"#.to_string();
    (jar, Html(html))
}

pub async fn create_post(
    jar: CookieJar,
    State((db, tera)): State<(Db, Arc<Tera>)>,
    Form(form): Form<PostForm>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if !user.role.can_post() {
            ctx.insert("error", "Only verified mechanics can create posts");
            let html = tera.render("error.html", &ctx).unwrap();
            return (jar, Html(html));
        }
        
        // Validation
        if form.title.trim().is_empty() || form.title.len() > 300 {
            ctx.insert("error", "Title must be between 1 and 300 characters");
            ctx.insert("user", &user);
            let conn = db.lock().unwrap();
            let categories = db::get_categories(&conn).unwrap_or_default();
            ctx.insert("categories", &categories);
            let html = tera.render("new_post.html", &ctx).unwrap();
            return (jar, Html(html));
        }
        
        if form.body.trim().is_empty() {
            ctx.insert("error", "Post body cannot be empty");
            ctx.insert("user", &user);
            let conn = db.lock().unwrap();
            let categories = db::get_categories(&conn).unwrap_or_default();
            ctx.insert("categories", &categories);
            let html = tera.render("new_post.html", &ctx).unwrap();
            return (jar, Html(html));
        }
        
        let conn = db.lock().unwrap();
        match db::create_post_with_tags(&conn, user.id, form.category_id, &form.title, &form.body, &form.tags) {
            Ok(post_id) => {
                let _ = db::log_activity(&conn, user.id, "create_post", Some("post"), Some(post_id), None, None);
                let html = format!(r#"<script>window.location.href = "/post/{}";</script>"#, post_id);
                return (jar, Html(html));
            }
            Err(_) => {
                ctx.insert("error", "Failed to create post");
                ctx.insert("user", &user);
                let categories = db::get_categories(&conn).unwrap_or_default();
                ctx.insert("categories", &categories);
                let html = tera.render("new_post.html", &ctx).unwrap();
                return (jar, Html(html));
            }
        }
    }
    
    let html = r#"<script>window.location.href = "/login";</script>"#.to_string();
    (jar, Html(html))
}

pub async fn view_post(
    jar: CookieJar,
    Path(id): Path<i64>,
    Query(query): Query<ListQuery>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    let comment_sort = query.sort.unwrap_or_else(|| "best".to_string());
    
    let (jar, user_id) = if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        let unread_count = db::get_unread_notification_count(&conn, user.id).unwrap_or(0);
        ctx.insert("user", &user);
        ctx.insert("user_id", &user.id);
        ctx.insert("unread_notifications", &unread_count);
        (jar, Some(user.id))
    } else {
        (jar, None)
    };
    
    let conn = db.lock().unwrap();
    
    match db::get_post_by_id(&conn, id) {
        Ok(Some(mut post)) => {
            if post.removed && user_id.map_or(true, |uid| uid != post.user_id) {
                ctx.insert("error", "This post has been removed");
                let html = tera.render("error.html", &ctx).unwrap();
                return (jar, Html(html));
            }
            
            // Add user context if logged in
            if let Some(uid) = user_id {
                post.user_vote = db::get_user_vote_for_post(&conn, uid, id).ok().flatten();
                post.is_bookmarked = Some(db::is_post_bookmarked(&conn, uid, id).unwrap_or(false));
            }
            
            let comments = db::get_comments_for_post_sorted(&conn, id, &comment_sort).unwrap_or_default();
            let threaded = thread_comments(comments, user_id, &conn);
            
            ctx.insert("post", &post);
            ctx.insert("comments", &threaded);
            ctx.insert("comment_sort", &comment_sort);
            ctx.insert("comment_count", &threaded.len());
            
            let html = tera.render("post.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
            (jar, Html(html))
        }
        _ => {
            ctx.insert("error", "Post not found");
            let html = tera.render("error.html", &ctx).unwrap();
            (jar, Html(html))
        }
    }
}

fn thread_comments(comments: Vec<Comment>, user_id: Option<i64>, conn: &rusqlite::Connection) -> Vec<Comment> {
    let mut top_level: Vec<Comment> = vec![];
    let mut by_parent: std::collections::HashMap<i64, Vec<Comment>> = std::collections::HashMap::new();
    
    for mut c in comments {
        // Add user vote if logged in
        if let Some(uid) = user_id {
            c.user_vote = db::get_user_vote_for_comment(conn, uid, c.id).ok().flatten();
        }
        
        if let Some(parent_id) = c.parent_id {
            by_parent.entry(parent_id).or_default().push(c);
        } else {
            top_level.push(c);
        }
    }
    
    fn attach_replies(comment: &mut Comment, by_parent: &std::collections::HashMap<i64, Vec<Comment>>, depth: i32) {
        comment.depth = depth;
        if let Some(replies) = by_parent.get(&comment.id) {
            comment.replies = replies.clone();
            for reply in &mut comment.replies {
                attach_replies(reply, by_parent, depth + 1);
            }
        }
    }
    
    for c in &mut top_level {
        attach_replies(c, &by_parent, 0);
    }
    
    top_level
}

pub async fn edit_post_page(
    jar: CookieJar,
    Path(id): Path<i64>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        
        match db::get_post_by_id(&conn, id) {
            Ok(Some(post)) => {
                // Check ownership or mod status
                if post.user_id != user.id && !user.role.can_moderate() {
                    ctx.insert("error", "You don't have permission to edit this post");
                    let html = tera.render("error.html", &ctx).unwrap();
                    return (jar, Html(html));
                }
                
                ctx.insert("user", &user);
                ctx.insert("post", &post);
                let tags = db::get_all_tags(&conn).unwrap_or_default();
                ctx.insert("tags", &tags);
                
                let html = tera.render("edit_post.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
                return (jar, Html(html));
            }
            _ => {
                ctx.insert("error", "Post not found");
                let html = tera.render("error.html", &ctx).unwrap();
                return (jar, Html(html));
            }
        }
    }
    
    let html = r#"<script>window.location.href = "/login";</script>"#.to_string();
    (jar, Html(html))
}

pub async fn edit_post_submit(
    jar: CookieJar,
    Path(id): Path<i64>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
    Form(form): Form<EditPostForm>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        
        match db::get_post_by_id(&conn, id) {
            Ok(Some(post)) => {
                if post.user_id != user.id && !user.role.can_moderate() {
                    ctx.insert("error", "You don't have permission to edit this post");
                    let html = tera.render("error.html", &ctx).unwrap();
                    return (jar, Html(html));
                }
                
                if let Err(_) = db::update_post(&conn, id, user.id, &form.title, &form.body) {
                    ctx.insert("error", "Failed to update post");
                    let html = tera.render("error.html", &ctx).unwrap();
                    return (jar, Html(html));
                }
                
                let _ = db::log_activity(&conn, user.id, "edit_post", Some("post"), Some(id), None, None);
                
                let html = format!(r#"<script>window.location.href = "/post/{}";</script>"#, id);
                return (jar, Html(html));
            }
            _ => {
                ctx.insert("error", "Post not found");
                let html = tera.render("error.html", &ctx).unwrap();
                return (jar, Html(html));
            }
        }
    }
    
    let html = r#"<script>window.location.href = "/login";</script>"#.to_string();
    (jar, Html(html))
}

pub async fn delete_post(
    jar: CookieJar,
    Path(id): Path<i64>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        
        match db::get_post_by_id(&conn, id) {
            Ok(Some(post)) => {
                if post.user_id != user.id && !user.role.can_moderate() {
                    return (jar, Html("Unauthorized".to_string()));
                }
                
                let _ = db::remove_post(&conn, id);
                let _ = db::log_activity(&conn, user.id, "delete_post", Some("post"), Some(id), None, None);
                
                let html = r#"<script>window.location.href = "/";</script>"#.to_string();
                return (jar, Html(html));
            }
            _ => {
                return (jar, Html("Post not found".to_string()));
            }
        }
    }
    
    (jar, Html("Unauthorized".to_string()))
}

pub async fn add_comment(
    jar: CookieJar,
    Path(post_id): Path<i64>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
    Form(form): Form<CommentForm>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        if form.body.trim().is_empty() {
            return (jar, Html("<div class=\"toast error\">Comment cannot be empty</div>".to_string()));
        }
        
        let conn = db.lock().unwrap();
        let _ = db::create_comment(&conn, post_id, user.id, form.parent_id, &form.body);
        
        // Return updated comments partial
        let comment_sort = "best";
        let comments = db::get_comments_for_post_sorted(&conn, post_id, comment_sort).unwrap_or_default();
        let threaded = thread_comments(comments, Some(user.id), &conn);
        
        let mut ctx = Context::new();
        ctx.insert("comments", &threaded);
        ctx.insert("post_id", &post_id);
        ctx.insert("user", &user);
        ctx.insert("user_id", &user.id);
        
        // Also return a toast notification
        let comments_html = tera.render("partials/comments.html", &ctx).unwrap_or_default();
        let html = format!(r#"
            {}
            <div id="toast-container" hx-swap-oob="beforeend">
                <div class="toast success">Comment posted!</div>
            </div>
        "#, comments_html);
        
        return (jar, Html(html));
    }
    
    (jar, Html("<div class=\"toast error\">Please log in to comment</div>".to_string()))
}

pub async fn edit_comment(
    jar: CookieJar,
    Path(comment_id): Path<i64>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
    Form(form): Form<EditCommentForm>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        
        // Verify ownership
        let comment_author: Option<i64> = conn.query_row(
            "SELECT user_id FROM comments WHERE id = ?1",
            rusqlite::params![comment_id],
            |r| r.get(0)
        ).ok();
        
        if comment_author != Some(user.id) && !user.role.can_moderate() {
            return (jar, Html("<div class=\"toast error\">Unauthorized</div>".to_string()));
        }
        
        let _ = db::update_comment(&conn, comment_id, user.id, &form.body);
        
        return (jar, Html("<div class=\"toast success\">Comment updated!</div>".to_string()));
    }
    
    (jar, Html("<div class=\"toast error\">Please log in</div>".to_string()))
}

pub async fn delete_comment(
    jar: CookieJar,
    Path(comment_id): Path<i64>,
    State((db, _)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        
        let comment_author: Option<i64> = conn.query_row(
            "SELECT user_id FROM comments WHERE id = ?1",
            rusqlite::params![comment_id],
            |r| r.get(0)
        ).ok();
        
        if comment_author != Some(user.id) && !user.role.can_moderate() {
            return (jar, Html("<div class=\"toast error\">Unauthorized</div>".to_string()));
        }
        
        let _ = db::remove_comment(&conn, comment_id);
        let _ = db::log_activity(&conn, user.id, "delete_comment", Some("comment"), Some(comment_id), None, None);
        
        return (jar, Html("<div class=\"toast success comment-deleted\">Comment deleted</div>".to_string()));
    }
    
    (jar, Html("<div class=\"toast error\">Please log in</div>".to_string()))
}

pub async fn vote_post(
    jar: CookieJar,
    Path(post_id): Path<i64>,
    State((db, _)): State<(Db, Arc<Tera>)>,
    Form(form): Form<VoteForm>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        let value = if form.value > 0 { 1 } else { -1 };
        match db::vote_post(&conn, user.id, post_id, value) {
            Ok(new_score) => {
                let user_vote = db::get_user_vote_for_post(&conn, user.id, post_id).ok().flatten();
                let up_class = if user_vote == Some(1) { "voted" } else { "" };
                let down_class = if user_vote == Some(-1) { "voted" } else { "" };
                
                let html = format!(
                    "<span class=\"score\" id=\"score-{post_id}\">{new_score}</span>",
                    post_id = post_id, new_score = new_score
                );
                return (jar, Html(html));
            }
            Err(_) => {
                return (jar, Html("Error".to_string()));
            }
        }
    }
    
    (jar, Html("<span class=\"toast error\">Please log in to vote</span>".to_string()))
}

pub async fn vote_comment(
    jar: CookieJar,
    Path(comment_id): Path<i64>,
    State((db, _)): State<(Db, Arc<Tera>)>,
    Form(form): Form<VoteForm>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        let value = if form.value > 0 { 1 } else { -1 };
        match db::vote_comment(&conn, user.id, comment_id, value) {
            Ok(new_score) => {
                return (jar, Html(format!(r#"<span class="score">{}</span>"#, new_score)));
            }
            Err(_) => {
                return (jar, Html("Error".to_string()));
            }
        }
    }
    
    (jar, Html("Login required".to_string()))
}

pub async fn set_best_answer(
    jar: CookieJar,
    Path((post_id, comment_id)): Path<(i64, i64)>,
    State((db, _)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        
        // Verify post ownership
        let post_author: Option<i64> = conn.query_row(
            "SELECT user_id FROM posts WHERE id = ?1",
            rusqlite::params![post_id],
            |r| r.get(0)
        ).ok();
        
        if post_author != Some(user.id) && !user.role.can_moderate() {
            return (jar, Html("<div class=\"toast error\">Only the post author can mark best answer</div>".to_string()));
        }
        
        // Toggle best answer
        let current_best: Option<i64> = conn.query_row(
            "SELECT best_answer_id FROM posts WHERE id = ?1",
            rusqlite::params![post_id],
            |r| r.get(0)
        ).ok().flatten();
        
        if current_best == Some(comment_id) {
            let _ = db::set_best_answer(&conn, post_id, None);
        } else {
            let _ = db::set_best_answer(&conn, post_id, Some(comment_id));
            
            // Notify comment author
            let comment_author: Option<i64> = conn.query_row(
                "SELECT user_id FROM comments WHERE id = ?1",
                rusqlite::params![comment_id],
                |r| r.get(0)
            ).ok();
            
            if let Some(author_id) = comment_author {
                if author_id != user.id {
                    let _ = db::create_notification(&conn, author_id, "best_answer", "Your answer was marked as the best answer!", Some(post_id), Some(comment_id), Some(user.id));
                }
            }
        }
        
        let html = format!(r#"<script>window.location.href = "/post/{}";</script>"#, post_id);
        return (jar, Html(html));
    }
    
    (jar, Html("Login required".to_string()))
}

pub async fn report_post(
    jar: CookieJar,
    Path(post_id): Path<i64>,
    State((db, _)): State<(Db, Arc<Tera>)>,
    Form(form): Form<ReportForm>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        let _ = db::create_report(&conn, user.id, Some(post_id), None, &form.reason);
        return (jar, Html(r#"<span class="reported">✓ Reported</span>"#.to_string()));
    }
    
    (jar, Html("Login required".to_string()))
}

pub async fn report_comment(
    jar: CookieJar,
    Path(comment_id): Path<i64>,
    State((db, _)): State<(Db, Arc<Tera>)>,
    Form(form): Form<ReportForm>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        let _ = db::create_report(&conn, user.id, None, Some(comment_id), &form.reason);
        return (jar, Html(r#"<span class="reported">✓ Reported</span>"#.to_string()));
    }
    
    (jar, Html("Login required".to_string()))
}
