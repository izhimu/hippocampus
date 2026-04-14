/// memory_gate — 仿生记忆门控系统（4脑区协同）
///
/// 模拟人脑4个脑区决定"什么值得记住"：
/// 1. 杏仁核 Amygdala — 情绪闪灯（权重0.35）
/// 2. 海马体 Hippocampus — 新奇度+预测违背（权重0.30）
/// 3. 前额叶 Prefrontal — 目标相关性（权重0.20）
/// 4. 颞叶 Temporal — 社交关联（权重0.15）

use std::collections::{HashMap, HashSet};

use crate::config::HippocampusConfig;
use crate::emotion;
use crate::search::tokenize;
use crate::store::EngramStore;

pub struct MemoryGate<'a> {
    store: &'a EngramStore,
    config: &'a HippocampusConfig,
}

pub struct MemoryDecision {
    pub should_remember: bool,
    pub importance: u8,
    pub tags: Vec<String>,
    pub decision_score: f64,
    pub emotion: String,
    pub emotion_score: f64,
    pub reason: String,
    pub components: BrainComponents,
}

pub struct BrainComponents {
    pub amygdala: BrainRegion,
    pub hippocampus: BrainRegion,
    pub prefrontal: BrainRegion,
    pub temporal: BrainRegion,
}

#[derive(Clone)]
pub struct BrainRegion {
    pub score: f64,
    pub reason: String,
}

impl<'a> MemoryGate<'a> {
    pub fn new(store: &'a EngramStore, config: &'a HippocampusConfig) -> Self {
        Self { store, config }
    }

    pub fn evaluate(&self, message: &str, session_context: &[String]) -> MemoryDecision {
        // 前置过滤
        let system_patterns = ["NO_REPLY", "HEARTBEAT_OK", "HEARTBEAT_", "[OpenClaw", "[Internal"];
        if system_patterns.iter().any(|p| message.contains(p)) {
            return self.reject("系统消息，过滤");
        }
        if message.trim().len() < 3 {
            return self.reject("消息过短");
        }
        if !message.chars().any(|c| c.is_ascii_alphanumeric() || ('\u{4e00}'..='\u{9fff}').contains(&c)) {
            return self.reject("无有效内容");
        }

        let existing = self.store.read_all().unwrap_or_default();
        let amy = self.amygdala_evaluate(message);
        let emotion_name = extract_emotion_from_reason(&amy.reason);
        let hip = self.hippocampus_evaluate(message, &existing);
        let pfc = self.prefrontal_evaluate(message, session_context);
        let tmp = self.temporal_evaluate(message);

        let decision_score = amy.score * 0.35
            + hip.score * 0.30
            + pfc.score * 0.20
            + tmp.score * 0.15;

        let importance = (decision_score * 10.0).clamp(1.0, 10.0) as u8;
        let should_remember = decision_score >= self.config.auto_memory_threshold;
        let tags = extract_tags(message, &emotion_name);

        let mut reasons = vec![];
        if amy.score >= 0.5 { reasons.push("杏仁核：强情绪"); }
        if hip.score >= 0.5 { reasons.push("海马体：高新奇度"); }
        if pfc.score >= 0.5 { reasons.push("前额叶：话题相关"); }
        if tmp.score >= 0.5 { reasons.push("颞叶：社交内容"); }

        let reason = if reasons.is_empty() {
            if should_remember { "综合达标".into() } else { "不满足记忆阈值".into() }
        } else {
            reasons.join("，")
        };

        MemoryDecision {
            should_remember,
            importance,
            tags,
            decision_score: (decision_score * 10000.0).round() / 10000.0,
            emotion: emotion_name,
            emotion_score: amy.score,
            reason,
            components: BrainComponents {
                amygdala: amy,
                hippocampus: hip,
                prefrontal: pfc,
                temporal: tmp,
            },
        }
    }

    // --- 杏仁核 ---
    fn amygdala_evaluate(&self, message: &str) -> BrainRegion {
        let result = emotion::detect(message);
        BrainRegion {
            score: result.emotion_score,
            reason: format!("情绪={}, 强度={:.1}", result.emotion, result.emotion_score),
        }
    }

    // --- 海马体 ---
    fn hippocampus_evaluate(&self, message: &str, existing: &[crate::engram::Engram]) -> BrainRegion {
        let words = tokenize(message);

        let mut existing_words: HashMap<String, u32> = HashMap::new();
        let mut existing_ngrams: HashMap<String, u32> = HashMap::new();
        let mut total_count: u64 = 0;
        for e in existing {
            for w in tokenize(&e.content) {
                *existing_words.entry(w.clone()).or_insert(0) += 1;
                total_count += 1;
            }
            let cjk: Vec<char> = e.content.chars().filter(|c| ('\u{4e00}'..='\u{9fff}').contains(c)).collect();
            for i in 0..cjk.len().saturating_sub(1) {
                let ng: String = cjk[i..=i + 1].iter().collect();
                *existing_ngrams.entry(ng).or_insert(0) += 1;
            }
        }
        let total_count = total_count.max(1) as f64;

        // 新词比例
        let new_count = words.iter().filter(|w| existing_words.get(*w).copied().unwrap_or(0) == 0).count();
        let novelty_ratio = if !words.is_empty() { new_count as f64 / words.len() as f64 } else { 0.3 };

        // IDF
        let unique: HashSet<&String> = words.iter().collect();
        let idf_sum: f64 = unique.iter().map(|w| {
            let freq = existing_words.get(*w).copied().unwrap_or(0) as f64;
            ((total_count + 1.0) / (freq + 1.0)).ln() + 1.0
        }).sum();
        let avg_idf = if unique.is_empty() { 0.0 } else { idf_sum / unique.len() as f64 };
        let idf_norm = (avg_idf / 5.0).min(1.0);

        // 信息增量
        let msg_cjk: Vec<char> = message.chars().filter(|c| ('\u{4e00}'..='\u{9fff}').contains(c)).collect();
        let mut msg_ngrams: HashSet<String> = HashSet::new();
        for i in 0..msg_cjk.len().saturating_sub(1) {
            msg_ngrams.insert(msg_cjk[i..=i + 1].iter().collect());
        }
        let max_ng = existing_ngrams.values().copied().max().unwrap_or(1).max(1) as f64;
        let overlap: f64 = msg_ngrams.iter().map(|ng| existing_ngrams.get(ng).copied().unwrap_or(0) as f64).sum();
        let max_overlap = msg_ngrams.len() as f64 * max_ng;
        let information_gain = if max_overlap > 0.0 { 1.0 - overlap / max_overlap } else { 0.8 };

        // 预测违背
        let surprise_words = ["但是","可是","不过","居然","竟然","没想到","谁知","原来","其实","改变","不再","突然","想不到","反常","异常"];
        let surprise = (surprise_words.iter().filter(|w| message.contains(**w)).count() as f64 * 0.15).min(0.5);

        let novelty = (novelty_ratio * 0.25 + idf_norm * 0.20 + information_gain * 0.30 + surprise * 0.25)
            .clamp(0.0, 1.0);

        let mut reasons = vec![];
        if novelty_ratio > 0.5 { reasons.push(format!("新词多({:.0}%)", novelty_ratio * 100.0)); }
        if information_gain > 0.7 { reasons.push("高信息增量".into()); }
        if surprise > 0.2 { reasons.push("含意外转折".into()); }

        BrainRegion {
            score: novelty,
            reason: if reasons.is_empty() { "常规信息".into() } else { reasons.join("，") },
        }
    }

    // --- 前额叶 ---
    fn prefrontal_evaluate(&self, message: &str, session_context: &[String]) -> BrainRegion {
        if session_context.is_empty() {
            return BrainRegion { score: 0.3, reason: "无上下文参考".into() };
        }
        let words = tokenize(message);
        if words.is_empty() {
            return BrainRegion { score: 0.1, reason: "无有效词汇".into() };
        }

        let mut session_words: HashMap<String, u32> = HashMap::new();
        for msg in session_context {
            for w in tokenize(msg) {
                *session_words.entry(w).or_insert(0) += 1;
            }
        }
        let max_freq = session_words.values().copied().max().unwrap_or(1);
        let top_words: HashSet<String> = session_words.iter()
            .filter(|(_, &f)| f >= (max_freq as f64 * 0.3) as u32)
            .map(|(w, _)| w.clone())
            .collect();

        let topic_match = words.iter().filter(|w| top_words.contains(*w)).count();
        let topic_relevance = topic_match as f64 / words.len() as f64;
        let repeat = words.iter().filter(|w| session_words.get(*w).copied().unwrap_or(0) >= 3).count();
        let progress = 1.0 - repeat as f64 / words.len() as f64;

        let len = message.len();
        let length = if len >= 10 && len <= 200 { 1.0 }
            else if len >= 5 { 0.5 }
            else if len <= 500 { 0.7 }
            else { 0.0 };

        let score = topic_relevance * 0.4 + progress * 0.35 + length * 0.25;

        let mut reasons = vec![];
        if topic_relevance > 0.5 { reasons.push(format!("话题匹配({:.0}%)", topic_relevance * 100.0)); }
        if progress < 0.3 { reasons.push("内容重复".into()); }
        if length > 0.8 { reasons.push("长度适中".into()); }

        BrainRegion {
            score,
            reason: if reasons.is_empty() { "一般".into() } else { reasons.join("，") },
        }
    }

    // --- 颞叶 ---
    fn temporal_evaluate(&self, message: &str) -> BrainRegion {
        let personal = ["我","你","他","她","我们","你们","他们","主人","老婆","老公","爸","妈","哥","姐","弟","妹","老板","同事","朋友","同学","老师"];
        let relation = ["的","和","跟","给","让","帮","告诉","说","问","回答","建议","决定","觉得","认为"];
        let social = ["见面","一起","约","聊","吃饭","开会","出差","旅行","帮助","支持","感谢","抱歉","生日","结婚","搬家"];

        let pc = personal.iter().filter(|w| message.contains(**w)).count();
        let rc = relation.iter().filter(|w| message.contains(**w)).count();
        let ac = social.iter().filter(|w| message.contains(**w)).count();

        let score = (pc as f64 / 2.0).min(1.0) * 0.4
            + (rc as f64 / 3.0).min(1.0) * 0.3
            + (ac as f64 / 2.0).min(1.0) * 0.3;
        let score = score.min(1.0);

        let mut reasons = vec![];
        if pc > 0 { reasons.push(format!("含人称({}个)", pc)); }
        if rc > 2 { reasons.push("含关系描述".into()); }
        if ac > 0 { reasons.push("含社交行为".into()); }

        BrainRegion {
            score,
            reason: if reasons.is_empty() { "无社交内容".into() } else { reasons.join("，") },
        }
    }

    fn reject(&self, reason: &str) -> MemoryDecision {
        let br = BrainRegion { score: 0.0, reason: "过滤".into() };
        MemoryDecision {
            should_remember: false,
            importance: 1,
            tags: vec![],
            decision_score: 0.0,
            emotion: "neutral".into(),
            emotion_score: 0.0,
            reason: reason.into(),
            components: BrainComponents {
                amygdala: br.clone(),
                hippocampus: br.clone(),
                prefrontal: br.clone(),
                temporal: br,
            },
        }
    }
}

fn extract_emotion_from_reason(reason: &str) -> String {
    let prefix = "情绪=";
    if let Some(pos) = reason.find(prefix) {
        let rest = &reason[pos + prefix.len()..];
        if let Some(comma) = rest.find(',') {
            return rest[..comma].trim().to_string();
        }
    }
    "neutral".to_string()
}

fn extract_tags(message: &str, emotion: &str) -> Vec<String> {
    let mut tags = vec![];
    if emotion != "neutral" {
        tags.push(emotion.to_string());
    }
    let decision_words = ["决定","选择","以后","固定","定期","不再","改为","取消"];
    if decision_words.iter().any(|w| message.contains(w)) {
        tags.push("决策".into());
    }
    let time_p = ["下周","明天","后天","周","月","号","点"];
    if time_p.iter().any(|p| message.contains(p)) {
        tags.push("时间".into());
    }
    if message.chars().any(|c| c.is_ascii_digit()) && message.chars().any(|c| "万亿千百元%".contains(c)) {
        tags.push("数据".into());
    }
    if tags.len() > 5 { tags.truncate(5); }
    tags
}
