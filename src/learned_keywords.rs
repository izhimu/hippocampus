/// learned_keywords — 自动关键词学习引擎（词频 + 共现）
///
/// 通过分析记忆门控的输入，自动学习哪些词与记忆意图/决策相关，
/// 使 gate 评估越用越准确。

use std::collections::HashMap;
use std::path::Path;

use crate::search::tokenize;
use crate::memory_gate::MemoryDecision;

/// 意图词：用户想记住的信号
const INTENT_WORDS: &[&str] = &["记住", "记一下", "帮我记", "需要记住", "别忘了", "务必记住"];

/// 决策词：用户做决定的信号
const DECISION_WORDS: &[&str] = &["决定", "选择", "以后", "固定", "定期", "不再", "改为", "取消", "打算"];

/// 单字停用词（CJK bigram 过滤）
const STOP_WORDS: &[&str] = &[
    "的是", "在了", "不是", "就是", "也是", "都是", "一个", "这个", "那个",
    "什么", "怎么", "可以", "没有", "他们", "她们", "我们", "你们", "自己",
    "已经", "可能", "因为", "所以", "如果", "但是", "不过", "而且", "或者",
    "还是", "然后", "虽然", "就是", "这些", "那些", "这样", "那样", "怎么",
    "什么", "为什么", "不知道", "没问题", "好的", "收到", "嗯嗯", "哈哈",
    "嘿嘿", "嗯", "啊", "哦", "吧", "呢", "吗", "嘛", "呀",
];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LearnedKeywords {
    /// 词频统计：词 → 出现次数
    pub word_freq: HashMap<String, u32>,
    /// 共现统计：词 → 共现信息
    pub cooccurrence: HashMap<String, CooccurrenceEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CooccurrenceEntry {
    /// 与意图词共现次数
    pub with_intent: u32,
    /// 与决策词共现次数
    pub with_decision: u32,
    /// 最后出现时间
    pub last_seen: String,
}

impl Default for LearnedKeywords {
    fn default() -> Self {
        Self::new()
    }
}

impl LearnedKeywords {
    pub fn new() -> Self {
        Self {
            word_freq: HashMap::new(),
            cooccurrence: HashMap::new(),
        }
    }

    /// 从 JSON 文件加载
    pub fn load(path: &Path) -> Self {
        if let Ok(data) = std::fs::read_to_string(path) {
            if let Ok(loaded) = serde_json::from_str(&data) {
                return loaded;
            }
        }
        Self::new()
    }

    /// 保存到 JSON 文件
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .unwrap_or_else(|_| "{}".to_string());
        std::fs::write(path, json)
    }

    /// gate 评估后实时更新（核心！）
    pub fn update_from_gate(&mut self, content: &str, _decision: &MemoryDecision) {
        let tokens = tokenize(content);
        let now = now_iso();

        // 检测消息中是否包含意图词/决策词
        let has_intent = INTENT_WORDS.iter().any(|w| content.contains(*w));
        let has_decision = DECISION_WORDS.iter().any(|w| content.contains(*w));

        for token in &tokens {
            // 过滤停用词
            if STOP_WORDS.contains(&token.as_str()) {
                continue;
            }

            // 更新词频
            *self.word_freq.entry(token.clone()).or_insert(0) += 1;

            // 更新共现
            let entry = self.cooccurrence.entry(token.clone()).or_insert_with(|| {
                CooccurrenceEntry {
                    with_intent: 0,
                    with_decision: 0,
                    last_seen: now.clone(),
                }
            });

            if has_intent {
                entry.with_intent += 1;
            }
            if has_decision {
                entry.with_decision += 1;
            }
            entry.last_seen = now.clone();
        }
    }

    /// 从 engram 更新（用于 reflect 时批量学习）
    pub fn update_from_engram(&mut self, content: &str) {
        let tokens = tokenize(content);
        let now = now_iso();

        let has_intent = INTENT_WORDS.iter().any(|w| content.contains(*w));
        let has_decision = DECISION_WORDS.iter().any(|w| content.contains(*w));

        for token in &tokens {
            if STOP_WORDS.contains(&token.as_str()) {
                continue;
            }

            *self.word_freq.entry(token.clone()).or_insert(0) += 1;

            let entry = self.cooccurrence.entry(token.clone()).or_insert_with(|| {
                CooccurrenceEntry {
                    with_intent: 0,
                    with_decision: 0,
                    last_seen: now.clone(),
                }
            });

            if has_intent {
                entry.with_intent += 1;
            }
            if has_decision {
                entry.with_decision += 1;
            }
            entry.last_seen = now.clone();
        }
    }

    /// 返回某个词的学习加分值
    pub fn get_boost(&self, word: &str) -> f64 {
        let mut boost = 0.0;

        // 词频加分
        let freq = self.word_freq.get(word).copied().unwrap_or(0);
        if freq >= 50 {
            boost = 0.20;
        } else if freq >= 20 {
            boost = 0.15;
        } else if freq >= 10 {
            boost = 0.10;
        }

        // 共现加分
        if let Some(entry) = self.cooccurrence.get(word) {
            if entry.with_intent >= 5 {
                boost += 0.35;
            } else if entry.with_intent >= 3 {
                boost += 0.25;
            }
            if entry.with_decision >= 3 {
                boost += 0.15;
            }
        }

        boost
    }

    /// 反思时清理：删除低频词，保存
    pub fn refine(&mut self) {
        let min_freq = 3;
        // 删除词频 < min_freq 的词
        self.word_freq.retain(|_, &mut freq| freq >= min_freq);
        self.cooccurrence.retain(|word, _| {
            self.word_freq.contains_key(word)
        });
    }

    /// 获取统计信息
    pub fn stats(&self) -> (usize, usize) {
        (self.word_freq.len(), self.cooccurrence.len())
    }

    /// 获取 top-N 关键词（按 boost 排序）
    pub fn top_keywords(&self, n: usize) -> Vec<(String, f64, u32)> {
        let mut all: Vec<(String, f64, u32)> = self.word_freq.iter()
            .map(|(word, &freq)| {
                let boost = self.get_boost(word);
                (word.clone(), boost, freq)
            })
            .filter(|(_, boost, _)| *boost > 0.0)
            .collect();

        all.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        all.truncate(n);
        all
    }
}

fn now_iso() -> String {
    // Simple ISO-8601 approximation
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("2026-04-14T{:02}:{:02}:00+08:00", (now / 3600) % 24, (now / 60) % 60)
}
