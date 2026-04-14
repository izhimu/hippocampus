/// 衰减公式 + 重排公式 + LTP强化

/// 根据重要性返回半衰期天数（与 Python _half_life_for_importance 一致）
pub fn half_life_for_importance(imp: i32) -> i32 {
    match imp {
        ..=3 => 7,
        4..=6 => 30,
        7..=9 => 90,
        10..=i32::MAX => 180,
    }
}

/// 时间衰减公式：decay = exp(-days_ago / half_life)
pub fn decay(days_ago: f64, half_life: f64) -> f64 {
    let hl = half_life.max(1.0);
    (-days_ago / hl).exp()
}

/// 综合评分公式（与 Python EngramStore.search 中的 final_score 一致）
/// final_score = (bm25_score * 0.01 + importance * 0.04 + access_count * 0.05) * decay
pub fn final_score(bm25_score: f64, importance: u32, access_count: u32, days_ago: f64, half_life: f64) -> f64 {
    let d = decay(days_ago, half_life);
    (bm25_score * 0.01 + importance as f64 * 0.04 + access_count as f64 * 0.05) * d
}

/// LTP（长期增强）：每 5 次访问，半衰期 ×1.2
pub fn ltp_boost(current_half_life: u64, access_count: u32) -> u64 {
    if access_count > 0 && access_count % 5 == 0 {
        (current_half_life as f64 * 1.2) as u64
    } else {
        current_half_life
    }
}

/// 重要性评分（简化版，与 Python importance_score 核心逻辑一致）
pub fn importance_score(content: &str) -> u32 {
    let mut score: u32 = 1;
    let content = content;

    // 决策词 +3
    const DECISION: &[&str] = &["决定", "决策", "选择", "确定", "计划", "方案"];
    if DECISION.iter().any(|w| content.contains(w)) {
        score += 3;
    }

    // 强调词 +2
    const EMPHASIS: &[&str] = &["重要", "核心", "关键", "必须", "长期", "永久", "持续", "务必", "切记", "记住", "固定", "定期"];
    if EMPHASIS.iter().any(|w| content.contains(w)) {
        score += 2;
    }

    // 情绪词 +1
    const EMOTION: &[&str] = &["开心", "难过", "生气", "担心", "焦虑", "高兴", "满意", "失望", "期待"];
    if EMOTION.iter().any(|w| content.contains(w)) {
        score += 1;
    }

    // 日期模式 +1
    if content.contains("年") || content.contains("月") || content.contains("日") {
        score += 1;
    }

    // 金额相关 +1
    const MONEY: &[&str] = &["元", "万", "亿", "%", "倍"];
    if MONEY.iter().any(|w| content.contains(w)) {
        score += 1;
    }

    // 包含数字 +1
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
}
