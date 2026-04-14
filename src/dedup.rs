/// dedup — 记忆去重合并（模式分离）
///
/// 基于 CJK 2-gram Jaccard 相似度对印迹去重。

use std::collections::HashSet;

use crate::store::EngramStore;

pub struct Deduplicator<'a> {
    store: &'a EngramStore,
}

#[derive(serde::Serialize)]
pub struct DuplicatePair {
    pub id_a: String,
    pub id_b: String,
    pub similarity: f64,
}

impl<'a> Deduplicator<'a> {
    pub fn new(store: &'a EngramStore) -> Self {
        Self { store }
    }

    /// CJK 2-gram Jaccard 相似度
    pub fn jaccard_similarity(a: &str, b: &str) -> f64 {
        let ng_a = cjk_2grams(a);
        let ng_b = cjk_2grams(b);
        if ng_a.is_empty() || ng_b.is_empty() {
            return 0.0;
        }
        let intersection = ng_a.intersection(&ng_b).count();
        let union = ng_a.union(&ng_b).count();
        intersection as f64 / union as f64
    }

    /// 在指定 layer 内查找相似对
    pub fn find_duplicates(&self, threshold: f64, layer: &str) -> Vec<DuplicatePair> {
        let engrams = match self.store.read_layer(layer) {
            Ok(e) => e,
            Err(_) => return vec![],
        };
        let mut pairs = vec![];
        for i in 0..engrams.len() {
            for j in (i + 1)..engrams.len() {
                let sim = Self::jaccard_similarity(&engrams[i].content, &engrams[j].content);
                if sim >= threshold {
                    pairs.push(DuplicatePair {
                        id_a: engrams[i].id.clone(),
                        id_b: engrams[j].id.clone(),
                        similarity: (sim * 1000.0).round() / 1000.0,
                    });
                }
            }
        }
        pairs
    }

    /// 合并两条记忆（保留 importance 更高的，tags 合并）
    pub fn merge(&self, id_a: &str, id_b: &str, dry_run: bool) -> Result<String, String> {
        let ea = self.store.get_by_id(id_a).map_err(|e| e.to_string())?
            .ok_or_else(|| "id_a not found".to_string())?;
        let eb = self.store.get_by_id(id_b).map_err(|e| e.to_string())?
            .ok_or_else(|| "id_b not found".to_string())?;

        // Keep higher importance
        let (kept, removed) = if ea.importance >= eb.importance {
            (ea, eb)
        } else {
            (eb, ea)
        };

        if dry_run {
            return Ok(format!("dry-run: would merge {} into {}", removed.id[..8.min(removed.id.len())].to_string(), kept.id[..8.min(kept.id.len())].to_string()));
        }

        // Merge tags
        let mut merged_tags: Vec<String> = kept.tags.clone();
        for t in &removed.tags {
            if !merged_tags.contains(t) {
                merged_tags.push(t.clone());
            }
        }

        // Update kept engram
        let kept_id = kept.id.clone();
        let kept_ac = kept.access_count + removed.access_count;
        let tags = merged_tags;
        self.store.update(&kept_id, move |e| {
            e.tags = tags;
            e.access_count = kept_ac;
        }).map_err(|e| e.to_string())?;

        // Delete removed engram
        let removed_id = removed.id;
        self.store.delete(&removed_id).map_err(|e| e.to_string())?;

        Ok(format!("merged {} into {}", removed_id[..8.min(removed_id.len())].to_string(), kept_id[..8.min(kept_id.len())].to_string()))
    }
}

fn cjk_2grams(text: &str) -> HashSet<String> {
    let chars: Vec<char> = text.chars().filter(|c| ('\u{4e00}'..='\u{9fff}').contains(c)).collect();
    let mut set = HashSet::new();
    for i in 0..chars.len().saturating_sub(1) {
        set.insert(chars[i..=i + 1].iter().collect());
    }
    set
}
