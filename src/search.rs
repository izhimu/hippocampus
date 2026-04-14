/// search — BM25 检索引擎 + CJK 分词 + 同义词扩展

use std::collections::HashMap;

use crate::config::HippocampusConfig;
use crate::engram::Engram;
use crate::scoring::{final_score, ltp_boost};
use crate::semantic_network::SemanticNetwork;
use crate::store::EngramStore;

pub struct SearchResult {
    pub engram: Engram,
    pub score: f64,
    pub bm25_score: f64,
    pub decay: f64,
}

pub struct BM25Search<'a> {
    store: &'a EngramStore,
    config: &'a HippocampusConfig,
}

impl<'a> BM25Search<'a> {
    pub fn new(store: &'a EngramStore, config: &'a HippocampusConfig) -> Self {
        Self { store, config }
    }

    /// 搜索入口
    pub fn search(
        &self,
        query: &str,
        top_k: usize,
        min_score: f64,
        include_l3: bool,
        emotion_filter: Option<&str>,
        _with_context: bool,
    ) -> Vec<SearchResult> {
        // 1. tokenize
        let mut tokens = tokenize(query);

        // 2. 同义词扩展
        if let Ok(network) = self.load_semantic_network() {
            let expanded = network.expand_query(&tokens, 5);
            tokens.extend(expanded);
        }

        // 3. 收集 engrams
        let mut engrams = vec![];
        for layer in &["L1", "L2"] {
            if let Ok(layer_data) = self.store.read_layer(layer) {
                engrams.extend(layer_data);
            }
        }
        if include_l3 {
            if let Ok(layer_data) = self.store.read_layer("L3") {
                engrams.extend(layer_data);
            }
        }

        // emotion filter
        if let Some(emotion) = emotion_filter {
            engrams.retain(|e| e.emotion == emotion);
        }

        // 4. BM25 scoring
        let bm25 = BM25Index::build(&engrams);
        let now = now_iso();

        let mut results: Vec<SearchResult> = engrams
            .into_iter()
            .filter_map(|engram| {
                let bm25_score = bm25.score(&tokens, &engram.content);
                if bm25_score < min_score {
                    return None;
                }
                let days_ago = days_since(&engram.created_at, &now);
                let d = (-days_ago / engram.half_life as f64).exp();
                let score = final_score(
                    bm25_score,
                    engram.importance,
                    engram.access_count,
                    days_ago,
                    engram.half_life as f64,
                );
                Some(SearchResult {
                    engram,
                    score,
                    bm25_score,
                    decay: d,
                })
            })
            .collect();

        // 5. LTP boost via update
        for r in &results {
            let eid = r.engram.id.clone();
            let new_hl = ltp_boost(r.engram.half_life, r.engram.access_count);
            let _ = self.store.update(&eid, |e| {
                e.access_count += 1;
                if new_hl != e.half_life {
                    e.half_life = new_hl;
                }
            });
        }

        // 6. Sort and take top_k
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        results
    }

    fn load_semantic_network(&self) -> std::io::Result<SemanticNetwork> {
        Ok(SemanticNetwork::new(self.config.semantic_network_path.to_string_lossy().to_string()))
    }
}

// --- CJK Tokenizer ---

pub fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = vec![];

    // English words + digits
    let mut word_buf = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            word_buf.push(ch);
        } else {
            if !word_buf.is_empty() {
                tokens.push(word_buf.to_lowercase());
                word_buf.clear();
            }
        }
    }
    if !word_buf.is_empty() {
        tokens.push(word_buf.to_lowercase());
    }

    // CJK segments: extract consecutive CJK runs
    let mut cjk_buf = String::new();
    for ch in text.chars() {
        if is_cjk(ch) {
            cjk_buf.push(ch);
        } else {
            if !cjk_buf.is_empty() {
                generate_ngrams(&cjk_buf, &mut tokens);
                cjk_buf.clear();
            }
        }
    }
    if !cjk_buf.is_empty() {
        generate_ngrams(&cjk_buf, &mut tokens);
    }

    // Filter single char and empty
    tokens.retain(|t| t.len() >= 2);
    tokens
}

fn generate_ngrams(segment: &str, tokens: &mut Vec<String>) {
    let chars: Vec<char> = segment.chars().collect();
    let n = chars.len();

    // 2-grams
    if n >= 2 {
        for i in 0..=(n - 2) {
            let s: String = chars[i..=i + 1].iter().collect();
            tokens.push(s);
        }
    }
    // 3-grams
    if n >= 3 {
        for i in 0..=(n - 3) {
            let s: String = chars[i..=i + 2].iter().collect();
            tokens.push(s);
        }
    }
}

fn is_cjk(ch: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&ch)
}

// --- BM25 Index ---

struct BM25Index {
    doc_count: usize,
    avg_dl: f64,
    idf: HashMap<String, f64>,
    doc_tf: Vec<HashMap<String, u32>>,
    doc_lengths: Vec<usize>,
}

impl BM25Index {
    fn build(engrams: &[Engram]) -> Self {
        let doc_count = engrams.len();
        let mut doc_tf = Vec::with_capacity(doc_count);
        let mut doc_lengths = Vec::with_capacity(doc_count);
        let mut df: HashMap<String, u32> = HashMap::new();
        let mut total_len: usize = 0;

        for e in engrams {
            let tokens = tokenize(&e.content);
            doc_lengths.push(tokens.len());
            total_len += tokens.len();
            let mut tf: HashMap<String, u32> = HashMap::new();
            for t in &tokens {
                *tf.entry(t.clone()).or_insert(0) += 1;
            }
            for t in tf.keys() {
                *df.entry(t.clone()).or_insert(0) += 1;
            }
            doc_tf.push(tf);
        }

        let avg_dl = if doc_count > 0 { total_len as f64 / doc_count as f64 } else { 1.0 };

        let idf = df
            .into_iter()
            .map(|(word, freq)| {
                let idf_val = ((doc_count as f64 - freq as f64 + 0.5) / (freq as f64 + 0.5) + 1.0).ln();
                (word, idf_val)
            })
            .collect();

        Self { doc_count, avg_dl, idf, doc_tf, doc_lengths }
    }

    fn score(&self, query_tokens: &[String], content: &str) -> f64 {
        let tokens = tokenize(content);
        let dl = tokens.len() as f64;
        let mut tf: HashMap<String, u32> = HashMap::new();
        for t in &tokens {
            *tf.entry(t.clone()).or_insert(0) += 1;
        }

        let k1 = 1.5;
        let b = 0.75;
        let avg_dl = self.avg_dl.max(1.0);

        let mut score = 0.0;
        for qt in query_tokens {
            let term_freq = match tf.get(qt) {
                Some(&f) => f as f64,
                None => continue,
            };
            let idf = self.idf.get(qt).copied().unwrap_or(0.0);
            let numerator = term_freq * (k1 + 1.0);
            let denominator = term_freq + k1 * (1.0 - b + b * (dl / avg_dl));
            score += idf * numerator / denominator;
        }
        score
    }
}

// --- Time helpers ---

fn now_iso() -> String {
    // Simple ISO-8601 approximation
    "2026-04-14T13:38:00+08:00".to_string()
}

fn days_since(created_at: &str, _now: &str) -> f64 {
    // Parse date portion and compute rough days
    let date_str = created_at.get(..10).unwrap_or("2026-04-14");
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return 0.0;
    }
    // Very rough: assume now is today
    let y: i64 = parts[0].parse().unwrap_or(2026);
    let m: i64 = parts[1].parse().unwrap_or(4);
    let d: i64 = parts[2].parse().unwrap_or(14);
    let created_days = y * 365 + m * 30 + d;
    let now_days = 2026i64 * 365 + 4 * 30 + 14;
    (now_days - created_days).max(0) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("今天天气不错hello world");
        assert!(tokens.contains(&"今天".to_string()));
        assert!(tokens.contains(&"天天".to_string()));
        assert!(tokens.contains(&"hello".to_string()));
        // Single char filtered
        assert!(!tokens.iter().any(|t| t.len() < 2));
    }
}
