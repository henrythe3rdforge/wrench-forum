use wrench_forum::db;
use wrench_forum::models::*;
use tempfile::NamedTempFile;

fn setup_test_db() -> db::Db {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap();
    db::init_db_with_path(path).expect("Failed to init test db")
}

// ============ Integration: User Registration Flow ============

#[test]
fn test_user_registration_flow() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    // 1. Create user
    let password_hash = wrench_forum::auth::hash_password("securepassword123").unwrap();
    let user_id = db::create_user(&conn, "newuser@example.com", &password_hash, "newuser").unwrap();
    
    // 2. User should be unverified by default
    let user = db::get_user_by_id(&conn, user_id).unwrap().unwrap();
    assert_eq!(user.role, UserRole::Unverified);
    assert!(!user.role.can_post());
    
    // 3. User profile should be created
    let profile = db::get_user_profile(&conn, user_id).unwrap();
    assert!(profile.is_some());
    
    // 4. User can login with correct password
    let (fetched_user, fetched_hash) = db::get_user_by_email(&conn, "newuser@example.com").unwrap().unwrap();
    assert!(wrench_forum::auth::verify_password("securepassword123", &fetched_hash));
    assert_eq!(fetched_user.username, "newuser");
}

// ============ Integration: Verification Flow ============

#[test]
fn test_verification_flow() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    // 1. Create user
    let user_id = db::create_user(&conn, "mechanic@example.com", "hash", "mechanic").unwrap();
    
    // 2. Submit verification request
    let request_id = db::create_verification_request(&conn, user_id, "I am ASE certified, number 12345", "ase_cert").unwrap();
    
    // 3. Check pending status
    assert!(db::has_pending_verification(&conn, user_id).unwrap());
    
    // 4. Admin approves
    let admin_id = db::create_user(&conn, "admin@example.com", "hash", "admin").unwrap();
    db::approve_verification(&conn, request_id, admin_id).unwrap();
    
    // 5. User is now verified
    let user = db::get_user_by_id(&conn, user_id).unwrap().unwrap();
    assert_eq!(user.role, UserRole::VerifiedMechanic);
    assert!(user.role.can_post());
    
    // 6. User should have received notification
    let notifications = db::get_user_notifications(&conn, user_id, 10).unwrap();
    assert!(!notifications.is_empty());
}

// ============ Integration: Post and Comment Flow ============

#[test]
fn test_post_and_comment_flow() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    // 1. Create verified user
    let user_id = db::create_user(&conn, "poster@example.com", "hash", "poster").unwrap();
    db::update_user_role(&conn, user_id, "verified_mechanic").unwrap();
    
    // 2. Create post
    let categories = db::get_categories(&conn).unwrap();
    let post_id = db::create_post(&conn, user_id, categories[0].id, "Help with my engine", "My engine is making a noise").unwrap();
    
    // 3. Post has initial upvote
    let post = db::get_post_by_id(&conn, post_id).unwrap().unwrap();
    assert_eq!(post.score, 1);
    
    // 4. Another user comments
    let commenter_id = db::create_user(&conn, "commenter@example.com", "hash", "commenter").unwrap();
    let comment_id = db::create_comment(&conn, post_id, commenter_id, None, "Have you checked the belts?").unwrap();
    
    // 5. Original poster should receive notification
    let notifications = db::get_user_notifications(&conn, user_id, 10).unwrap();
    let has_reply_notif = notifications.iter().any(|n| n.notification_type == NotificationType::PostReply);
    assert!(has_reply_notif);
    
    // 6. OP marks as best answer
    db::set_best_answer(&conn, post_id, Some(comment_id)).unwrap();
    
    // 7. Commenter should receive notification
    let commenter_notifs = db::get_user_notifications(&conn, commenter_id, 10).unwrap();
    let has_best_notif = commenter_notifs.iter().any(|n| n.notification_type == NotificationType::BestAnswer);
    assert!(has_best_notif);
}

// ============ Integration: Voting and Karma Flow ============

#[test]
fn test_voting_karma_flow() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    // 1. Create poster and voter
    let poster_id = db::create_user(&conn, "poster@example.com", "hash", "poster").unwrap();
    let voter_id = db::create_user(&conn, "voter@example.com", "hash", "voter").unwrap();
    
    let categories = db::get_categories(&conn).unwrap();
    let post_id = db::create_post(&conn, poster_id, categories[0].id, "Test post", "Content").unwrap();
    
    // 2. Initial karma from self-upvote
    let poster = db::get_user_by_id(&conn, poster_id).unwrap().unwrap();
    assert_eq!(poster.karma, 1);
    
    // 3. Voter upvotes
    db::vote_post(&conn, voter_id, post_id, 1).unwrap();
    let poster = db::get_user_by_id(&conn, poster_id).unwrap().unwrap();
    assert_eq!(poster.karma, 2);
    
    // 4. Voter changes to downvote
    db::vote_post(&conn, voter_id, post_id, -1).unwrap();
    let poster = db::get_user_by_id(&conn, poster_id).unwrap().unwrap();
    assert_eq!(poster.karma, 0); // +1 from self, -1 from downvote = 0
}

// ============ Integration: Store Rating Flow ============

#[test]
fn test_store_rating_flow() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    // 1. Create verified users
    let submitter_id = db::create_user(&conn, "submitter@example.com", "hash", "submitter").unwrap();
    db::update_user_role(&conn, submitter_id, "verified_mechanic").unwrap();
    
    let voter1_id = db::create_user(&conn, "voter1@example.com", "hash", "voter1").unwrap();
    db::update_user_role(&conn, voter1_id, "verified_mechanic").unwrap();
    
    let voter2_id = db::create_user(&conn, "voter2@example.com", "hash", "voter2").unwrap();
    db::update_user_role(&conn, voter2_id, "verified_mechanic").unwrap();
    
    // 2. Submit store
    let store_id = db::create_store(&conn, "Good Parts Store", "https://goodparts.com", Some("Reliable parts"), "OEM", submitter_id).unwrap();
    
    // 3. Two positive votes
    db::vote_store(&conn, store_id, voter1_id, true).unwrap();
    db::vote_store(&conn, store_id, voter2_id, true).unwrap();
    
    // 4. Check reliability score
    let stores = db::get_stores(&conn, None).unwrap();
    let store = stores.iter().find(|s| s.id == store_id).unwrap();
    assert_eq!(store.reliability_score, Some(100.0));
    
    // 5. One negative vote
    db::vote_store(&conn, store_id, voter2_id, false).unwrap();
    
    let stores = db::get_stores(&conn, None).unwrap();
    let store = stores.iter().find(|s| s.id == store_id).unwrap();
    assert_eq!(store.reliability_score, Some(50.0)); // 1/2 = 50%
}

// ============ Integration: Moderation Flow ============

#[test]
fn test_moderation_flow() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    // 1. Create users
    let poster_id = db::create_user(&conn, "poster@example.com", "hash", "poster").unwrap();
    db::update_user_role(&conn, poster_id, "verified_mechanic").unwrap();
    
    let reporter_id = db::create_user(&conn, "reporter@example.com", "hash", "reporter").unwrap();
    
    let mod_id = db::create_user(&conn, "mod@example.com", "hash", "moderator").unwrap();
    db::update_user_role(&conn, mod_id, "moderator").unwrap();
    
    // 2. Create problematic post
    let categories = db::get_categories(&conn).unwrap();
    let post_id = db::create_post(&conn, poster_id, categories[0].id, "Bad post", "Spam content").unwrap();
    
    // 3. User reports
    let report_id = db::create_report(&conn, reporter_id, Some(post_id), None, "This is spam").unwrap();
    
    // 4. Report shows in queue
    let reports = db::get_unresolved_reports(&conn).unwrap();
    assert_eq!(reports.len(), 1);
    
    // 5. Mod removes post
    db::remove_post(&conn, post_id).unwrap();
    db::log_activity(&conn, mod_id, "remove_post", Some("post"), Some(post_id), None, None).unwrap();
    
    // 6. Resolve report
    db::resolve_report(&conn, report_id).unwrap();
    
    let reports = db::get_unresolved_reports(&conn).unwrap();
    assert_eq!(reports.len(), 0);
    
    // 7. Activity is logged
    let activity = db::get_recent_activity(&conn, 10).unwrap();
    let has_remove_action = activity.iter().any(|a| a.action == "remove_post");
    assert!(has_remove_action);
}

// ============ Integration: Bookmark Flow ============

#[test]
fn test_bookmark_flow() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "user@example.com", "hash", "user").unwrap();
    db::update_user_role(&conn, user_id, "verified_mechanic").unwrap();
    
    let categories = db::get_categories(&conn).unwrap();
    let post_id = db::create_post(&conn, user_id, categories[0].id, "Useful post", "Good info").unwrap();
    
    // 1. Not bookmarked initially
    assert!(!db::is_post_bookmarked(&conn, user_id, post_id).unwrap());
    
    // 2. Add bookmark
    db::add_bookmark(&conn, user_id, post_id).unwrap();
    assert!(db::is_post_bookmarked(&conn, user_id, post_id).unwrap());
    
    // 3. Get bookmarks
    let bookmarks = db::get_user_bookmarks(&conn, user_id).unwrap();
    assert_eq!(bookmarks.len(), 1);
    
    // 4. Remove bookmark
    db::remove_bookmark(&conn, user_id, post_id).unwrap();
    let bookmarks = db::get_user_bookmarks(&conn, user_id).unwrap();
    assert_eq!(bookmarks.len(), 0);
}

// ============ Integration: Search Flow ============

#[test]
fn test_search_flow() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    let user_id = db::create_user(&conn, "user@example.com", "hash", "user").unwrap();
    db::update_user_role(&conn, user_id, "verified_mechanic").unwrap();
    
    let categories = db::get_categories(&conn).unwrap();
    
    // Create posts with different keywords
    db::create_post(&conn, user_id, categories[0].id, "Brake pad replacement guide", "How to replace brake pads").unwrap();
    db::create_post(&conn, user_id, categories[0].id, "Engine overheating fix", "Coolant system repair").unwrap();
    db::create_post(&conn, user_id, categories[0].id, "Transmission fluid change", "Fluid replacement guide").unwrap();
    
    // Search for brake-related
    let results = db::search_posts(&conn, "brake", None, 10).unwrap();
    assert_eq!(results.len(), 1);
    
    // Search for guide
    let results = db::search_posts(&conn, "guide", None, 10).unwrap();
    assert_eq!(results.len(), 2);
    
    // Global search includes stores
    db::create_store(&conn, "Brake Parts Co", "https://brakeparts.com", Some("Brake specialists"), "Parts", user_id).unwrap();
    
    let global_results = db::global_search(&conn, "brake", 10).unwrap();
    assert!(global_results.len() >= 2); // At least 1 post and 1 store
}

// ============ Integration: Mention Notification Flow ============

#[test]
fn test_mention_flow() {
    let db = setup_test_db();
    let conn = db.lock().unwrap();
    
    // Create users
    let poster_id = db::create_user(&conn, "poster@example.com", "hash", "poster").unwrap();
    db::update_user_role(&conn, poster_id, "verified_mechanic").unwrap();
    
    let mentioned_id = db::create_user(&conn, "mentioned@example.com", "hash", "mentioned_user").unwrap();
    
    let categories = db::get_categories(&conn).unwrap();
    let post_id = db::create_post(&conn, poster_id, categories[0].id, "Question", "Content").unwrap();
    
    // Comment with mention
    db::create_comment(&conn, post_id, poster_id, None, "Hey @mentioned_user what do you think?").unwrap();
    
    // Mentioned user should have notification
    let notifications = db::get_user_notifications(&conn, mentioned_id, 10).unwrap();
    let has_mention = notifications.iter().any(|n| n.notification_type == NotificationType::Mention);
    assert!(has_mention);
}
