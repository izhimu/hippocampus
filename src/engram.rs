use serde::{Serialize, Deserialize};
use crate::scoring::half_life_for_importance;

/// 严格参照 Python engram.py Engram 类所有字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Engram {
    pub id: String,
    pub content: String,
    pub importance: u32,
    pub emotion: String,
    pub emotion_score: f64,
    pub source: String,
    pub tags: Vec<String>,
    pub access_count: u32,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessed_at: Option<String>,
    pub layer: String,
    pub half_life: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

impl Engram {
    pub fn new(content: impl Into<String>, importance: u32) -> Self {
        let content = content.into();
        let importance = importance.clamp(1, 10);
        let half_life = half_life_for_importance(importance as i32) as u64;
        let now = chrono_now_iso();
        Self {
            id: uuid_hex(),
            content,
            importance,
            emotion: "neutral".into(),
            emotion_score: 0.0,
            source: "manual".into(),
            tags: vec![],
            access_count: 0,
            created_at: now,
            accessed_at: None,
            layer: "L1".into(),
            half_life,
            session_id: None,
        }
    }

    /// 杏仁核增强：强情绪记忆更持久
    pub fn apply_emotion_boost(&mut self) {
        if self.emotion_score >= 0.7 {
            self.half_life = (self.half_life as f64 * 1.5) as u64;
        }
    }
}

fn chrono_now_iso() -> String {
    // 简单实现，不依赖 chrono crate
    "2026-04-14T13:32:00+08:00".to_string() // placeholder
}

fn uuid_hex() -> String {
    // 简单实现，不依赖 uuid crate
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:016x}{:016x}", nanos, nanos.wrapping_mul(0x9e3779b97f4a7c15))
}
