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
}

#[derive(Debug, Clone, Serialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Post {
    pub id: i64,
    pub user_id: i64,
    pub category_id: i64,
    pub title: String,
    pub body: String,
    pub score: i64,
    pub created_at: String,
    pub removed: bool,
    // Joined fields
    pub username: Option<String>,
    pub user_role: Option<String>,
    pub category_name: Option<String>,
    pub category_slug: Option<String>,
    pub comment_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Comment {
    pub id: i64,
    pub post_id: i64,
    pub user_id: i64,
    pub parent_id: Option<i64>,
    pub body: String,
    pub score: i64,
    pub created_at: String,
    pub removed: bool,
    // Joined fields
    pub username: Option<String>,
    pub user_role: Option<String>,
    pub replies: Vec<Comment>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Store {
    pub id: i64,
    pub name: String,
    pub url: String,
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
