use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Unverified,
    VerifiedMechanic,
    Moderator,
    Admin,
}

impl UserRole {
    pub fn from_str(s: &str) -> Self {
        match s {
            "verified_mechanic" => UserRole::VerifiedMechanic,
            "moderator" => UserRole::Moderator,
            "admin" => UserRole::Admin,
            _ => UserRole::Unverified,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            UserRole::Unverified => "unverified",
            UserRole::VerifiedMechanic => "verified_mechanic",
            UserRole::Moderator => "moderator",
            UserRole::Admin => "admin",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            UserRole::Unverified => "Unverified",
            UserRole::VerifiedMechanic => "Verified Mechanic",
            UserRole::Moderator => "Moderator",
            UserRole::Admin => "Admin",
        }
    }

    pub fn can_post(&self) -> bool {
        matches!(self, UserRole::VerifiedMechanic | UserRole::Moderator | UserRole::Admin)
    }

    pub fn can_moderate(&self) -> bool {
        matches!(self, UserRole::Moderator | UserRole::Admin)
    }

    pub fn is_admin(&self) -> bool {
        matches!(self, UserRole::Admin)
    }

    pub fn can_vote_stores(&self) -> bool {
        matches!(self, UserRole::VerifiedMechanic | UserRole::Moderator | UserRole::Admin)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub username: String,
    pub role: UserRole,
    pub created_at: String,
    pub banned: bool,
    pub karma: i64,
    pub flair: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserProfile {
    pub user_id: i64,
    pub avatar_path: Option<String>,
    pub bio: Option<String>,
    pub specialties: Option<String>,
    pub location: Option<String>,
    pub website: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub post_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostTag {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Post {
    pub id: i64,
    pub user_id: i64,
    pub category_id: i64,
    pub title: String,
    pub body: String,
    pub body_html: Option<String>,
    pub score: i64,
    pub created_at: String,
    pub edited_at: Option<String>,
    pub removed: bool,
    pub pinned: bool,
    pub best_answer_id: Option<i64>,
    // Joined fields
    pub username: Option<String>,
    pub user_role: Option<String>,
    pub user_flair: Option<String>,
    pub user_avatar: Option<String>,
    pub category_name: Option<String>,
    pub category_slug: Option<String>,
    pub comment_count: Option<i64>,
    pub tags: Option<Vec<PostTag>>,
    pub is_bookmarked: Option<bool>,
    pub user_vote: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Comment {
    pub id: i64,
    pub post_id: i64,
    pub user_id: i64,
    pub parent_id: Option<i64>,
    pub body: String,
    pub body_html: Option<String>,
    pub score: i64,
    pub created_at: String,
    pub edited_at: Option<String>,
    pub removed: bool,
    pub is_best_answer: bool,
    // Joined fields
    pub username: Option<String>,
    pub user_role: Option<String>,
    pub user_flair: Option<String>,
    pub user_avatar: Option<String>,
    pub replies: Vec<Comment>,
    pub user_vote: Option<i64>,
    pub depth: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Store {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub description: Option<String>,
    pub category: String,
    pub submitted_by: i64,
    pub created_at: String,
    // Computed
    pub positive_votes: i64,
    pub total_votes: i64,
    pub reliability_score: Option<f64>,
    pub submitter_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerificationRequest {
    pub id: i64,
    pub user_id: i64,
    pub proof_text: String,
    pub proof_type: String,
    pub status: String, // pending, approved, denied
    pub reviewed_by: Option<i64>,
    pub created_at: String,
    // Joined
    pub username: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub id: i64,
    pub reporter_id: i64,
    pub post_id: Option<i64>,
    pub comment_id: Option<i64>,
    pub reason: String,
    pub resolved: bool,
    pub created_at: String,
    // Joined
    pub reporter_name: Option<String>,
    pub post_title: Option<String>,
    pub comment_body: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Session {
    pub token: String,
    pub user_id: i64,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Bookmark {
    pub id: i64,
    pub user_id: i64,
    pub post_id: i64,
    pub created_at: String,
    // Joined
    pub post_title: Option<String>,
    pub post_score: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationType {
    Reply,
    Mention,
    PostReply,
    BestAnswer,
    Upvote,
    System,
}

impl NotificationType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "reply" => NotificationType::Reply,
            "mention" => NotificationType::Mention,
            "post_reply" => NotificationType::PostReply,
            "best_answer" => NotificationType::BestAnswer,
            "upvote" => NotificationType::Upvote,
            _ => NotificationType::System,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            NotificationType::Reply => "reply",
            NotificationType::Mention => "mention",
            NotificationType::PostReply => "post_reply",
            NotificationType::BestAnswer => "best_answer",
            NotificationType::Upvote => "upvote",
            NotificationType::System => "system",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Notification {
    pub id: i64,
    pub user_id: i64,
    pub notification_type: NotificationType,
    pub content: String,
    pub read: bool,
    pub post_id: Option<i64>,
    pub comment_id: Option<i64>,
    pub from_user_id: Option<i64>,
    pub created_at: String,
    // Joined
    pub from_username: Option<String>,
    pub post_title: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Announcement {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub active: bool,
    pub pinned: bool,
    pub announcement_type: String, // info, warning, success
    pub created_by: i64,
    pub created_at: String,
    pub expires_at: Option<String>,
    // Joined
    pub creator_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Upload {
    pub id: i64,
    pub user_id: i64,
    pub filename: String,
    pub original_name: String,
    pub path: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostEdit {
    pub id: i64,
    pub post_id: i64,
    pub user_id: i64,
    pub old_title: Option<String>,
    pub old_body: String,
    pub edit_reason: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommentEdit {
    pub id: i64,
    pub comment_id: i64,
    pub user_id: i64,
    pub old_body: String,
    pub edit_reason: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActivityLog {
    pub id: i64,
    pub user_id: i64,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<i64>,
    pub details: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: String,
    // Joined
    pub username: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub result_type: String, // "post" or "store"
    pub id: i64,
    pub title: String,
    pub snippet: String,
    pub url: String,
    pub score: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PaginationInfo {
    pub page: i64,
    pub per_page: i64,
    pub total_items: i64,
    pub total_pages: i64,
    pub has_prev: bool,
    pub has_next: bool,
}

impl PaginationInfo {
    pub fn new(page: i64, per_page: i64, total_items: i64) -> Self {
        let total_pages = (total_items + per_page - 1) / per_page;
        Self {
            page,
            per_page,
            total_items,
            total_pages,
            has_prev: page > 1,
            has_next: page < total_pages,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UserStats {
    pub post_count: i64,
    pub comment_count: i64,
    pub karma: i64,
    pub upvotes_received: i64,
    pub downvotes_received: i64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ForumStats {
    pub total_users: i64,
    pub verified_users: i64,
    pub total_posts: i64,
    pub total_comments: i64,
    pub total_stores: i64,
    pub posts_today: i64,
}
