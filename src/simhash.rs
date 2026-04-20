/// SimHash — 64-bit locality-sensitive hashing for approximate semantic matching
///
/// Uses CJK n-gram tokenization (same as search module) to generate fingerprints,
/// then computes Hamming distance for similarity.

use crate::search::tokenize;

/// Generate a 64-bit SimHash fingerprint from text
pub fn simhash(text: &str) -> u64 {
    let tokens = tokenize(text);
    if tokens.is_empty() {
        return 0;
    }

    let mut counts = [0i64; 64];

    for token in &tokens {
        let hash = fnv1a_64(token.as_bytes());
        for i in 0..64 {
            if hash & (1u64 << i) != 0 {
                counts[i] += 1;
            } else {
                counts[i] -= 1;
            }
        }
    }

    let mut fingerprint: u64 = 0;
    for i in 0..64 {
        if counts[i] > 0 {
            fingerprint |= 1u64 << i;
        }
    }
    fingerprint
}

/// Compute Hamming distance between two 64-bit fingerprints
pub fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// Convert Hamming distance to similarity score in [0, 1]
/// distance=0 → 1.0, distance=64 → 0.0
pub fn hamming_similarity(a: u64, b: u64) -> f64 {
    1.0 - hamming_distance(a, b) as f64 / 64.0
}

/// FNV-1a 64-bit hash
fn fnv1a_64(data: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simhash_identical() {
        let fp1 = simhash("今天天气很好");
        let fp2 = simhash("今天天气很好");
        assert_eq!(hamming_distance(fp1, fp2), 0);
        assert!((hamming_similarity(fp1, fp2) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_simhash_similar() {
        let fp1 = simhash("今天天气很好适合出去玩");
        let fp2 = simhash("今天天气不错适合出去散步");
        let dist = hamming_distance(fp1, fp2);
        assert!(dist < 20, "Similar texts should have low hamming distance, got {}", dist);
    }

    #[test]
    fn test_simhash_different() {
        let fp1 = simhash("数据库连接配置信息");
        let fp2 = simhash("今天天气很好适合出去玩");
        let sim = hamming_similarity(fp1, fp2);
        assert!(sim < 0.7, "Different texts should have lower similarity, got {}", sim);
    }

    #[test]
    fn test_simhash_deterministic() {
        let fp1 = simhash("测试内容一致性");
        let fp2 = simhash("测试内容一致性");
        assert_eq!(fp1, fp2);
    }
}
