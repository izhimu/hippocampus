/// Unified stop words list for the entire system

use std::collections::HashSet;
use std::sync::LazyLock;

pub static STOP_WORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let words: &[&str] = &[
        // CJK
        "的", "了", "在", "是", "我", "你", "他", "她", "它", "们", "这", "那", "之",
        "与", "和", "或", "而", "且", "但", "也", "就", "又", "到", "自", "从", "由",
        "于", "着", "把", "给", "等", "被", "让", "向", "往", "过", "得",
        "吗", "呢", "吧", "啊", "哦", "嗯", "哈", "嘛", "呀",
        "有", "不", "都", "会", "可以", "要", "所", "如", "为", "没",
        "用", "中", "个", "上", "下", "里", "去", "来", "对", "很",
        "更", "最", "已", "及", "其", "并", "还", "将", "只", "因",
        "则", "以", "至", "该", "些", "么",
        "这个", "那个", "一个", "什么", "怎么", "没有", "不是", "就是",
        "也是", "都是", "我们", "他们", "她们", "你们", "如果", "但是",
        "因为", "所以", "或者", "虽然", "不过", "而且", "还是", "然后",
        "已经", "可能", "这些", "那些", "这样", "那样",
        "不知道", "没问题", "好的", "收到", "嗯嗯", "哈哈", "嘿嘿",
        // English
        "the", "a", "an", "and", "or", "but", "if", "then", "else", "when",
        "where", "why", "how", "what", "which", "who", "whom", "this", "that",
        "these", "those", "am", "is", "are", "was", "were", "be", "been",
        "being", "have", "has", "had", "having", "do", "does", "did", "doing",
        "to", "from", "up", "down", "in", "out", "on", "off", "over", "under",
        "again", "further", "once", "here", "there", "all", "any", "both",
        "each", "few", "more", "most", "other", "some", "such", "no", "nor",
        "not", "only", "own", "same", "so", "than", "too", "very", "can",
        "will", "just", "don", "should", "now", "about", "above", "after",
        "before", "between", "into", "through", "during", "for", "with",
        "at", "by", "of", "s", "t",
    ];
    words.iter().copied().collect()
});

/// CJK-only single-character stop chars (for n-gram filtering)
pub static CJK_STOP_CHARS: LazyLock<HashSet<char>> = LazyLock::new(|| {
    "的了在是我你他她它们这那有不也都就会要和与或但而所如为从到被把让给用没之等中个上下里去来对很更最已于其又并将只因则以至该些么吗呢吧啊哦嗯哈嘛呀"
        .chars().collect()
});

/// Check if a word is a stop word
pub fn is_stop_word(word: &str) -> bool {
    STOP_WORDS.contains(word.to_lowercase().as_str())
}

/// Check if all chars in a string are CJK stop chars
pub fn all_cjk_stop_chars(s: &str) -> bool {
    s.chars().all(|c| CJK_STOP_CHARS.contains(&c))
}
