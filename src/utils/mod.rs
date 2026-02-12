use sha2::{Digest, Sha256};

/// Computes a SHA-256 hash of the input string.
///
/// # Arguments
/// * `input` - The string to hash
///
/// # Returns
/// A lowercase hexadecimal string representation of the hash
#[allow(dead_code)]
pub fn hash_string(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Truncates a string to a maximum length, appending "..." if truncated.
///
/// This function is UTF-8 safe and will not panic on multi-byte characters.
///
/// # Arguments
/// * `s` - The string to truncate
/// * `max_len` - Maximum length in characters (not bytes)
///
/// # Returns
/// The original string if shorter than max_len, otherwise truncated with "..."
#[allow(dead_code)]
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if max_len < 4 {
        // Not enough room for "..." plus at least one character
        return s.chars().take(max_len).collect();
    }

    let char_count = s.chars().count();
    if char_count <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_string_short() {
        assert_eq!(truncate_string("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_string_exact() {
        assert_eq!(truncate_string("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_string_long() {
        assert_eq!(truncate_string("hello world", 8), "hello...");
    }

    #[test]
    fn test_truncate_string_unicode() {
        // Test with multi-byte UTF-8 characters (Japanese)
        let japanese = "こんにちは世界"; // 7 characters
        assert_eq!(truncate_string(japanese, 10), japanese);
        assert_eq!(truncate_string(japanese, 6), "こんに...");
    }

    #[test]
    fn test_truncate_string_emoji() {
        let emoji = "🦀🔥💻"; // 3 characters (each is 4 bytes)
        assert_eq!(truncate_string(emoji, 5), emoji);
        assert_eq!(truncate_string(emoji, 2), "🦀🔥"); // Too short for "..."
    }

    #[test]
    fn test_truncate_string_min_length() {
        assert_eq!(truncate_string("hello", 3), "hel");
        assert_eq!(truncate_string("hello", 1), "h");
    }

    #[test]
    fn test_hash_string() {
        let hash = hash_string("test");
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex characters
    }
}
