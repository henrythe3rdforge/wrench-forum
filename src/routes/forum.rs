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
}

#[derive(Deserialize)]
pub struct CommentForm {
    pub body: String,
    pub parent_id: Option<i64>,
}

#[derive(Deserialize)]
pub struct VoteForm {
    pub value: i64,
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub sort: Option<String>,
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
    
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        ctx.insert("user", &user);
        
        let conn = db.lock().unwrap();
        let category = db::get_category_by_slug(&conn, &slug);
        let categories = db::get_categories(&conn).unwrap_or_default();
        let posts = db::get_posts(&conn, Some(&slug), &sort, 50, 0).unwrap_or_default();
        
        ctx.insert("categories", &categories);
        ctx.insert("category", &category.ok().flatten());
        ctx.insert("posts", &posts);
        ctx.insert("sort", &sort);
        ctx.insert("current_slug", &slug);
        
        let html = tera.render("category.html", &ctx).unwrap_or_else(|e| format!("Error: {}", e));
        return (jar, Html(html));
    }
    
    let conn = db.lock().unwrap();
    let category = db::get_category_by_slug(&conn, &slug);
    let categories = db::get_categories(&conn).unwrap_or_default();
    let posts = db::get_posts(&conn, Some(&slug), &sort, 50, 0).unwrap_or_default();
    
    ctx.insert("categories", &categories);
    ctx.insert("category", &category.ok().flatten());
    ctx.insert("posts", &posts);
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
            let html = tera.render("error.html", &ctx).unwrap();
            return (jar, Html(html));
        }
        
        ctx.insert("user", &user);
        let conn = db.lock().unwrap();
        let categories = db::get_categories(&conn).unwrap_or_default();
        ctx.insert("categories", &categories);
        
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
        
        let conn = db.lock().unwrap();
        match db::create_post(&conn, user.id, form.category_id, &form.title, &form.body) {
            Ok(post_id) => {
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
    State((db, tera)): State<(Db, Arc<Tera>)>,
) -> (CookieJar, Html<String>) {
    let mut ctx = Context::new();
    
    let jar = if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        ctx.insert("user", &user);
        ctx.insert("user_id", &user.id);
        jar
    } else {
        jar
    };
    
    let conn = db.lock().unwrap();
    
    match db::get_post_by_id(&conn, id) {
        Ok(Some(post)) => {
            if post.removed {
                ctx.insert("error", "This post has been removed");
                let html = tera.render("error.html", &ctx).unwrap();
                return (jar, Html(html));
            }
            
            let comments = db::get_comments_for_post(&conn, id).unwrap_or_default();
            let threaded = thread_comments(comments);
            
            ctx.insert("post", &post);
            ctx.insert("comments", &threaded);
            
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

fn thread_comments(comments: Vec<Comment>) -> Vec<Comment> {
    let mut top_level: Vec<Comment> = vec![];
    let mut by_parent: std::collections::HashMap<i64, Vec<Comment>> = std::collections::HashMap::new();
    
    for c in comments {
        if let Some(parent_id) = c.parent_id {
            by_parent.entry(parent_id).or_default().push(c);
        } else {
            top_level.push(c);
        }
    }
    
    fn attach_replies(comment: &mut Comment, by_parent: &std::collections::HashMap<i64, Vec<Comment>>) {
        if let Some(replies) = by_parent.get(&comment.id) {
            comment.replies = replies.clone();
            for reply in &mut comment.replies {
                attach_replies(reply, by_parent);
            }
        }
    }
    
    for c in &mut top_level {
        attach_replies(c, &by_parent);
    }
    
    top_level
}

pub async fn add_comment(
    jar: CookieJar,
    Path(post_id): Path<i64>,
    State((db, tera)): State<(Db, Arc<Tera>)>,
    Form(form): Form<CommentForm>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        let _ = db::create_comment(&conn, post_id, user.id, form.parent_id, &form.body);
        
        // Return updated comments partial
        let comments = db::get_comments_for_post(&conn, post_id).unwrap_or_default();
        let threaded = thread_comments(comments);
        
        let mut ctx = Context::new();
        ctx.insert("comments", &threaded);
        ctx.insert("post_id", &post_id);
        ctx.insert("user", &user);
        ctx.insert("user_id", &user.id);
        
        let html = tera.render("partials/comments.html", &ctx).unwrap_or_default();
        return (jar, Html(html));
    }
    
    (jar, Html("Login required".to_string()))
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
                return (jar, Html(format!(r#"<span class="score">{}</span>"#, new_score)));
            }
            Err(_) => {
                return (jar, Html("Error".to_string()));
            }
        }
    }
    
    (jar, Html("Login required".to_string()))
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

pub async fn report_post(
    jar: CookieJar,
    Path(post_id): Path<i64>,
    State((db, _)): State<(Db, Arc<Tera>)>,
    Form(form): Form<ReportForm>,
) -> (CookieJar, Html<String>) {
    if let Some((user, jar)) = ensure_session(jar.clone(), &db) {
        let conn = db.lock().unwrap();
        let _ = db::create_report(&conn, user.id, Some(post_id), None, &form.reason);
        return (jar, Html(r#"<span class="reported">Reported ✓</span>"#.to_string()));
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
        return (jar, Html(r#"<span class="reported">Reported ✓</span>"#.to_string()));
    }
    
    (jar, Html("Login required".to_string()))
}
