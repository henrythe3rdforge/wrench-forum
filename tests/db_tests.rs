use wrench_forum::db;
use wrench_forum::models::*;
use tempfile::NamedTempFile;

fn setup_test_db() -> db::Db {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap();
    db::init_db_with_path(path).expect("Failed to init test db")
}

// ============ User Tests ============

#[test]
fn test_create_and_get_user() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "test@example.com", "hash123", "testuser")
        .expect("Failed to create user");
    
    assert!(user_id > 0);
    
    let user = db::get_user_by_id(&conn, user_id)
        .expect("Query failed")
        .expect("User not found");
    
    assert_eq!(user.id, user_id);
    assert_eq!(user.username, "testuser");
    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.role, UserRole::Unverified);
    assert!(!user.banned);
    assert_eq!(user.karma, 0);
}

#[test]
fn test_get_user_by_email() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    db::create_user(&conn, "test@example.com", "hash123", "testuser").unwrap();
    
    let (user, password_hash) = db::get_user_by_email(&conn, "test@example.com")
        .expect("Query failed")
        .expect("User not found");
    
    assert_eq!(user.username, "testuser");
    assert_eq!(password_hash, "hash123");
}

#[test]
fn test_get_user_by_username() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    db::create_user(&conn, "test@example.com", "hash123", "testuser").unwrap();
    
    let user = db::get_user_by_username(&conn, "testuser")
        .expect("Query failed")
        .expect("User not found");
    
    assert_eq!(user.email, "test@example.com");
}

#[test]
fn test_update_user_role() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "test@example.com", "hash123", "testuser").unwrap();
    
    db::update_user_role(&conn, user_id, "verified_mechanic").unwrap();
    
    let user = db::get_user_by_id(&conn, user_id).unwrap().unwrap();
    assert_eq!(user.role, UserRole::VerifiedMechanic);
}

#[test]
fn test_ban_user() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "test@example.com", "hash123", "testuser").unwrap();
    
    db::set_user_banned(&conn, user_id, true).unwrap();
    
    let user = db::get_user_by_id(&conn, user_id).unwrap().unwrap();
    assert!(user.banned);
    
    let banned_users = db::get_banned_users(&conn).unwrap();
    assert_eq!(banned_users.len(), 1);
    
    db::set_user_banned(&conn, user_id, false).unwrap();
    let user = db::get_user_by_id(&conn, user_id).unwrap().unwrap();
    assert!(!user.banned);
}

#[test]
fn test_user_karma() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "test@example.com", "hash123", "testuser").unwrap();
    
    db::update_user_karma(&conn, user_id, 10).unwrap();
    let user = db::get_user_by_id(&conn, user_id).unwrap().unwrap();
    assert_eq!(user.karma, 10);
    
    db::update_user_karma(&conn, user_id, -5).unwrap();
    let user = db::get_user_by_id(&conn, user_id).unwrap().unwrap();
    assert_eq!(user.karma, 5);
}

// ============ Session Tests ============

#[test]
fn test_session_lifecycle() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "test@example.com", "hash123", "testuser").unwrap();
    
    db::create_session(&conn, "token123", user_id, "2030-01-01 00:00:00").unwrap();
    
    let session = db::get_session(&conn, "token123")
        .expect("Query failed")
        .expect("Session not found");
    
    assert_eq!(session.token, "token123");
    assert_eq!(session.user_id, user_id);
    
    db::delete_session(&conn, "token123").unwrap();
    
    let session = db::get_session(&conn, "token123").unwrap();
    assert!(session.is_none());
}

// ============ Category Tests ============

#[test]
fn test_categories() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    // Default categories are seeded
    let categories = db::get_categories(&conn).unwrap();
    assert!(!categories.is_empty());
    
    let engine = db::get_category_by_slug(&conn, "engine").unwrap();
    assert!(engine.is_some());
    assert_eq!(engine.unwrap().name, "Engine");
}

// ============ Post Tests ============

#[test]
fn test_create_and_get_post() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "test@example.com", "hash123", "testuser").unwrap();
    let categories = db::get_categories(&conn).unwrap();
    let category_id = categories[0].id;
    
    let post_id = db::create_post(&conn, user_id, category_id, "Test Title", "Test body content")
        .expect("Failed to create post");
    
    let post = db::get_post_by_id(&conn, post_id).unwrap().unwrap();
    
    assert_eq!(post.title, "Test Title");
    assert_eq!(post.body, "Test body content");
    assert_eq!(post.score, 1); // Auto-upvoted
    assert!(!post.removed);
}

#[test]
fn test_get_posts_sorted() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "test@example.com", "hash123", "testuser").unwrap();
    let categories = db::get_categories(&conn).unwrap();
    let category_id = categories[0].id;
    
    db::create_post(&conn, user_id, category_id, "Post 1", "Content 1").unwrap();
    db::create_post(&conn, user_id, category_id, "Post 2", "Content 2").unwrap();
    
    let posts = db::get_posts(&conn, None, "new", 10, 0).unwrap();
    assert_eq!(posts.len(), 2);
    
    // Newest first
    assert_eq!(posts[0].title, "Post 2");
    assert_eq!(posts[1].title, "Post 1");
}

#[test]
fn test_post_pagination() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "test@example.com", "hash123", "testuser").unwrap();
    let categories = db::get_categories(&conn).unwrap();
    let category_id = categories[0].id;
    
    for i in 0..15 {
        db::create_post(&conn, user_id, category_id, &format!("Post {}", i), "Content").unwrap();
    }
    
    let (posts, pagination) = db::get_posts_paginated(&conn, None, "new", 1, 10).unwrap();
    assert_eq!(posts.len(), 10);
    assert_eq!(pagination.total_items, 15);
    assert_eq!(pagination.total_pages, 2);
    assert!(pagination.has_next);
    assert!(!pagination.has_prev);
}

#[test]
fn test_update_post() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "test@example.com", "hash123", "testuser").unwrap();
    let categories = db::get_categories(&conn).unwrap();
    let post_id = db::create_post(&conn, user_id, categories[0].id, "Original", "Body").unwrap();
    
    db::update_post(&conn, post_id, user_id, "Updated Title", "Updated body").unwrap();
    
    let post = db::get_post_by_id(&conn, post_id).unwrap().unwrap();
    assert_eq!(post.title, "Updated Title");
    assert!(post.edited_at.is_some());
}

#[test]
fn test_remove_and_restore_post() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "test@example.com", "hash123", "testuser").unwrap();
    let categories = db::get_categories(&conn).unwrap();
    let post_id = db::create_post(&conn, user_id, categories[0].id, "Test", "Body").unwrap();
    
    db::remove_post(&conn, post_id).unwrap();
    let post = db::get_post_by_id(&conn, post_id).unwrap().unwrap();
    assert!(post.removed);
    
    db::restore_post(&conn, post_id).unwrap();
    let post = db::get_post_by_id(&conn, post_id).unwrap().unwrap();
    assert!(!post.removed);
}

// ============ Comment Tests ============

#[test]
fn test_create_comment() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "test@example.com", "hash123", "testuser").unwrap();
    let categories = db::get_categories(&conn).unwrap();
    let post_id = db::create_post(&conn, user_id, categories[0].id, "Test", "Body").unwrap();
    
    let comment_id = db::create_comment(&conn, post_id, user_id, None, "My comment").unwrap();
    
    let comments = db::get_comments_for_post(&conn, post_id).unwrap();
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].body, "My comment");
    assert_eq!(comments[0].score, 1);
}

#[test]
fn test_nested_comments() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "test@example.com", "hash123", "testuser").unwrap();
    let categories = db::get_categories(&conn).unwrap();
    let post_id = db::create_post(&conn, user_id, categories[0].id, "Test", "Body").unwrap();
    
    let parent_id = db::create_comment(&conn, post_id, user_id, None, "Parent comment").unwrap();
    let _child_id = db::create_comment(&conn, post_id, user_id, Some(parent_id), "Reply").unwrap();
    
    let comments = db::get_comments_for_post(&conn, post_id).unwrap();
    assert_eq!(comments.len(), 2);
}

// ============ Vote Tests ============

#[test]
fn test_vote_post() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user1_id = db::create_user(&conn, "user1@example.com", "hash", "user1").unwrap();
    let user2_id = db::create_user(&conn, "user2@example.com", "hash", "user2").unwrap();
    let categories = db::get_categories(&conn).unwrap();
    let post_id = db::create_post(&conn, user1_id, categories[0].id, "Test", "Body").unwrap();
    
    // Initial score is 1 (auto-upvote)
    let post = db::get_post_by_id(&conn, post_id).unwrap().unwrap();
    assert_eq!(post.score, 1);
    
    // User2 upvotes
    let new_score = db::vote_post(&conn, user2_id, post_id, 1).unwrap();
    assert_eq!(new_score, 2);
    
    // User2 changes to downvote
    let new_score = db::vote_post(&conn, user2_id, post_id, -1).unwrap();
    assert_eq!(new_score, 0);
    
    // User2 removes vote (click downvote again)
    let new_score = db::vote_post(&conn, user2_id, post_id, -1).unwrap();
    assert_eq!(new_score, 1);
}

#[test]
fn test_get_user_vote() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user1_id = db::create_user(&conn, "user1@example.com", "hash", "user1").unwrap();
    let user2_id = db::create_user(&conn, "user2@example.com", "hash", "user2").unwrap();
    let categories = db::get_categories(&conn).unwrap();
    let post_id = db::create_post(&conn, user1_id, categories[0].id, "Test", "Body").unwrap();
    
    // user1 auto-upvoted
    let vote = db::get_user_vote_for_post(&conn, user1_id, post_id).unwrap();
    assert_eq!(vote, Some(1));
    
    // user2 hasn't voted
    let vote = db::get_user_vote_for_post(&conn, user2_id, post_id).unwrap();
    assert_eq!(vote, None);
}

// ============ Store Tests ============

#[test]
fn test_create_and_get_store() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "test@example.com", "hash", "testuser").unwrap();
    
    let store_id = db::create_store(&conn, "Test Store", "https://test.com", Some("A great store"), "General", user_id).unwrap();
    
    let stores = db::get_stores(&conn, None).unwrap();
    assert_eq!(stores.len(), 1);
    assert_eq!(stores[0].name, "Test Store");
}

#[test]
fn test_store_voting() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user1_id = db::create_user(&conn, "user1@example.com", "hash", "user1").unwrap();
    let user2_id = db::create_user(&conn, "user2@example.com", "hash", "user2").unwrap();
    
    let store_id = db::create_store(&conn, "Test Store", "https://test.com", None, "General", user1_id).unwrap();
    
    db::vote_store(&conn, store_id, user1_id, true).unwrap();
    db::vote_store(&conn, store_id, user2_id, true).unwrap();
    
    let stores = db::get_stores(&conn, None).unwrap();
    let store = stores.iter().find(|s| s.id == store_id).unwrap();
    assert_eq!(store.positive_votes, 2);
    assert_eq!(store.total_votes, 2);
    assert_eq!(store.reliability_score, Some(100.0));
}

// ============ Bookmark Tests ============

#[test]
fn test_bookmarks() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "test@example.com", "hash", "testuser").unwrap();
    let categories = db::get_categories(&conn).unwrap();
    let post_id = db::create_post(&conn, user_id, categories[0].id, "Test", "Body").unwrap();
    
    assert!(!db::is_post_bookmarked(&conn, user_id, post_id).unwrap());
    
    db::add_bookmark(&conn, user_id, post_id).unwrap();
    assert!(db::is_post_bookmarked(&conn, user_id, post_id).unwrap());
    
    let bookmarks = db::get_user_bookmarks(&conn, user_id).unwrap();
    assert_eq!(bookmarks.len(), 1);
    
    db::remove_bookmark(&conn, user_id, post_id).unwrap();
    assert!(!db::is_post_bookmarked(&conn, user_id, post_id).unwrap());
}

// ============ Notification Tests ============

#[test]
fn test_notifications() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "test@example.com", "hash", "testuser").unwrap();
    
    db::create_notification(&conn, user_id, "reply", "Test notification", None, None, None).unwrap();
    
    let notifications = db::get_user_notifications(&conn, user_id, 10).unwrap();
    assert_eq!(notifications.len(), 1);
    assert!(!notifications[0].read);
    
    let unread = db::get_unread_notification_count(&conn, user_id).unwrap();
    assert_eq!(unread, 1);
    
    db::mark_notification_read(&conn, notifications[0].id).unwrap();
    let unread = db::get_unread_notification_count(&conn, user_id).unwrap();
    assert_eq!(unread, 0);
}

// ============ Verification Tests ============

#[test]
fn test_verification_request() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "test@example.com", "hash", "testuser").unwrap();
    
    db::create_verification_request(&conn, user_id, "I am ASE certified", "ase_cert").unwrap();
    
    assert!(db::has_pending_verification(&conn, user_id).unwrap());
    
    let pending = db::get_pending_verification_requests(&conn).unwrap();
    assert_eq!(pending.len(), 1);
}

// ============ Report Tests ============

#[test]
fn test_reports() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "test@example.com", "hash", "testuser").unwrap();
    let categories = db::get_categories(&conn).unwrap();
    let post_id = db::create_post(&conn, user_id, categories[0].id, "Test", "Body").unwrap();
    
    db::create_report(&conn, user_id, Some(post_id), None, "Spam").unwrap();
    
    let reports = db::get_unresolved_reports(&conn).unwrap();
    assert_eq!(reports.len(), 1);
    
    db::resolve_report(&conn, reports[0].id).unwrap();
    let reports = db::get_unresolved_reports(&conn).unwrap();
    assert_eq!(reports.len(), 0);
}

// ============ Search Tests ============

#[test]
fn test_search_posts() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "test@example.com", "hash", "testuser").unwrap();
    let categories = db::get_categories(&conn).unwrap();
    
    db::create_post(&conn, user_id, categories[0].id, "How to fix brakes", "Brake pads replacement guide").unwrap();
    db::create_post(&conn, user_id, categories[0].id, "Engine overheating", "Coolant issues").unwrap();
    
    let results = db::search_posts(&conn, "brake", None, 10).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].title.contains("brakes"));
}

// ============ Stats Tests ============

#[test]
fn test_forum_stats() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "test@example.com", "hash", "testuser").unwrap();
    let categories = db::get_categories(&conn).unwrap();
    db::create_post(&conn, user_id, categories[0].id, "Test", "Body").unwrap();
    
    let stats = db::get_forum_stats(&conn).unwrap();
    assert_eq!(stats.total_users, 1);
    assert_eq!(stats.total_posts, 1);
}
