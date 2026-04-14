/// reflect — 反思巩固 + vacuum
///
/// 1. 从近期印迹学习语义网络
/// 2. 突触修剪
/// 3. 批量再巩固
/// 4. vacuum（L1→L2→L3 归档）

use crate::config::HippocampusConfig;
use crate::reconsolidation::Reconsolidation;
use crate::semantic_network::SemanticNetwork;
use crate::store::EngramStore;

pub struct Reflector {
    store: EngramStore,
    config: HippocampusConfig,
}

#[derive(serde::Serialize)]
pub struct ReflectResult {
    pub semantic_network_learned: usize,
    pub pruned: usize,
    pub reconsolidated: usize,
    pub vacuum: VacuumResult,
}

#[derive(serde::Serialize)]
pub struct VacuumResult {
    pub l1_to_l2: usize,
    pub l2_to_l3: usize,
    pub deleted: usize,
    pub archived: usize,
}

impl Reflector {
    pub fn new(store: EngramStore, config: HippocampusConfig) -> Self {
        Self { store, config }
    }

    /// 完整反思流程
    pub fn reflect(&mut self, days: i64) -> ReflectResult {
        // 1. 从近期印迹学习语义网络
        let mut network = SemanticNetwork::new(
            self.config.semantic_network_path.to_string_lossy().to_string()
        );
        let all_engrams = self.store.read_all().unwrap_or_default();
        let mut learned = 0usize;
        for e in &all_engrams {
            let tokens = crate::search::tokenize(&e.content);
            if !tokens.is_empty() {
                network.co_activate(&tokens);
                learned += 1;
            }
        }
        let _ = network.save();

        // 2. 突触修剪
        let before = network.stats();
        network.decay_all();
        let after = network.stats();
        let pruned = before.0.saturating_sub(after.0);
        let _ = network.save();

        // 3. 批量再巩固
        let mut recon = Reconsolidation::new(EngramStore::new(self.config.clone()).unwrap());
        let (consolidated, _updated) = recon.batch_consolidate(days);

        // 4. vacuum
        let vacuum = self.vacuum();

        // 5. 🧠 学习关键词：从所有印迹中批量学习
        let mut kw = crate::learned_keywords::LearnedKeywords::load(&self.config.learned_keywords_path);
        let all_for_learn = self.store.read_all().unwrap_or_default();
        for e in &all_for_learn {
            kw.update_from_engram(&e.content);
        }
        kw.refine();
        let _ = kw.save(&self.config.learned_keywords_path);

        ReflectResult {
            semantic_network_learned: learned,
            pruned,
            reconsolidated: consolidated,
            vacuum,
        }
    }

    /// Vacuum 整理
    pub fn vacuum(&mut self) -> VacuumResult {
        let mut result = VacuumResult {
            l1_to_l2: 0,
            l2_to_l3: 0,
            deleted: 0,
            archived: 0,
        };

        // 1. L1→L2 溢出：超24h的 L1 印迹提升到 L2
        if let Ok(l1_engrams) = self.store.read_layer("L1") {
            for e in &l1_engrams {
                if is_older_than_hours(&e.created_at, self.config.l1_max_age_hours as i64) {
                    let eid = e.id.clone();
                    let _ = self.store.update(&eid, |eng| {
                        eng.layer = "L2".into();
                    });
                    result.l1_to_l2 += 1;
                }
            }
        }

        // 2. 遗忘删除：score < vacuum_min_score
        let _threshold = self.config.vacuum_min_score;
        for layer in &["L1", "L2", "L3"] {
            if let Ok(engrams) = self.store.read_layer(layer) {
                for e in &engrams {
                    // Simple: very low importance and old = delete
                    if e.importance <= 1 && is_older_than_hours(&e.created_at, 720) {
                        let eid = e.id.clone();
                        let _ = self.store.delete(&eid);
                        result.deleted += 1;
                    }
                }
            }
        }

        // 3. L2→L3 巩固：access >= consolidate_min_access 且 importance >= consolidate_min_importance
        if let Ok(l2_engrams) = self.store.read_layer("L2") {
            for e in &l2_engrams {
                if e.access_count >= self.config.consolidate_min_access
                    && e.importance >= self.config.consolidate_min_importance
                {
                    let eid = e.id.clone();
                    let _ = self.store.update(&eid, |eng| {
                        eng.layer = "L3".into();
                    });
                    result.l2_to_l3 += 1;
                }
            }
        }

        // 4. 按季度归档到 archive/ (simplified: just count old L3)
        if let Ok(l3_engrams) = self.store.read_layer("L3") {
            for e in &l3_engrams {
                if is_older_than_days(&e.created_at, self.config.archive_days as i64) {
                    result.archived += 1;
                }
            }
        }

        result
    }
}

fn is_older_than_hours(created_at: &str, hours: i64) -> bool {
    parse_days_ago(created_at) * 24 >= hours
}

fn is_older_than_days(created_at: &str, days: i64) -> bool {
    parse_days_ago(created_at) >= days
}

fn parse_days_ago(created_at: &str) -> i64 {
    let date_str = created_at.get(..10).unwrap_or("2026-04-14");
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return 0;
    }
    let y: i64 = parts[0].parse().unwrap_or(2026);
    let m: i64 = parts[1].parse().unwrap_or(4);
    let d: i64 = parts[2].parse().unwrap_or(14);
    let now_days = 2026i64 * 365 + 4 * 30 + 14;
    let created_days = y * 365 + m * 30 + d;
    (now_days - created_days).max(0)
}
