/// Cognitive Map — Successor Representation (SR) graph for engram-to-engram relationships
///
/// Maintains a sparse SR matrix where M[A][B] represents the expected future
/// occupancy of B given current state A. Updated via TD learning during
/// sequential recall, and used for spreading activation during search.

use std::collections::HashMap;

pub struct CognitiveMap {
    /// Sparse SR matrix: M[source][target] = predictive weight
    matrix: HashMap<String, HashMap<String, f64>>,
    /// Maximum number of nodes to track
    max_nodes: usize,
    /// TD learning rate
    alpha: f64,
    /// Discount factor for TD update
    gamma: f64,
}

impl CognitiveMap {
    pub fn new(max_nodes: usize) -> Self {
        Self {
            matrix: HashMap::new(),
            max_nodes,
            alpha: 0.1,
            gamma: 0.9,
        }
    }

    /// TD learning update: when transitioning from A to B
    /// M[A][j] += α * (indicator_B[j] + γ * M[B][j] - M[A][j])
    /// Optimized: collect deltas first, then apply (avoids cloning entire row)
    pub fn td_update(&mut self, from_id: &str, to_id: &str) {
        if from_id == to_id {
            return;
        }

        self.ensure_node(from_id);
        self.ensure_node(to_id);

        let to_id_owned = to_id.to_string();
        let gamma = self.gamma;
        let alpha = self.alpha;

        // Collect deltas without cloning
        let (deltas, to_row_self_val): (Vec<(String, f64)>, f64) = {
            let to_row = self.matrix.get(&to_id_owned);
            let from_row = self.matrix.get(from_id).unwrap();
            let self_val = to_row.and_then(|r| r.get(&to_id_owned)).copied().unwrap_or(0.0);
            let ds: Vec<(String, f64)> = from_row.iter().map(|(j, from_val)| {
                let indicator = if *j == to_id_owned { 1.0 } else { 0.0 };
                let to_val = to_row.and_then(|r| r.get(j)).copied().unwrap_or(0.0);
                let delta = alpha * (indicator + gamma * to_val - *from_val);
                (j.clone(), delta)
            }).collect();
            (ds, self_val)
        };

        // Apply deltas
        let from_row = self.matrix.get_mut(from_id).unwrap();
        for (j, delta) in deltas {
            *from_row.entry(j).or_insert(0.0) += delta;
        }

        // Ensure the to_id entry exists in from_row
        let from_row = self.matrix.get_mut(from_id).unwrap();
        if !from_row.contains_key(&to_id_owned) {
            from_row.insert(to_id_owned, alpha * (1.0 + gamma * to_row_self_val));
        }
    }

    /// Hebbian consolidation: strengthen edge A→B by rate η
    pub fn consolidate_edge(&mut self, from_id: &str, to_id: &str, rate: f64) {
        if let Some(row) = self.matrix.get_mut(from_id) {
            if let Some(w) = row.get_mut(to_id) {
                *w *= 1.0 + rate;
            }
        }
    }

    /// Get related engrams sorted by SR weight (descending)
    pub fn get_related(&self, engram_id: &str, top_k: usize) -> Vec<(String, f64)> {
        let mut related: Vec<(String, f64)> = self.matrix
            .get(engram_id)
            .map(|row| {
                row.iter()
                    .filter(|(_, &w)| w > 0.01)
                    .map(|(k, &v)| (k.clone(), v))
                    .collect()
            })
            .unwrap_or_default();

        related.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        related.truncate(top_k);
        related
    }

    /// Random walk from a starting node, following SR weights as probabilities
    pub fn random_walk(&self, start_id: &str, steps: usize) -> Vec<String> {
        let mut path = Vec::with_capacity(steps);
        let mut current = start_id.to_string();

        for _ in 0..steps {
            let neighbors = self.get_related(&current, 10);
            if neighbors.is_empty() {
                break;
            }

            // Weighted sampling
            let total_weight: f64 = neighbors.iter().map(|(_, w)| w).sum();
            if total_weight <= 0.0 {
                break;
            }

            let mut rng_val = simple_random() * total_weight;
            let mut chosen = neighbors[0].0.clone();
            for (id, w) in &neighbors {
                rng_val -= w;
                if rng_val <= 0.0 {
                    chosen = id.clone();
                    break;
                }
            }

            path.push(chosen.clone());
            current = chosen;
        }

        path
    }

    /// Prune low-weight edges and excess nodes
    pub fn prune(&mut self, min_weight: f64) {
        for row in self.matrix.values_mut() {
            row.retain(|_, w| *w >= min_weight);
        }

        // Remove empty rows
        self.matrix.retain(|_, row| !row.is_empty());

        // Evict lowest-degree nodes if over capacity
        while self.matrix.len() > self.max_nodes {
            let min_node = self.matrix
                .keys()
                .min_by_key(|k| {
                    self.matrix.get(*k).map(|r| r.len()).unwrap_or(0)
                })
                .cloned();
            if let Some(node) = min_node {
                self.matrix.remove(&node);
                for row in self.matrix.values_mut() {
                    row.remove(&node);
                }
            } else {
                break;
            }
        }
    }

    /// Get all node IDs in the map
    pub fn node_ids(&self) -> Vec<String> {
        self.matrix.keys().cloned().collect()
    }

    /// Number of nodes
    pub fn len(&self) -> usize {
        self.matrix.len()
    }

    pub fn is_empty(&self) -> bool {
        self.matrix.is_empty()
    }

    fn ensure_node(&mut self, id: &str) {
        if !self.matrix.contains_key(id) {
            self.matrix.insert(id.to_string(), HashMap::new());
        }
    }

    /// Serialize to JSON-compatible structure
    pub fn to_json_value(&self) -> serde_json::Value {
        let mut outer = serde_json::Map::new();
        for (k, row) in &self.matrix {
            let mut inner = serde_json::Map::new();
            for (k2, v) in row {
                inner.insert(k2.clone(), serde_json::Value::from(*v));
            }
            outer.insert(k.clone(), serde_json::Value::Object(inner));
        }
        serde_json::Value::Object(outer)
    }

    /// Deserialize from JSON
    pub fn from_json_value(val: &serde_json::Value, max_nodes: usize) -> Self {
        let mut matrix = HashMap::new();
        if let Some(obj) = val.as_object() {
            for (k, v) in obj {
                if let Some(inner) = v.as_object() {
                    let mut row = HashMap::new();
                    for (k2, v2) in inner {
                        if let Some(w) = v2.as_f64() {
                            row.insert(k2.clone(), w);
                        }
                    }
                    matrix.insert(k.clone(), row);
                }
            }
        }
        Self {
            matrix,
            max_nodes,
            alpha: 0.1,
            gamma: 0.9,
        }
    }

    /// Load from file
    pub fn load(path: &std::path::Path, max_nodes: usize) -> std::io::Result<Self> {
        if !path.exists() {
            return Ok(Self::new(max_nodes));
        }
        let data = std::fs::read_to_string(path)?;
        let val: serde_json::Value = serde_json::from_str(&data).unwrap_or(serde_json::Value::Object(Default::default()));
        Ok(Self::from_json_value(&val, max_nodes))
    }

    /// Save to file
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let val = self.to_json_value();
        let data = serde_json::to_string_pretty(&val)?;
        std::fs::write(path, data)
    }
}

/// Use shared fast_random_f64 for weighted sampling
fn simple_random() -> f64 {
    crate::util::fast_random_f64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_td_update_creates_edge() {
        let mut map = CognitiveMap::new(100);
        map.td_update("A", "B");
        let related = map.get_related("A", 10);
        assert!(related.iter().any(|(id, _)| id == "B"), "A→B edge should exist");
    }

    #[test]
    fn test_td_update_no_self_loop() {
        let mut map = CognitiveMap::new(100);
        map.td_update("A", "A");
        assert!(map.is_empty());
    }

    #[test]
    fn test_multiple_updates_strengthen() {
        let mut map = CognitiveMap::new(100);
        map.td_update("A", "B");
        map.td_update("A", "B");
        map.td_update("A", "B");
        let related = map.get_related("A", 10);
        let w = related.iter().find(|(id, _)| id == "B").map(|(_, w)| *w).unwrap_or(0.0);
        assert!(w > 0.1, "Multiple updates should strengthen the edge, got {}", w);
    }

    #[test]
    fn test_prune_removes_weak_edges() {
        let mut map = CognitiveMap::new(100);
        map.td_update("A", "B");
        // Manually set a very weak edge
        if let Some(row) = map.matrix.get_mut("A") {
            row.insert("C".into(), 0.0001);
        }
        map.prune(0.01);
        let related = map.get_related("A", 10);
        assert!(!related.iter().any(|(id, _)| id == "C"), "Weak edge should be pruned");
        assert!(related.iter().any(|(id, _)| id == "B"), "Strong edge should remain");
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut map = CognitiveMap::new(100);
        map.td_update("A", "B");
        map.td_update("B", "C");
        let json = map.to_json_value();
        let map2 = CognitiveMap::from_json_value(&json, 100);
        let related = map2.get_related("A", 10);
        assert!(related.iter().any(|(id, _)| id == "B"));
    }

    #[test]
    fn test_get_related_sorted() {
        let mut map = CognitiveMap::new(100);
        map.td_update("A", "B");
        map.td_update("A", "C");
        map.td_update("A", "B"); // B should be stronger
        let related = map.get_related("A", 10);
        assert_eq!(related[0].0, "B", "Strongest edge should be first");
    }
}
