<p align="center">
  <h1 align="center">🧠 Hippocampus</h1>
  <p align="center"><strong>仿生认知记忆系统 — Biomimetic Cognitive Memory System</strong></p>
  <p align="center">Rust · 零依赖 · 越用越聪明</p>
</p>

---

## 简介

Hippocampus 是一个受神经科学启发的 AI 记忆系统，用 Rust 编写，零外部依赖（仅 serde）。它模拟人脑海马体的工作机制，为 AI Agent 提供长期、分层的认知记忆能力。

核心理念：**记忆不是数据库，而是活的神经网络。**

---

## ✨ 核心特性

| # | 特性 | 说明 |
|---|------|------|
| 1 | 📊 **三层记忆** | L1(工作记忆) → L2(短期) → L3(长期)，模拟人脑记忆巩固 |
| 2 | 🚪 **记忆门控** | 4脑区协同：杏仁核·海马体·前额叶·颞叶，自动判断是否值得记忆 |
| 3 | 🧠 **自动学习** | 词频学习 + 共现学习，越用越懂你 |
| 4 | 🔍 **BM25 检索** | 内置 BM25 引擎 + CJK bigram 分词 + 同义词扩展 |
| 5 | 🧪 **时间衰减** | `decay = exp(-days_ago / half_life)` 指数遗忘曲线 |
| 6 | 🧬 **语义网络** | Hebbian Learning 扩散激活 + 二阶联想 |
| 7 | 🔄 **再巩固** | 每次回忆触发记忆重新编辑 |
| 8 | ✂️ **去重合并** | CJK 2-gram Jaccard 相似度 + 智能合并 |
| 9 | 😊 **情绪增强** | 杏仁核模拟：强情绪记忆半衰期 ×1.5 |
| 10 | 🧲 **LTP 强化** | 每访问 5 次，半衰期 ×1.2 |
| 11 | 📋 **反思巩固** | 日终自动 L1→L2→L3 溢出 + vacuum 清理 |
| 12 | 📦 **一键迁移** | `hippocampus import` 从 OpenClaw memory 批量导入 |
| 13 | 🔌 **框架集成** | OpenClaw SKILL + Claude Code CLAUDE.md + MCP(规划中) |
| 14 | 💎 **零依赖** | 仅需 serde/serde_json，无需数据库 |

---

## 🖥️ CLI 使用

### 基础命令

```bash
# 初始化（数据目录默认 ~/.hippocampus）
hippocampus init

# 记忆操作
hippocampus remember --content "内容" --importance 7 --tags "标签"
hippocampus recall --query "关键词" --top-k 5
hippocampus gate --message "用户消息"          # 只评估
hippocampus gate --message "用户消息" --write  # 评估+写入

# 维护
hippocampus reflect --days 3       # 反思巩固
hippocampus reconsolidate --days 30 # 记忆再巩固
hippocampus dedup --dry-run        # 去重检测
hippocampus vacuum                 # 清理整理
hippocampus stats                  # 统计
hippocampus learned --top 10       # 查看学习到的关键词
```

### 集成命令

```bash
# 一键安装集成（OpenClaw + Claude Code）
hippocampus install              # 全部安装
hippocampus install --openclaw   # 只装 OpenClaw
hippocampus install --claude     # 只装 Claude Code

# 从 OpenClaw memory 批量迁移
hippocampus import --source /path/to/memory [--dry-run] [--clean-tests]
```

### 数据目录

默认 `~/.hippocampus`，可通过 `HIPPOCAMPUS_HOME` 环境变量覆盖：

```bash
export HIPPOCAMPUS_HOME=/custom/path
```

存储格式：
```
~/.hippocampus/
├── config.json
├── engrams_L1.jsonl     # 工作记忆
├── engrams_L2.jsonl     # 短期记忆
├── engrams_L3.jsonl     # 长期记忆
├── learned_keywords.json # 自动学习的关键词
├── semantic_network.json # 语义网络
└── sessions.jsonl       # 会话追踪
```

---

## 🧬 仿生学对照

| 神经科学 | Hippocampus 模块 | 功能 |
|---------|-----------------|------|
| 海马体 | `EngramStore` | 印迹存储与分层 |
| 杏仁核 | `Emotion` | 情绪检测与增强 |
| 前额叶 | `MemoryGate` | 目标相关 + 记忆意图判断 |
| 颞叶 | `MemoryGate` | 社交关联识别 |
| 突触可塑性 | `SemanticNetwork` | Hebbian 共现学习 |
| 记忆巩固 | `Reconsolidation` | 回忆时重新编辑 |
| 长期增强 (LTP) | `Scoring::ltp_boost` | 高频访问强化 |
| 突触修剪 | `SemanticNetwork::decay_all` | 弱连接清理 |
| 遗忘曲线 | `Scoring::decay` | 指数时间衰减 |
| 海马体溢出 | `Reflector::vacuum` | L1→L2→L3 自动迁移 |
| 经验学习 | `LearnedKeywords` | 词频+共现自动学习 |

---

## 🚪 MemoryGate 记忆门控

4个脑区协同判断一条消息是否值得记忆：

| 脑区 | 权重 | 判断维度 |
|------|------|---------|
| 杏仁核 | 35% | 情绪强度（joy/anger/fear/sadness/surprise/disgust） |
| 海马体 | 30% | 新奇度（新词比例 + IDF + 信息增量 + 预测违背） |
| 前额叶 | 20% | 记忆意图 + 决策词 + 话题匹配 + **学习关键词** |
| 颞叶 | 15% | 社交关联（人称 + 关系词 + 社交行为） |

**特殊加分：**
- "记住"、"帮我记" 等记忆意图 → 前额叶 +0.5，importance ≥ 7
- "决定"、"以后" 等决策词 → 前额叶 +0.12/个，importance ≥ 5
- 自动学习的关键词 → 前额叶 +0.10~0.35（越用越聪明）

综合评分 ≥ 0.3 → 自动写入记忆。

---

## 🧠 自动学习机制

Hippocampus 会从使用中自动学习，越用越聪明：

```
用户消息 → gate 评估 → 写入印迹
                ↓
         实时提取关键词 + 共现统计
                ↓
    learned_keywords.json 持久化
                ↓
    下次 gate 时自动加分
                ↓
    reflect 时清理低频词 + 汇总学习
```

```bash
# 查看学习到的关键词
hippocampus learned --top 10

# 重置学习数据
hippocampus learned --reset
```

---

## 📐 核心公式

**时间衰减：** `decay = exp(-days_ago / half_life)`

**半衰期分级：**

| importance | half_life |
|-----------|-----------|
| 1-3 | 7 天 |
| 4-6 | 30 天 |
| 7-9 | 90 天 |
| 10 | 永久记忆（180天） |

**综合评分：** `final_score = (bm25 × 0.01 + importance × 0.04 + access_count × 0.05) × decay`

**LTP 强化：** 每 5 次访问 → `half_life × 1.2`

**杏仁核增强：** `emotion_score ≥ 0.7` → `half_life × 1.5`

---

## 🏗️ 架构

```
┌─────────────────────────────────────────────────┐
│                   CLI / API                      │
├─────────────────────────────────────────────────┤
│              Hippocampus (入口)                   │
├──────┬──────┬──────┬───────┬───────┬────────────┤
│ Gate │Search│ Store│Reflect│ Dedup │Reconsolidation│
├──────┴──────┴──────┴───────┴───────┴────────────┤
│  Emotion │ SemanticNet │ Scoring │ LearnedKw     │
├──────────┴─────────────┴─────────┴───────────────┤
│         JSONL 文件存储 (L1/L2/L3)                │
└─────────────────────────────────────────────────┘
```

---

## 🔌 框架集成

### OpenClaw

```bash
# 一键安装
hippocampus install --openclaw
```

自动配置：
- SKILL.md → `~/.openclaw/workspace/skills/hippocampus/`
- 定时任务 → `hippocampus reflect` 每天 22:30
- HIPPOCAMPUS_HOME 环境变量

### Claude Code

```bash
# 一键安装
hippocampus install --claude
```

自动配置：
- CLAUDE.md → `~/.claude/CLAUDE.md`（全局指令）
- Hooks → `~/.claude/settings.json`（Stop hook 自动记忆）

---

## 📦 安装

```bash
# 从源码构建
git clone https://github.com/izhimu/hippocampus.git
cd hippocampus
cargo build --release

# 安装到 PATH
cargo install --path .

# 初始化
hippocampus init

# 验证
hippocampus stats
```

---

## 🗂️ 模块列表

| 模块 | 文件 | 功能 |
|------|------|------|
| config | `config.rs` | 配置管理 |
| engram | `engram.rs` | 印迹数据结构 |
| store | `store.rs` | JSONL 分层存储 |
| scoring | `scoring.rs` | 衰减/评分/LTP |
| search | `search.rs` | BM25 + CJK bigram 分词 + 同义词扩展 |
| emotion | `emotion.rs` | 杏仁核情绪检测 |
| semantic_network | `semantic_network.rs` | Hebbian 语义网络 |
| memory_gate | `memory_gate.rs` | 4脑区门控 + 自动学习集成 |
| learned_keywords | `learned_keywords.rs` | 词频 + 共现自动学习 |
| reconsolidation | `reconsolidation.rs` | 记忆再巩固 |
| dedup | `dedup.rs` | 去重合并 |
| session | `session.rs` | 会话追踪 |
| reflect | `reflect.rs` | 反思 + vacuum + 批量学习 |
| lib | `lib.rs` | 统一 API |
| main | `main.rs` | 完整 CLI |

---

## 🗺️ Roadmap

- [ ] MCP Server（跨框架通用工具协议）
- [ ] 嵌入式向量检索
- [ ] Web Dashboard（印迹可视化）
- [ ] 记忆图谱可视化
- [ ] FFI 绑定（Python/Node.js）

---

## License

MIT
