pub mod config;
pub mod dedup;
pub mod emotion;
pub mod engram;
pub mod gateway;
pub mod learned_keywords;
pub mod memory_gate;
pub mod reconsolidation;
pub mod scoring;
pub mod search;
pub mod semantic_network;
pub mod session;
pub mod store;
pub mod reflect;

// re-export
pub use config::HippocampusConfig;
pub use engram::Engram;
pub use store::{EngramStore, StoreStats};
pub use dedup::Deduplicator;
pub use memory_gate::{MemoryGate, MemoryDecision, BrainComponents, BrainRegion};
pub use reconsolidation::Reconsolidation;
pub use session::{Session, SessionManager};
pub use learned_keywords::LearnedKeywords;
pub use reflect::{Reflector, ReflectResult, VacuumResult};
pub use scoring::{decay, final_score, half_life_for_importance, importance_score, ltp_boost};

use std::path::Path;

/// 🧠 Hippocampus — 统一入口
pub struct Hippocampus {
    pub config: HippocampusConfig,
    pub store: EngramStore,
}

impl Hippocampus {
    /// 创建新实例（自动建目录）
    pub fn new(data_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config = HippocampusConfig::new(None, Some(data_dir));
        let store = EngramStore::new(config.clone())?;
        Ok(Self { config, store })
    }

    /// 加载已有实例
    pub fn load(data_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Self::new(data_dir)
    }

    /// 记忆一条内容
    pub fn remember(
        &mut self,
        content: &str,
        importance: u8,
        source: &str,
        tags: &[&str],
        session_id: Option<&str>,
        layer: &str,
        permanent: bool,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut engram = Engram::new(content, importance as u32);
        engram.source = source.to_string();
        engram.tags = tags.iter().map(|s| s.to_string()).collect();
        engram.session_id = session_id.map(|s| s.to_string());
        engram.layer = layer.to_string();

        // 杏仁核情绪增强
        let emo = emotion::detect(content);
        engram.emotion = emo.emotion;
        engram.emotion_score = emo.emotion_score;
        engram.apply_emotion_boost();

        if permanent {
            // 永久记忆：直接 L3，importance=10
            engram.layer = "L3".to_string();
            engram.importance = 10;
            engram.half_life = half_life_for_importance(10) as u64;
        }

        let id = self.store.append(&engram)?;
        Ok(id)
    }

    /// 检索记忆
    pub fn recall(
        &self,
        query: &str,
        top_k: usize,
        min_score: f64,
        include_l3: bool,
        emotion_filter: Option<&str>,
        with_context: Option<&str>,
    ) -> Vec<serde_json::Value> {
        let engine = search::BM25Search::new(&self.store, &self.config);
        let results = engine.search(query, top_k, min_score, include_l3, emotion_filter, with_context.is_some());
        results
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "engram_id": &r.engram.id,
                    "content": &r.engram.content,
                    "score": r.score,
                    "bm25_score": r.bm25_score,
                    "decay": r.decay,
                    "importance": r.engram.importance,
                    "emotion": &r.engram.emotion,
                    "layer": &r.engram.layer,
                    "tags": &r.engram.tags,
                })
            })
            .collect()
    }

    /// 记忆门控：判断是否值得记住
    pub fn should_remember(&self, message: &str) -> MemoryDecision {
        let gate = MemoryGate::new(&self.store, &self.config);
        gate.evaluate(message, &[])
    }

    /// 自动记忆（门控 + 写入）
    pub fn auto_remember(
        &mut self,
        message: &str,
        source: &str,
        session_id: Option<&str>,
        force: bool,
    ) -> Result<MemoryDecision, Box<dyn std::error::Error>> {
        let decision = self.should_remember(message);
        if decision.should_remember || force {
            let tags: Vec<&str> = decision.tags.iter().map(|s| s.as_str()).collect();
            self.remember(
                message,
                decision.importance,
                source,
                &tags,
                session_id,
                "L1",
                false,
            )?;
        }
        Ok(decision)
    }

    /// 反思巩固
    pub fn reflect(&mut self, days: i64) -> Result<ReflectResult, Box<dyn std::error::Error>> {
        let mut reflector = Reflector::new(
            EngramStore::new(self.config.clone())?,
            self.config.clone(),
        );
        Ok(reflector.reflect(days))
    }

    /// 去重扫描
    pub fn find_duplicates(&self, threshold: f64) -> Vec<dedup::DuplicatePair> {
        let dedup = Deduplicator::new(&self.store);
        let mut all = vec![];
        for layer in &["L1", "L2", "L3"] {
            all.extend(dedup.find_duplicates(threshold, layer));
        }
        all
    }

    /// 合并重复
    pub fn merge_duplicates(&self, id_a: &str, id_b: &str, dry_run: bool) -> Result<String, Box<dyn std::error::Error>> {
        let dedup = Deduplicator::new(&self.store);
        dedup.merge(id_a, id_b, dry_run).map_err(|e| e.into())
    }

    /// 统计
    pub fn stats(&self) -> StoreStats {
        self.store.stats().unwrap_or_default()
    }

    /// Vacuum
    pub fn vacuum(&mut self) -> Result<VacuumResult, Box<dyn std::error::Error>> {
        let mut reflector = Reflector::new(
            EngramStore::new(self.config.clone())?,
            self.config.clone(),
        );
        Ok(reflector.vacuum())
    }

    /// 初始化目录结构
    pub fn init(data_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(data_dir)?;
        // 创建空配置
        let config_path = Path::new(data_dir).join("config.json");
        if !config_path.exists() {
            let config = HippocampusConfig::new(None, Some(data_dir));
            config.save_config(None)?;
        }
        // 创建空 layer 文件
        for f in &["engrams_L1.jsonl", "engrams_L2.jsonl", "engrams_L3.jsonl"] {
            let p = Path::new(data_dir).join(f);
            if !p.exists() {
                std::fs::write(&p, "")?;
            }
        }
        // 创建语义网络
        let sn_path = Path::new(data_dir).join("semantic_network.json");
        if !sn_path.exists() {
            std::fs::write(&sn_path, "{}")?;
        }
        // 创建同义词表
        let syn_path = Path::new(data_dir).join("synonyms.json");
        if !syn_path.exists() {
            std::fs::write(&syn_path, "{}")?;
        }
        Ok(())
    }
}
