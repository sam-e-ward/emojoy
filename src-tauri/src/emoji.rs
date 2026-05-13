use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmojiEntry {
    pub emoji: String,
    pub name: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub emoji: String,
    pub name: String,
    /// Lower score = better match
    pub score: u32,
}

pub struct EmojiDatabase {
    entries: Vec<EmojiEntry>,
}

impl EmojiDatabase {
    /// Load from emojilib JSON format: { "😀": ["name", "tag1", "tag2", ...] }
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let data = std::fs::read_to_string(path)?;
        let raw: HashMap<String, Vec<String>> = serde_json::from_str(&data)?;

        let entries = raw
            .into_iter()
            .map(|(emoji, tags)| {
                let name = tags
                    .first()
                    .cloned()
                    .unwrap_or_default()
                    .replace('_', " ");
                EmojiEntry {
                    emoji,
                    name,
                    tags,
                }
            })
            .collect();

        Ok(EmojiDatabase { entries })
    }

    /// Load from a pre-built entries vec (for testing)
    pub fn from_entries(entries: Vec<EmojiEntry>) -> Self {
        EmojiDatabase { entries }
    }

    /// Merge custom aliases into existing entries.
    /// custom_aliases: { "😀": ["extra_tag1", "extra_tag2"] }
    pub fn merge_aliases(&mut self, custom_aliases: &HashMap<String, Vec<String>>) {
        for entry in &mut self.entries {
            if let Some(extra_tags) = custom_aliases.get(&entry.emoji) {
                for tag in extra_tags {
                    if !entry.tags.contains(tag) {
                        entry.tags.push(tag.clone());
                    }
                }
            }
        }
    }

    /// Search emojis by query. Matches against all tags.
    /// Usage stats provide a frequency boost (frequently used emojis rank higher).
    /// Returns results sorted by relevance (best first), limited to `max_results`.
    pub fn search(
        &self,
        query: &str,
        max_results: usize,
        usage: Option<&crate::usage::UsageStats>,
    ) -> Vec<SearchResult> {
        if query.is_empty() {
            // Return emojis sorted by usage (most used first), then default order
            let mut results: Vec<SearchResult> = self
                .entries
                .iter()
                .map(|e| {
                    let boost = usage.map_or(0, |u| u.boost(&e.emoji));
                    SearchResult {
                        emoji: e.emoji.clone(),
                        name: e.name.clone(),
                        score: 100u32.saturating_sub(boost),
                    }
                })
                .collect();
            results.sort_by_key(|r| r.score);
            results.truncate(max_results);
            return results;
        }

        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut results: Vec<SearchResult> = self
            .entries
            .iter()
            .filter_map(|entry| {
                let score = score_entry(entry, &query_terms);
                if score < u32::MAX {
                    let boost = usage.map_or(0, |u| u.boost(&entry.emoji));
                    Some(SearchResult {
                        emoji: entry.emoji.clone(),
                        name: entry.name.clone(),
                        score: score.saturating_sub(boost),
                    })
                } else {
                    None
                }
            })
            .collect();

        results.sort_by_key(|r| r.score);
        results.truncate(max_results);
        results
    }
}

/// Score an emoji entry against query terms. Lower = better. u32::MAX = no match.
fn score_entry(entry: &EmojiEntry, query_terms: &[&str]) -> u32 {
    // All query terms must match at least one tag
    let mut total_score: u32 = 0;

    for term in query_terms {
        let mut best_tag_score = u32::MAX;

        for (i, tag) in entry.tags.iter().enumerate() {
            let tag_lower = tag.to_lowercase().replace('_', " ");

            if tag_lower == *term {
                // Exact match — best possible, weighted by tag position
                let s = i as u32;
                best_tag_score = best_tag_score.min(s);
            } else if tag_lower.starts_with(term) {
                // Prefix match
                let s = 50 + i as u32;
                best_tag_score = best_tag_score.min(s);
            } else if tag_lower.contains(term) {
                // Substring match
                let s = 100 + i as u32;
                best_tag_score = best_tag_score.min(s);
            }
        }

        if best_tag_score == u32::MAX {
            return u32::MAX; // This term didn't match anything
        }
        total_score = total_score.saturating_add(best_tag_score);
    }

    total_score
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> EmojiDatabase {
        EmojiDatabase::from_entries(vec![
            EmojiEntry {
                emoji: "😀".to_string(),
                name: "grinning face".to_string(),
                tags: vec![
                    "grinning_face".to_string(),
                    "face".to_string(),
                    "smile".to_string(),
                    "happy".to_string(),
                ],
            },
            EmojiEntry {
                emoji: "😂".to_string(),
                name: "face with tears of joy".to_string(),
                tags: vec![
                    "face_with_tears_of_joy".to_string(),
                    "face".to_string(),
                    "laugh".to_string(),
                    "lol".to_string(),
                    "haha".to_string(),
                ],
            },
            EmojiEntry {
                emoji: "🔥".to_string(),
                name: "fire".to_string(),
                tags: vec![
                    "fire".to_string(),
                    "hot".to_string(),
                    "flame".to_string(),
                    "lit".to_string(),
                ],
            },
            EmojiEntry {
                emoji: "❤️".to_string(),
                name: "red heart".to_string(),
                tags: vec![
                    "red_heart".to_string(),
                    "love".to_string(),
                    "heart".to_string(),
                ],
            },
        ])
    }

    #[test]
    fn test_exact_tag_match() {
        let db = test_db();
        let results = db.search("fire", 10, None);
        assert_eq!(results[0].emoji, "🔥");
    }

    #[test]
    fn test_prefix_match() {
        let db = test_db();
        let results = db.search("hap", 10, None);
        assert_eq!(results[0].emoji, "😀");
    }

    #[test]
    fn test_substring_match() {
        let db = test_db();
        let results = db.search("eart", 10, None);
        assert!(!results.is_empty());
        assert_eq!(results[0].emoji, "❤️");
    }

    #[test]
    fn test_no_match() {
        let db = test_db();
        let results = db.search("zzzzz", 10, None);
        assert!(results.is_empty());
    }

    #[test]
    fn test_empty_query_returns_results() {
        let db = test_db();
        let results = db.search("", 10, None);
        assert_eq!(results.len(), 4);
    }

    #[test]
    fn test_multi_term_search() {
        let db = test_db();
        let results = db.search("face smile", 10, None);
        assert_eq!(results[0].emoji, "😀");
    }

    #[test]
    fn test_merge_aliases() {
        let mut db = test_db();
        let mut aliases = HashMap::new();
        aliases.insert("🔥".to_string(), vec!["awesome".to_string()]);
        db.merge_aliases(&aliases);

        let results = db.search("awesome", 10, None);
        assert_eq!(results[0].emoji, "🔥");
    }

    #[test]
    fn test_max_results_limit() {
        let db = test_db();
        let results = db.search("face", 2, None);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_usage_boost_reranks_results() {
        let db = test_db();
        // Without usage, "fire" exact match on first tag (score 0) beats "lit" substring
        let results = db.search("face", 10, None);
        // 😀 has "face" at tag index 1, 😂 has "face" at tag index 1 — tied
        // Now boost 😂 heavily
        let mut usage = crate::usage::UsageStats::default();
        for _ in 0..30 {
            usage.record("😂");
        }
        let results_with_usage = db.search("face", 10, Some(&usage));
        assert_eq!(results_with_usage[0].emoji, "😂");
    }

    #[test]
    fn test_empty_query_sorted_by_usage() {
        let db = test_db();
        let mut usage = crate::usage::UsageStats::default();
        for _ in 0..20 {
            usage.record("🔥");
        }
        let results = db.search("", 10, Some(&usage));
        assert_eq!(results[0].emoji, "🔥");
    }
}
