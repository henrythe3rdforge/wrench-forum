use rusqlite::{Connection, Result, params};
use std::sync::{Arc, Mutex};
use crate::models::*;

pub type Db = Arc<Mutex<Connection>>;

pub fn init_db() -> Result<Db> {
    let conn = Connection::open("wrench-forum.db")?;
    create_tables(&conn)?;
    Ok(Arc::new(Mutex::new(conn)))
}

fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(r#"
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            email TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            username TEXT UNIQUE NOT NULL,
            role TEXT NOT NULL DEFAULT 'unverified',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            banned INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS verification_requests (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL REFERENCES users(id),
            proof_text TEXT NOT NULL,
            proof_type TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            reviewed_by INTEGER REFERENCES users(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            slug TEXT UNIQUE NOT NULL,
            description TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL REFERENCES users(id),
            category_id INTEGER NOT NULL REFERENCES categories(id),
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            score INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            removed INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS comments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            post_id INTEGER NOT NULL REFERENCES posts(id),
            user_id INTEGER NOT NULL REFERENCES users(id),
            parent_id INTEGER REFERENCES comments(id),
            body TEXT NOT NULL,
            score INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            removed INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS votes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL REFERENCES users(id),
            post_id INTEGER REFERENCES posts(id),
            comment_id INTEGER REFERENCES comments(id),
            value INTEGER NOT NULL CHECK (value IN (-1, 1)),
            UNIQUE(user_id, post_id, comment_id)
        );

        CREATE TABLE IF NOT EXISTS stores (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            url TEXT NOT NULL,
            category TEXT NOT NULL,
            submitted_by INTEGER NOT NULL REFERENCES users(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS store_votes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            store_id INTEGER NOT NULL REFERENCES stores(id),
            user_id INTEGER NOT NULL REFERENCES users(id),
            positive INTEGER NOT NULL,
            UNIQUE(store_id, user_id)
        );

        CREATE TABLE IF NOT EXISTS reports (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            reporter_id INTEGER NOT NULL REFERENCES users(id),
            post_id INTEGER REFERENCES posts(id),
            comment_id INTEGER REFERENCES comments(id),
            reason TEXT NOT NULL,
            resolved INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS sessions (
            token TEXT PRIMARY KEY,
            user_id INTEGER NOT NULL REFERENCES users(id),
            expires_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_posts_category ON posts(category_id);
        CREATE INDEX IF NOT EXISTS idx_posts_user ON posts(user_id);
        CREATE INDEX IF NOT EXISTS idx_comments_post ON comments(post_id);
        CREATE INDEX IF NOT EXISTS idx_votes_post ON votes(post_id);
        CREATE INDEX IF NOT EXISTS idx_votes_comment ON votes(comment_id);
    "#)?;
    Ok(())
}

// User queries
pub fn create_user(conn: &Connection, email: &str, password_hash: &str, username: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO users (email, password_hash, username) VALUES (?1, ?2, ?3)",
        params![email, password_hash, username],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_user_by_email(conn: &Connection, email: &str) -> Result<Option<(User, String)>> {
    let mut stmt = conn.prepare(
        "SELECT id, email, password_hash, username, role, created_at, banned FROM users WHERE email = ?1"
    )?;
    let mut rows = stmt.query(params![email])?;
    if let Some(row) = rows.next()? {
        Ok(Some((User {
            id: row.get(0)?,
            email: row.get(1)?,
            username: row.get(3)?,
            role: UserRole::from_str(&row.get::<_, String>(4)?),
            created_at: row.get(5)?,
            banned: row.get::<_, i64>(6)? != 0,
        }, row.get(2)?)))
    } else {
        Ok(None)
    }
}

pub fn get_user_by_id(conn: &Connection, id: i64) -> Result<Option<User>> {
    let mut stmt = conn.prepare(
        "SELECT id, email, username, role, created_at, banned FROM users WHERE id = ?1"
    )?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(User {
            id: row.get(0)?,
            email: row.get(1)?,
            username: row.get(2)?,
            role: UserRole::from_str(&row.get::<_, String>(3)?),
            created_at: row.get(4)?,
            banned: row.get::<_, i64>(5)? != 0,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_user_by_username(conn: &Connection, username: &str) -> Result<Option<User>> {
    let mut stmt = conn.prepare(
        "SELECT id, email, username, role, created_at, banned FROM users WHERE username = ?1"
    )?;
    let mut rows = stmt.query(params![username])?;
    if let Some(row) = rows.next()? {
        Ok(Some(User {
            id: row.get(0)?,
            email: row.get(1)?,
            username: row.get(2)?,
            role: UserRole::from_str(&row.get::<_, String>(3)?),
            created_at: row.get(4)?,
            banned: row.get::<_, i64>(5)? != 0,
        }))
    } else {
        Ok(None)
    }
}

pub fn update_user_role(conn: &Connection, user_id: i64, role: &str) -> Result<()> {
    conn.execute("UPDATE users SET role = ?1 WHERE id = ?2", params![role, user_id])?;
    Ok(())
}

pub fn set_user_banned(conn: &Connection, user_id: i64, banned: bool) -> Result<()> {
    conn.execute("UPDATE users SET banned = ?1 WHERE id = ?2", params![banned as i64, user_id])?;
    Ok(())
}

pub fn get_all_users(conn: &Connection) -> Result<Vec<User>> {
    let mut stmt = conn.prepare(
        "SELECT id, email, username, role, created_at, banned FROM users ORDER BY created_at DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(User {
            id: row.get(0)?,
            email: row.get(1)?,
            username: row.get(2)?,
            role: UserRole::from_str(&row.get::<_, String>(3)?),
            created_at: row.get(4)?,
            banned: row.get::<_, i64>(5)? != 0,
        })
    })?;
    rows.collect()
}

pub fn get_banned_users(conn: &Connection) -> Result<Vec<User>> {
    let mut stmt = conn.prepare(
        "SELECT id, email, username, role, created_at, banned FROM users WHERE banned = 1 ORDER BY created_at DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(User {
            id: row.get(0)?,
            email: row.get(1)?,
            username: row.get(2)?,
            role: UserRole::from_str(&row.get::<_, String>(3)?),
            created_at: row.get(4)?,
            banned: row.get::<_, i64>(5)? != 0,
        })
    })?;
    rows.collect()
}

// Session queries
pub fn create_session(conn: &Connection, token: &str, user_id: i64, expires_at: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO sessions (token, user_id, expires_at) VALUES (?1, ?2, ?3)",
        params![token, user_id, expires_at],
    )?;
    Ok(())
}

pub fn get_session(conn: &Connection, token: &str) -> Result<Option<Session>> {
    let mut stmt = conn.prepare("SELECT token, user_id, expires_at FROM sessions WHERE token = ?1")?;
    let mut rows = stmt.query(params![token])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Session {
            token: row.get(0)?,
            user_id: row.get(1)?,
            expires_at: row.get(2)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn delete_session(conn: &Connection, token: &str) -> Result<()> {
    conn.execute("DELETE FROM sessions WHERE token = ?1", params![token])?;
    Ok(())
}

// Category queries
pub fn get_categories(conn: &Connection) -> Result<Vec<Category>> {
    let mut stmt = conn.prepare("SELECT id, name, slug, description FROM categories ORDER BY name")?;
    let rows = stmt.query_map([], |row| {
        Ok(Category {
            id: row.get(0)?,
            name: row.get(1)?,
            slug: row.get(2)?,
            description: row.get(3)?,
        })
    })?;
    rows.collect()
}

pub fn get_category_by_slug(conn: &Connection, slug: &str) -> Result<Option<Category>> {
    let mut stmt = conn.prepare("SELECT id, name, slug, description FROM categories WHERE slug = ?1")?;
    let mut rows = stmt.query(params![slug])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Category {
            id: row.get(0)?,
            name: row.get(1)?,
            slug: row.get(2)?,
            description: row.get(3)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn create_category(conn: &Connection, name: &str, slug: &str, description: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO categories (name, slug, description) VALUES (?1, ?2, ?3)",
        params![name, slug, description],
    )?;
    Ok(conn.last_insert_rowid())
}

// Post queries
pub fn create_post(conn: &Connection, user_id: i64, category_id: i64, title: &str, body: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO posts (user_id, category_id, title, body, score) VALUES (?1, ?2, ?3, ?4, 1)",
        params![user_id, category_id, title, body],
    )?;
    let post_id = conn.last_insert_rowid();
    // Auto-upvote own post
    conn.execute(
        "INSERT INTO votes (user_id, post_id, value) VALUES (?1, ?2, 1)",
        params![user_id, post_id],
    )?;
    Ok(post_id)
}

pub fn get_posts(conn: &Connection, category_slug: Option<&str>, sort: &str, limit: i64, offset: i64) -> Result<Vec<Post>> {
    let order = match sort {
        "top" => "p.score DESC, p.created_at DESC",
        "new" => "p.created_at DESC",
        _ => "p.score DESC, p.created_at DESC", // hot (default)
    };

    let sql = format!(
        r#"SELECT p.id, p.user_id, p.category_id, p.title, p.body, p.score, p.created_at, p.removed,
           u.username, u.role, c.name, c.slug,
           (SELECT COUNT(*) FROM comments WHERE post_id = p.id AND removed = 0) as comment_count
           FROM posts p
           JOIN users u ON p.user_id = u.id
           JOIN categories c ON p.category_id = c.id
           WHERE p.removed = 0
           {}
           ORDER BY {}
           LIMIT ?1 OFFSET ?2"#,
        if category_slug.is_some() { "AND c.slug = ?3" } else { "" },
        order
    );

    let mut stmt = conn.prepare(&sql)?;
    
    let rows = if let Some(slug) = category_slug {
        stmt.query_map(params![limit, offset, slug], map_post)?
    } else {
        stmt.query_map(params![limit, offset], map_post)?
    };
    rows.collect()
}

fn map_post(row: &rusqlite::Row) -> rusqlite::Result<Post> {
    Ok(Post {
        id: row.get(0)?,
        user_id: row.get(1)?,
        category_id: row.get(2)?,
        title: row.get(3)?,
        body: row.get(4)?,
        score: row.get(5)?,
        created_at: row.get(6)?,
        removed: row.get::<_, i64>(7)? != 0,
        username: row.get(8).ok(),
        user_role: row.get(9).ok(),
        category_name: row.get(10).ok(),
        category_slug: row.get(11).ok(),
        comment_count: row.get(12).ok(),
    })
}

pub fn get_post_by_id(conn: &Connection, id: i64) -> Result<Option<Post>> {
    let mut stmt = conn.prepare(
        r#"SELECT p.id, p.user_id, p.category_id, p.title, p.body, p.score, p.created_at, p.removed,
           u.username, u.role, c.name, c.slug,
           (SELECT COUNT(*) FROM comments WHERE post_id = p.id AND removed = 0) as comment_count
           FROM posts p
           JOIN users u ON p.user_id = u.id
           JOIN categories c ON p.category_id = c.id
           WHERE p.id = ?1"#
    )?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(map_post(row)?))
    } else {
        Ok(None)
    }
}

pub fn get_posts_by_user(conn: &Connection, user_id: i64) -> Result<Vec<Post>> {
    let mut stmt = conn.prepare(
        r#"SELECT p.id, p.user_id, p.category_id, p.title, p.body, p.score, p.created_at, p.removed,
           u.username, u.role, c.name, c.slug,
           (SELECT COUNT(*) FROM comments WHERE post_id = p.id AND removed = 0) as comment_count
           FROM posts p
           JOIN users u ON p.user_id = u.id
           JOIN categories c ON p.category_id = c.id
           WHERE p.user_id = ?1
           ORDER BY p.created_at DESC"#
    )?;
    let rows = stmt.query_map(params![user_id], map_post)?;
    rows.collect()
}

pub fn remove_post(conn: &Connection, post_id: i64) -> Result<()> {
    conn.execute("UPDATE posts SET removed = 1 WHERE id = ?1", params![post_id])?;
    Ok(())
}

pub fn restore_post(conn: &Connection, post_id: i64) -> Result<()> {
    conn.execute("UPDATE posts SET removed = 0 WHERE id = ?1", params![post_id])?;
    Ok(())
}

// Comment queries
pub fn create_comment(conn: &Connection, post_id: i64, user_id: i64, parent_id: Option<i64>, body: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO comments (post_id, user_id, parent_id, body, score) VALUES (?1, ?2, ?3, ?4, 1)",
        params![post_id, user_id, parent_id, body],
    )?;
    let comment_id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO votes (user_id, comment_id, value) VALUES (?1, ?2, 1)",
        params![user_id, comment_id],
    )?;
    Ok(comment_id)
}

pub fn get_comments_for_post(conn: &Connection, post_id: i64) -> Result<Vec<Comment>> {
    let mut stmt = conn.prepare(
        r#"SELECT c.id, c.post_id, c.user_id, c.parent_id, c.body, c.score, c.created_at, c.removed,
           u.username, u.role
           FROM comments c
           JOIN users u ON c.user_id = u.id
           WHERE c.post_id = ?1 AND c.removed = 0
           ORDER BY c.score DESC, c.created_at ASC"#
    )?;
    let rows = stmt.query_map(params![post_id], |row| {
        Ok(Comment {
            id: row.get(0)?,
            post_id: row.get(1)?,
            user_id: row.get(2)?,
            parent_id: row.get(3)?,
            body: row.get(4)?,
            score: row.get(5)?,
            created_at: row.get(6)?,
            removed: row.get::<_, i64>(7)? != 0,
            username: row.get(8).ok(),
            user_role: row.get(9).ok(),
            replies: vec![],
        })
    })?;
    rows.collect()
}

pub fn remove_comment(conn: &Connection, comment_id: i64) -> Result<()> {
    conn.execute("UPDATE comments SET removed = 1 WHERE id = ?1", params![comment_id])?;
    Ok(())
}

// Vote queries
pub fn vote_post(conn: &Connection, user_id: i64, post_id: i64, value: i64) -> Result<i64> {
    // Check existing vote
    let existing: Option<i64> = conn.query_row(
        "SELECT value FROM votes WHERE user_id = ?1 AND post_id = ?2",
        params![user_id, post_id],
        |row| row.get(0),
    ).ok();

    match existing {
        Some(old_value) if old_value == value => {
            // Remove vote (toggle off)
            conn.execute(
                "DELETE FROM votes WHERE user_id = ?1 AND post_id = ?2",
                params![user_id, post_id],
            )?;
            conn.execute(
                "UPDATE posts SET score = score - ?1 WHERE id = ?2",
                params![value, post_id],
            )?;
        }
        Some(old_value) => {
            // Change vote
            conn.execute(
                "UPDATE votes SET value = ?1 WHERE user_id = ?2 AND post_id = ?3",
                params![value, user_id, post_id],
            )?;
            conn.execute(
                "UPDATE posts SET score = score - ?1 + ?2 WHERE id = ?3",
                params![old_value, value, post_id],
            )?;
        }
        None => {
            // New vote
            conn.execute(
                "INSERT INTO votes (user_id, post_id, value) VALUES (?1, ?2, ?3)",
                params![user_id, post_id, value],
            )?;
            conn.execute(
                "UPDATE posts SET score = score + ?1 WHERE id = ?2",
                params![value, post_id],
            )?;
        }
    }

    // Return new score
    conn.query_row("SELECT score FROM posts WHERE id = ?1", params![post_id], |row| row.get(0))
}

pub fn vote_comment(conn: &Connection, user_id: i64, comment_id: i64, value: i64) -> Result<i64> {
    let existing: Option<i64> = conn.query_row(
        "SELECT value FROM votes WHERE user_id = ?1 AND comment_id = ?2",
        params![user_id, comment_id],
        |row| row.get(0),
    ).ok();

    match existing {
        Some(old_value) if old_value == value => {
            conn.execute(
                "DELETE FROM votes WHERE user_id = ?1 AND comment_id = ?2",
                params![user_id, comment_id],
            )?;
            conn.execute(
                "UPDATE comments SET score = score - ?1 WHERE id = ?2",
                params![value, comment_id],
            )?;
        }
        Some(old_value) => {
            conn.execute(
                "UPDATE votes SET value = ?1 WHERE user_id = ?2 AND comment_id = ?3",
                params![value, user_id, comment_id],
            )?;
            conn.execute(
                "UPDATE comments SET score = score - ?1 + ?2 WHERE id = ?3",
                params![old_value, value, comment_id],
            )?;
        }
        None => {
            conn.execute(
                "INSERT INTO votes (user_id, comment_id, value) VALUES (?1, ?2, ?3)",
                params![user_id, comment_id, value],
            )?;
            conn.execute(
                "UPDATE comments SET score = score + ?1 WHERE id = ?2",
                params![value, comment_id],
            )?;
        }
    }

    conn.query_row("SELECT score FROM comments WHERE id = ?1", params![comment_id], |row| row.get(0))
}

pub fn get_user_vote_for_post(conn: &Connection, user_id: i64, post_id: i64) -> Result<Option<i64>> {
    match conn.query_row(
        "SELECT value FROM votes WHERE user_id = ?1 AND post_id = ?2",
        params![user_id, post_id],
        |row| row.get(0),
    ) {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn get_user_vote_for_comment(conn: &Connection, user_id: i64, comment_id: i64) -> Result<Option<i64>> {
    match conn.query_row(
        "SELECT value FROM votes WHERE user_id = ?1 AND comment_id = ?2",
        params![user_id, comment_id],
        |row| row.get(0),
    ) {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

// Store queries
pub fn create_store(conn: &Connection, name: &str, url: &str, category: &str, submitted_by: i64) -> Result<i64> {
    conn.execute(
        "INSERT INTO stores (name, url, category, submitted_by) VALUES (?1, ?2, ?3, ?4)",
        params![name, url, category, submitted_by],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_stores(conn: &Connection, category: Option<&str>) -> Result<Vec<Store>> {
    let sql = format!(
        r#"SELECT s.id, s.name, s.url, s.category, s.submitted_by, s.created_at,
           u.username,
           (SELECT COUNT(*) FROM store_votes WHERE store_id = s.id AND positive = 1) as pos,
           (SELECT COUNT(*) FROM store_votes WHERE store_id = s.id) as total
           FROM stores s
           JOIN users u ON s.submitted_by = u.id
           {}
           ORDER BY s.name"#,
        if category.is_some() { "WHERE s.category = ?1" } else { "" }
    );

    let mut stmt = conn.prepare(&sql)?;
    
    let rows = if let Some(cat) = category {
        stmt.query_map(params![cat], map_store)?
    } else {
        stmt.query_map([], map_store)?
    };
    rows.collect()
}

fn map_store(row: &rusqlite::Row) -> rusqlite::Result<Store> {
    let pos: i64 = row.get(7)?;
    let total: i64 = row.get(8)?;
    Ok(Store {
        id: row.get(0)?,
        name: row.get(1)?,
        url: row.get(2)?,
        category: row.get(3)?,
        submitted_by: row.get(4)?,
        created_at: row.get(5)?,
        submitter_name: row.get(6).ok(),
        positive_votes: pos,
        total_votes: total,
        reliability_score: if total > 0 { Some((pos as f64 / total as f64) * 100.0) } else { None },
    })
}

pub fn vote_store(conn: &Connection, store_id: i64, user_id: i64, positive: bool) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO store_votes (store_id, user_id, positive) VALUES (?1, ?2, ?3)",
        params![store_id, user_id, positive as i64],
    )?;
    Ok(())
}

pub fn get_user_store_vote(conn: &Connection, store_id: i64, user_id: i64) -> Result<Option<bool>> {
    match conn.query_row(
        "SELECT positive FROM store_votes WHERE store_id = ?1 AND user_id = ?2",
        params![store_id, user_id],
        |row| row.get::<_, i64>(0).map(|v| v != 0),
    ) {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn get_store_categories(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT DISTINCT category FROM stores ORDER BY category")?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    rows.collect()
}

// Verification queries
pub fn create_verification_request(conn: &Connection, user_id: i64, proof_text: &str, proof_type: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO verification_requests (user_id, proof_text, proof_type) VALUES (?1, ?2, ?3)",
        params![user_id, proof_text, proof_type],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_pending_verification_requests(conn: &Connection) -> Result<Vec<VerificationRequest>> {
    let mut stmt = conn.prepare(
        r#"SELECT v.id, v.user_id, v.proof_text, v.proof_type, v.status, v.reviewed_by, v.created_at,
           u.username, u.email
           FROM verification_requests v
           JOIN users u ON v.user_id = u.id
           WHERE v.status = 'pending'
           ORDER BY v.created_at ASC"#
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(VerificationRequest {
            id: row.get(0)?,
            user_id: row.get(1)?,
            proof_text: row.get(2)?,
            proof_type: row.get(3)?,
            status: row.get(4)?,
            reviewed_by: row.get(5)?,
            created_at: row.get(6)?,
            username: row.get(7).ok(),
            email: row.get(8).ok(),
        })
    })?;
    rows.collect()
}

pub fn approve_verification(conn: &Connection, request_id: i64, reviewer_id: i64) -> Result<()> {
    // Get user_id from request
    let user_id: i64 = conn.query_row(
        "SELECT user_id FROM verification_requests WHERE id = ?1",
        params![request_id],
        |row| row.get(0),
    )?;
    
    conn.execute(
        "UPDATE verification_requests SET status = 'approved', reviewed_by = ?1 WHERE id = ?2",
        params![reviewer_id, request_id],
    )?;
    conn.execute(
        "UPDATE users SET role = 'verified_mechanic' WHERE id = ?1",
        params![user_id],
    )?;
    Ok(())
}

pub fn deny_verification(conn: &Connection, request_id: i64, reviewer_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE verification_requests SET status = 'denied', reviewed_by = ?1 WHERE id = ?2",
        params![reviewer_id, request_id],
    )?;
    Ok(())
}

pub fn has_pending_verification(conn: &Connection, user_id: i64) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM verification_requests WHERE user_id = ?1 AND status = 'pending'",
        params![user_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

// Report queries
pub fn create_report(conn: &Connection, reporter_id: i64, post_id: Option<i64>, comment_id: Option<i64>, reason: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO reports (reporter_id, post_id, comment_id, reason) VALUES (?1, ?2, ?3, ?4)",
        params![reporter_id, post_id, comment_id, reason],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_unresolved_reports(conn: &Connection) -> Result<Vec<Report>> {
    let mut stmt = conn.prepare(
        r#"SELECT r.id, r.reporter_id, r.post_id, r.comment_id, r.reason, r.resolved, r.created_at,
           u.username,
           p.title,
           c.body
           FROM reports r
           JOIN users u ON r.reporter_id = u.id
           LEFT JOIN posts p ON r.post_id = p.id
           LEFT JOIN comments c ON r.comment_id = c.id
           WHERE r.resolved = 0
           ORDER BY r.created_at ASC"#
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Report {
            id: row.get(0)?,
            reporter_id: row.get(1)?,
            post_id: row.get(2)?,
            comment_id: row.get(3)?,
            reason: row.get(4)?,
            resolved: row.get::<_, i64>(5)? != 0,
            created_at: row.get(6)?,
            reporter_name: row.get(7).ok(),
            post_title: row.get(8).ok(),
            comment_body: row.get(9).ok(),
        })
    })?;
    rows.collect()
}

pub fn resolve_report(conn: &Connection, report_id: i64) -> Result<()> {
    conn.execute("UPDATE reports SET resolved = 1 WHERE id = ?1", params![report_id])?;
    Ok(())
}
