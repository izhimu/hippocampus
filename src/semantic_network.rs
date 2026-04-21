/// semantic_network — 扩散激活 + Hebbian Learning

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct Node {
    activation: f64,
    connections: HashMap<String, f64>,
}

pub struct SemanticNetwork {
    nodes: HashMap<String, Node>,
    network_path: String,
}

impl SemanticNetwork {
    pub fn new(network_path: impl Into<String>) -> Self {
        let path = network_path.into();
        let nodes = Self::load_json(&path);
        Self { nodes, network_path: path }
    }

    fn load_json(path: &str) -> HashMap<String, Node> {
        if !Path::new(path).exists() {
            return HashMap::new();
        }
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        if let Some(parent) = Path::new(&self.network_path).parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.nodes).unwrap_or_default();
        fs::write(&self.network_path, json)
    }

    /// Hebbian Learning：同一上下文中出现的词互相增强连接
    pub fn co_activate(&mut self, words: &[String]) {
        // 全局衰减 ×0.95
        for node in self.nodes.values_mut() {
            node.activation *= 0.95;
        }

        for i in 0..words.len() {
            let w1 = &words[i];
            if w1.is_empty() || w1.len() < 2 || is_stop_word(w1) {
                continue;
            }
            self.nodes.entry(w1.clone()).or_insert_with(|| Node {
                activation: 1.0,
                connections: HashMap::new(),
            }).activation = 1.0;

            for j in (i + 1)..words.len() {
                let w2 = &words[j];
                if w2.is_empty() || w2.len() < 2 || w1 == w2 || is_stop_word(w2) {
                    continue;
                }
                self.nodes.entry(w2.clone()).or_insert_with(|| Node {
                    activation: 1.0,
                    connections: HashMap::new(),
                });

                // Hebbian: 共现 +0.1，上限 1.0
                let cur1 = self.nodes[w1].connections.get(w2).copied().unwrap_or(0.0);
                self.nodes.get_mut(w1).unwrap().connections.insert(w2.clone(), (cur1 + 0.1).min(1.0));

                let cur2 = self.nodes[w2].connections.get(w1).copied().unwrap_or(0.0);
                self.nodes.get_mut(w2).unwrap().connections.insert(w1.clone(), (cur2 + 0.1).min(1.0));
            }
        }
    }

    /// 一阶直接连接 + 二阶扩散（引入侧向抑制）
    pub fn get_associations(&self, word: &str, threshold: f64) -> Vec<(String, f64)> {
        let mut visited = HashSet::new();
        visited.insert(word.to_string());
        let mut raw_results = vec![];

        // 1. 获取一阶关联
        if let Some(node) = self.nodes.get(word) {
            for (w, &strength) in &node.connections {
                raw_results.push((w.clone(), strength, 1)); // 1 代表一阶
            }
        }

        // 2. 获取二阶关联
        if let Some(node) = self.nodes.get(word) {
            for (w1, &s1) in &node.connections {
                if s1 >= 0.4 { // 只有足够强的连接才允许二阶扩散
                    if let Some(n1) = self.nodes.get(w1) {
                        for (w2, &s2) in &n1.connections {
                            if w2 != word {
                                let score = s1 * s2 * 0.4; // 二阶衰减
                                raw_results.push((w2.clone(), score, 2));
                            }
                        }
                    }
                }
            }
        }

        if raw_results.is_empty() { return vec![]; }

        // 3. 🧠 侧向抑制 (Lateral Inhibition)
        // 找到最强的关联度
        let max_strength = raw_results.iter().map(|r| r.1).fold(0.0, f64::max);
        
        let mut final_results = vec![];
        let mut seen_max = HashMap::new();

        for (w, s, _level) in raw_results {
            // 抑制算法：弱于最强项一定比例的连接被进一步压制（边缘对比度增强）
            // 如果某连接强度只有最强的 30%，则视其为噪点，加速衰减
            let inhibition_factor = if s < max_strength * 0.4 { 0.2 } else { 1.0 };
            let final_s = (s * inhibition_factor * 1000.0).round() / 1000.0;
            
            if final_s >= threshold {
                let entry = seen_max.entry(w.clone()).or_insert(0.0);
                if final_s > *entry {
                    *entry = final_s;
                }
            }
        }

        for (w, s) in seen_max {
            final_results.push((w, s));
        }

        final_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        final_results
    }

    /// 查询扩展（引入注意力竞争）
    pub fn expand_query(&self, tokens: &[String], top_k: usize) -> Vec<String> {
        let mut candidates: HashMap<String, f64> = HashMap::new();
        let original_tokens: HashSet<String> = tokens.iter().cloned().collect();

        for t in tokens {
            // 降低阈值获取更多候选，但在后续进行竞争过滤
            for (assoc, strength) in self.get_associations(t, 0.2) {
                if !original_tokens.contains(&assoc) {
                    // 🧠 共激活增强：如果多个 token 指向同一个关联词，累加权重
                    *candidates.entry(assoc).or_insert(0.0) += strength;
                }
            }
        }

        let mut final_candidates: Vec<(String, f64)> = candidates.into_iter().collect();
        // 根据最终累加权重排序
        final_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        final_candidates.into_iter()
            .take(top_k)
            .map(|(w, _)| w)
            .collect()
    }

    /// 突触修剪
    pub fn decay_all(&mut self) {
        let mut to_remove = vec![];

        for (w, node) in &mut self.nodes {
            let mut dead_conns = vec![];
            for (cw, strength) in &mut node.connections {
                *strength *= 0.99;
                if *strength < 0.05 {
                    dead_conns.push(cw.clone());
                }
            }
            for cw in dead_conns {
                node.connections.remove(&cw);
                to_remove.push((w.clone(), cw));
            }
            node.activation *= 0.99;
        }

        // 清理空节点
        self.nodes.retain(|_, n| !n.connections.is_empty() || n.activation >= 0.01);
    }

    /// 节点数和边数
    pub fn stats(&self) -> (usize, usize) {
        let nodes = self.nodes.len();
        let edges = self.nodes.values().map(|n| n.connections.len()).sum::<usize>() / 2;
        (nodes, edges)
    }
}

fn is_stop_word(word: &str) -> bool {
    crate::stop_words::is_stop_word(word)
}
