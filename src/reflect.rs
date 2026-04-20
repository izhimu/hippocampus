/// reflect — 反思巩固 + vacuum
///
/// 1. 从近期印迹学习语义网络
/// 2. 突触修剪
/// 3. 批量再巩固
/// 4. vacuum（L1→L2→L3 归档）

use crate::config::HippocampusConfig;
use crate::cognitive_map::CognitiveMap;
use crate::conflict::ConflictResolver;
use crate::engram::Engram;
use crate::reconsolidation::Reconsolidation;
use crate::semantic_network::SemanticNetwork;
use crate::store::EngramStore;

pub struct Reflector {
    store: EngramStore,
    config: HippocampusConfig,
}

/// 🧠 语义化接口 (Scheme 4): 允许将零散记忆抽象为知识
pub trait Semanticizer {
    fn summarize(&self, contents: &[String]) -> Option<String>;
}

/// 基础语义化实现（暂无 LLM 时使用）
pub struct PlaceholderSemanticizer;
impl Semanticizer for PlaceholderSemanticizer {
    fn summarize(&self, contents: &[String]) -> Option<String> {
        if contents.len() < 3 { return None; }
        // 简单逻辑：如果没有 LLM，只在反思日志记录合并动作，不执行实际合并
        None
    }
}

#[derive(serde::Serialize)]
pub struct ReflectResult {
    pub semantic_network_learned: usize,
    pub pruned: usize,
    pub reconsolidated: usize,
    pub vacuum: VacuumResult,
    pub semanticized_count: usize,
    pub cognitive_map_nodes: usize,
    pub conflicts: crate::conflict::ConflictReport,
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

        // 4. SleepGate conflict resolution (before vacuum)
        let resolver = ConflictResolver::new(8);
        let conflicts = resolver.resolve(&self.store);

        // 5. vacuum
        let vacuum = self.vacuum();

        // 6. 语义化抽象
        let semanticized_count = self.semanticize(&PlaceholderSemanticizer);

        // 8. 学习关键词
        let mut kw = crate::learned_keywords::LearnedKeywords::load(&self.config.learned_keywords_path);
        let all_for_learn = self.store.read_all().unwrap_or_default();
        for e in &all_for_learn {
            kw.update_from_engram(&e.content);
        }
        kw.refine();
        let _ = kw.save(&self.config.learned_keywords_path);

        // 9. 认知地图：从同一 session 的连续 engram 学习 SR 关系
        let cog_map_path = self.config.cognitive_dir.join("cognitive_map.json");
        let mut cog_map = CognitiveMap::load(&cog_map_path, 5000).unwrap_or_else(|_| CognitiveMap::new(5000));
        let mut session_engrams: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for e in &all_for_learn {
            if let Some(ref sid) = e.session_id {
                session_engrams.entry(sid.clone()).or_default().push(e.id.clone());
            }
        }
        for (_sid, ids) in &session_engrams {
            for window in ids.windows(2) {
                cog_map.td_update(&window[0], &window[1]);
            }
        }
        cog_map.prune(0.01);
        let _ = cog_map.save(&cog_map_path);

        // 10. 生成式回放：随机游走巩固记忆链路
        self.generative_replay(&mut cog_map, &all_for_learn);
        let _ = cog_map.save(&cog_map_path);

        ReflectResult {
            semantic_network_learned: learned,
            pruned,
            reconsolidated: consolidated,
            vacuum,
            semanticized_count,
            cognitive_map_nodes: cog_map.len(),
            conflicts,
        }
    }

    /// 🧠 语义化抽象逻辑
    fn semanticize<S: Semanticizer>(&self, engine: &S) -> usize {
        let mut count = 0;
        if let Ok(l1) = self.store.read_layer("L1") {
            if l1.len() < 5 { return 0; }
            
            // 按情境标签分组 (简单模拟话题聚类)
            let mut groups: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
            for e in l1 {
                for tag in e.tags {
                    if tag.starts_with("ctx:") {
                        groups.entry(tag).or_default().push(e.content.clone());
                    }
                }
            }
            
            for (_ctx, contents) in groups {
                if contents.len() >= 3 {
                    if let Some(_summary) = engine.summarize(&contents) {
                        // 未来实现：
                        // 1. 创建新的 L3 语义记忆 (summary)
                        // 2. 删除原始 L1 记忆
                        count += 1;
                    }
                }
            }
        }
        count
    }

    /// 生成式回放：从高激活节点出发，随机游走巩固 SR 链路
    fn generative_replay(&self, cog_map: &mut CognitiveMap, engrams: &[Engram]) {
        if cog_map.is_empty() || engrams.is_empty() {
            return;
        }

        // Find top-N engrams by ACT-R activation as starting points
        let decay_rate = self.config.actr_decay_rate;
        let mut scored: Vec<(String, f64)> = engrams.iter()
            .filter_map(|e| {
                if cog_map.get_related(&e.id, 1).is_empty() {
                    return None;
                }
                let activation = crate::scoring::actr_decay_factor(&e.access_history, decay_rate);
                Some((e.id.clone(), activation))
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let start_nodes: Vec<&str> = scored.iter().take(10).map(|(id, _): &(_, f64)| id.as_str()).collect();

        if start_nodes.is_empty() {
            return;
        }

        let consolidation_rate = 0.05;
        let iterations = 100;

        for _ in 0..iterations {
            // Pick a random start node
            let start_idx = (simple_random_usize()) % start_nodes.len();
            let start = start_nodes[start_idx];

            let path = cog_map.random_walk(start, 5);
            // Hebbian consolidation along the path
            for window in path.windows(2) {
                cog_map.consolidate_edge(window[0].as_str(), window[1].as_str(), consolidation_rate);
            }
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
    crate::search::days_since(created_at, "") as i64
}

fn simple_random_usize() -> usize {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    (ns as usize).wrapping_mul(0x2545F4914F6CDD1D) >> 32
}
