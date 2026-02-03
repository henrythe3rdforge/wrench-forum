use rusqlite::{Connection, Result, params};
use std::sync::{Arc, Mutex};
use crate::models::*;

pub type Db = Arc<Mutex<Connection>>;

pub fn init_db() -> Result<Db> {
    let conn = Connection::open("wrench-forum.db")?;
    create_tables(&conn)?;
    seed_defaults(&conn)?;
    Ok(Arc::new(Mutex::new(conn)))
}

pub fn init_db_with_path(path: &str) -> Result<Db> {
    let conn = Connection::open(path)?;
    create_tables(&conn)?;
    Ok(Arc::new(Mutex::new(conn)))
}

fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(r#"
        -- Core user table
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            email TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            username TEXT UNIQUE NOT NULL,
            role TEXT NOT NULL DEFAULT 'unverified',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            banned INTEGER NOT NULL DEFAULT 0,
            karma INTEGER NOT NULL DEFAULT 0,
            flair TEXT
        );

        -- User profiles (extended info)
        CREATE TABLE IF NOT EXISTS user_profiles (
            user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
            avatar_path TEXT,
            bio TEXT,
            specialties TEXT,
            location TEXT,
            website TEXT
        );

        -- Verification requests
        CREATE TABLE IF NOT EXISTS verification_requests (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL REFERENCES users(id),
            proof_text TEXT NOT NULL,
            proof_type TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            reviewed_by INTEGER REFERENCES users(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Categories
        CREATE TABLE IF NOT EXISTS categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            slug TEXT UNIQUE NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            icon TEXT,
            color TEXT DEFAULT '#6b7280'
        );

        -- Post tags/flair
        CREATE TABLE IF NOT EXISTS post_tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            color TEXT NOT NULL DEFAULT '#6b7280',
            description TEXT
        );

        -- Posts
        CREATE TABLE IF NOT EXISTS posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL REFERENCES users(id),
            category_id INTEGER NOT NULL REFERENCES categories(id),
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            body_html TEXT,
            score INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            edited_at TEXT,
            removed INTEGER NOT NULL DEFAULT 0,
            pinned INTEGER NOT NULL DEFAULT 0,
            best_answer_id INTEGER REFERENCES comments(id)
        );

        -- Post to tag mapping
        CREATE TABLE IF NOT EXISTS post_tag_map (
            post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
            tag_id INTEGER NOT NULL REFERENCES post_tags(id) ON DELETE CASCADE,
            PRIMARY KEY (post_id, tag_id)
        );

        -- Comments
        CREATE TABLE IF NOT EXISTS comments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            post_id INTEGER NOT NULL REFERENCES posts(id),
            user_id INTEGER NOT NULL REFERENCES users(id),
            parent_id INTEGER REFERENCES comments(id),
            body TEXT NOT NULL,
            body_html TEXT,
            score INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            edited_at TEXT,
            removed INTEGER NOT NULL DEFAULT 0
        );

        -- Votes (for posts and comments)
        CREATE TABLE IF NOT EXISTS votes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL REFERENCES users(id),
            post_id INTEGER REFERENCES posts(id),
            comment_id INTEGER REFERENCES comments(id),
            value INTEGER NOT NULL CHECK (value IN (-1, 1)),
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(user_id, post_id, comment_id)
        );

        -- Stores
        CREATE TABLE IF NOT EXISTS stores (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            url TEXT NOT NULL,
            description TEXT,
            category TEXT NOT NULL,
            submitted_by INTEGER NOT NULL REFERENCES users(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Store votes
        CREATE TABLE IF NOT EXISTS store_votes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            store_id INTEGER NOT NULL REFERENCES stores(id),
            user_id INTEGER NOT NULL REFERENCES users(id),
            positive INTEGER NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(store_id, user_id)
        );

        -- Reports
        CREATE TABLE IF NOT EXISTS reports (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            reporter_id INTEGER NOT NULL REFERENCES users(id),
            post_id INTEGER REFERENCES posts(id),
            comment_id INTEGER REFERENCES comments(id),
            reason TEXT NOT NULL,
            resolved INTEGER NOT NULL DEFAULT 0,
            resolved_by INTEGER REFERENCES users(id),
            resolution_note TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Sessions
        CREATE TABLE IF NOT EXISTS sessions (
            token TEXT PRIMARY KEY,
            user_id INTEGER NOT NULL REFERENCES users(id),
            expires_at TEXT NOT NULL,
            ip_address TEXT,
            user_agent TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Bookmarks
        CREATE TABLE IF NOT EXISTS bookmarks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL REFERENCES users(id),
            post_id INTEGER NOT NULL REFERENCES posts(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(user_id, post_id)
        );

        -- Notifications
        CREATE TABLE IF NOT EXISTS notifications (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL REFERENCES users(id),
            notification_type TEXT NOT NULL,
            content TEXT NOT NULL,
            read INTEGER NOT NULL DEFAULT 0,
            post_id INTEGER REFERENCES posts(id),
            comment_id INTEGER REFERENCES comments(id),
            from_user_id INTEGER REFERENCES users(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Announcements
        CREATE TABLE IF NOT EXISTS announcements (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            active INTEGER NOT NULL DEFAULT 1,
            pinned INTEGER NOT NULL DEFAULT 0,
            announcement_type TEXT NOT NULL DEFAULT 'info',
            created_by INTEGER NOT NULL REFERENCES users(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            expires_at TEXT
        );

        -- File uploads
        CREATE TABLE IF NOT EXISTS uploads (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL REFERENCES users(id),
            filename TEXT NOT NULL,
            original_name TEXT NOT NULL,
            path TEXT NOT NULL,
            mime_type TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Post edit history
        CREATE TABLE IF NOT EXISTS post_edits (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            post_id INTEGER NOT NULL REFERENCES posts(id),
            user_id INTEGER NOT NULL REFERENCES users(id),
            old_title TEXT,
            old_body TEXT NOT NULL,
            edit_reason TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Comment edit history
        CREATE TABLE IF NOT EXISTS comment_edits (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            comment_id INTEGER NOT NULL REFERENCES comments(id),
            user_id INTEGER NOT NULL REFERENCES users(id),
            old_body TEXT NOT NULL,
            edit_reason TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Activity logs for moderation
        CREATE TABLE IF NOT EXISTS activity_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL REFERENCES users(id),
            action TEXT NOT NULL,
            target_type TEXT,
            target_id INTEGER,
            details TEXT,
            ip_address TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Indexes for performance
        CREATE INDEX IF NOT EXISTS idx_posts_category ON posts(category_id);
        CREATE INDEX IF NOT EXISTS idx_posts_user ON posts(user_id);
        CREATE INDEX IF NOT EXISTS idx_posts_created ON posts(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_posts_score ON posts(score DESC);
        CREATE INDEX IF NOT EXISTS idx_comments_post ON comments(post_id);
        CREATE INDEX IF NOT EXISTS idx_comments_user ON comments(user_id);
        CREATE INDEX IF NOT EXISTS idx_votes_post ON votes(post_id);
        CREATE INDEX IF NOT EXISTS idx_votes_comment ON votes(comment_id);
        CREATE INDEX IF NOT EXISTS idx_votes_user ON votes(user_id);
        CREATE INDEX IF NOT EXISTS idx_bookmarks_user ON bookmarks(user_id);
        CREATE INDEX IF NOT EXISTS idx_notifications_user ON notifications(user_id);
        CREATE INDEX IF NOT EXISTS idx_notifications_read ON notifications(read);
        CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
        CREATE INDEX IF NOT EXISTS idx_activity_user ON activity_logs(user_id);
        CREATE INDEX IF NOT EXISTS idx_activity_created ON activity_logs(created_at DESC);

        -- Full text search (optional, for SQLite FTS5)
        -- CREATE VIRTUAL TABLE IF NOT EXISTS posts_fts USING fts5(title, body, content=posts, content_rowid=id);
    "#)?;
    Ok(())
}

fn seed_defaults(conn: &Connection) -> Result<()> {
    // Seed default categories if none exist
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0))?;
    if count == 0 {
        conn.execute_batch(r#"
            INSERT INTO categories (name, slug, description, icon, color) VALUES
            ('Engine', 'engine', 'Engine diagnostics, repairs, and maintenance', '🔧', '#ef4444'),
            ('Transmission', 'transmission', 'Manual and automatic transmission topics', '⚙️', '#f97316'),
            ('Electrical', 'electrical', 'Wiring, electronics, and electrical systems', '⚡', '#eab308'),
            ('Suspension', 'suspension', 'Shocks, struts, and steering components', '🚗', '#22c55e'),
            ('Brakes', 'brakes', 'Brake systems, pads, rotors, and fluid', '🛑', '#ef4444'),
            ('HVAC', 'hvac', 'Heating, ventilation, and air conditioning', '❄️', '#06b6d4'),
            ('Body & Interior', 'body-interior', 'Bodywork, paint, and interior repairs', '🎨', '#8b5cf6'),
            ('Tools & Equipment', 'tools', 'Tool recommendations and workshop setup', '🛠️', '#6b7280'),
            ('General Discussion', 'general', 'Off-topic and general mechanic chat', '💬', '#3b82f6');

            INSERT INTO post_tags (name, color, description) VALUES
            ('Question', '#3b82f6', 'Asking for help or advice'),
            ('Discussion', '#8b5cf6', 'Open discussion topic'),
            ('Tutorial', '#22c55e', 'How-to guide or walkthrough'),
            ('Solved', '#10b981', 'Issue has been resolved'),
            ('Tips & Tricks', '#f97316', 'Helpful tips and shortcuts'),
            ('Safety', '#ef4444', 'Safety-related information');
        "#)?;
    }
    Ok(())
}

// ============ User Functions ============

pub fn create_user(conn: &Connection, email: &str, password_hash: &str, username: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO users (email, password_hash, username) VALUES (?1, ?2, ?3)",
        params![email, password_hash, username],
    )?;
    let user_id = conn.last_insert_rowid();
    // Create empty profile
    conn.execute(
        "INSERT INTO user_profiles (user_id) VALUES (?1)",
        params![user_id],
    )?;
    Ok(user_id)
}

pub fn get_user_by_email(conn: &Connection, email: &str) -> Result<Option<(User, String)>> {
    let mut stmt = conn.prepare(
        "SELECT id, email, password_hash, username, role, created_at, banned, karma, flair FROM users WHERE email = ?1"
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
            karma: row.get(7)?,
            flair: row.get(8)?,
        }, row.get(2)?)))
    } else {
        Ok(None)
    }
}

pub fn get_user_by_id(conn: &Connection, id: i64) -> Result<Option<User>> {
    let mut stmt = conn.prepare(
        "SELECT id, email, username, role, created_at, banned, karma, flair FROM users WHERE id = ?1"
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
            karma: row.get(6)?,
            flair: row.get(7)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_user_by_username(conn: &Connection, username: &str) -> Result<Option<User>> {
    let mut stmt = conn.prepare(
        "SELECT id, email, username, role, created_at, banned, karma, flair FROM users WHERE username = ?1"
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
            karma: row.get(6)?,
            flair: row.get(7)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn update_user_role(conn: &Connection, user_id: i64, role: &str) -> Result<()> {
    conn.execute("UPDATE users SET role = ?1 WHERE id = ?2", params![role, user_id])?;
    Ok(())
}

pub fn update_user_flair(conn: &Connection, user_id: i64, flair: &str) -> Result<()> {
    conn.execute("UPDATE users SET flair = ?1 WHERE id = ?2", params![flair, user_id])?;
    Ok(())
}

pub fn update_user_karma(conn: &Connection, user_id: i64, delta: i64) -> Result<()> {
    conn.execute("UPDATE users SET karma = karma + ?1 WHERE id = ?2", params![delta, user_id])?;
    Ok(())
}

pub fn set_user_banned(conn: &Connection, user_id: i64, banned: bool) -> Result<()> {
    conn.execute("UPDATE users SET banned = ?1 WHERE id = ?2", params![banned as i64, user_id])?;
    Ok(())
}

pub fn get_all_users(conn: &Connection) -> Result<Vec<User>> {
    let mut stmt = conn.prepare(
        "SELECT id, email, username, role, created_at, banned, karma, flair FROM users ORDER BY created_at DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(User {
            id: row.get(0)?,
            email: row.get(1)?,
            username: row.get(2)?,
            role: UserRole::from_str(&row.get::<_, String>(3)?),
            created_at: row.get(4)?,
            banned: row.get::<_, i64>(5)? != 0,
            karma: row.get(6)?,
            flair: row.get(7)?,
        })
    })?;
    rows.collect()
}

pub fn get_banned_users(conn: &Connection) -> Result<Vec<User>> {
    let mut stmt = conn.prepare(
        "SELECT id, email, username, role, created_at, banned, karma, flair FROM users WHERE banned = 1 ORDER BY created_at DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(User {
            id: row.get(0)?,
            email: row.get(1)?,
            username: row.get(2)?,
            role: UserRole::from_str(&row.get::<_, String>(3)?),
            created_at: row.get(4)?,
            banned: row.get::<_, i64>(5)? != 0,
            karma: row.get(6)?,
            flair: row.get(7)?,
        })
    })?;
    rows.collect()
}

pub fn get_user_profile(conn: &Connection, user_id: i64) -> Result<Option<UserProfile>> {
    let mut stmt = conn.prepare(
        "SELECT user_id, avatar_path, bio, specialties, location, website FROM user_profiles WHERE user_id = ?1"
    )?;
    let mut rows = stmt.query(params![user_id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(UserProfile {
            user_id: row.get(0)?,
            avatar_path: row.get(1)?,
            bio: row.get(2)?,
            specialties: row.get(3)?,
            location: row.get(4)?,
            website: row.get(5)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn update_user_profile(conn: &Connection, user_id: i64, bio: Option<&str>, specialties: Option<&str>, location: Option<&str>, website: Option<&str>) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO user_profiles (user_id, avatar_path, bio, specialties, location, website) 
         VALUES (?1, (SELECT avatar_path FROM user_profiles WHERE user_id = ?1), ?2, ?3, ?4, ?5)",
        params![user_id, bio, specialties, location, website],
    )?;
    Ok(())
}

pub fn update_user_avatar(conn: &Connection, user_id: i64, avatar_path: &str) -> Result<()> {
    conn.execute(
        "UPDATE user_profiles SET avatar_path = ?1 WHERE user_id = ?2",
        params![avatar_path, user_id],
    )?;
    Ok(())
}

pub fn get_user_stats(conn: &Connection, user_id: i64) -> Result<UserStats> {
    let post_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM posts WHERE user_id = ?1 AND removed = 0",
        params![user_id], |r| r.get(0)
    ).unwrap_or(0);
    
    let comment_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM comments WHERE user_id = ?1 AND removed = 0",
        params![user_id], |r| r.get(0)
    ).unwrap_or(0);
    
    let karma: i64 = conn.query_row(
        "SELECT karma FROM users WHERE id = ?1",
        params![user_id], |r| r.get(0)
    ).unwrap_or(0);
    
    let upvotes_received: i64 = conn.query_row(
        "SELECT COALESCE(SUM(CASE WHEN v.value = 1 THEN 1 ELSE 0 END), 0) FROM votes v
         JOIN posts p ON v.post_id = p.id WHERE p.user_id = ?1",
        params![user_id], |r| r.get(0)
    ).unwrap_or(0);
    
    let downvotes_received: i64 = conn.query_row(
        "SELECT COALESCE(SUM(CASE WHEN v.value = -1 THEN 1 ELSE 0 END), 0) FROM votes v
         JOIN posts p ON v.post_id = p.id WHERE p.user_id = ?1",
        params![user_id], |r| r.get(0)
    ).unwrap_or(0);
    
    Ok(UserStats {
        post_count,
        comment_count,
        karma,
        upvotes_received,
        downvotes_received,
    })
}

// ============ Session Functions ============

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

pub fn delete_user_sessions(conn: &Connection, user_id: i64) -> Result<()> {
    conn.execute("DELETE FROM sessions WHERE user_id = ?1", params![user_id])?;
    Ok(())
}

// ============ Category Functions ============

pub fn get_categories(conn: &Connection) -> Result<Vec<Category>> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.name, c.slug, c.description, c.icon, c.color,
         (SELECT COUNT(*) FROM posts WHERE category_id = c.id AND removed = 0) as post_count
         FROM categories c ORDER BY c.name"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Category {
            id: row.get(0)?,
            name: row.get(1)?,
            slug: row.get(2)?,
            description: row.get(3)?,
            icon: row.get(4)?,
            color: row.get(5)?,
            post_count: row.get(6)?,
        })
    })?;
    rows.collect()
}

pub fn get_category_by_slug(conn: &Connection, slug: &str) -> Result<Option<Category>> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.name, c.slug, c.description, c.icon, c.color,
         (SELECT COUNT(*) FROM posts WHERE category_id = c.id AND removed = 0) as post_count
         FROM categories c WHERE c.slug = ?1"
    )?;
    let mut rows = stmt.query(params![slug])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Category {
            id: row.get(0)?,
            name: row.get(1)?,
            slug: row.get(2)?,
            description: row.get(3)?,
            icon: row.get(4)?,
            color: row.get(5)?,
            post_count: row.get(6)?,
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

// ============ Post Tag Functions ============

pub fn get_all_tags(conn: &Connection) -> Result<Vec<PostTag>> {
    let mut stmt = conn.prepare("SELECT id, name, color, description FROM post_tags ORDER BY name")?;
    let rows = stmt.query_map([], |row| {
        Ok(PostTag {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            description: row.get(3)?,
        })
    })?;
    rows.collect()
}

pub fn get_tags_for_post(conn: &Connection, post_id: i64) -> Result<Vec<PostTag>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.color, t.description 
         FROM post_tags t 
         JOIN post_tag_map m ON t.id = m.tag_id 
         WHERE m.post_id = ?1"
    )?;
    let rows = stmt.query_map(params![post_id], |row| {
        Ok(PostTag {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            description: row.get(3)?,
        })
    })?;
    rows.collect()
}

pub fn add_tag_to_post(conn: &Connection, post_id: i64, tag_id: i64) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO post_tag_map (post_id, tag_id) VALUES (?1, ?2)",
        params![post_id, tag_id],
    )?;
    Ok(())
}

pub fn remove_tag_from_post(conn: &Connection, post_id: i64, tag_id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM post_tag_map WHERE post_id = ?1 AND tag_id = ?2",
        params![post_id, tag_id],
    )?;
    Ok(())
}

pub fn set_post_tags(conn: &Connection, post_id: i64, tag_ids: &[i64]) -> Result<()> {
    conn.execute("DELETE FROM post_tag_map WHERE post_id = ?1", params![post_id])?;
    for tag_id in tag_ids {
        conn.execute(
            "INSERT INTO post_tag_map (post_id, tag_id) VALUES (?1, ?2)",
            params![post_id, tag_id],
        )?;
    }
    Ok(())
}

// ============ Post Functions ============

pub fn create_post(conn: &Connection, user_id: i64, category_id: i64, title: &str, body: &str) -> Result<i64> {
    let body_html = render_markdown(body);
    conn.execute(
        "INSERT INTO posts (user_id, category_id, title, body, body_html, score) VALUES (?1, ?2, ?3, ?4, ?5, 1)",
        params![user_id, category_id, title, body, body_html],
    )?;
    let post_id = conn.last_insert_rowid();
    // Auto-upvote own post
    conn.execute(
        "INSERT INTO votes (user_id, post_id, value) VALUES (?1, ?2, 1)",
        params![user_id, post_id],
    )?;
    // Add karma
    let _ = update_user_karma(conn, user_id, 1);
    Ok(post_id)
}

pub fn create_post_with_tags(conn: &Connection, user_id: i64, category_id: i64, title: &str, body: &str, tag_ids: &[i64]) -> Result<i64> {
    let post_id = create_post(conn, user_id, category_id, title, body)?;
    set_post_tags(conn, post_id, tag_ids)?;
    Ok(post_id)
}

pub fn get_posts(conn: &Connection, category_slug: Option<&str>, sort: &str, limit: i64, offset: i64) -> Result<Vec<Post>> {
    let order = match sort {
        "top" => "p.score DESC, p.created_at DESC",
        "new" => "p.created_at DESC",
        "controversial" => "ABS(p.score) ASC, p.created_at DESC",
        _ => "p.score DESC, p.created_at DESC", // hot (default)
    };

    let sql = format!(
        r#"SELECT p.id, p.user_id, p.category_id, p.title, p.body, p.body_html, p.score, p.created_at, 
           p.edited_at, p.removed, p.pinned, p.best_answer_id,
           u.username, u.role, u.flair,
           (SELECT avatar_path FROM user_profiles WHERE user_id = u.id) as avatar,
           c.name, c.slug,
           (SELECT COUNT(*) FROM comments WHERE post_id = p.id AND removed = 0) as comment_count
           FROM posts p
           JOIN users u ON p.user_id = u.id
           JOIN categories c ON p.category_id = c.id
           WHERE p.removed = 0
           {}
           ORDER BY p.pinned DESC, {}
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

pub fn get_posts_paginated(conn: &Connection, category_slug: Option<&str>, sort: &str, page: i64, per_page: i64) -> Result<(Vec<Post>, PaginationInfo)> {
    let offset = (page - 1) * per_page;
    
    // Count total
    let count_sql = if category_slug.is_some() {
        "SELECT COUNT(*) FROM posts p JOIN categories c ON p.category_id = c.id WHERE p.removed = 0 AND c.slug = ?1"
    } else {
        "SELECT COUNT(*) FROM posts WHERE removed = 0"
    };
    
    let total: i64 = if let Some(slug) = category_slug {
        conn.query_row(count_sql, params![slug], |r| r.get(0))?
    } else {
        conn.query_row(count_sql, [], |r| r.get(0))?
    };
    
    let posts = get_posts(conn, category_slug, sort, per_page, offset)?;
    let pagination = PaginationInfo::new(page, per_page, total);
    
    Ok((posts, pagination))
}

fn map_post(row: &rusqlite::Row) -> rusqlite::Result<Post> {
    Ok(Post {
        id: row.get(0)?,
        user_id: row.get(1)?,
        category_id: row.get(2)?,
        title: row.get(3)?,
        body: row.get(4)?,
        body_html: row.get(5)?,
        score: row.get(6)?,
        created_at: row.get(7)?,
        edited_at: row.get(8)?,
        removed: row.get::<_, i64>(9)? != 0,
        pinned: row.get::<_, i64>(10)? != 0,
        best_answer_id: row.get(11)?,
        username: row.get(12).ok(),
        user_role: row.get(13).ok(),
        user_flair: row.get(14).ok(),
        user_avatar: row.get(15).ok(),
        category_name: row.get(16).ok(),
        category_slug: row.get(17).ok(),
        comment_count: row.get(18).ok(),
        tags: None,
        is_bookmarked: None,
        user_vote: None,
    })
}

pub fn get_post_by_id(conn: &Connection, id: i64) -> Result<Option<Post>> {
    let mut stmt = conn.prepare(
        r#"SELECT p.id, p.user_id, p.category_id, p.title, p.body, p.body_html, p.score, p.created_at,
           p.edited_at, p.removed, p.pinned, p.best_answer_id,
           u.username, u.role, u.flair,
           (SELECT avatar_path FROM user_profiles WHERE user_id = u.id) as avatar,
           c.name, c.slug,
           (SELECT COUNT(*) FROM comments WHERE post_id = p.id AND removed = 0) as comment_count
           FROM posts p
           JOIN users u ON p.user_id = u.id
           JOIN categories c ON p.category_id = c.id
           WHERE p.id = ?1"#
    )?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        let mut post = map_post(row)?;
        post.tags = Some(get_tags_for_post(conn, id).unwrap_or_default());
        Ok(Some(post))
    } else {
        Ok(None)
    }
}

pub fn get_post_with_user_context(conn: &Connection, id: i64, user_id: i64) -> Result<Option<Post>> {
    let mut post = match get_post_by_id(conn, id)? {
        Some(p) => p,
        None => return Ok(None),
    };
    
    post.user_vote = get_user_vote_for_post(conn, user_id, id)?;
    post.is_bookmarked = Some(is_post_bookmarked(conn, user_id, id)?);
    
    Ok(Some(post))
}

pub fn get_posts_by_user(conn: &Connection, user_id: i64) -> Result<Vec<Post>> {
    let mut stmt = conn.prepare(
        r#"SELECT p.id, p.user_id, p.category_id, p.title, p.body, p.body_html, p.score, p.created_at,
           p.edited_at, p.removed, p.pinned, p.best_answer_id,
           u.username, u.role, u.flair,
           (SELECT avatar_path FROM user_profiles WHERE user_id = u.id) as avatar,
           c.name, c.slug,
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

pub fn update_post(conn: &Connection, post_id: i64, user_id: i64, title: &str, body: &str) -> Result<()> {
    // Save edit history
    let (old_title, old_body): (String, String) = conn.query_row(
        "SELECT title, body FROM posts WHERE id = ?1",
        params![post_id],
        |r| Ok((r.get(0)?, r.get(1)?))
    )?;
    
    conn.execute(
        "INSERT INTO post_edits (post_id, user_id, old_title, old_body) VALUES (?1, ?2, ?3, ?4)",
        params![post_id, user_id, old_title, old_body],
    )?;
    
    let body_html = render_markdown(body);
    conn.execute(
        "UPDATE posts SET title = ?1, body = ?2, body_html = ?3, edited_at = datetime('now') WHERE id = ?4",
        params![title, body, body_html, post_id],
    )?;
    Ok(())
}

pub fn remove_post(conn: &Connection, post_id: i64) -> Result<()> {
    conn.execute("UPDATE posts SET removed = 1 WHERE id = ?1", params![post_id])?;
    Ok(())
}

pub fn restore_post(conn: &Connection, post_id: i64) -> Result<()> {
    conn.execute("UPDATE posts SET removed = 0 WHERE id = ?1", params![post_id])?;
    Ok(())
}

pub fn pin_post(conn: &Connection, post_id: i64, pinned: bool) -> Result<()> {
    conn.execute("UPDATE posts SET pinned = ?1 WHERE id = ?2", params![pinned as i64, post_id])?;
    Ok(())
}

pub fn set_best_answer(conn: &Connection, post_id: i64, comment_id: Option<i64>) -> Result<()> {
    conn.execute("UPDATE posts SET best_answer_id = ?1 WHERE id = ?2", params![comment_id, post_id])?;
    Ok(())
}

pub fn get_trending_posts(conn: &Connection, limit: i64) -> Result<Vec<Post>> {
    // Posts from last 7 days with high engagement
    let mut stmt = conn.prepare(
        r#"SELECT p.id, p.user_id, p.category_id, p.title, p.body, p.body_html, p.score, p.created_at,
           p.edited_at, p.removed, p.pinned, p.best_answer_id,
           u.username, u.role, u.flair,
           (SELECT avatar_path FROM user_profiles WHERE user_id = u.id) as avatar,
           c.name, c.slug,
           (SELECT COUNT(*) FROM comments WHERE post_id = p.id AND removed = 0) as comment_count
           FROM posts p
           JOIN users u ON p.user_id = u.id
           JOIN categories c ON p.category_id = c.id
           WHERE p.removed = 0 AND p.created_at > datetime('now', '-7 days')
           ORDER BY (p.score + (SELECT COUNT(*) FROM comments WHERE post_id = p.id) * 2) DESC
           LIMIT ?1"#
    )?;
    let rows = stmt.query_map(params![limit], map_post)?;
    rows.collect()
}

// ============ Comment Functions ============

pub fn create_comment(conn: &Connection, post_id: i64, user_id: i64, parent_id: Option<i64>, body: &str) -> Result<i64> {
    let body_html = render_markdown(body);
    conn.execute(
        "INSERT INTO comments (post_id, user_id, parent_id, body, body_html, score) VALUES (?1, ?2, ?3, ?4, ?5, 1)",
        params![post_id, user_id, parent_id, body, body_html],
    )?;
    let comment_id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO votes (user_id, comment_id, value) VALUES (?1, ?2, 1)",
        params![user_id, comment_id],
    )?;
    
    // Notify post author
    let post_author: i64 = conn.query_row(
        "SELECT user_id FROM posts WHERE id = ?1",
        params![post_id],
        |r| r.get(0)
    )?;
    
    if post_author != user_id {
        create_notification(conn, post_author, "post_reply", "Someone replied to your post", Some(post_id), Some(comment_id), Some(user_id))?;
    }
    
    // If it's a reply, notify parent comment author
    if let Some(pid) = parent_id {
        let parent_author: i64 = conn.query_row(
            "SELECT user_id FROM comments WHERE id = ?1",
            params![pid],
            |r| r.get(0)
        )?;
        if parent_author != user_id && parent_author != post_author {
            create_notification(conn, parent_author, "reply", "Someone replied to your comment", Some(post_id), Some(comment_id), Some(user_id))?;
        }
    }
    
    // Check for @mentions
    let mentions = extract_mentions(body);
    for mention in mentions {
        if let Ok(Some(mentioned_user)) = get_user_by_username(conn, &mention) {
            if mentioned_user.id != user_id {
                create_notification(conn, mentioned_user.id, "mention", &format!("You were mentioned in a comment"), Some(post_id), Some(comment_id), Some(user_id))?;
            }
        }
    }
    
    Ok(comment_id)
}

pub fn get_comments_for_post(conn: &Connection, post_id: i64) -> Result<Vec<Comment>> {
    get_comments_for_post_sorted(conn, post_id, "best")
}

pub fn get_comments_for_post_sorted(conn: &Connection, post_id: i64, sort: &str) -> Result<Vec<Comment>> {
    let order = match sort {
        "new" => "c.created_at DESC",
        "old" => "c.created_at ASC",
        "controversial" => "ABS(c.score) ASC, c.created_at DESC",
        _ => "c.score DESC, c.created_at ASC", // best
    };
    
    let sql = format!(
        r#"SELECT c.id, c.post_id, c.user_id, c.parent_id, c.body, c.body_html, c.score, c.created_at, 
           c.edited_at, c.removed,
           u.username, u.role, u.flair,
           (SELECT avatar_path FROM user_profiles WHERE user_id = u.id) as avatar,
           (SELECT best_answer_id FROM posts WHERE id = c.post_id) as best_id
           FROM comments c
           JOIN users u ON c.user_id = u.id
           WHERE c.post_id = ?1 AND c.removed = 0
           ORDER BY {}"#,
        order
    );
    
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![post_id], |row| {
        let best_id: Option<i64> = row.get(14)?;
        let comment_id: i64 = row.get(0)?;
        Ok(Comment {
            id: comment_id,
            post_id: row.get(1)?,
            user_id: row.get(2)?,
            parent_id: row.get(3)?,
            body: row.get(4)?,
            body_html: row.get(5)?,
            score: row.get(6)?,
            created_at: row.get(7)?,
            edited_at: row.get(8)?,
            removed: row.get::<_, i64>(9)? != 0,
            username: row.get(10).ok(),
            user_role: row.get(11).ok(),
            user_flair: row.get(12).ok(),
            user_avatar: row.get(13).ok(),
            is_best_answer: best_id == Some(comment_id),
            replies: vec![],
            user_vote: None,
            depth: 0,
        })
    })?;
    rows.collect()
}

pub fn get_comments_by_user(conn: &Connection, user_id: i64) -> Result<Vec<Comment>> {
    let mut stmt = conn.prepare(
        r#"SELECT c.id, c.post_id, c.user_id, c.parent_id, c.body, c.body_html, c.score, c.created_at,
           c.edited_at, c.removed,
           u.username, u.role, u.flair,
           (SELECT avatar_path FROM user_profiles WHERE user_id = u.id) as avatar,
           (SELECT best_answer_id FROM posts WHERE id = c.post_id) as best_id
           FROM comments c
           JOIN users u ON c.user_id = u.id
           WHERE c.user_id = ?1 AND c.removed = 0
           ORDER BY c.created_at DESC"#
    )?;
    let rows = stmt.query_map(params![user_id], |row| {
        let best_id: Option<i64> = row.get(14)?;
        let comment_id: i64 = row.get(0)?;
        Ok(Comment {
            id: comment_id,
            post_id: row.get(1)?,
            user_id: row.get(2)?,
            parent_id: row.get(3)?,
            body: row.get(4)?,
            body_html: row.get(5)?,
            score: row.get(6)?,
            created_at: row.get(7)?,
            edited_at: row.get(8)?,
            removed: row.get::<_, i64>(9)? != 0,
            username: row.get(10).ok(),
            user_role: row.get(11).ok(),
            user_flair: row.get(12).ok(),
            user_avatar: row.get(13).ok(),
            is_best_answer: best_id == Some(comment_id),
            replies: vec![],
            user_vote: None,
            depth: 0,
        })
    })?;
    rows.collect()
}

pub fn update_comment(conn: &Connection, comment_id: i64, user_id: i64, body: &str) -> Result<()> {
    // Save edit history
    let old_body: String = conn.query_row(
        "SELECT body FROM comments WHERE id = ?1",
        params![comment_id],
        |r| r.get(0)
    )?;
    
    conn.execute(
        "INSERT INTO comment_edits (comment_id, user_id, old_body) VALUES (?1, ?2, ?3)",
        params![comment_id, user_id, old_body],
    )?;
    
    let body_html = render_markdown(body);
    conn.execute(
        "UPDATE comments SET body = ?1, body_html = ?2, edited_at = datetime('now') WHERE id = ?3",
        params![body, body_html, comment_id],
    )?;
    Ok(())
}

pub fn remove_comment(conn: &Connection, comment_id: i64) -> Result<()> {
    conn.execute("UPDATE comments SET removed = 1 WHERE id = ?1", params![comment_id])?;
    Ok(())
}

// ============ Vote Functions ============

pub fn vote_post(conn: &Connection, user_id: i64, post_id: i64, value: i64) -> Result<i64> {
    let existing: Option<i64> = conn.query_row(
        "SELECT value FROM votes WHERE user_id = ?1 AND post_id = ?2",
        params![user_id, post_id],
        |row| row.get(0),
    ).ok();

    let post_author: i64 = conn.query_row(
        "SELECT user_id FROM posts WHERE id = ?1",
        params![post_id],
        |r| r.get(0)
    )?;

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
            let _ = update_user_karma(conn, post_author, -value);
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
            let _ = update_user_karma(conn, post_author, value - old_value);
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
            let _ = update_user_karma(conn, post_author, value);
        }
    }

    conn.query_row("SELECT score FROM posts WHERE id = ?1", params![post_id], |row| row.get(0))
}

pub fn vote_comment(conn: &Connection, user_id: i64, comment_id: i64, value: i64) -> Result<i64> {
    let existing: Option<i64> = conn.query_row(
        "SELECT value FROM votes WHERE user_id = ?1 AND comment_id = ?2",
        params![user_id, comment_id],
        |row| row.get(0),
    ).ok();

    let comment_author: i64 = conn.query_row(
        "SELECT user_id FROM comments WHERE id = ?1",
        params![comment_id],
        |r| r.get(0)
    )?;

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
            let _ = update_user_karma(conn, comment_author, -value);
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
            let _ = update_user_karma(conn, comment_author, value - old_value);
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
            let _ = update_user_karma(conn, comment_author, value);
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

// ============ Store Functions ============

pub fn create_store(conn: &Connection, name: &str, url: &str, description: Option<&str>, category: &str, submitted_by: i64) -> Result<i64> {
    conn.execute(
        "INSERT INTO stores (name, url, description, category, submitted_by) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![name, url, description, category, submitted_by],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_stores(conn: &Connection, category: Option<&str>) -> Result<Vec<Store>> {
    let sql = format!(
        r#"SELECT s.id, s.name, s.url, s.description, s.category, s.submitted_by, s.created_at,
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
    let pos: i64 = row.get(8)?;
    let total: i64 = row.get(9)?;
    Ok(Store {
        id: row.get(0)?,
        name: row.get(1)?,
        url: row.get(2)?,
        description: row.get(3)?,
        category: row.get(4)?,
        submitted_by: row.get(5)?,
        created_at: row.get(6)?,
        submitter_name: row.get(7).ok(),
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

pub fn search_stores(conn: &Connection, query: &str) -> Result<Vec<Store>> {
    let search_pattern = format!("%{}%", query);
    let mut stmt = conn.prepare(
        r#"SELECT s.id, s.name, s.url, s.description, s.category, s.submitted_by, s.created_at,
           u.username,
           (SELECT COUNT(*) FROM store_votes WHERE store_id = s.id AND positive = 1) as pos,
           (SELECT COUNT(*) FROM store_votes WHERE store_id = s.id) as total
           FROM stores s
           JOIN users u ON s.submitted_by = u.id
           WHERE s.name LIKE ?1 OR s.description LIKE ?1
           ORDER BY s.name"#
    )?;
    let rows = stmt.query_map(params![search_pattern], map_store)?;
    rows.collect()
}

// ============ Verification Functions ============

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
    
    create_notification(conn, user_id, "system", "Your verification has been approved! You can now create posts.", None, None, None)?;
    
    Ok(())
}

pub fn deny_verification(conn: &Connection, request_id: i64, reviewer_id: i64) -> Result<()> {
    let user_id: i64 = conn.query_row(
        "SELECT user_id FROM verification_requests WHERE id = ?1",
        params![request_id],
        |row| row.get(0),
    )?;
    
    conn.execute(
        "UPDATE verification_requests SET status = 'denied', reviewed_by = ?1 WHERE id = ?2",
        params![reviewer_id, request_id],
    )?;
    
    create_notification(conn, user_id, "system", "Your verification request was not approved. Please submit additional proof.", None, None, None)?;
    
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

// ============ Report Functions ============

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

// ============ Bookmark Functions ============

pub fn add_bookmark(conn: &Connection, user_id: i64, post_id: i64) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO bookmarks (user_id, post_id) VALUES (?1, ?2)",
        params![user_id, post_id],
    )?;
    Ok(())
}

pub fn remove_bookmark(conn: &Connection, user_id: i64, post_id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM bookmarks WHERE user_id = ?1 AND post_id = ?2",
        params![user_id, post_id],
    )?;
    Ok(())
}

pub fn is_post_bookmarked(conn: &Connection, user_id: i64, post_id: i64) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM bookmarks WHERE user_id = ?1 AND post_id = ?2",
        params![user_id, post_id],
        |r| r.get(0),
    )?;
    Ok(count > 0)
}

pub fn get_user_bookmarks(conn: &Connection, user_id: i64) -> Result<Vec<Post>> {
    let mut stmt = conn.prepare(
        r#"SELECT p.id, p.user_id, p.category_id, p.title, p.body, p.body_html, p.score, p.created_at,
           p.edited_at, p.removed, p.pinned, p.best_answer_id,
           u.username, u.role, u.flair,
           (SELECT avatar_path FROM user_profiles WHERE user_id = u.id) as avatar,
           c.name, c.slug,
           (SELECT COUNT(*) FROM comments WHERE post_id = p.id AND removed = 0) as comment_count
           FROM posts p
           JOIN users u ON p.user_id = u.id
           JOIN categories c ON p.category_id = c.id
           JOIN bookmarks b ON b.post_id = p.id
           WHERE b.user_id = ?1 AND p.removed = 0
           ORDER BY b.created_at DESC"#
    )?;
    let rows = stmt.query_map(params![user_id], map_post)?;
    rows.collect()
}

// ============ Notification Functions ============

pub fn create_notification(conn: &Connection, user_id: i64, notification_type: &str, content: &str, post_id: Option<i64>, comment_id: Option<i64>, from_user_id: Option<i64>) -> Result<i64> {
    conn.execute(
        "INSERT INTO notifications (user_id, notification_type, content, post_id, comment_id, from_user_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![user_id, notification_type, content, post_id, comment_id, from_user_id],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_user_notifications(conn: &Connection, user_id: i64, limit: i64) -> Result<Vec<Notification>> {
    let mut stmt = conn.prepare(
        r#"SELECT n.id, n.user_id, n.notification_type, n.content, n.read, n.post_id, n.comment_id, n.from_user_id, n.created_at,
           u.username,
           p.title
           FROM notifications n
           LEFT JOIN users u ON n.from_user_id = u.id
           LEFT JOIN posts p ON n.post_id = p.id
           WHERE n.user_id = ?1
           ORDER BY n.created_at DESC
           LIMIT ?2"#
    )?;
    let rows = stmt.query_map(params![user_id, limit], |row| {
        Ok(Notification {
            id: row.get(0)?,
            user_id: row.get(1)?,
            notification_type: NotificationType::from_str(&row.get::<_, String>(2)?),
            content: row.get(3)?,
            read: row.get::<_, i64>(4)? != 0,
            post_id: row.get(5)?,
            comment_id: row.get(6)?,
            from_user_id: row.get(7)?,
            created_at: row.get(8)?,
            from_username: row.get(9).ok(),
            post_title: row.get(10).ok(),
        })
    })?;
    rows.collect()
}

pub fn get_unread_notification_count(conn: &Connection, user_id: i64) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM notifications WHERE user_id = ?1 AND read = 0",
        params![user_id],
        |r| r.get(0),
    )
}

pub fn mark_notification_read(conn: &Connection, notification_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE notifications SET read = 1 WHERE id = ?1",
        params![notification_id],
    )?;
    Ok(())
}

pub fn mark_all_notifications_read(conn: &Connection, user_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE notifications SET read = 1 WHERE user_id = ?1",
        params![user_id],
    )?;
    Ok(())
}

// ============ Announcement Functions ============

pub fn create_announcement(conn: &Connection, title: &str, content: &str, announcement_type: &str, created_by: i64, expires_at: Option<&str>) -> Result<i64> {
    conn.execute(
        "INSERT INTO announcements (title, content, announcement_type, created_by, expires_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![title, content, announcement_type, created_by, expires_at],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_active_announcements(conn: &Connection) -> Result<Vec<Announcement>> {
    let mut stmt = conn.prepare(
        r#"SELECT a.id, a.title, a.content, a.active, a.pinned, a.announcement_type, a.created_by, a.created_at, a.expires_at,
           u.username
           FROM announcements a
           JOIN users u ON a.created_by = u.id
           WHERE a.active = 1 AND (a.expires_at IS NULL OR a.expires_at > datetime('now'))
           ORDER BY a.pinned DESC, a.created_at DESC"#
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Announcement {
            id: row.get(0)?,
            title: row.get(1)?,
            content: row.get(2)?,
            active: row.get::<_, i64>(3)? != 0,
            pinned: row.get::<_, i64>(4)? != 0,
            announcement_type: row.get(5)?,
            created_by: row.get(6)?,
            created_at: row.get(7)?,
            expires_at: row.get(8)?,
            creator_name: row.get(9).ok(),
        })
    })?;
    rows.collect()
}

pub fn deactivate_announcement(conn: &Connection, announcement_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE announcements SET active = 0 WHERE id = ?1",
        params![announcement_id],
    )?;
    Ok(())
}

// ============ Upload Functions ============

pub fn create_upload(conn: &Connection, user_id: i64, filename: &str, original_name: &str, path: &str, mime_type: &str, size_bytes: i64) -> Result<i64> {
    conn.execute(
        "INSERT INTO uploads (user_id, filename, original_name, path, mime_type, size_bytes) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![user_id, filename, original_name, path, mime_type, size_bytes],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_upload(conn: &Connection, id: i64) -> Result<Option<Upload>> {
    let mut stmt = conn.prepare(
        "SELECT id, user_id, filename, original_name, path, mime_type, size_bytes, created_at FROM uploads WHERE id = ?1"
    )?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Upload {
            id: row.get(0)?,
            user_id: row.get(1)?,
            filename: row.get(2)?,
            original_name: row.get(3)?,
            path: row.get(4)?,
            mime_type: row.get(5)?,
            size_bytes: row.get(6)?,
            created_at: row.get(7)?,
        }))
    } else {
        Ok(None)
    }
}

// ============ Activity Log Functions ============

pub fn log_activity(conn: &Connection, user_id: i64, action: &str, target_type: Option<&str>, target_id: Option<i64>, details: Option<&str>, ip_address: Option<&str>) -> Result<()> {
    conn.execute(
        "INSERT INTO activity_logs (user_id, action, target_type, target_id, details, ip_address) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![user_id, action, target_type, target_id, details, ip_address],
    )?;
    Ok(())
}

pub fn get_user_activity(conn: &Connection, user_id: i64, limit: i64) -> Result<Vec<ActivityLog>> {
    let mut stmt = conn.prepare(
        r#"SELECT a.id, a.user_id, a.action, a.target_type, a.target_id, a.details, a.ip_address, a.created_at,
           u.username
           FROM activity_logs a
           JOIN users u ON a.user_id = u.id
           WHERE a.user_id = ?1
           ORDER BY a.created_at DESC
           LIMIT ?2"#
    )?;
    let rows = stmt.query_map(params![user_id, limit], |row| {
        Ok(ActivityLog {
            id: row.get(0)?,
            user_id: row.get(1)?,
            action: row.get(2)?,
            target_type: row.get(3)?,
            target_id: row.get(4)?,
            details: row.get(5)?,
            ip_address: row.get(6)?,
            created_at: row.get(7)?,
            username: row.get(8).ok(),
        })
    })?;
    rows.collect()
}

pub fn get_recent_activity(conn: &Connection, limit: i64) -> Result<Vec<ActivityLog>> {
    let mut stmt = conn.prepare(
        r#"SELECT a.id, a.user_id, a.action, a.target_type, a.target_id, a.details, a.ip_address, a.created_at,
           u.username
           FROM activity_logs a
           JOIN users u ON a.user_id = u.id
           ORDER BY a.created_at DESC
           LIMIT ?1"#
    )?;
    let rows = stmt.query_map(params![limit], |row| {
        Ok(ActivityLog {
            id: row.get(0)?,
            user_id: row.get(1)?,
            action: row.get(2)?,
            target_type: row.get(3)?,
            target_id: row.get(4)?,
            details: row.get(5)?,
            ip_address: row.get(6)?,
            created_at: row.get(7)?,
            username: row.get(8).ok(),
        })
    })?;
    rows.collect()
}

// ============ Search Functions ============

pub fn search_posts(conn: &Connection, query: &str, category_slug: Option<&str>, limit: i64) -> Result<Vec<Post>> {
    let search_pattern = format!("%{}%", query);
    
    let sql = format!(
        r#"SELECT p.id, p.user_id, p.category_id, p.title, p.body, p.body_html, p.score, p.created_at,
           p.edited_at, p.removed, p.pinned, p.best_answer_id,
           u.username, u.role, u.flair,
           (SELECT avatar_path FROM user_profiles WHERE user_id = u.id) as avatar,
           c.name, c.slug,
           (SELECT COUNT(*) FROM comments WHERE post_id = p.id AND removed = 0) as comment_count
           FROM posts p
           JOIN users u ON p.user_id = u.id
           JOIN categories c ON p.category_id = c.id
           WHERE p.removed = 0 AND (p.title LIKE ?1 OR p.body LIKE ?1)
           {}
           ORDER BY p.score DESC
           LIMIT ?2"#,
        if category_slug.is_some() { "AND c.slug = ?3" } else { "" }
    );
    
    let mut stmt = conn.prepare(&sql)?;
    let rows = if let Some(slug) = category_slug {
        stmt.query_map(params![search_pattern, limit, slug], map_post)?
    } else {
        stmt.query_map(params![search_pattern, limit], map_post)?
    };
    rows.collect()
}

pub fn global_search(conn: &Connection, query: &str, limit: i64) -> Result<Vec<SearchResult>> {
    let search_pattern = format!("%{}%", query);
    let mut results = Vec::new();
    
    // Search posts
    let mut stmt = conn.prepare(
        r#"SELECT 'post' as type, id, title, substr(body, 1, 200) as snippet, '/post/' || id as url, score, created_at
           FROM posts WHERE removed = 0 AND (title LIKE ?1 OR body LIKE ?1)
           ORDER BY score DESC LIMIT ?2"#
    )?;
    let post_rows = stmt.query_map(params![&search_pattern, limit / 2], |row| {
        Ok(SearchResult {
            result_type: row.get(0)?,
            id: row.get(1)?,
            title: row.get(2)?,
            snippet: row.get(3)?,
            url: row.get(4)?,
            score: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;
    for row in post_rows {
        results.push(row?);
    }
    
    // Search stores
    let mut stmt = conn.prepare(
        r#"SELECT 'store' as type, id, name, COALESCE(description, '') as snippet, '/stores#store-' || id as url, NULL as score, created_at
           FROM stores WHERE name LIKE ?1 OR description LIKE ?1
           ORDER BY name LIMIT ?2"#
    )?;
    let store_rows = stmt.query_map(params![&search_pattern, limit / 2], |row| {
        Ok(SearchResult {
            result_type: row.get(0)?,
            id: row.get(1)?,
            title: row.get(2)?,
            snippet: row.get(3)?,
            url: row.get(4)?,
            score: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;
    for row in store_rows {
        results.push(row?);
    }
    
    Ok(results)
}

// ============ Stats Functions ============

pub fn get_forum_stats(conn: &Connection) -> Result<ForumStats> {
    let total_users: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?;
    let verified_users: i64 = conn.query_row("SELECT COUNT(*) FROM users WHERE role = 'verified_mechanic'", [], |r| r.get(0))?;
    let total_posts: i64 = conn.query_row("SELECT COUNT(*) FROM posts WHERE removed = 0", [], |r| r.get(0))?;
    let total_comments: i64 = conn.query_row("SELECT COUNT(*) FROM comments WHERE removed = 0", [], |r| r.get(0))?;
    let total_stores: i64 = conn.query_row("SELECT COUNT(*) FROM stores", [], |r| r.get(0))?;
    let posts_today: i64 = conn.query_row(
        "SELECT COUNT(*) FROM posts WHERE removed = 0 AND date(created_at) = date('now')",
        [], |r| r.get(0)
    )?;
    
    Ok(ForumStats {
        total_users,
        verified_users,
        total_posts,
        total_comments,
        total_stores,
        posts_today,
    })
}

// ============ Helper Functions ============

fn render_markdown(text: &str) -> String {
    use pulldown_cmark::{Parser, Options, html};
    
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    
    let parser = Parser::new_ext(text, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    
    html_output
}

fn extract_mentions(text: &str) -> Vec<String> {
    let re = regex::Regex::new(r"@(\w+)").unwrap();
    re.captures_iter(text)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_render_markdown() {
        let input = "**bold** and *italic*";
        let output = render_markdown(input);
        assert!(output.contains("<strong>bold</strong>"));
        assert!(output.contains("<em>italic</em>"));
    }
    
    #[test]
    fn test_extract_mentions() {
        let text = "Hello @john and @jane, what do you think?";
        let mentions = extract_mentions(text);
        assert_eq!(mentions.len(), 2);
        assert!(mentions.contains(&"john".to_string()));
        assert!(mentions.contains(&"jane".to_string()));
    }
}
