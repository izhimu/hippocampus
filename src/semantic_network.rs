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

    /// 一阶直接连接 + 二阶扩散（衰减×0.5）
    pub fn get_associations(&self, word: &str, threshold: f64) -> Vec<(String, f64)> {
        let mut visited = HashSet::new();
        visited.insert(word.to_string());
        let mut result = vec![];

        // 一阶
        if let Some(node) = self.nodes.get(word) {
            for (w, &strength) in &node.connections {
                if strength >= threshold {
                    result.push((w.clone(), strength));
                    visited.insert(w.clone());
                }
            }
        }

        // 二阶扩散
        if let Some(node) = self.nodes.get(word) {
            for (w1, &s1) in &node.connections {
                if s1 >= 0.2 {
                    if let Some(n1) = self.nodes.get(w1) {
                        for (w2, &s2) in &n1.connections {
                            if !visited.contains(w2) && w2 != word {
                                let score = (s1 * s2 * 0.5 * 1000.0).round() / 1000.0;
                                if score >= 0.05 {
                                    result.push((w2.clone(), score));
                                    visited.insert(w2.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        result
    }

    /// 查询扩展
    pub fn expand_query(&self, tokens: &[String], top_k: usize) -> Vec<String> {
        let mut added: HashSet<String> = tokens.iter().cloned().collect();
        let mut extra = vec![];

        for t in tokens {
            for (assoc, _) in self.get_associations(t, 0.3) {
                if !added.contains(&assoc) {
                    added.insert(assoc.clone());
                    extra.push(assoc);
                    if extra.len() >= top_k {
                        return extra;
                    }
                }
            }
        }
        extra
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
    const STOPS: &[&str] = &[
        "的","了","在","是","我","你","他","她","它","们","这","那","有","不","也","都","就",
        "会","可以","要","和","与","或","但","而","所","如","为","从","到","被","把","让","给",
        "用","没","之","等","中","个","上","下","里","去","来","过","对","很","更","最","已",
        "于","及","其","又","并","或","还","将","只","因","则","以","至","该","些","么","啊",
        "吧","呢","吗","哦","嗯","哈","嘛","这个","那个","一个","什么","怎么","没有","不是",
        "我们","他们","如果","但是","因为","所以","或者","虽然",
    ];
    STOPS.contains(&word)
}
