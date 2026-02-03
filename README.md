# 🔧 Wrench Forum

A Reddit-style forum for mechanics, built with Rust, Axum, HTMX, and SQLite.

## Features

- **User System**: Register/login with email + password (argon2 hashing)
- **Mechanic Verification**: Submit credentials, get verified badge
- **Forum**: Categories, posts, threaded comments, upvote/downvote
- **Moderation**: Report content, mod queue, ban management  
- **Parts Stores**: Community-rated store directory with reliability scores

## Stack

- **Backend**: Rust + Axum 0.8
- **Frontend**: HTMX 2.0 + Plain CSS
- **Database**: SQLite (rusqlite)
- **Templates**: Tera

## Getting Started

```bash
# Build and run
cargo run

# Server starts at http://localhost:3000
```

## User Roles

| Role | Can Post | Can Comment | Can Vote Stores | Can Moderate |
|------|----------|-------------|-----------------|--------------|
| Unverified | ❌ | ✅ | ❌ | ❌ |
| Verified Mechanic | ✅ | ✅ | ✅ | ❌ |
| Moderator | ✅ | ✅ | ✅ | ✅ |
| Admin | ✅ | ✅ | ✅ | ✅ |

## Project Structure

```
wrench-forum/
├── src/
│   ├── main.rs          # Entry point, router setup
│   ├── db.rs            # Database schema and queries
│   ├── models.rs        # Data structures
│   ├── auth.rs          # Password hashing, sessions
│   └── routes/          # Request handlers
├── templates/           # Tera HTML templates
├── static/              # CSS, HTMX
└── scripts/             # Seed data
```

## Seeding Data

```bash
# First, start the server to create the database
cargo run &

# Then run the seed script
chmod +x scripts/seed.sh
./scripts/seed.sh
```

## Routes

### Public
- `GET /` - Home page
- `GET /category/{slug}` - Category posts
- `GET /post/{id}` - View post
- `GET /user/{username}` - User profile
- `GET /stores` - Parts stores

### Auth
- `GET/POST /register` - Registration
- `GET/POST /login` - Login
- `GET /logout` - Logout

### Protected
- `GET/POST /post/new` - Create post (verified only)
- `POST /post/{id}/comment` - Add comment
- `POST /post/{id}/vote` - Vote on post
- `POST /comment/{id}/vote` - Vote on comment
- `GET/POST /verification` - Submit verification request

### Admin
- `GET /admin` - Admin panel
- `POST /admin/verify/{id}/approve` - Approve verification
- `POST /admin/verify/{id}/deny` - Deny verification

### Moderation
- `GET /mod` - Mod queue
- `POST /mod/post/{id}/remove` - Remove post
- `POST /mod/comment/{id}/remove` - Remove comment
- `POST /mod/user/{id}/ban` - Ban user
- `POST /mod/user/{id}/unban` - Unban user

## License

MIT
