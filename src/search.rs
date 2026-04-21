/// search — BM25 + SimHash (SDM) 检索引擎 + CJK 分词 + 同义词扩展

use std::collections::{HashMap, HashSet};

use crate::config::HippocampusConfig;
use crate::engram::Engram;
use crate::scoring::{final_score_actr, ltp_boost};
use crate::semantic_network::SemanticNetwork;
use crate::simhash;
use crate::stop_words::{all_cjk_stop_chars, is_stop_word};
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

        // 3. 收集 engrams (from cache)
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

        // 4. Compute query SimHash fingerprint
        let query_fp = simhash::simhash(query);

        // 5. SimHash pre-filter: skip engrams too far from query
        let max_hamming = 25u32;
        engrams.retain(|e| {
            let fp = if e.fingerprint != 0 { e.fingerprint } else { simhash::simhash(&e.content) };
            simhash::hamming_distance(query_fp, fp) <= max_hamming
        });

        // 6. BM25 scoring
        let bm25 = BM25Index::build(&engrams, self.config.bm25_k1, self.config.bm25_b);

        let context_clues: HashSet<String> = tokens.iter()
            .filter(|t| t.len() > 3)
            .cloned()
            .collect();

        let mut results: Vec<SearchResult> = engrams
            .into_iter()
            .enumerate()
            .filter_map(|(idx, engram)| {
                let mut bm25_score = bm25.score_by_index(&tokens, idx);

                // Context boost
                for tag in &engram.tags {
                    if let Some(ctx_val) = tag.strip_prefix("ctx:") {
                        if context_clues.contains(ctx_val) {
                            bm25_score += 1.0;
                        }
                    }
                }

                let fused_score = bm25_score;

                let decay_rate = self.config.actr_decay_rate;
                let d = crate::scoring::actr_decay_factor(&engram.access_history, decay_rate);
                let score = final_score_actr(
                    fused_score,
                    engram.importance,
                    engram.access_count,
                    &engram.access_history,
                    engram.half_life as f64,
                    decay_rate,
                );

                if score < min_score && fused_score < min_score && bm25_score < min_score {
                    return None;
                }
                Some(SearchResult {
                    engram,
                    score,
                    bm25_score: fused_score,
                    decay: d,
                })
            })
            .collect();

        // 7. LTP boost — batch update
        let update_map: HashMap<String, (u32, u64)> = results.iter()
            .map(|r| {
                let access_count = r.engram.access_count + 1;
                let new_hl = ltp_boost(r.engram.half_life, access_count);
                (r.engram.id.clone(), (access_count, new_hl))
            })
            .collect();

        let _ = self.store.batch_update(&update_map, |e, &(ac, new_hl)| {
            let now = crate::engram::chrono_now_iso();
            e.accessed_at = Some(now.clone());
            e.access_count = ac;
            e.access_history.push(now);
            if e.access_history.len() > 50 {
                e.access_history.drain(..e.access_history.len() - 50);
            }
            if new_hl != e.half_life {
                e.half_life = new_hl;
            }
        });

        // Build an in-memory ID index
        let id_index: HashMap<String, &SearchResult> = results.iter()
            .map(|r| (r.engram.id.clone(), r))
            .collect();

        // 8. SR spreading
        let cog_map_path = self.config.cognitive_dir.join("cognitive_map.json");
        let cog_map = crate::cognitive_map::CognitiveMap::load(&cog_map_path, 5000).ok();

        if let Some(ref cog_map) = cog_map {
            let mut sr_candidates: Vec<SearchResult> = vec![];
            let mut seen_ids: HashSet<String> = results.iter().map(|r| r.engram.id.clone()).collect();

            for r in &results {
                let related = cog_map.get_related(&r.engram.id, 3);
                for (rel_id, rel_weight) in related {
                    if seen_ids.contains(&rel_id) { continue; }
                    seen_ids.insert(rel_id.clone());

                    let engram_opt: Option<Engram> = if let Some(sr) = id_index.get(&rel_id) {
                        Some(sr.engram.clone())
                    } else {
                        self.store.get_by_id(&rel_id).ok().flatten()
                    };
                    let Some(engram) = engram_opt else { continue };

                    let sdm_sim = if engram.fingerprint != 0 {
                        simhash::hamming_similarity(query_fp, engram.fingerprint)
                    } else {
                        0.0
                    };
                    let fused = sdm_sim * 5.0 * rel_weight;
                    if fused >= min_score {
                        let decay_rate = self.config.actr_decay_rate;
                        let d = crate::scoring::actr_decay_factor(&engram.access_history, decay_rate);
                        let score = crate::scoring::final_score_actr(
                            fused, engram.importance, engram.access_count,
                            &engram.access_history, engram.half_life as f64, decay_rate,
                        );
                        sr_candidates.push(SearchResult {
                            engram,
                            score: score * 0.8,
                            bm25_score: fused,
                            decay: d,
                        });
                    }
                }
            }
            results.extend(sr_candidates);
        }

        // 9. Sort and take top_k
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        results
    }

    fn load_semantic_network(&self) -> std::io::Result<SemanticNetwork> {
        Ok(SemanticNetwork::new(self.config.semantic_network_path.to_string_lossy().to_string()))
    }
}

// --- CJK Tokenizer (single-pass, HashSet dedup) ---

fn stem_english(word: &str) -> String {
    if word.len() < 4 || !word.chars().all(|c| c.is_ascii_alphabetic()) {
        return word.to_string();
    }

    if word.ends_with("sses") {
        return word[..word.len()-2].to_string();
    }
    if word.ends_with("ies") {
        return word[..word.len()-3].to_string() + "i";
    }
    if word.ends_with('s') && !word.ends_with("ss") && !word.ends_with("us") && !word.ends_with("is") {
        return word[..word.len()-1].to_string();
    }

    if word.ends_with("eed") {
        let stem = &word[..word.len()-3];
        if count_consonant_sequences(stem) > 0 {
            return word[..word.len()-1].to_string();
        }
        return word.to_string();
    }

    if word.ends_with("ing") && word.len() > 5 {
        let stem = &word[..word.len()-3];
        if has_vowel(stem) {
            return stem_or_double(stem);
        }
        return word.to_string();
    }
    if word.ends_with("ed") && word.len() > 4 {
        let stem = &word[..word.len()-2];
        if has_vowel(stem) {
            return stem_or_double(stem);
        }
        return word.to_string();
    }

    if word.ends_with('y') && word.len() > 3 {
        let stem = &word[..word.len()-1];
        if has_vowel(stem) {
            return stem.to_string() + "i";
        }
    }

    if word.ends_with("ational") && word.len() > 8 {
        return word[..word.len()-5].to_string() + "e";
    }
    if word.ends_with("tional") && word.len() > 7 {
        return word[..word.len()-2].to_string();
    }
    if word.ends_with("ization") {
        return word[..word.len()-5].to_string() + "e";
    }
    if word.ends_with("fulness") {
        return word[..word.len()-4].to_string();
    }
    if word.ends_with("ousness") {
        return word[..word.len()-4].to_string();
    }
    if word.ends_with("iveness") {
        return word[..word.len()-4].to_string();
    }

    word.to_string()
}

fn has_vowel(s: &str) -> bool {
    s.chars().any(|c| matches!(c.to_ascii_lowercase(), 'a'|'e'|'i'|'o'|'u'))
}

fn count_consonant_sequences(s: &str) -> usize {
    let mut count = 0;
    let mut in_consonant = false;
    for c in s.chars() {
        let is_vowel = matches!(c.to_ascii_lowercase(), 'a'|'e'|'i'|'o'|'u');
        if !is_vowel && !in_consonant {
            count += 1;
            in_consonant = true;
        } else if is_vowel {
            in_consonant = false;
        }
    }
    count
}

fn stem_or_double(stem: &str) -> String {
    if stem.is_empty() { return stem.to_string(); }
    let chars: Vec<char> = stem.chars().collect();
    let n = chars.len();
    if n >= 2 {
        let last = chars[n-1];
        let prev = chars[n-2];
        if last == prev && matches!(last, 'b'|'d'|'g'|'l'|'m'|'n'|'p'|'r'|'t') {
            return chars[..n-1].iter().collect();
        }
    }
    if stem.ends_with("at") || stem.ends_with("bl") || stem.ends_with("iz") {
        return stem.to_string() + "e";
    }
    stem.to_string()
}

/// Single-pass tokenizer: processes English and CJK in one iteration
pub fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut seen = HashSet::new();

    let mut word_buf = String::new();
    let mut cjk_buf = String::new();

    let flush_word = |word_buf: &mut String, tokens: &mut Vec<String>, seen: &mut HashSet<String>| {
        if word_buf.is_empty() { return; }
        let w = word_buf.to_lowercase();
        let stemmed = stem_english(&w);
        for t in [w, stemmed] {
            if t.len() >= 2 && !is_stop_word(&t) && seen.insert(t.clone()) {
                tokens.push(t);
            }
        }
        word_buf.clear();
    };

    let flush_cjk = |cjk_buf: &mut String, tokens: &mut Vec<String>, seen: &mut HashSet<String>| {
        if cjk_buf.is_empty() { return; }
        generate_ngrams(&cjk_buf, tokens, seen);
        cjk_buf.clear();
    };

    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            flush_cjk(&mut cjk_buf, &mut tokens, &mut seen);
            word_buf.push(ch);
        } else if is_cjk(ch) {
            flush_word(&mut word_buf, &mut tokens, &mut seen);
            cjk_buf.push(ch);
        } else {
            flush_word(&mut word_buf, &mut tokens, &mut seen);
            flush_cjk(&mut cjk_buf, &mut tokens, &mut seen);
        }
    }
    flush_word(&mut word_buf, &mut tokens, &mut seen);
    flush_cjk(&mut cjk_buf, &mut tokens, &mut seen);

    tokens
}

fn generate_ngrams(segment: &str, tokens: &mut Vec<String>, seen: &mut HashSet<String>) {
    let chars: Vec<char> = segment.chars().collect();
    let gen_n = chars.len().min(20);

    if gen_n >= 2 {
        for i in 0..=gen_n - 2 {
            let s: String = chars[i..=i + 1].iter().collect();
            if !all_cjk_stop_chars(&s) && seen.insert(s.clone()) {
                tokens.push(s);
            }
        }
    }
    if gen_n >= 3 {
        for i in 0..=gen_n - 3 {
            let s: String = chars[i..=i + 2].iter().collect();
            if !all_cjk_stop_chars(&s) && seen.insert(s.clone()) {
                tokens.push(s);
            }
        }
    }
}

fn is_cjk(ch: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&ch)
}

// --- BM25 Index ---

struct BM25Index {
    avg_dl: f64,
    idf: HashMap<String, f64>,
    doc_tf: Vec<HashMap<String, u32>>,
    doc_lengths: Vec<usize>,
    k1: f64,
    b: f64,
}

impl BM25Index {
    fn build(engrams: &[Engram], k1: f64, b: f64) -> Self {
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

        Self { avg_dl, idf, doc_tf, doc_lengths, k1, b }
    }

    fn score_by_index(&self, query_tokens: &[String], doc_idx: usize) -> f64 {
        let tf = match self.doc_tf.get(doc_idx) {
            Some(t) => t,
            None => return 0.0,
        };
        let dl = *self.doc_lengths.get(doc_idx).unwrap_or(&1) as f64;

        let k1 = self.k1;
        let b = self.b;
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

use crate::engram::parse_iso_date;

pub fn now_iso() -> String {
    crate::engram::chrono_now_iso()
}

pub fn days_since(created_at: &str, _now: &str) -> f64 {
    let created = match parse_iso_date(created_at) {
        Some(d) => d,
        None => return 0.0,
    };
    let today = chrono::Local::now().date_naive();
    (today - created).num_days().max(0) as f64
}

pub fn hours_since(iso_timestamp: &str) -> f64 {
    let dt = match chrono::DateTime::parse_from_rfc3339(iso_timestamp) {
        Ok(d) => d.with_timezone(&chrono::Local).naive_local(),
        Err(_) => return 0.0,
    };
    let now = chrono::Local::now().naive_local();
    (now - dt).num_seconds() as f64 / 3600.0
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
        assert!(!tokens.iter().any(|t| t.len() < 2));
    }

    #[test]
    fn test_tokenize_no_duplicates() {
        let tokens = tokenize("hello hello world world");
        let unique: HashSet<&String> = tokens.iter().collect();
        assert_eq!(tokens.len(), unique.len(), "No duplicate tokens");
    }
}
