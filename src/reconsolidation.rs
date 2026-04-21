/// reconsolidation — 记忆再巩固（Reconsolidation）
///
/// 每次回忆记忆时，记忆被"重新打开"、修改、重新存储。
/// 防抖机制：同一印迹短时间内不重复巩固。

use std::collections::{HashMap, HashSet};

use crate::store::EngramStore;

pub struct Reconsolidation {
    store: EngramStore,
    recently_consolidated: HashMap<String, u64>,
}

pub struct ReconsolidationResult {
    pub action: String,
    pub engram_id_prefix: String,
    pub changes: Option<ReconsolidationChanges>,
}

pub struct ReconsolidationChanges {
    pub importance: Option<ImportanceChange>,
    pub tags_added: Vec<String>,
}

pub struct ImportanceChange {
    pub old: u32,
    pub new: u32,
}

const STOP_WORDS: &[&str] = &[
    "这个","那个","一个","什么","怎么","可以","没有","不是",
    "我们","他们","如果","但是","因为","所以","或者","虽然",
    // keep local list for CJK bigram-specific filtering in tag discovery
];

impl Reconsolidation {
    pub fn new(store: EngramStore) -> Self {
        Self {
            store,
            recently_consolidated: HashMap::new(),
        }
    }

    /// 当一条记忆被检索命中时触发再巩固
    pub fn on_recall(&mut self, engram_id: &str, recall_context: Option<&str>) -> ReconsolidationResult {
        let engram = match self.store.get_by_id(engram_id) {
            Ok(Some(e)) => e,
            _ => return ReconsolidationResult {
                action: "skipped".into(),
                engram_id_prefix: engram_id[..8.min(engram_id.len())].into(),
                changes: None,
            },
        };

        let now = now_ts();
        if let Some(&last) = self.recently_consolidated.get(engram_id) {
            if now - last < 3600 {
                return ReconsolidationResult {
                    action: "skipped".into(),
                    engram_id_prefix: engram_id[..8.min(engram_id.len())].into(),
                    changes: None,
                };
            }
        }
        self.recently_consolidated.insert(engram_id.into(), now);

        let mut changes = ReconsolidationChanges {
            importance: None,
            tags_added: vec![],
        };

        if let Some(ctx) = recall_context {
            let new_imp = self.reevaluate_importance(&engram, ctx);
            if new_imp != engram.importance {
                let old = engram.importance;
                let eid = engram_id.to_string();
                let _ = self.store.update(&eid, |e| { e.importance = new_imp; });
                changes.importance = Some(ImportanceChange { old, new: new_imp });
            }

            let new_tags = self.discover_new_tags(&engram, ctx);
            if !new_tags.is_empty() {
                let mut merged: Vec<String> = engram.tags.clone();
                for t in &new_tags {
                    if !merged.contains(t) {
                        merged.push(t.clone());
                    }
                }
                let eid = engram_id.to_string();
                let tags = merged.clone();
                let _ = self.store.update(&eid, move |e| { e.tags = tags; });
                changes.tags_added = new_tags;
            }
        }

        let action = if changes.importance.is_some() || !changes.tags_added.is_empty() {
            "reconsolidated"
        } else {
            "confirmed"
        };

        ReconsolidationResult {
            action: action.into(),
            engram_id_prefix: engram_id[..8.min(engram_id.len())].into(),
            changes: if action == "confirmed" { None } else { Some(changes) },
        }
    }

    /// 批量再巩固：遍历近期记忆，模拟睡眠重播
    pub fn batch_consolidate(&mut self, _days: i64) -> (usize, usize) {
        let mut consolidated = 0usize;

        if let Ok(engrams) = self.store.read_layer("L1") {
            // Filter by days
            for e in &engrams {
                let _ = self.on_recall(&e.id, Some(&e.content));
                consolidated += 1;
            }
        }

        (consolidated, self.recently_consolidated.len())
    }

    fn reevaluate_importance(&self, engram: &crate::engram::Engram, _context: &str) -> u32 {
        engram.importance
    }

    fn discover_new_tags(&self, engram: &crate::engram::Engram, context: &str) -> Vec<String> {
        let context_words: HashSet<String> = extract_cjk_words(context, 2, 4)
            .into_iter().collect();
        let engram_words: HashSet<String> = extract_cjk_words(&engram.content, 2, 4)
            .into_iter().collect();

        context_words.difference(&engram_words)
            .filter(|w| !STOP_WORDS.contains(&w.as_str()))
            .take(2)
            .cloned()
            .collect()
    }
}

fn extract_cjk_words(text: &str, min_len: usize, max_len: usize) -> Vec<String> {
    let cjk: Vec<char> = text.chars().filter(|c| ('\u{4e00}'..='\u{9fff}').contains(c)).collect();
    let mut words = vec![];
    if cjk.is_empty() { return words; }
    for len in min_len..=max_len {
        if len > cjk.len() { continue; }
        for i in 0..=cjk.len() - len {
            let s: String = cjk[i..i + len].iter().collect();
            words.push(s);
        }
    }
    words
}

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
