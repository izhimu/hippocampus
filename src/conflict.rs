/// Conflict detection and resolution — SleepGate
///
/// Detects contradictory memories by semantic clustering (SimHash),
/// then resolves by keeping the newest and deprecating older entries.

use crate::engram::Engram;
use crate::simhash;
use crate::store::EngramStore;

pub struct ConflictResolver {
    hamming_threshold: u32,
    max_clusters: usize,
}

#[derive(serde::Serialize)]
pub struct ConflictReport {
    pub clusters_found: usize,
    pub conflicts_resolved: usize,
    pub deprecated: usize,
    pub merged: usize,
}

impl ConflictResolver {
    pub fn new(hamming_threshold: u32) -> Self {
        Self {
            hamming_threshold,
            max_clusters: 100,
        }
    }

    /// Run conflict detection and resolution on all engrams
    pub fn resolve(&self, store: &EngramStore) -> ConflictReport {
        let all = match store.read_all() {
            Ok(e) => e,
            Err(_) => return ConflictReport {
                clusters_found: 0, conflicts_resolved: 0, deprecated: 0, merged: 0,
            },
        };

        // 1. Cluster by SimHash proximity
        let clusters = self.cluster_by_fingerprint(&all);

        let mut report = ConflictReport {
            clusters_found: clusters.len(),
            conflicts_resolved: 0,
            deprecated: 0,
            merged: 0,
        };

        // 2. For each cluster with 2+ members, resolve conflicts
        for cluster in &clusters {
            if cluster.len() < 2 {
                continue;
            }

            // Sort by created_at descending (newest first)
            let mut sorted = cluster.clone();
            sorted.sort_by(|a, b| b.created_at.cmp(&a.created_at));

            // Check for actual conflicts (numeric/path contradictions)
            let has_conflict = self.detect_conflict_in_cluster(&sorted);

            if has_conflict {
                report.conflicts_resolved += 1;

                // Strategy: keep newest, deprecate older
                for old in &sorted[1..] {
                    let old_id = old.id.clone();
                    let _ = store.update(&old_id, |e| {
                        e.importance = e.importance.saturating_sub(2);
                        // Add deprecated tag
                        if !e.tags.contains(&"deprecated".to_string()) {
                            e.tags.push("deprecated".into());
                        }
                    });
                    report.deprecated += 1;
                }
            }
        }

        report
    }

    /// Cluster engrams by SimHash fingerprint proximity
    /// Optimized: sort by fingerprint, then use sliding window comparison
    fn cluster_by_fingerprint<'a>(&self, engrams: &'a [Engram]) -> Vec<Vec<&'a Engram>> {
        // Pre-compute fingerprints and sort by fingerprint value
        let mut indexed: Vec<(u64, usize)> = engrams.iter().enumerate().map(|(i, e)| {
            let fp = if e.fingerprint != 0 { e.fingerprint } else { simhash::simhash(&e.content) };
            (fp, i)
        }).collect();
        indexed.sort_by_key(|(fp, _)| *fp);

        let n = indexed.len();
        let mut assigned = vec![false; n];
        let mut clusters = vec![];

        for i in 0..n {
            if assigned[i] {
                continue;
            }
            let mut cluster = vec![indexed[i].1];
            assigned[i] = true;
            let fp_i = indexed[i].0;

            // Only compare within a window of sorted fingerprints
            for j in (i + 1)..n {
                if assigned[j] { continue; }
                // Early termination: if sorted fingerprints are too far apart numerically,
                // hamming distance will also be large
                if indexed[j].0.wrapping_sub(fp_i) > (1u64 << (64 - self.hamming_threshold)) {
                    break;
                }
                if simhash::hamming_distance(fp_i, indexed[j].0) <= self.hamming_threshold {
                    cluster.push(indexed[j].1);
                    assigned[j] = true;
                }
            }

            if cluster.len() >= 2 {
                clusters.push(cluster.iter().map(|&idx| &engrams[idx]).collect());
            }

            if clusters.len() >= self.max_clusters {
                break;
            }
        }

        clusters
    }

    /// Detect if a cluster contains actual contradictions
    fn detect_conflict_in_cluster(&self, cluster: &[&Engram]) -> bool {
        if cluster.len() < 2 {
            return false;
        }

        // Extract numeric values from each engram
        let nums: Vec<Vec<f64>> = cluster.iter()
            .map(|e| extract_numbers(&e.content))
            .collect();

        // Check if any two have overlapping context but different numbers
        for i in 0..nums.len() {
            for j in (i + 1)..nums.len() {
                if !nums[i].is_empty() && !nums[j].is_empty() {
                    // If contents are semantically similar (same cluster) but numbers differ
                    if numbers_conflict(&nums[i], &nums[j]) {
                        return true;
                    }
                }
            }
        }

        // Check for explicit negation patterns
        for i in 0..cluster.len() {
            for j in (i + 1)..cluster.len() {
                if has_negation_conflict(&cluster[i].content, &cluster[j].content) {
                    return true;
                }
            }
        }

        false
    }
}

/// Extract numeric values from text
fn extract_numbers(text: &str) -> Vec<f64> {
    let mut nums = vec![];
    let mut buf = String::new();
    let mut has_dot = false;

    for ch in text.chars() {
        if ch.is_ascii_digit() {
            buf.push(ch);
        } else if ch == '.' && !has_dot && !buf.is_empty() {
            buf.push(ch);
            has_dot = true;
        } else {
            if !buf.is_empty() {
                if let Ok(n) = buf.parse() {
                    nums.push(n);
                }
                buf.clear();
                has_dot = false;
            }
        }
    }
    if !buf.is_empty() {
        if let Ok(n) = buf.parse() {
            nums.push(n);
        }
    }
    nums
}

/// Check if two sets of numbers conflict (overlap but differ)
fn numbers_conflict(a: &[f64], b: &[f64]) -> bool {
    // Simple: if both have similar count but values differ significantly
    if a.len() != b.len() || a.is_empty() {
        return false;
    }
    for (x, y) in a.iter().zip(b.iter()) {
        if (*x - *y).abs() > 0.01 {
            return true;
        }
    }
    false
}

/// Check for negation patterns between two texts
fn has_negation_conflict(a: &str, b: &str) -> bool {
    const NEGATIONS: &[&str] = &["不", "没", "非", "无", "别", "未", "不要", "不是", "没有"];
    for neg in NEGATIONS {
        if (a.contains(neg) && !b.contains(neg)) || (!a.contains(neg) && b.contains(neg)) {
            // Check if the rest of the content is similar
            let a_clean = a.replace(neg, "");
            let b_clean = b.replace(neg, "");
            if a_clean == b_clean {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_numbers() {
        let nums = extract_numbers("配置端口为8080，超时30秒");
        assert!(nums.contains(&8080.0));
        assert!(nums.contains(&30.0));
    }

    #[test]
    fn test_extract_numbers_float() {
        let nums = extract_numbers("价格是99.5元");
        assert!(nums.contains(&99.5));
    }

    #[test]
    fn test_numbers_conflict_same() {
        assert!(!numbers_conflict(&[8080.0], &[8080.0]));
    }

    #[test]
    fn test_numbers_conflict_different() {
        assert!(numbers_conflict(&[8080.0], &[9090.0]));
    }

    #[test]
    fn test_negation_conflict() {
        assert!(has_negation_conflict("需要备份", "不需要备份"));
        assert!(!has_negation_conflict("需要备份", "需要备份"));
    }
}
