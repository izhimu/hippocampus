/// ACT-R base-level activation + scoring formulas

/// 根据重要性返回半衰期天数
pub fn half_life_for_importance(imp: i32) -> i32 {
    match imp {
        ..=3 => 7,
        4..=6 => 30,
        7..=9 => 90,
        10..=i32::MAX => 180,
    }
}

/// ACT-R base-level activation: B_i = ln(Σ t_k^(-d))
/// where d is decay rate (default 0.5), t_k is hours since k-th access
/// Returns the raw activation value (log domain)
pub fn actr_activation(access_history: &[String], decay_rate: f64) -> f64 {
    if access_history.is_empty() {
        return f64::NEG_INFINITY;
    }

    use crate::search::hours_since;

    let sum: f64 = access_history
        .iter()
        .filter_map(|ts| {
            let hours = hours_since(ts);
            if hours <= 0.0 {
                // Just accessed: use a small epsilon to avoid infinity
                Some(0.001_f64.powf(-decay_rate))
            } else {
                Some(hours.powf(-decay_rate))
            }
        })
        .sum();

    if sum <= 0.0 {
        f64::NEG_INFINITY
    } else {
        sum.ln()
    }
}

/// Normalize ACT-R activation to [0, 1] for use as a decay factor
/// Uses sigmoid mapping centered around typical activation range
pub fn actr_decay_factor(access_history: &[String], decay_rate: f64) -> f64 {
    if access_history.is_empty() {
        return 1.0; // No history = no decay (freshly imported/legacy engrams)
    }
    let activation = actr_activation(access_history, decay_rate);
    if activation.is_infinite() && activation.is_sign_negative() {
        return 0.0;
    }
    // Sigmoid normalization: maps ln-domain to (0, 1)
    // Center around 0, with spread of 2.0
    1.0 / (1.0 + (-activation / 2.0).exp())
}

/// Legacy exponential decay (kept for backward compat and as fallback)
pub fn decay(days_ago: f64, half_life: f64) -> f64 {
    let hl = half_life.max(1.0);
    (-days_ago / hl).exp()
}

/// Combined scoring with ACT-R activation
/// Uses ACT-R decay if access_history is available, falls back to no decay
pub fn final_score_actr(
    bm25_score: f64,
    importance: u32,
    access_count: u32,
    access_history: &[String],
    _half_life: f64,
    decay_rate: f64,
) -> f64 {
    let d = if access_history.is_empty() {
        1.0
    } else {
        actr_decay_factor(access_history, decay_rate)
    };
    // Log compression of BM25: acts as regularizer to prevent noisy high-BM25
    // results from dominating. Dynamic range: BM25 0→10 maps to 0.0→1.44
    let rel_score = (1.0 + bm25_score).ln() * 0.6;
    let imp_score = importance as f64 * 0.04;
    let freq_score = (1.0 + access_count as f64).ln() * 0.05;
    (rel_score + imp_score + freq_score) * d
}

/// Legacy final_score kept for backward compatibility
pub fn final_score(bm25_score: f64, importance: u32, access_count: u32, days_ago: f64, half_life: f64) -> f64 {
    let d = decay(days_ago, half_life);
    let rel_score = (1.0 + bm25_score).ln() * 0.6;
    let imp_score = importance as f64 * 0.04;
    let freq_score = (1.0 + access_count as f64).ln() * 0.05;
    (rel_score + imp_score + freq_score) * d
}

/// LTP: every 5 accesses, half_life × 1.2
pub fn ltp_boost(current_half_life: u64, access_count: u32) -> u64 {
    if access_count > 0 && access_count % 5 == 0 {
        (current_half_life as f64 * 1.2) as u64
    } else {
        current_half_life
    }
}

/// 重要性评分
pub fn importance_score(content: &str) -> u32 {
    let mut score: u32 = 1;

    const DECISION: &[&str] = &["决定", "决策", "选择", "确定", "计划", "方案"];
    if DECISION.iter().any(|w| content.contains(w)) {
        score += 3;
    }

    const EMPHASIS: &[&str] = &["重要", "核心", "关键", "必须", "长期", "永久", "持续", "务必", "切记", "记住", "固定", "定期"];
    if EMPHASIS.iter().any(|w| content.contains(w)) {
        score += 2;
    }

    const EMOTION: &[&str] = &["开心", "难过", "生气", "担心", "焦虑", "高兴", "满意", "失望", "期待"];
    if EMOTION.iter().any(|w| content.contains(w)) {
        score += 1;
    }

    if content.contains("年") || content.contains("月") || content.contains("日") {
        score += 1;
    }

    const MONEY: &[&str] = &["元", "万", "亿", "%", "倍"];
    if MONEY.iter().any(|w| content.contains(w)) {
        score += 1;
    }

    if content.chars().any(|c| c.is_ascii_digit()) {
        score += 1;
    }

    score.min(10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_half_life() {
        assert_eq!(half_life_for_importance(1), 7);
        assert_eq!(half_life_for_importance(3), 7);
        assert_eq!(half_life_for_importance(5), 30);
        assert_eq!(half_life_for_importance(9), 90);
        assert_eq!(half_life_for_importance(10), 180);
    }

    #[test]
    fn test_decay() {
        let d = decay(0.0, 30.0);
        assert!((d - 1.0).abs() < 0.001);
        let d2 = decay(30.0, 30.0);
        assert!((d2 - std::f64::consts::E.powf(-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_ltp_boost() {
        assert_eq!(ltp_boost(100, 5), 120);
        assert_eq!(ltp_boost(100, 10), 120);
        assert_eq!(ltp_boost(100, 3), 100);
    }

    #[test]
    fn test_importance_score() {
        let s = importance_score("这是一个重要的决定");
        assert!(s > 1);
    }

    #[test]
    fn test_actr_activation_recent_access() {
        // Use a very recent timestamp (now)
        let now = crate::engram::chrono_now_iso();
        let history = vec![now];
        let activation = actr_activation(&history, 0.5);
        assert!(activation.is_finite(), "Recent access should give finite activation");
        assert!(activation > 0.0, "Recent access should give positive activation");
    }

    #[test]
    fn test_actr_activation_empty_history() {
        let activation = actr_activation(&[], 0.5);
        assert!(activation.is_infinite() && activation.is_sign_negative());
    }

    #[test]
    fn test_actr_activation_multiple_accesses() {
        // Multiple accesses should give higher activation than single
        let now = crate::engram::chrono_now_iso();
        let single = vec![now.clone()];
        let multi = vec![now.clone(), now.clone(), now];
        let a_single = actr_activation(&single, 0.5);
        let a_multi = actr_activation(&multi, 0.5);
        assert!(a_multi > a_single, "Multiple accesses should yield higher activation");
    }

    #[test]
    fn test_actr_decay_factor_range() {
        let now = crate::engram::chrono_now_iso();
        let factor = actr_decay_factor(&[now], 0.5);
        assert!(factor > 0.0 && factor <= 1.0, "Decay factor should be in (0, 1], got {}", factor);
    }
}
