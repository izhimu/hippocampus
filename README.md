<p align="center">
  <h1 align="center">🧠 Hippocampus</h1>
  <p align="center"><strong>仿生认知记忆系统 — Biomimetic Cognitive Memory System</strong></p>
  <p align="center">Rust · 认知科学驱动 · 越用越聪明</p>
</p>

---

## 简介

Hippocampus 是一个受神经科学和认知科学启发的 AI 记忆系统，用 Rust 编写。它模拟人脑海马体的工作机制，融合 ACT-R 认知架构、SimHash 语义匹配、Successor Representation 认知地图等认知科学理论，为 AI Agent 提供长期、分层的认知记忆能力。

核心理念：**记忆不是数据库，而是活的神经网络。**

---

## ✨ 核心特性

| # | 特性 | 说明 |
|---|------|------|
| 1 | 📊 **三层记忆** | L1(工作记忆) → L2(短期) → L3(长期)，模拟人脑记忆巩固 |
| 2 | 🚪 **记忆门控** | 4脑区协同：杏仁核·海马体·前额叶·颞叶，自动判断是否值得记忆 |
| 3 | 🧠 **自动学习** | 词频学习 + 共现学习，越用越懂你 |
| 4 | 🔍 **BM25 + SimHash 检索** | BM25 关键词检索 + SimHash 64-bit 局部敏感哈希语义匹配融合 |
| 5 | 📉 **ACT-R 幂律遗忘** | `B_i = ln(Σ t_k^(-d))` 多时间戳基线激活，替代固定指数衰减 |
| 6 | 🗺️ **认知地图 (SR)** | Engram 级 Successor Representation 图谱，TD 学习 + 扩散召回 |
| 7 | 🔄 **冲突消解 (SleepGate)** | SimHash 语义聚类检测矛盾记忆，自动保留最新、降级旧的 |
| 8 | 💭 **生成式回放** | 认知地图随机游走 Hebbian 巩固，模拟海马体睡眠回放 |
| 9 | 🧬 **语义网络** | Hebbian Learning 扩散激活 + 二阶联想 |
| 10 | 🔄 **再巩固** | 每次回忆触发记忆重新编辑 |
| 11 | ✂️ **去重合并** | CJK 2-gram Jaccard 相似度 + 智能合并 |
| 12 | 😊 **情绪增强** | 杏仁核模拟：强情绪记忆半衰期 ×1.5 |
| 13 | 🧲 **LTP 强化** | 高频访问自然提升 ACT-R 激活值 |
| 14 | 📋 **反思巩固** | 日终自动 L1→L2→L3 溢出 + 冲突消解 + 回放巩固 + vacuum 清理 |
| 15 | 📦 **一键迁移** | `hippocampus import` 从 OpenClaw memory 批量导入 |
| 16 | 🔌 **框架集成** | OpenClaw SKILL + Claude Code CLAUDE.md + Hook |
| 17 | 🌐 **Web 控制台** | Gateway 内置 Three.js 3D 脑模型可视化 |
| 18 | 🪝 **Hook 系统** | Claude Code / 通用适配器自动记忆 + 召回注入 |

---

## 🖥️ CLI 完整命令

### 初始化

```bash
hippocampus init                  # 初始化数据目录（默认 ~/.hippocampus）
```

### 记忆操作

```bash
# 手动写入记忆
hippocampus remember --content "用户喜欢基金定投" --importance 7 --tags "金融,投资"
hippocampus remember --content "决定每月定投2000元" --importance 8 --permanent
hippocampus remember --content "临时笔记" --layer L1 --source "manual"

# 检索记忆
hippocampus recall --query "基金策略" --top-k 5
hippocampus recall --query "基金策略" --top-k 5 --brief          # 紧凑模式（每条一行）
hippocampus recall --query "开心的事" --emotion joy              # 按情绪过滤
hippocampus recall --query "投资" --with-context "当前市场"      # 上下文增强检索
hippocampus recall --query "最近" --l1l2-only                    # 仅搜索 L1+L2

# 门控评估（自动判断是否值得记忆）
hippocampus gate --message "用户说今天很开心"                     # 仅评估，不写入
hippocampus gate --message "记住我喜欢蓝色" --write              # 评估 + 写入
hippocampus gate --message "这个必须记住" --write --force         # 强制写入
```

### recall --brief 紧凑模式

`recall` 新增 `--brief` flag，输出格式为每条结果一行：
```
[0.8700] [L2] 用户表达了对基金定投策略的兴趣，倾向于沪深300和中证500...
[0.7200] [L3] 主人决定每月定投2000元，分散到3只基金中...
```
适合 Agent 自动化调用场景，节省 token 消耗。

### 维护命令

```bash
# 反思巩固（L1→L2→L3 溢出 + 语义网络学习 + 冲突消解 + 认知地图 + 回放巩固）
hippocampus reflect --days 3

# 记忆再巩固
hippocampus reconsolidate --days 30
hippocampus reconsolidate --days 30 --dry-run

# 去重检测
hippocampus dedup
hippocampus dedup --similarity 0.8 --dry-run

# 同义词自动学习
hippocampus learn-synonyms
hippocampus learn-synonyms --dry-run --top-k 20

# 清理整理（删除过期记忆）
hippocampus vacuum

# 统计信息
hippocampus stats

# 查看自动学习到的关键词
hippocampus learned --top 10
hippocampus learned --reset    # 重置学习数据
```

### 数据导入

```bash
# 从 OpenClaw memory 目录批量导入
hippocampus import --source ~/.openclaw/workspace/memory/
hippocampus import --source ~/.openclaw/workspace/memory/ --dry-run
hippocampus import --source ~/.openclaw/workspace/memory/ --clean-tests    # 清理测试数据
hippocampus import --source ~/.openclaw/workspace/memory/ --min-importance 5
```

自动识别文件类型并分配层级/重要性：

| 文件名模式 | 层级 | 重要性 | 标签 |
|-----------|------|--------|------|
| `MEMORY.md` | L3 | 8 | 核心记忆 |
| `SECURITY_RULES.md` | L3 | 9 | 安全规则 |
| `fund_names*` | L3 | 7 | 基金 |
| `financial-*` | L3 | 6 | 金融知识 |
| `YYYY-MM-DD.md` | L2 | 3-5 | 每日记录 |
| 其他 `.md` | L2 | 4 | 归档 |

### 框架集成

```bash
# 一键安装（全部）
hippocampus install --all

# 仅安装 OpenClaw 适配器
hippocampus install --openclaw

# 仅安装 Claude Code 适配器
hippocampus install --claude
```

### Gateway（Web 可视化控制台）

```bash
# 前台启动（开发调试）
hippocampus gateway                    # 默认端口 8088
hippocampus gateway --port 3000        # 自定义端口

# 后台管理
hippocampus gateway start              # 后台启动
hippocampus gateway start --port 3000  # 后台启动（自定义端口）
hippocampus gateway stop               # 停止后台进程

# 查看帮助
hippocampus gateway --help
```

**API 端点：**

| 方法 | 路径 | 功能 |
|------|------|------|
| GET | `/api/stats` | 记忆统计 |
| GET | `/api/engrams` | 列出印迹 |
| POST | `/api/recall` | 按查询检索 |
| POST | `/api/gate` | 门控评估（不写入） |
| POST | `/api/gate/execute` | 门控评估 + 写入 |
| GET | `/api/events` | WebSocket 实时事件流 |

**3D 脑模型可视化：** 内置 Three.js 实时渲染，展示 4 脑区活跃度、记忆写入/检索动效。

### Hook 系统

```bash
# Claude Code Hook（通过 settings.json 自动调用）
hippocampus hook auto --claude < stdin     # 自动模式：recall + record
hippocampus hook recall --claude < stdin   # 仅召回
hippocampus hook record --claude < stdin   # 仅记录
hippocampus hook summarize --claude < stdin  # 会话摘要
hippocampus hook reflect --claude < stdin  # 反思巩固

# 通用 Hook
hippocampus hook auto < stdin             # 自动模式
hippocampus hook recall < stdin           # 仅召回
hippocampus hook record < stdin           # 仅记录
```

### 环境变量

```bash
export HIPPOCAMPUS_HOME=/custom/path    # 自定义数据目录（默认 ~/.hippocampus）
```

### 数据目录结构

```
~/.hippocampus/
├── config.json
├── engrams_L1.jsonl         # 工作记忆
├── engrams_L2.jsonl         # 短期记忆
├── engrams_L3.jsonl         # 长期记忆
├── learned_keywords.json    # 自动学习的关键词
├── semantic_network.json    # 词级语义网络（Hebbian）
├── cognitive_map.json       # Engram 级认知地图（SR 矩阵）
├── sessions.jsonl           # 会话追踪
├── last_gate.json           # 最近一次 gate 决策（供 Gateway 读取）
├── gateway.pid              # Gateway 后台进程 PID
└── gateway.log              # Gateway 日志
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
| 认知地图 | `CognitiveMap` | SR 矩阵 + TD 学习 + 扩散召回 |
| 记忆巩固 | `Reconsolidation` | 回忆时重新编辑 |
| 长期增强 (LTP) | `Scoring::ltp_boost` | 高频访问强化 |
| 突触修剪 | `SemanticNetwork::decay_all` | 弱连接清理 |
| 幂律遗忘 | `Scoring::actr_activation` | ACT-R 基线激活方程 |
| SimHash (SDM) | `SimHash` | 64-bit 局部敏感哈希近似语义匹配 |
| 睡眠门控 | `ConflictResolver` | 冲突检测 + 语义消解 |
| 睡眠回放 | `Reflector::generative_replay` | 随机游走 Hebbian 巩固 |
| 海马体溢出 | `Reflector::vacuum` | L1→L2→L3 自动迁移 |
| 经验学习 | `LearnedKeywords` | 词频+共现自动学习 |

---

## 🚪 MemoryGate 记忆门控

4个脑区协同判断一条消息是否值得记忆：

| 脑区 | 权重 | 判断维度 |
|------|------|---------|
| 🟥 杏仁核 | **35%** | 情绪强度（joy/anger/fear/sadness/surprise/disgust） |
| 🟦 海马体 | **30%** | 新奇度（新词比例 + IDF + 信息增量 + 预测违背） |
| 🟩 前额叶 | **20%** | 记忆意图 + 决策词 + 话题匹配 + 学习关键词 |
| 🟨 颞叶 | **15%** | 社交关联（人称 + 关系词 + 社交行为） |

**特殊加分：**
- "记住"、"帮我记" 等记忆意图 → 前额叶 +0.5，importance ≥ 7
- "决定"、"以后" 等决策词 → 前额叶 +0.12/个，importance ≥ 5
- 自动学习的关键词 → 前额叶 +0.10~0.35（越用越聪明）

**`--force` 模式**：跳过门控评估，直接以高优先级写入记忆。

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

---

## 📐 核心公式

### ACT-R 基线激活

`B_i = ln(Σ t_k^(-d))`，其中 `d=0.5` 为衰减率，`t_k` 为第 k 次访问距今的小时数

激活值通过 sigmoid 归一化映射到 [0, 1] 作为衰减因子：`decay = 1 / (1 + exp(-B_i / 2))`

### SimHash 语义匹配

64-bit FNV-1a 局部敏感哈希，海明距离衡量语义相似度：`similarity = 1 - hamming_distance / 64`

### 综合评分

`final_score = (ln(1+bm25_score) × 0.6 + importance × 0.04 + ln(1+access_count) × 0.05) × actr_decay`

- **BM25**：对数压缩 `ln(1+x)×0.6` 防止噪声高分主导
- **Importance**：固定权重 0.04/importance
- **Frequency**：访问次数对数 `ln(1+n)×0.05`
- **ACT-R Decay**：有访问历史时按幂律衰减，批量导入无历史时不衰减

### 检索引擎

BM25 关键词检索，支持：
- **CJK**：2-gram + 3-gram 分词
- **English**：Porter stemming + stop words 过滤
- **同义词扩展**：基于语义网络的同义词查询扩展
- **SR 扩散召回**：认知图谱关联记忆增强

### 半衰期分级

| importance | half_life |
|-----------|-----------|
| 1-3 | 7 天 |
| 4-6 | 30 天 |
| 7-9 | 90 天 |
| 10 | 永久记忆（180天） |

### LTP 强化

高频访问自然提升 ACT-R 激活值（不再单独 `half_life × 1.2`）

### 杏仁核增强

`emotion_score ≥ 0.7` → `half_life × 1.5`

---

## 🏗️ 架构

```
输入消息
  │
  ▼
┌─────────────────────────┐
│  Surprise-Driven Gate   │  4脑区加权 + 情绪劫持
│  (杏仁核·海马体·前额叶·颞叶) │
└──────────┬──────────────┘
           │ 通过门控
           ▼
┌─────────────────────────┐
│  Engram Store (JSONL)   │  新增 fingerprint (SimHash)
│  L1 → L2 → L3 三层      │  access_history 时间戳列表
└──────────┬──────────────┘
           │
           ▼
┌─────────────────────────┐
│  检索引擎               │
│  BM25 + SimHash(SDM)    │  64-bit 局部敏感哈希语义匹配
│  × ACT-R 基线激活       │  幂律遗忘曲线
│  + SR 认知地图扩散       │  关联记忆召回
└──────────┬──────────────┘
           │
           ▼
┌─────────────────────────┐
│  Reflect 梦境系统       │
│  1. 语义网络学习        │
│  2. 突触修剪            │
│  3. SleepGate 冲突消解  │  语义聚类 + 矛盾消解
│  4. SR 随机游走回放      │  生成式巩固
│  5. 层级晋升 (vacuum)   │
└─────────────────────────┘
```

### CLI ↔ Gateway 同步

CLI 执行 `remember`、`recall`、`gate` 等命令时，会自动向 Gateway 发送 `POST /api/notify` 实时同步脑区活跃度，无需额外配置。

---

## 🔌 框架集成

### OpenClaw

```bash
hippocampus install --openclaw
```

自动配置：
- SKILL.md → `~/.openclaw/workspace/skills/hippocampus/`
- 定时任务 → `hippocampus reflect` 每天 22:30 + 月度 vacuum
- HIPPOCAMPUS_HOME 环境变量

### Claude Code

```bash
hippocampus install --claude
```

自动配置：
- SKILL.md → `~/.claude/skills/hippocampus/`
- 全局 CLAUDE.md → 追加 hippocampus 使用提示
- Hooks → `~/.claude/settings.json`（Stop hook 自动记忆 + UserPromptSubmit 召回注入）

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

# （可选）启动 Web 控制台
hippocampus gateway start
```

---

## 🗂️ 模块列表

| 模块 | 文件 | 功能 |
|------|------|------|
| config | `config.rs` | 配置管理 |
| engram | `engram.rs` | 印迹数据结构（含 fingerprint + access_history） |
| store | `store.rs` | JSONL 分层存储 |
| scoring | `scoring.rs` | ACT-R 激活 + 衰减 + 评分 + LTP |
| search | `search.rs` | BM25 + SimHash 融合检索 + SR 扩散 |
| simhash | `simhash.rs` | 64-bit SimHash 指纹 + 海明距离 |
| cognitive_map | `cognitive_map.rs` | SR 认知地图 + TD 学习 + 随机游走 |
| conflict | `conflict.rs` | SleepGate 冲突检测 + 消解 |
| emotion | `emotion.rs` | 杏仁核情绪检测 |
| semantic_network | `semantic_network.rs` | Hebbian 词级语义网络 |
| memory_gate | `memory_gate.rs` | 4脑区门控 + 自动学习集成 |
| learned_keywords | `learned_keywords.rs` | 词频 + 共现自动学习 |
| reconsolidation | `reconsolidation.rs` | 记忆再巩固 |
| dedup | `dedup.rs` | 去重合并 |
| session | `session.rs` | 会话追踪 |
| reflect | `reflect.rs` | 反思 + 冲突消解 + 回放巩固 + vacuum |
| gateway | `gateway.rs` | Web 可视化控制台 |
| lib | `lib.rs` | 统一 API |
| main | `main.rs` | 完整 CLI |

---

## 📊 LongMemEval 基准测试

内置 `bench-longmemeval` 基准工具，基于 `LongMemEval` 数据集评估记忆召回能力。

```bash
cd bench-longmemeval && cargo build --release

# 快速测试（每类 5 题）
./target/release/bench-longmemeval --samples 5 --mode recall

# 完整测试（全部 500 题，recall + gate 对比）
./target/release/bench-longmemeval --mode full

# 仅 recall 测试
./target/release/bench-longmemeval --mode recall
```

**500 题完整测试结果（纯 BM25，无向量检索）：**

| 指标 | 值 |
|------|-----|
| Recall@1 | 64.0% |
| Recall@10 | 85.8% |
| MRR | 0.72 |

| 题目类型 | R@1 | R@10 |
|---------|-----|------|
| single-session-assistant | 96.4% | 100% |
| knowledge-update | 79.5% | 95.7% |
| multi-session | 63.2% | 91.4% |
| temporal-reasoning | 58.6% | 85.7% |
| single-session-user | 52.9% | 83.5% |
| single-session-preference | 16.7% | 58.3% |

测试模式：
- `--mode recall`：所有 haystack 直接写入 L2，测试纯召回能力
- `--mode gate`：haystack 通过 gate 筛选，只保留值得记忆的，再测试召回
- `--mode full`：两种都测，输出对比报告

---

## 🗺️ Roadmap

- [x] ~~ACT-R 基线激活方程~~（幂律遗忘曲线）
- [x] ~~SimHash + SDM 语义召回~~
- [x] ~~Engram 级认知地图 (SR)~~
- [x] ~~SleepGate 冲突消解~~
- [x] ~~生成式回放巩固~~
- [x] ~~LongMemEval 基准测试~~（Recall@1=64%, R@10=85.8%）
- [ ] MCP Server（跨框架通用工具协议）
- [ ] 嵌入式向量检索（提升 preference 类召回）
- [ ] 记忆图谱可视化增强
- [ ] FFI 绑定（Python/Node.js）

---

## License

MIT
