#!/bin/bash
# Seed script for wrench-forum
# Run with: ./scripts/seed.sh

set -e

DB="wrench-forum.db"

# Check if database exists
if [ ! -f "$DB" ]; then
    echo "Database not found. Start the server first to create it, then run this script."
    exit 1
fi

echo "Seeding database..."

# Hash for "admin123" - pre-computed argon2 hash
ADMIN_HASH='$argon2id$v=19$m=19456,t=2,p=1$random_salt_here$hashed_value'

# We'll use a simpler approach - insert directly with placeholder hashes
# The real app will hash passwords properly

sqlite3 "$DB" << 'EOF'
-- Admin user (password: admin123)
INSERT OR IGNORE INTO users (email, password_hash, username, role) 
VALUES ('admin@wrench.forum', '$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$hash', 'admin', 'admin');

-- Verified mechanics
INSERT OR IGNORE INTO users (email, password_hash, username, role) 
VALUES ('mike@garage.com', '$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$hash', 'MikeTheMechanic', 'verified_mechanic');

INSERT OR IGNORE INTO users (email, password_hash, username, role) 
VALUES ('sarah@autofix.com', '$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$hash', 'SarahWrench', 'verified_mechanic');

INSERT OR IGNORE INTO users (email, password_hash, username, role) 
VALUES ('joe@transmission.com', '$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$hash', 'TransmissionJoe', 'verified_mechanic');

-- Regular users
INSERT OR IGNORE INTO users (email, password_hash, username, role) 
VALUES ('newbie@email.com', '$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$hash', 'CarNewbie', 'unverified');

-- Categories
INSERT OR IGNORE INTO categories (name, slug, description) VALUES 
('Engine', 'engine', 'Engine diagnostics, repairs, and maintenance'),
('Transmission', 'transmission', 'Manual and automatic transmission issues'),
('Brakes', 'brakes', 'Brake pads, rotors, ABS, and brake fluid'),
('Electrical', 'electrical', 'Wiring, batteries, alternators, and electronics'),
('Suspension', 'suspension', 'Shocks, struts, control arms, and alignment');

-- Sample posts (by verified mechanics)
INSERT OR IGNORE INTO posts (user_id, category_id, title, body, score) VALUES 
(2, 1, 'Common P0300 Random Misfire Causes', 'Here are the most common causes I see for P0300 codes:

1. Spark plugs worn or fouled
2. Ignition coils failing
3. Vacuum leaks
4. Fuel injector issues
5. Low fuel pressure

Always start with the basics - check spark plugs first. If you have over 80k miles and original plugs, replace them.', 15);

INSERT OR IGNORE INTO posts (user_id, category_id, title, body, score) VALUES 
(3, 2, 'When to Change Transmission Fluid', 'I see a lot of debate about this. Here is my take after 20 years in the business:

Manual: Every 30-60k miles
Automatic: Every 30-60k miles if driven normally, more often if towing

The "lifetime fluid" marketing is nonsense. I have seen too many transmissions fail at 100k because people believed that.', 23);

INSERT OR IGNORE INTO posts (user_id, category_id, title, body, score) VALUES 
(4, 3, 'DIY Brake Job Tips', 'About to do your first brake job? Here are some tips:

1. Always replace pads in pairs (both sides)
2. Clean and lube the slide pins
3. Compress the piston slowly
4. Bed in new pads properly - 10 stops from 30mph
5. Check your brake fluid level

Do not cheap out on pads. I recommend ceramic for most daily drivers.', 18);

INSERT OR IGNORE INTO posts (user_id, category_id, title, body, score) VALUES 
(2, 4, 'Diagnosing Parasitic Draw', 'Battery dead every morning? Here is how to find the draw:

1. Fully charge battery
2. Disconnect negative terminal
3. Connect ammeter between terminal and cable
4. Wait 30 min for modules to sleep
5. Should read under 50mA

If higher, pull fuses one at a time to find the circuit. Most common culprits: aftermarket stereos, trunk lights, glove box lights.', 12);

INSERT OR IGNORE INTO posts (user_id, category_id, title, body, score) VALUES 
(3, 5, 'Symptoms of Worn Struts', 'How do you know when struts need replacing?

- Bouncy ride, especially over bumps
- Nose diving when braking
- Uneven tire wear
- Clunking noises
- Vehicle sways in wind or on curves

Most struts last 50-100k miles. If yours are original and over 80k, inspect them.', 9);

-- More sample posts
INSERT OR IGNORE INTO posts (user_id, category_id, title, body, score) VALUES 
(4, 1, 'Oil Change Intervals - The Real Story', 'Stop following the 3000 mile myth. Modern oils and engines do not need that.

Most cars: Follow the manual (usually 5-7.5k with synthetic)
If you drive hard/short trips: Cut interval by 25%
If you tow: Use severe service interval

Check your oil level monthly regardless.', 28);

INSERT OR IGNORE INTO posts (user_id, category_id, title, body, score) VALUES 
(2, 2, 'Automatic Transmission Slipping', 'Transmission slipping between gears? Check these first:

1. Fluid level (with engine warm, in Park)
2. Fluid condition (should be red, not brown)
3. Check for codes

If fluid is brown or smells burnt, the damage may already be done. Fresh fluid can make it worse by loosening debris.', 14);

INSERT OR IGNORE INTO posts (user_id, category_id, title, body, score) VALUES 
(3, 4, 'LED Headlight Conversions', 'Before swapping your halogens for LEDs:

1. Check your state laws
2. Reflector housings need specific LED bulbs
3. Projector housings work better with LEDs
4. You might need CANbus adapters
5. Cooling is important - get bulbs with fans

Cheap Amazon LEDs often have poor beam patterns. Spend the money on quality.', 11);

INSERT OR IGNORE INTO posts (user_id, category_id, title, body, score) VALUES 
(4, 3, 'ABS Light Troubleshooting', 'ABS light on? Here is my diagnostic approach:

1. Scan for codes (generic OBDII may not show ABS codes)
2. Most common: wheel speed sensor issues
3. Check sensor wiring at wheels
4. Check reluctor rings for damage
5. Check ABS module grounds

Wheel speed sensors are cheap and easy to replace yourself.', 16);

INSERT OR IGNORE INTO posts (user_id, category_id, title, body, score) VALUES 
(2, 5, 'Alignment After Suspension Work', 'Yes, you need an alignment after:
- Replacing struts/shocks (sometimes)
- Tie rod ends (always)
- Control arms (always)
- Ball joints (always)
- Lowering/lifting

Do not skip this. A bad alignment destroys tires fast and hurts handling.', 22);

-- Sample stores
INSERT OR IGNORE INTO stores (name, url, category, submitted_by) VALUES 
('RockAuto', 'https://www.rockauto.com', 'Aftermarket Parts', 2),
('FCP Euro', 'https://www.fcpeuro.com', 'OEM Parts', 3),
('Amazon Automotive', 'https://www.amazon.com/automotive', 'General', 4),
('Harbor Freight', 'https://www.harborfreight.com', 'Tools', 2),
('Advance Auto Parts', 'https://www.advanceautoparts.com', 'Aftermarket Parts', 3);

-- Store votes
INSERT OR IGNORE INTO store_votes (store_id, user_id, positive) VALUES 
(1, 2, 1), (1, 3, 1), (1, 4, 1),  -- RockAuto: 3 positive
(2, 2, 1), (2, 3, 1), (2, 4, 1),  -- FCP Euro: 3 positive
(3, 2, 0), (3, 3, 1), (3, 4, 0),  -- Amazon: mixed
(4, 2, 1), (4, 3, 0), (4, 4, 1),  -- Harbor Freight: 2/3
(5, 2, 1), (5, 3, 1);             -- Advance: 2 positive

-- Auto-upvotes for posts
INSERT OR IGNORE INTO votes (user_id, post_id, value) VALUES
(2, 1, 1), (3, 2, 1), (4, 3, 1), (2, 4, 1), (3, 5, 1),
(4, 6, 1), (2, 7, 1), (3, 8, 1), (4, 9, 1), (2, 10, 1);

-- Additional votes
INSERT OR IGNORE INTO votes (user_id, post_id, value) VALUES
(3, 1, 1), (4, 1, 1), (2, 2, 1), (4, 2, 1), (2, 3, 1), (3, 3, 1);

EOF

echo "Seed complete!"
echo ""
echo "Created:"
echo "  - Admin user: admin@wrench.forum (Note: password hash is placeholder)"
echo "  - 3 verified mechanics"
echo "  - 5 categories"
echo "  - 10 sample posts"
echo "  - 5 sample stores with votes"
echo ""
echo "To login, register a new account through the UI (the seed users have placeholder hashes)."
