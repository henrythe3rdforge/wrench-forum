use wrench_forum::auth;

#[test]
fn test_password_hashing() {
    let password = "my_secure_password_123";
    let hash = auth::hash_password(password).expect("Failed to hash password");
    
    // Hash should not equal the original password
    assert_ne!(hash, password);
    
    // Hash should be verifiable
    assert!(auth::verify_password(password, &hash));
    
    // Wrong password should not verify
    assert!(!auth::verify_password("wrong_password", &hash));
}

#[test]
fn test_password_hashing_different_salts() {
    let password = "same_password";
    
    let hash1 = auth::hash_password(password).unwrap();
    let hash2 = auth::hash_password(password).unwrap();
    
    // Same password should produce different hashes (different salts)
    assert_ne!(hash1, hash2);
    
    // But both should verify correctly
    assert!(auth::verify_password(password, &hash1));
    assert!(auth::verify_password(password, &hash2));
}

#[test]
fn test_session_token_generation() {
    let token1 = auth::create_session_token();
    let token2 = auth::create_session_token();
    
    // Tokens should be unique
    assert_ne!(token1, token2);
    
    // Token should be a valid UUID (36 chars with dashes)
    assert_eq!(token1.len(), 36);
    assert!(token1.contains('-'));
}

#[test]
fn test_session_expiry() {
    let expiry = auth::session_expiry();
    
    // Should be a valid datetime string
    assert!(expiry.len() > 0);
    assert!(expiry.contains("-")); // Date format
    assert!(expiry.contains(":")); // Time format
}

#[test]
fn test_email_validation() {
    // Valid emails
    assert!(auth::is_valid_email("test@example.com"));
    assert!(auth::is_valid_email("user.name@sub.domain.org"));
    assert!(auth::is_valid_email("user+tag@example.co"));
    assert!(auth::is_valid_email("a@b.c"));
    
    // Invalid emails
    assert!(!auth::is_valid_email(""));
    assert!(!auth::is_valid_email("invalid"));
    assert!(!auth::is_valid_email("@example.com"));
    assert!(!auth::is_valid_email("test@"));
    assert!(!auth::is_valid_email("test@nodot"));
    assert!(!auth::is_valid_email("test@@example.com"));
    assert!(!auth::is_valid_email("test @example.com"));
}

#[test]
fn test_username_validation() {
    // Valid usernames
    assert!(auth::is_valid_username("john_doe"));
    assert!(auth::is_valid_username("User123"));
    assert!(auth::is_valid_username("abc"));
    assert!(auth::is_valid_username("a_b_c"));
    assert!(auth::is_valid_username("12345"));
    assert!(auth::is_valid_username("A".repeat(20).as_str()));
    
    // Invalid usernames
    assert!(!auth::is_valid_username("ab")); // Too short
    assert!(!auth::is_valid_username("A".repeat(21).as_str())); // Too long
    assert!(!auth::is_valid_username("user@name")); // Invalid char
    assert!(!auth::is_valid_username("user name")); // Spaces
    assert!(!auth::is_valid_username("user-name")); // Dash not allowed
    assert!(!auth::is_valid_username("")); // Empty
}

#[test]
fn test_password_validation() {
    // Valid passwords
    assert!(auth::is_valid_password("password123"));
    assert!(auth::is_valid_password("12345678"));
    assert!(auth::is_valid_password("abcdefgh"));
    assert!(auth::is_valid_password("a".repeat(100).as_str())); // Long is fine
    
    // Invalid passwords
    assert!(!auth::is_valid_password("short")); // Too short
    assert!(!auth::is_valid_password("1234567")); // 7 chars
    assert!(!auth::is_valid_password("")); // Empty
}

#[test]
fn test_verify_password_invalid_hash() {
    // Should handle invalid hash gracefully
    assert!(!auth::verify_password("password", "not_a_valid_hash"));
    assert!(!auth::verify_password("password", ""));
}
