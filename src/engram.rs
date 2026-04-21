use serde::{Serialize, Deserialize};
use crate::scoring::half_life_for_importance;

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
    #[serde(default = "default_access_history")]
    pub access_history: Vec<String>,
    #[serde(default)]
    pub fingerprint: u64,
}

fn default_access_history() -> Vec<String> {
    vec![]
}

impl Engram {
    pub fn new(content: impl Into<String>, importance: u32) -> Self {
        let content = content.into();
        let importance = importance.clamp(1, 10);
        let half_life = half_life_for_importance(importance as i32) as u64;
        let now = chrono_now_iso();
        let fingerprint = crate::simhash::simhash(&content);
        Self {
            id: crate::util::uuid_hex(),
            content,
            importance,
            emotion: "neutral".into(),
            emotion_score: 0.0,
            source: "manual".into(),
            tags: vec![],
            access_count: 0,
            created_at: now.clone(),
            accessed_at: None,
            layer: "L1".into(),
            half_life,
            session_id: None,
            access_history: vec![now],
            fingerprint,
        }
    }

    pub fn apply_emotion_boost(&mut self) {
        if self.emotion_score >= 0.7 {
            self.half_life = (self.half_life as f64 * 1.5) as u64;
        }
    }

    pub fn record_access(&mut self) {
        let now = chrono_now_iso();
        self.accessed_at = Some(now.clone());
        self.access_count += 1;
        self.access_history.push(now);
        // Cap at 50 entries to prevent unbounded growth
        if self.access_history.len() > 50 {
            self.access_history.drain(..self.access_history.len() - 50);
        }
    }
}

pub fn chrono_now_iso() -> String {
    chrono::Local::now().to_rfc3339()
}

pub fn parse_iso_date(iso: &str) -> Option<chrono::NaiveDate> {
    iso.get(..10)?.parse().ok()
}
