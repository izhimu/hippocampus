<p align="center">
  <h1 align="center">🧠 Hippocampus</h1>
  <p align="center"><strong>仿生认知记忆系统 — Biomimetic Cognitive Memory System</strong></p>
</p>

---

## 简介

Hippocampus 是一个受神经科学启发的 AI 记忆系统，用 Rust 编写，零外部依赖（除 serde）。它模拟人脑海马体的工作机制，为 AI Agent 提供长期、分层的认知记忆能力。

核心理念：**记忆不是数据库，而是活的神经网络。**

---

## ✨ 核心特性

| # | 特性 | 说明 |
|---|------|------|
| 1 | 📊 **三层记忆** | L1(工作记忆) → L2(短期) → L3(长期)，模拟人脑记忆巩固过程 |
| 2 | 🚪 **记忆门控** | 4脑区协同判断：杏仁核·海马体·前额叶·颞叶 |
| 3 | 🔍 **BM25 检索** | 内置 BM25 引擎 + CJK 2/3-gram 分词 |
| 4 | 🧪 **时间衰减** | `decay = exp(-days_ago / half_life)` 指数遗忘曲线 |
| 5 | 🧬 **语义网络** | Hebbian Learning 扩散激活 + 二阶联想 |
| 6 | 🔄 **再巩固** | 每次回忆触发记忆重新编辑（Reconsolidation） |
| 7 | ✂️ **去重合并** | CJK 2-gram Jaccard 相似度 + 智能合并 |
| 8 | 😊 **情绪增强** | 杏仁核模拟：强情绪记忆半衰期 ×1.5 |
| 9 | 🧹 **突触修剪** | 弱连接自动衰减清理 |
| 10 | 🧲 **LTP 强化** | 每访问 5 次，半衰期 ×1.2（长期增强） |
| 11 | 📋 **反思巩固** | 日终自动 L1→L2→L3 溢出 + 真空整理 |
| 12 | 💎 **零依赖** | 仅需 serde/serde_json，无需数据库 |

---

## 🧬 仿生学对照

| 神经科学 | Hippocampus 模块 | 功能 |
|---------|-----------------|------|
| 海马体 (Hippocampus) | `EngramStore` | 印迹存储与分层 |
| 杏仁核 (Amygdala) | `Emotion` | 情绪检测与增强 |
| 前额叶 (Prefrontal) | `MemoryGate` | 目标相关判断 |
| 颞叶 (Temporal) | `MemoryGate` | 社交关联识别 |
| 突触可塑性 | `SemanticNetwork` | Hebbian 共现学习 |
| 记忆巩固 | `Reconsolidation` | 回忆时重新编辑 |
| 长期增强 (LTP) | `Scoring::ltp_boost` | 高频访问强化 |
| 突触修剪 | `SemanticNetwork::decay_all` | 弱连接清理 |
| 遗忘曲线 | `Scoring::decay` | 指数时间衰减 |
| 海马体溢出 | `Reflector::vacuum` | L1→L2→L3 自动迁移 |

---

## 🏗️ 架构

```
┌─────────────────────────────────────────────┐
│                  CLI / API                   │
├─────────────────────────────────────────────┤
│             Hippocampus (入口)               │
├──────┬──────┬──────┬───────┬───────┬────────┤
│ Gate │Search│ Store│Reflect│ Dedup │Reconso │
├──────┴──────┴──────┴───────┴───────┴────────┤
│  Emotion │ SemanticNetwork │ Scoring        │
├──────────┴─────────────────┴────────────────┤
│         JSONL 文件存储 (L1/L2/L3)           │
└─────────────────────────────────────────────┘
```

---

## 📦 Engram 数据结构

```rust
pub struct Engram {
    pub id: String,              // 唯一标识（时间戳哈希）
    pub content: String,         // 记忆内容
    pub importance: u32,         // 重要性 1-10
    pub emotion: String,         // 情绪标签（joy/anger/fear/sadness/surprise/disgust/neutral）
    pub emotion_score: f64,      // 情绪强度 0.0-1.0
    pub source: String,          // 来源（manual/dialogue/system）
    pub tags: Vec<String>,       // 自动提取标签
    pub access_count: u32,       // 访问次数
    pub created_at: String,      // 创建时间 ISO-8601
    pub accessed_at: Option<String>,
    pub layer: String,           // L1/L2/L3
    pub half_life: u64,          // 衰减半衰期（天）
    pub session_id: Option<String>,
}
```

---

## 🚪 MemoryGate 4脑区

| 脑区 | 权重 | 判断维度 |
|------|------|---------|
| 杏仁核 | 35% | 情绪强度（关键词检测 joy/anger/fear/sadness/surprise/disgust） |
| 海马体 | 30% | 新奇度（新词比例 + IDF + 信息增量 + 预测违背） |
| 前额叶 | 20% | 目标相关性（话题匹配 + 进展 + 长度） |
| 颞叶 | 15% | 社交关联（人称 + 关系词 + 社交行为） |

综合评分 ≥ `auto_memory_threshold`（默认 0.3）→ 记忆写入。

---

## 📐 核心公式

**时间衰减：**
```
decay = exp(-days_ago / half_life)
```

**半衰期分级：**
| importance | half_life |
|-----------|-----------|
| 1-3 | 7 天 |
| 4-6 | 30 天 |
| 7-9 | 90 天 |
| 10 | 180 天 |

**综合评分：**
```
final_score = (bm25 × 0.01 + importance × 0.04 + access_count × 0.05) × decay
```

**LTP 强化：** 每 5 次访问 → `half_life × 1.2`

**杏仁核增强：** `emotion_score ≥ 0.7` → `half_life × 1.5`

---

## 🖥️ CLI 使用

```bash
# 设置数据目录
export HIPPOCAMPUS_HOME=./my_memory

# 初始化
hippocampus init

# 手动记忆
hippocampus remember --content "主人下周一去上海出差" --importance 7 --source dialogue --tags "出行,时间"

# 检索
hippocampus recall --query "出差" --top-k 5

# 记忆门控（只评估不写入）
hippocampus gate --message "今天天气不错"
hippocampus gate --message "基金大跌亏了两万"

# 记忆门控（评估+写入）
hippocampus gate --message "基金大跌亏了两万" --write

# 反思巩固
hippocampus reflect --days 7

# 去重扫描
hippocampus dedup --similarity 0.7

# 学习同义词
hippocampus learn-synonyms --top-k 20

# 查看统计
hippocampus stats

# 真空整理
hippocampus vacuum
```

---

## 🦀 Rust API

```rust
use hippocampus::Hippocampus;

// 初始化
let mut hippo = Hippocampus::new("./my_memory")?;

// 写入记忆
let id = hippo.remember(
    "主人喜欢投资科技基金",
    8,
    "dialogue",
    &["投资", "偏好"],
    None,   // session_id
    "L1",
    false,  // permanent
)?;

// 检索
let results = hippo.recall("基金", 5, 0.0, true, None, None);

// 记忆门控
let decision = hippo.should_remember("主人下周生日");
if decision.should_remember {
    hippo.auto_remember("主人下周生日", "dialogue", None, false)?;
}

// 反思
let result = hippo.reflect(7)?;

// 统计
let stats = hippo.stats();
```

---

## 📦 安装

```bash
# 从源码构建
git clone https://github.com/izhimu/hippocampus.git
cd hippocampus
cargo build --release

# 或本地安装
cargo install --path .
```

---

## 🗂️ 模块列表

| 模块 | 文件 | 功能 |
|------|------|------|
| config | `config.rs` | 配置管理 |
| engram | `engram.rs` | 印迹数据结构 |
| store | `store.rs` | JSONL 分层存储 |
| scoring | `scoring.rs` | 衰减/评分/LTP |
| search | `search.rs` | BM25 + CJK 分词 |
| emotion | `emotion.rs` | 杏仁核情绪检测 |
| semantic_network | `semantic_network.rs` | Hebbian 语义网络 |
| memory_gate | `memory_gate.rs` | 4脑区门控 |
| reconsolidation | `reconsolidation.rs` | 记忆再巩固 |
| dedup | `dedup.rs` | 去重合并 |
| session | `session.rs` | 会话追踪 |
| reflect | `reflect.rs` | 反思 + vacuum |

---

## 🗺️ Roadmap

- [ ] 嵌入式向量检索（embedded vectors）
- [ ] 分布式存储后端
- [ ] Web Dashboard
- [ ] 记忆导出/导入
- [ ] FFI 绑定（Python/Node.js）
- [ ] 记忆图谱可视化

---

## License

MIT
