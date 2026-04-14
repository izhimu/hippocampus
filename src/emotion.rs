/// emotion — 杏仁核模拟：零依赖关键词情绪检测器

/// 情绪检测结果
#[derive(Debug, Clone)]
pub struct EmotionResult {
    pub emotion: String,    // neutral/joy/anger/sadness/fear/surprise/disgust
    pub emotion_score: f64, // 0.0-1.0
}

const JOY_WORDS: &[&str] = &[
    "开心","高兴","太好了","棒","喜欢","爱","幸福","快乐","满足","愉快",
    "享受","美好","不错","厉害","优秀","完美","太棒","好开心","笑","哈哈",
    "嘻嘻","耶","赞","酷","爽","嘻嘻","嗯嗯","好的呀","真好","好棒",
    "可爱","有趣","好玩","开心",
];

const ANGER_WORDS: &[&str] = &[
    "气死","烦","怒","混蛋","垃圾","可恶","该死","愤怒","暴怒","讨厌",
    "恶心","受不了","无语","火大","暴躁","怒了","气死","烦死了","滚","闭嘴",
    "废物","白痴","智障","恶心","妈的",
];

const SADNESS_WORDS: &[&str] = &[
    "难过","伤心","哭","悲伤","失落","沮丧","心痛","遗憾","思念","抑郁",
    "消沉","泪","痛苦","绝望","无奈","孤独","寂寞","想哭","不开心","低落",
    "郁闷","惆怅","忧伤",
];

const FEAR_WORDS: &[&str] = &[
    "害怕","恐惧","焦虑","担心","紧张","不安","恐慌","可怕","担忧","害怕",
    "焦虑","忐忑","心慌","惶恐","畏惧","可怕",
];

const SURPRISE_WORDS: &[&str] = &[
    "惊讶","震惊","没想到","居然","竟然","天哪","哇","卧槽","我的天","不敢相信",
    "原来如此","意外","吃惊","吓到","天呐","我去","真的假的","不会吧",
];

const DISGUST_WORDS: &[&str] = &[
    "恶心","厌恶","反胃","受不了","变态","猥琐","脏","臭","邋遢","反胃","作呕",
];

/// 关键词匹配情绪检测
pub fn detect(text: &str) -> EmotionResult {
    let joy_count = JOY_WORDS.iter().filter(|w| text.contains(**w)).count();
    let anger_count = ANGER_WORDS.iter().filter(|w| text.contains(**w)).count();
    let sadness_count = SADNESS_WORDS.iter().filter(|w| text.contains(**w)).count();
    let fear_count = FEAR_WORDS.iter().filter(|w| text.contains(**w)).count();
    let surprise_count = SURPRISE_WORDS.iter().filter(|w| text.contains(**w)).count();
    let disgust_count = DISGUST_WORDS.iter().filter(|w| text.contains(**w)).count();

    let candidates: Vec<(&str, usize)> = vec![
        ("joy", joy_count),
        ("anger", anger_count),
        ("sadness", sadness_count),
        ("fear", fear_count),
        ("surprise", surprise_count),
        ("disgust", disgust_count),
    ];

    let best = candidates.iter().max_by_key(|(_, c)| c);

    match best {
        Some((emotion, count)) if *count > 0 => EmotionResult {
            emotion: emotion.to_string(),
            emotion_score: (*count as f64 * 0.2 + 0.1).min(1.0),
        },
        _ => EmotionResult {
            emotion: "neutral".to_string(),
            emotion_score: 0.0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neutral() {
        let r = detect("今天天气怎么样");
        assert_eq!(r.emotion, "neutral");
    }

    #[test]
    fn test_joy() {
        let r = detect("今天太开心了，真的太棒了！");
        assert_eq!(r.emotion, "joy");
        assert!(r.emotion_score > 0.0);
    }

    #[test]
    fn test_anger() {
        let r = detect("气死了，垃圾代码！");
        assert_eq!(r.emotion, "anger");
    }
}
