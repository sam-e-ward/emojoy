use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageStats {
    pub counts: HashMap<String, u32>,
}

impl UsageStats {
    fn path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("emojoy");
        config_dir.join("usage.json")
    }

    pub fn load() -> Self {
        let path = Self::path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    pub fn record(&mut self, emoji: &str) {
        *self.counts.entry(emoji.to_string()).or_insert(0) += 1;
    }

    /// Returns a frequency boost for scoring. Higher usage = larger boost (subtracted from score).
    /// Capped so it influences but doesn't completely dominate tag relevance.
    pub fn boost(&self, emoji: &str) -> u32 {
        let count = self.counts.get(emoji).copied().unwrap_or(0);
        // Diminishing returns: boost = min(count, 50)
        // This means a frequently used emoji gets up to 50 points subtracted from its score.
        count.min(50)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_boost() {
        let mut stats = UsageStats::default();
        assert_eq!(stats.boost("🔥"), 0);

        stats.record("🔥");
        stats.record("🔥");
        stats.record("🔥");
        assert_eq!(stats.boost("🔥"), 3);
    }

    #[test]
    fn test_boost_caps_at_50() {
        let mut stats = UsageStats::default();
        for _ in 0..100 {
            stats.record("😂");
        }
        assert_eq!(stats.boost("😂"), 50);
    }

    #[test]
    fn test_unknown_emoji_zero_boost() {
        let stats = UsageStats::default();
        assert_eq!(stats.boost("🤷"), 0);
    }
}
