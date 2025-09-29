use once_cell::sync::Lazy;
use std::collections::HashSet;

static WORD_BLOCKLIST: Lazy<HashSet<String>> = Lazy::new(|| {
    let word_blocklist = std::env::var("WORD_BLOCKLIST").unwrap_or(String::from("[]"));
    let words: Vec<String> = serde_json::from_str(&word_blocklist).unwrap_or_default();
    words.into_iter().map(|w| w.to_lowercase()).collect()
});

pub fn has_blocked_words(str: &str) -> bool {
    if WORD_BLOCKLIST.is_empty() {
        return false;
    }

    let json_lower = str.to_lowercase();
    WORD_BLOCKLIST.iter().any(|word| json_lower.contains(word))
}
