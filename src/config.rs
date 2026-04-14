use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::fs;

fn dirs_home() -> Option<PathBuf> {
    env::var("HOME").ok().map(PathBuf::from)
}

/// 严格参照 Python config.py 所有字段和默认值
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HippocampusConfig {
    /// cognitive_memory 目录绝对路径
    pub cognitive_dir: PathBuf,
    /// workspace 目录（cognitive_dir 的父目录）
    pub workspace: PathBuf,
    /// 同义词表路径
    pub synonyms_path: PathBuf,
    /// 语义网络路径
    pub semantic_network_path: PathBuf,
    /// 分层文件名
    pub layer_files: HashMap<String, String>,

    // vacuum 参数
    pub l1_max_age_hours: u64,
    pub vacuum_min_score: f64,
    pub consolidate_min_access: u32,
    pub consolidate_min_importance: u32,
    pub archive_days: u64,

    // BM25 参数
    pub bm25_k1: f64,
    pub bm25_b: f64,

    // 情绪检测参数
    pub emotion_half_life_boost: f64,
    pub emotion_threshold: f64,

    // 去重参数
    pub dedup_similarity_threshold: f64,

    // auto_memory 配置
    pub auto_memory_enabled: bool,
    pub auto_memory_threshold: f64,
    pub auto_memory_blacklist: Vec<String>,
}

impl Default for HippocampusConfig {
    fn default() -> Self {
        Self::new(None, None)
    }
}

impl HippocampusConfig {
    pub fn new(workspace: Option<&str>, cognitive_dir: Option<&str>) -> Self {
        let cog = if let Some(cd) = cognitive_dir {
            PathBuf::from(cd)
        } else if let Ok(val) = env::var("HIPPOCAMPUS_HOME") {
            PathBuf::from(val)
        } else {
            // fallback: ~/.hippocampus
            dirs_home().unwrap_or_else(|| PathBuf::from(".hippocampus"))
            .join(".hippocampus")
        };

        let cognitive_dir = cog;
        let workspace = PathBuf::from(workspace.unwrap_or(
            cognitive_dir.parent().map(|p| p.to_str().unwrap_or(".")).unwrap_or("."),
        ));

        let synonyms_path = cognitive_dir.join("synonyms.json");
        let semantic_network_path = cognitive_dir.join("semantic_network.json");

        let mut layer_files = HashMap::new();
        layer_files.insert("L1".into(), "engrams_L1.jsonl".into());
        layer_files.insert("L2".into(), "engrams_L2.jsonl".into());
        layer_files.insert("L3".into(), "engrams_L3.jsonl".into());

        let mut cfg = HippocampusConfig {
            cognitive_dir,
            workspace,
            synonyms_path,
            semantic_network_path,
            layer_files,
            l1_max_age_hours: 24,
            vacuum_min_score: 0.1,
            consolidate_min_access: 10,
            consolidate_min_importance: 6,
            archive_days: 180,
            bm25_k1: 1.5,
            bm25_b: 0.75,
            emotion_half_life_boost: 1.5,
            emotion_threshold: 0.7,
            dedup_similarity_threshold: 0.7,
            auto_memory_enabled: true,
            auto_memory_threshold: 0.3,
            auto_memory_blacklist: vec!["NO_REPLY".into(), "HEARTBEAT_OK".into(), "收到".into(), "好的".into()],
        };

        // 加载配置文件覆盖
        let config_path = cfg.cognitive_dir.join("config.json");
        if config_path.exists() {
            cfg.load_config(&config_path);
        }

        cfg
    }

    fn load_config(&mut self, path: &Path) {
        if let Ok(data) = fs::read_to_string(path) {
            if let Ok(saved) = serde_json::from_str::<serde_json::Value>(&data) {
                if let Some(obj) = saved.as_object() {
                    if let Some(v) = obj.get("l1_max_age_hours").and_then(|v| v.as_u64()) {
                        self.l1_max_age_hours = v;
                    }
                    if let Some(v) = obj.get("vacuum_min_score").and_then(|v| v.as_f64()) {
                        self.vacuum_min_score = v;
                    }
                    if let Some(v) = obj.get("consolidate_min_access").and_then(|v| v.as_u64()) {
                        self.consolidate_min_access = v as u32;
                    }
                    if let Some(v) = obj.get("consolidate_min_importance").and_then(|v| v.as_u64()) {
                        self.consolidate_min_importance = v as u32;
                    }
                    if let Some(v) = obj.get("archive_days").and_then(|v| v.as_u64()) {
                        self.archive_days = v;
                    }
                    if let Some(v) = obj.get("bm25_k1").and_then(|v| v.as_f64()) {
                        self.bm25_k1 = v;
                    }
                    if let Some(v) = obj.get("bm25_b").and_then(|v| v.as_f64()) {
                        self.bm25_b = v;
                    }
                    if let Some(v) = obj.get("emotion_half_life_boost").and_then(|v| v.as_f64()) {
                        self.emotion_half_life_boost = v;
                    }
                    if let Some(v) = obj.get("emotion_threshold").and_then(|v| v.as_f64()) {
                        self.emotion_threshold = v;
                    }
                    if let Some(v) = obj.get("dedup_similarity_threshold").and_then(|v| v.as_f64()) {
                        self.dedup_similarity_threshold = v;
                    }
                    if let Some(v) = obj.get("auto_memory_enabled").and_then(|v| v.as_bool()) {
                        self.auto_memory_enabled = v;
                    }
                    if let Some(v) = obj.get("auto_memory_threshold").and_then(|v| v.as_f64()) {
                        self.auto_memory_threshold = v;
                    }
                    if let Some(v) = obj.get("auto_memory_blacklist").and_then(|v| v.as_array()) {
                        self.auto_memory_blacklist = v.iter()
                            .filter_map(|x| x.as_str().map(String::from))
                            .collect();
                    }
                }
            }
        }
    }

    pub fn layer_path(&self, layer: &str) -> PathBuf {
        let filename = self.layer_files.get(layer)
            .map(|s| s.as_str())
            .unwrap_or("engrams_L1.jsonl");
        self.cognitive_dir.join(filename)
    }

    pub fn save_config(&self, path: Option<&Path>) -> std::io::Result<()> {
        let p = path.map(|p| p.to_path_buf()).unwrap_or_else(|| {
            self.cognitive_dir.join("config.json")
        });
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent)?;
        }
        let obj = serde_json::json!({
            "l1_max_age_hours": self.l1_max_age_hours,
            "vacuum_min_score": self.vacuum_min_score,
            "consolidate_min_access": self.consolidate_min_access,
            "consolidate_min_importance": self.consolidate_min_importance,
            "archive_days": self.archive_days,
            "bm25_k1": self.bm25_k1,
            "bm25_b": self.bm25_b,
            "emotion_half_life_boost": self.emotion_half_life_boost,
            "emotion_threshold": self.emotion_threshold,
            "dedup_similarity_threshold": self.dedup_similarity_threshold,
        });
        fs::write(&p, serde_json::to_string_pretty(&obj).unwrap())
    }
}
