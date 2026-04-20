/// LongMemEval Benchmark — Recall + Gate evaluation
///
/// Modes:
///   recall  — import all haystack → recall → measure Recall@K, MRR
///   gate    — feed haystack through gate --write → recall → measure gate quality
///   full    — both recall & gate, compare results (default)
///
/// Streaming JSON parser handles the 2.6GB dataset without OOM.

use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

// ── Config ──────────────────────────────────────────────────────────────────

const DEFAULT_DATASET: &str = "/home/haoran/Data/Model/longmemeval_m_cleaned.json";
const DEFAULT_SAMPLES: usize = 0; // 0 = all
const DEFAULT_TOP_K: usize = 10;
const TEST_HOME: &str = "/tmp/hippocampus_bench";
const CONTENT_CAP: usize = 4000;

// ── Dataset types ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct QuestionFull {
    question_id: String,
    question_type: String,
    question: String,
    #[serde(deserialize_with = "deserialize_answer")]
    answer: String,
    haystack_sessions: Vec<Vec<Message>>,
    haystack_session_ids: Vec<String>,
    haystack_dates: Vec<String>,
    #[serde(default)]
    answer_session_ids: Vec<String>,
}

fn deserialize_answer<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    Ok(match v {
        serde_json::Value::String(s) => s,
        other => other.to_string(),
    })
}

#[derive(Deserialize)]
struct QuestionLight {
    question_type: String,
    question_id: String,
}

#[derive(Deserialize)]
struct Message {
    role: String,
    content: String,
}

// ── Metrics ─────────────────────────────────────────────────────────────────

#[derive(Default, Clone)]
struct RecallMetrics {
    total: usize,
    hit_at: [usize; 4], // @1, @3, @5, @10
    mrr_sum: f64,
    overlap_scores: Vec<f64>,
    latencies: Vec<f64>,
}

impl RecallMetrics {
    fn recall_at(&self, k: usize) -> f64 {
        if self.total == 0 { 0.0 } else { self.hit_at[k] as f64 / self.total as f64 }
    }
    fn mrr(&self) -> f64 {
        if self.total == 0 { 0.0 } else { self.mrr_sum / self.total as f64 }
    }
    fn avg_overlap(&self) -> f64 {
        if self.overlap_scores.is_empty() { 0.0 }
        else { self.overlap_scores.iter().sum::<f64>() / self.overlap_scores.len() as f64 }
    }
    fn avg_latency_ms(&self) -> f64 {
        if self.latencies.is_empty() { 0.0 }
        else { self.latencies.iter().sum::<f64>() / self.latencies.len() as f64 * 1000.0 }
    }
}

#[derive(Default)]
struct GateMetrics {
    total_sessions: usize,
    accepted_sessions: usize,
    answer_sessions_preserved: usize,
    total_answer_sessions: usize,
    post_gate_recall: RecallMetrics,
}

impl GateMetrics {
    fn acceptance_rate(&self) -> f64 {
        if self.total_sessions == 0 { 0.0 }
        else { self.accepted_sessions as f64 / self.total_sessions as f64 }
    }
    fn answer_preservation_rate(&self) -> f64 {
        if self.total_answer_sessions == 0 { 0.0 }
        else { self.answer_sessions_preserved as f64 / self.total_answer_sessions as f64 }
    }
}

// ── Per-question result ─────────────────────────────────────────────────────

struct QRecallResult {
    question_id: String,
    question_type: String,
    recall_count: usize,
    hit_position: Option<usize>,
    overlap: f64,
    latency_ms: f64,
}

struct QGateResult {
    question_id: String,
    question_type: String,
    total_sessions: usize,
    accepted_sessions: usize,
    answer_preserved: usize,
    answer_total: usize,
    post_gate_hit_position: Option<usize>,
    post_gate_overlap: f64,
    post_gate_latency_ms: f64,
}

// ── Streaming JSON array parser ────────────────────────────────────────────

struct JsonStream {
    reader: BufReader<fs::File>,
    depth: i32,
    in_str: bool,
    esc: bool,
    buf: Vec<u8>,
}

impl JsonStream {
    fn open(path: &str) -> Self {
        let f = fs::File::open(path).unwrap_or_else(|e| {
            eprintln!("Cannot open {}: {}", path, e);
            std::process::exit(1);
        });
        Self {
            reader: BufReader::with_capacity(16 * 1024 * 1024, f),
            depth: 0, in_str: false, esc: false,
            buf: Vec::with_capacity(2 * 1024 * 1024),
        }
    }

    fn next_raw(&mut self) -> Option<String> {
        let mut byte = [0u8; 1];
        self.buf.clear();
        loop {
            if self.reader.read_exact(&mut byte).is_err() { return None; }
            let ch = byte[0];
            if self.esc { self.esc = false; if self.depth > 0 { self.buf.push(ch); } continue; }
            if self.in_str {
                match ch {
                    b'\\' => { self.esc = true; self.buf.push(ch); }
                    b'"' => { self.in_str = false; self.buf.push(ch); }
                    _ => { self.buf.push(ch); }
                }
                continue;
            }
            match ch {
                b'"' => { self.in_str = true; if self.depth > 0 { self.buf.push(ch); } }
                b'{' => {
                    self.depth += 1;
                    if self.depth == 1 { self.buf.clear(); }
                    self.buf.push(ch);
                }
                b'}' => {
                    self.buf.push(ch);
                    self.depth -= 1;
                    if self.depth == 0 {
                        return std::str::from_utf8(&self.buf).ok().map(|s| s.to_string());
                    }
                }
                _ => { if self.depth > 0 { self.buf.push(ch); } }
            }
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn setup_clean_env() {
    let home = Path::new(TEST_HOME);
    if home.exists() { let _ = fs::remove_dir_all(home); }
    fs::create_dir_all(home).unwrap();
    for layer in &["L1", "L2", "L3"] {
        fs::write(home.join(format!("engrams_{}.jsonl", layer)), "").unwrap();
    }
    fs::write(home.join("config.json"), "{}").unwrap();
    fs::write(home.join("learned_keywords.json"), r#"{"word_freq":{},"cooccurrence":{}}"#).unwrap();
    fs::write(home.join("semantic_network.json"), "{}").unwrap();
    fs::write(home.join("synonyms.json"), "{}").unwrap();
    let cog = home.join("cognitive");
    fs::create_dir_all(&cog).unwrap();
    fs::write(cog.join("cognitive_map.json"), "{}").unwrap();
}

fn shift_date(date_str: &str, base: chrono::NaiveDateTime, target: chrono::NaiveDateTime) -> String {
    let clean = date_str.split('(').next().unwrap_or(date_str).trim();
    match chrono::NaiveDateTime::parse_from_str(clean, "%Y/%m/%d %H:%M") {
        Ok(orig) => {
            let shifted = target + (orig - base);
            shifted.format("%Y-%m-%dT%H:%M:%S+08:00").to_string()
        }
        Err(_) => target.format("%Y-%m-%dT%H:%M:%S+08:00").to_string(),
    }
}

/// Build a text representation from a session's messages
fn session_to_text(session: &[Message], cap: usize) -> String {
    let mut content = String::with_capacity(cap + 100);
    for msg in session {
        if !content.is_empty() { content.push('\n'); }
        match msg.role.as_str() {
            "user" => content.push_str("User: "),
            "assistant" => content.push_str("Assistant: "),
            other => { content.push_str(other); content.push_str(": "); }
        }
        content.push_str(&msg.content);
        if content.len() > cap { break; }
    }
    if content.len() > cap {
        let mut end = cap;
        while !content.is_char_boundary(end) { end -= 1; }
        content.truncate(end);
    }
    content
}

/// Import haystack sessions directly into L2 JSONL (bypass gate)
fn import_haystack_direct(q: &QuestionFull, target_base: chrono::NaiveDateTime) {
    let base = chrono::NaiveDateTime::parse_from_str("2023-05-20 00:00", "%Y-%m-%d %H:%M").unwrap();
    let path = Path::new(TEST_HOME).join("engrams_L2.jsonl");
    let mut file = fs::File::create(&path).unwrap();

    for ((session, sid), date_str) in q.haystack_sessions.iter()
        .zip(&q.haystack_session_ids)
        .zip(&q.haystack_dates)
    {
        let content = session_to_text(session, CONTENT_CAP);
        let created_at = shift_date(date_str, base, target_base);
        let escaped_content = content.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r").replace('\t', "\\t");
        writeln!(file, r#"{{"id":"{}","content":"{}","importance":5,"emotion":"neutral","emotion_score":0.0,"source":"benchmark","tags":["sid:{}"],"access_count":0,"created_at":"{}","layer":"L2","half_life":30}}"#,
            sid, escaped_content, sid, created_at).unwrap();
    }
}

/// Import haystack sessions through gate (only accepted sessions get written)
fn import_haystack_via_gate(q: &QuestionFull) -> (usize, usize, usize, usize) {
    // (total_sessions, accepted, answer_preserved, total_answer_ids)
    // Build answer set with both "answer_X" and "X" forms
    let mut answer_set: HashSet<&str> = HashSet::new();
    for s in &q.answer_session_ids {
        answer_set.insert(s.as_str());
        if let Some(stripped) = s.strip_prefix("answer_") {
            answer_set.insert(stripped);
        }
    }
    let mut total = 0usize;
    let mut accepted = 0usize;
    let mut answer_preserved = 0usize;

    // Track which answer sessions were accepted
    let mut accepted_answer_ids: HashSet<&str> = HashSet::new();

    for (session, sid) in q.haystack_sessions.iter().zip(&q.haystack_session_ids) {
        total += 1;
        let content = session_to_text(session, CONTENT_CAP);

        // Call gate --write
        let output = Command::new("hippocampus")
            .args(["gate", "--message", &content, "--write"])
            .env("HIPPOCAMPUS_HOME", TEST_HOME)
            .env("NO_PROXY", "127.0.0.1,localhost")
            .output();

        match output {
            Ok(out) => {
                if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                    let should = val.get("should_remember").and_then(|v| v.as_bool()).unwrap_or(false);
                    if should {
                        accepted += 1;
                        if answer_set.contains(sid.as_str()) {
                            answer_preserved += 1;
                            accepted_answer_ids.insert(sid.as_str());
                        }
                    }
                }
            }
            Err(_) => {}
        }
    }

    (total, accepted, answer_preserved, q.answer_session_ids.len())
}

fn run_recall(query: &str, top_k: usize) -> serde_json::Value {
    match Command::new("hippocampus")
        .args(["recall", "--query", query, "--top-k", &top_k.to_string(), "--min-score", "0.001"])
        .env("HIPPOCAMPUS_HOME", TEST_HOME)
        .env("NO_PROXY", "127.0.0.1,localhost")
        .output()
    {
        Ok(out) => serde_json::from_slice(&out.stdout).unwrap_or(serde_json::json!({})),
        Err(_) => serde_json::json!({}),
    }
}

fn text_overlap(content: &str, answer: &str) -> f64 {
    let answer_words: HashSet<&str> = answer.split_whitespace().collect();
    if answer_words.is_empty() { return 0.0; }
    let lower = content.to_lowercase();
    let content_words: HashSet<&str> = lower.split_whitespace().collect();
    answer_words.iter().filter(|w| content_words.contains(*w)).count() as f64 / answer_words.len() as f64
}

/// Check recall results against answer session IDs.
/// Handles the "answer_" prefix: answer_session_ids may be "answer_X" while tags are "sid:X".
/// Returns (hits[@1,@3,@5,@10], best_overlap, hit_position)
fn check_answer(recall: &serde_json::Value, answer: &str, sids: &[String]) -> ([bool; 4], f64, Option<usize>) {
    // Build set with both "answer_X" and "X" forms
    let mut sid_set: HashSet<&str> = HashSet::new();
    for s in sids {
        sid_set.insert(s.as_str());
        if let Some(stripped) = s.strip_prefix("answer_") {
            sid_set.insert(stripped); // leak-safe: stripped borrows from s which lives in sids
        }
    }
    let mut hits = [false; 4];
    let mut best_ov = 0.0f64;
    let mut hit_pos = None;

    let results = match recall.get("results").and_then(|v| v.as_array()) {
        Some(r) => r,
        None => return (hits, best_ov, hit_pos),
    };

    for (rank, r) in results.iter().enumerate() {
        let rank = rank + 1;
        if let Some(tags) = r.get("tags").and_then(|v| v.as_array()) {
            for tag in tags {
                if let Some(s) = tag.as_str().and_then(|s| s.strip_prefix("sid:")) {
                    if sid_set.contains(s) {
                        if hit_pos.is_none() { hit_pos = Some(rank); }
                        for (i, k) in [1, 3, 5, 10].iter().enumerate() {
                            if rank <= *k { hits[i] = true; }
                        }
                    }
                }
            }
        }
        if let Some(c) = r.get("content").and_then(|v| v.as_str()) {
            best_ov = best_ov.max(text_overlap(c, answer));
        }
    }
    (hits, best_ov, hit_pos)
}

fn hippo_init() {
    let _ = Command::new("hippocampus")
        .arg("init")
        .env("HIPPOCAMPUS_HOME", TEST_HOME)
        .output();
}

// ── Main ────────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut dataset_path = DEFAULT_DATASET.to_string();
    let mut samples_per_type = DEFAULT_SAMPLES;
    let mut top_k = DEFAULT_TOP_K;
    let mut mode = "full".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--dataset" => { dataset_path = args.get(i + 1).cloned().unwrap_or_default(); i += 2; }
            "--samples" => { samples_per_type = args.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(DEFAULT_SAMPLES); i += 2; }
            "--top-k" => { top_k = args.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(DEFAULT_TOP_K); i += 2; }
            "--mode" => { mode = args.get(i + 1).cloned().unwrap_or_else(|| "full".to_string()); i += 2; }
            "--help" | "-h" => { print_help(); return; }
            _ => { i += 1; }
        }
    }

    let do_recall = mode == "recall" || mode == "full";
    let do_gate = mode == "gate" || mode == "full";

    println!("{}", "=".repeat(72));
    println!("  Hippocampus LongMemEval Benchmark");
    println!("{}", "=".repeat(72));
    println!("  Dataset: {}", dataset_path);
    println!("  Samples/type: {}", if samples_per_type == 0 { "ALL".to_string() } else { samples_per_type.to_string() });
    println!("  Top-K: {}", top_k);
    println!("  Mode: {} {}", mode, if do_recall { "[Recall]" } else { "" }
        .to_string() + if do_gate { " [Gate]" } else { "" });
    println!("{}", "=".repeat(72));

    // ── Pass 1: Lightweight scan ──────────────────────────────────────────
    println!("\n  [Pass 1] Scanning dataset (streaming)...");

    let mut by_type: HashMap<String, Vec<usize>> = HashMap::new();
    let mut total_questions = 0;

    let mut stream = JsonStream::open(&dataset_path);
    while let Some(raw) = stream.next_raw() {
        total_questions += 1;
        if let Ok(light) = serde_json::from_str::<QuestionLight>(&raw) {
            by_type.entry(light.question_type).or_default().push(total_questions - 1);
        }
    }

    let mut types_sorted: Vec<String> = by_type.keys().cloned().collect();
    types_sorted.sort();

    println!("  Total: {} questions", total_questions);
    for qt in &types_sorted {
        println!("    {}: {}", qt, by_type[qt].len());
    }

    // Select indices
    let mut selected_set: HashSet<usize> = HashSet::new();
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    use rand::SeedableRng;
    for qt in &types_sorted {
        let indices = &by_type[qt];
        let n = if samples_per_type == 0 { indices.len() } else { samples_per_type.min(indices.len()) };
        let mut shuffled = indices.clone();
        for j in (1..shuffled.len()).rev() {
            let k = rand::Rng::gen_range(&mut rng, 0..=j);
            shuffled.swap(j, k);
        }
        for idx in shuffled.iter().take(n) {
            selected_set.insert(*idx);
        }
    }
    drop(by_type);

    let num_selected = selected_set.len();
    println!("  Selected {} questions", num_selected);

    // ── Pass 2+3: Stream & process one question at a time (memory-safe) ────
    println!("\n  [Pass 2] Streaming & processing questions one at a time...");

    let target_base = chrono::Local::now().naive_local() - chrono::Duration::days(5);

    // Per-type metrics
    let mut recall_by_type: HashMap<String, RecallMetrics> = HashMap::new();
    let mut gate_by_type: HashMap<String, GateMetrics> = HashMap::new();

    // Per-question results for report
    let mut recall_results: Vec<QRecallResult> = Vec::new();
    let mut gate_results: Vec<QGateResult> = Vec::new();

    let mut stream = JsonStream::open(&dataset_path);
    let mut idx: usize = 0;
    let mut processed: usize = 0;

    while let Some(raw) = stream.next_raw() {
        if selected_set.contains(&idx) {
            if let Ok(q) = serde_json::from_str::<QuestionFull>(&raw) {
                let qt = &q.question_type;
                let q_preview: String = q.question.chars().take(80).collect();
                processed += 1;
                println!("\n  [{}/{}] {} | {}", processed, num_selected, qt, q.question_id);
                println!("    Q: {}...", q_preview);

                // ── Recall test ───────────────────────────────────────────
                if do_recall {
                    setup_clean_env();
                    hippo_init();
                    import_haystack_direct(&q, target_base);

                    let t0 = Instant::now();
                    let recall = run_recall(&q.question, top_k);
                    let lat = t0.elapsed().as_secs_f64();

                    let (hits, overlap, hit_pos) = check_answer(&recall, &q.answer, &q.answer_session_ids);

                    let stats = recall_by_type.entry(qt.clone()).or_default();
                    stats.total += 1;
                    for j in 0..4 { if hits[j] { stats.hit_at[j] += 1; } }
                    if let Some(pos) = hit_pos {
                        stats.mrr_sum += 1.0 / pos as f64;
                    }
                    stats.overlap_scores.push(overlap);
                    stats.latencies.push(lat);

                    let rc = recall.get("results_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    let hit_s = hit_pos.map(|p| format!("HIT@{}", p)).unwrap_or_else(|| "MISS".into());
                    println!("    [Recall] {} results | {} | overlap={:.2} | {:.0}ms", rc, hit_s, overlap, lat * 1000.0);

                    recall_results.push(QRecallResult {
                        question_id: q.question_id.clone(),
                        question_type: qt.clone(),
                        recall_count: rc,
                        hit_position: hit_pos,
                        overlap,
                        latency_ms: lat * 1000.0,
                    });
                }

                // ── Gate test ─────────────────────────────────────────────
                if do_gate {
                    setup_clean_env();
                    hippo_init();

                    let (total_sess, accepted, ans_preserved, ans_total) = import_haystack_via_gate(&q);

                    let t0 = Instant::now();
                    let recall = run_recall(&q.question, top_k);
                    let lat = t0.elapsed().as_secs_f64();

                    let (hits, overlap, hit_pos) = check_answer(&recall, &q.answer, &q.answer_session_ids);

                    let gm = gate_by_type.entry(qt.clone()).or_default();
                    gm.total_sessions += total_sess;
                    gm.accepted_sessions += accepted;
                    gm.answer_sessions_preserved += ans_preserved;
                    gm.total_answer_sessions += ans_total;
                    gm.post_gate_recall.total += 1;
                    for j in 0..4 { if hits[j] { gm.post_gate_recall.hit_at[j] += 1; } }
                    if let Some(pos) = hit_pos {
                        gm.post_gate_recall.mrr_sum += 1.0 / pos as f64;
                    }
                    gm.post_gate_recall.overlap_scores.push(overlap);
                    gm.post_gate_recall.latencies.push(lat);

                    let rc = recall.get("results_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    let hit_s = hit_pos.map(|p| format!("HIT@{}", p)).unwrap_or_else(|| "MISS".into());
                    println!("    [Gate]   {}/{} accepted | {}/{} answer preserved | {} results | {} | {:.0}ms",
                        accepted, total_sess, ans_preserved, ans_total, rc, hit_s, lat * 1000.0);

                    gate_results.push(QGateResult {
                        question_id: q.question_id.clone(),
                        question_type: qt.clone(),
                        total_sessions: total_sess,
                        accepted_sessions: accepted,
                        answer_preserved: ans_preserved,
                        answer_total: ans_total,
                        post_gate_hit_position: hit_pos,
                        post_gate_overlap: overlap,
                        post_gate_latency_ms: lat * 1000.0,
                    });
                }
            }
        }
        idx += 1;
        if processed == num_selected { break; }
    }
    drop(selected_set);

    // ── Report ──────────────────────────────────────────────────────────
    println!("\n{}", "=".repeat(72));
    println!("  BENCHMARK REPORT");
    println!("{}", "=".repeat(72));

    // Aggregate recall metrics
    if do_recall {
        let mut overall = RecallMetrics::default();
        for s in recall_by_type.values() {
            overall.total += s.total;
            for j in 0..4 { overall.hit_at[j] += s.hit_at[j]; }
            overall.mrr_sum += s.mrr_sum;
            overall.overlap_scores.extend_from_slice(&s.overlap_scores);
            overall.latencies.extend_from_slice(&s.latencies);
        }

        println!("\n  ┌── Recall Test ({} questions) ──", overall.total);
        println!("  │");
        for (i, k) in [1, 3, 5, 10].iter().enumerate() {
            println!("  │  Recall@{}: {:>3}/{} = {:5.1}%", k, overall.hit_at[i], overall.total, overall.recall_at(i) * 100.0);
        }
        println!("  │  MRR:       {:.4}", overall.mrr());
        println!("  │  Avg overlap: {:.3}", overall.avg_overlap());
        println!("  │  Avg latency: {:.0}ms", overall.avg_latency_ms());
        println!("  │");
        println!("  │  Per-Type:");
        println!("  │  {}", "-".repeat(78));
        println!("  │  {:<30} {:>5} {:>6} {:>6} {:>6} {:>6} {:>7} {:>7}",
            "Type", "N", "R@1", "R@3", "R@5", "R@10", "MRR", "Latency");
        println!("  │  {}", "-".repeat(78));
        for qt in &types_sorted {
            let s = match recall_by_type.get(qt) { Some(s) => s, None => continue };
            if s.total == 0 { continue; }
            println!("  │  {:<30} {:>5} {:>5.1}% {:>5.1}% {:>5.1}% {:>5.1}% {:>6.3} {:>5.0}ms",
                qt, s.total,
                s.recall_at(0) * 100.0, s.recall_at(1) * 100.0,
                s.recall_at(2) * 100.0, s.recall_at(3) * 100.0,
                s.mrr(), s.avg_latency_ms());
        }
        println!("  │  {}", "-".repeat(78));
        println!("  └──");
    }

    if do_gate {
        let mut overall = GateMetrics::default();
        for g in gate_by_type.values() {
            overall.total_sessions += g.total_sessions;
            overall.accepted_sessions += g.accepted_sessions;
            overall.answer_sessions_preserved += g.answer_sessions_preserved;
            overall.total_answer_sessions += g.total_answer_sessions;
            overall.post_gate_recall.total += g.post_gate_recall.total;
            for j in 0..4 { overall.post_gate_recall.hit_at[j] += g.post_gate_recall.hit_at[j]; }
            overall.post_gate_recall.mrr_sum += g.post_gate_recall.mrr_sum;
            overall.post_gate_recall.overlap_scores.extend_from_slice(&g.post_gate_recall.overlap_scores);
            overall.post_gate_recall.latencies.extend_from_slice(&g.post_gate_recall.latencies);
        }

        println!("\n  ┌── Gate Test ──");
        println!("  │");
        println!("  │  Acceptance rate: {}/{} = {:.1}%",
            overall.accepted_sessions, overall.total_sessions, overall.acceptance_rate() * 100.0);
        println!("  │  Answer preservation: {}/{} = {:.1}%",
            overall.answer_sessions_preserved, overall.total_answer_sessions, overall.answer_preservation_rate() * 100.0);
        println!("  │");
        println!("  │  Post-Gate Recall ({} questions):", overall.post_gate_recall.total);
        for (i, k) in [1, 3, 5, 10].iter().enumerate() {
            println!("  │    Recall@{}: {:>3}/{} = {:5.1}%",
                k, overall.post_gate_recall.hit_at[i], overall.post_gate_recall.total,
                overall.post_gate_recall.recall_at(i) * 100.0);
        }
        println!("  │  MRR:       {:.4}", overall.post_gate_recall.mrr());
        println!("  │  Avg overlap: {:.3}", overall.post_gate_recall.avg_overlap());
        println!("  │  Avg latency: {:.0}ms", overall.post_gate_recall.avg_latency_ms());
        println!("  │");
        println!("  │  Per-Type:");
        println!("  │  {}", "-".repeat(90));
        println!("  │  {:<28} {:>5} {:>5} {:>7} {:>7} {:>6} {:>6} {:>6} {:>6}",
            "Type", "N", "Acc%", "AnsPres", "AnsTot", "R@1", "R@5", "R@10", "MRR");
        println!("  │  {}", "-".repeat(90));
        for qt in &types_sorted {
            let g = match gate_by_type.get(qt) { Some(g) => g, None => continue };
            if g.post_gate_recall.total == 0 { continue; }
            let acc_rate = if g.total_sessions == 0 { 0.0 } else { g.accepted_sessions as f64 / g.total_sessions as f64 * 100.0 };
            println!("  │  {:<28} {:>5} {:>4.0}% {:>6}/{:<4} {:>5.1}% {:>5.1}% {:>5.1}% {:>5.3}",
                qt, g.post_gate_recall.total, acc_rate,
                g.answer_sessions_preserved, g.total_answer_sessions,
                g.post_gate_recall.recall_at(0) * 100.0,
                g.post_gate_recall.recall_at(2) * 100.0,
                g.post_gate_recall.recall_at(3) * 100.0,
                g.post_gate_recall.mrr());
        }
        println!("  │  {}", "-".repeat(90));
        println!("  └──");
    }

    // ── Compare mode ────────────────────────────────────────────────────
    if do_recall && do_gate {
        println!("\n  ┌── Recall vs Gate Comparison ──");
        let recall_overall = aggregate_recall(&recall_by_type);
        let gate_overall = aggregate_recall_from_gate(&gate_by_type);
        println!("  │  {:<20} {:>10} {:>10} {:>10}", "Metric", "Recall", "Gate", "Delta");
        println!("  │  {}", "-".repeat(52));
        for (i, k) in [1, 3, 5, 10].iter().enumerate() {
            let r = recall_overall.recall_at(i) * 100.0;
            let g = gate_overall.recall_at(i) * 100.0;
            let d = g - r;
            println!("  │  {:<20} {:>9.1}% {:>9.1}% {:>+9.1}%", format!("Recall@{}", k), r, g, d);
        }
        let r_mrr = recall_overall.mrr();
        let g_mrr = gate_overall.mrr();
        println!("  │  {:<20} {:>10.4} {:>10.4} {:>+10.4}", "MRR", r_mrr, g_mrr, g_mrr - r_mrr);
        println!("  └──");
    }

    // ── Save JSON report ────────────────────────────────────────────────
    let results_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("results");
    let _ = fs::create_dir_all(&results_dir);

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let report_path = results_dir.join(format!("report_{}.json", timestamp));

    let samples_str = if samples_per_type == 0 { "all".to_string() } else { samples_per_type.to_string() };
    let mut report = serde_json::json!({
        "timestamp": chrono::Local::now().to_rfc3339(),
        "config": {
            "dataset": dataset_path,
            "samples_per_type": samples_str,
            "top_k": top_k,
            "mode": mode,
            "total_questions": num_selected,
        },
    });

    if do_recall {
        let overall = aggregate_recall(&recall_by_type);
        report["recall"] = serde_json::json!({
            "overall": metrics_to_json(&overall),
            "per_type": types_sorted.iter().map(|qt| {
                let s = recall_by_type.get(qt);
                (qt.clone(), s.map(|s| metrics_to_json(s)).unwrap_or(serde_json::json!(null)))
            }).collect::<HashMap<_, _>>(),
            "results": recall_results.iter().map(|r| serde_json::json!({
                "question_id": r.question_id,
                "question_type": r.question_type,
                "recall_count": r.recall_count,
                "hit_position": r.hit_position,
                "overlap": (r.overlap * 1000.0).round() / 1000.0,
                "latency_ms": (r.latency_ms * 10.0).round() / 10.0,
            })).collect::<Vec<_>>(),
        });
    }

    if do_gate {
        let mut overall = GateMetrics::default();
        for g in gate_by_type.values() {
            overall.total_sessions += g.total_sessions;
            overall.accepted_sessions += g.accepted_sessions;
            overall.answer_sessions_preserved += g.answer_sessions_preserved;
            overall.total_answer_sessions += g.total_answer_sessions;
            overall.post_gate_recall.total += g.post_gate_recall.total;
            for j in 0..4 { overall.post_gate_recall.hit_at[j] += g.post_gate_recall.hit_at[j]; }
            overall.post_gate_recall.mrr_sum += g.post_gate_recall.mrr_sum;
            overall.post_gate_recall.overlap_scores.extend_from_slice(&g.post_gate_recall.overlap_scores);
            overall.post_gate_recall.latencies.extend_from_slice(&g.post_gate_recall.latencies);
        }

        report["gate"] = serde_json::json!({
            "acceptance_rate": overall.acceptance_rate(),
            "answer_preservation_rate": overall.answer_preservation_rate(),
            "total_sessions": overall.total_sessions,
            "accepted_sessions": overall.accepted_sessions,
            "answer_preserved": overall.answer_sessions_preserved,
            "answer_total": overall.total_answer_sessions,
            "post_gate_recall": metrics_to_json(&overall.post_gate_recall),
            "per_type": types_sorted.iter().map(|qt| {
                let g = gate_by_type.get(qt);
                (qt.clone(), g.map(|g| serde_json::json!({
                    "acceptance_rate": g.acceptance_rate(),
                    "answer_preservation_rate": g.answer_preservation_rate(),
                    "post_gate_recall": metrics_to_json(&g.post_gate_recall),
                })).unwrap_or(serde_json::json!(null)))
            }).collect::<HashMap<_, _>>(),
            "results": gate_results.iter().map(|r| serde_json::json!({
                "question_id": r.question_id,
                "question_type": r.question_type,
                "total_sessions": r.total_sessions,
                "accepted_sessions": r.accepted_sessions,
                "answer_preserved": r.answer_preserved,
                "answer_total": r.answer_total,
                "post_gate_hit_position": r.post_gate_hit_position,
                "post_gate_overlap": (r.post_gate_overlap * 1000.0).round() / 1000.0,
                "post_gate_latency_ms": (r.post_gate_latency_ms * 10.0).round() / 10.0,
            })).collect::<Vec<_>>(),
        });
    }

    fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("\n  Report saved: {}", report_path.display());
}

// ── Utility ─────────────────────────────────────────────────────────────────

fn aggregate_recall(by_type: &HashMap<String, RecallMetrics>) -> RecallMetrics {
    let mut overall = RecallMetrics::default();
    for s in by_type.values() {
        overall.total += s.total;
        for j in 0..4 { overall.hit_at[j] += s.hit_at[j]; }
        overall.mrr_sum += s.mrr_sum;
        overall.overlap_scores.extend_from_slice(&s.overlap_scores);
        overall.latencies.extend_from_slice(&s.latencies);
    }
    overall
}

fn aggregate_recall_from_gate(by_type: &HashMap<String, GateMetrics>) -> RecallMetrics {
    let mut overall = RecallMetrics::default();
    for g in by_type.values() {
        let s = &g.post_gate_recall;
        overall.total += s.total;
        for j in 0..4 { overall.hit_at[j] += s.hit_at[j]; }
        overall.mrr_sum += s.mrr_sum;
        overall.overlap_scores.extend_from_slice(&s.overlap_scores);
        overall.latencies.extend_from_slice(&s.latencies);
    }
    overall
}

fn metrics_to_json(m: &RecallMetrics) -> serde_json::Value {
    serde_json::json!({
        "total": m.total,
        "recall_at_1": m.recall_at(0),
        "recall_at_3": m.recall_at(1),
        "recall_at_5": m.recall_at(2),
        "recall_at_10": m.recall_at(3),
        "mrr": m.mrr(),
        "avg_overlap": m.avg_overlap(),
        "avg_latency_ms": m.avg_latency_ms(),
    })
}

fn print_help() {
    eprintln!("Hippocampus LongMemEval Benchmark");
    eprintln!();
    eprintln!("Usage: bench-longmemeval [OPTIONS]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --dataset PATH   Dataset JSON path (default: {})", DEFAULT_DATASET);
    eprintln!("  --samples N      Samples per type, 0=all (default: {})", DEFAULT_SAMPLES);
    eprintln!("  --top-k N        Recall top-K (default: {})", DEFAULT_TOP_K);
    eprintln!("  --mode MODE      recall | gate | full (default: full)");
    eprintln!("  --help           Show this help");
}
