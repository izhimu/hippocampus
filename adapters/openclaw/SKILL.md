---
name: hippocampus
description: >
  Hippocampus 仿生认知记忆系统。本地 Rust CLI，通过 exec 工具调用 `hippocampus` 命令。
  在每轮对话前自动 recall 相关记忆辅助回答，对话后通过 gate 门控自动评估是否值得记忆。
  当用户说"记住"、"记一下"时使用 remember 手动写入。定时任务中使用 reflect/vacuum 维护记忆。
  适用于所有需要记忆检索、个性化回复、长期上下文保持的场景。
---

# Hippocampus — 仿生认知记忆系统

通过 exec 工具调用 `hippocampus` 命令，数据存储于 `$HIPPOCAMPUS_HOME`（默认 `~/.hippocampus`）。

## 记忆层级

| 层级 | 名称 | 持续时间 | 文件 |
|------|------|---------|------|
| L1 | 工作记忆 | 当前会话 | engrams_L1.jsonl |
| L2 | 短期记忆 | 数天 | engrams_L2.jsonl |
| L3 | 长期记忆 | 永久 | engrams_L3.jsonl |

## 日常使用（每轮对话）

### 对话前：recall 检索

```bash
hippocampus recall --query "关键词" --top-k 3 --brief
```

- 每轮回答前必须执行（简单问候如"你好"、"早安"可跳过）
- `--brief` 输出格式：`[score] [layer] content前100字`，节省 token

### 对话后：gate 自动门控

```bash
hippocampus gate --message "用户原始消息" --write
```

- 每轮对话结束后必须执行（简单问候可跳过）
- 4 脑区协同评分，阈值 0.3，达到则自动写入 L2
- 分数不足的记忆自动丢弃，无需人工判断

### 手动记忆

用户明确要求记忆时：

```bash
# 用户说"记住"
hippocampus remember --content "内容" --importance 8

# 重要决策/偏好
hippocampus remember --content "内容" --importance 7 --permanent
```

`--importance` 范围 1-10，`--permanent` 直接写入 L3。

## 命令参考

| 命令 | 用途 | 关键参数 |
|------|------|---------|
| `recall` | 记忆召回 | `--query`, `--top-k N`, `--min-score F`, `--brief`, `--l1l2-only`, `--emotion E`, `--with-context` |
| `remember` | 手动记忆 | `--content`, `--importance N(1-10)`, `--source`, `--tags "a,b"`, `--layer L1/L2/L3`, `--permanent` |
| `gate` | 自动门控 | `--message`, `--write`, `--force` |
| `stats` | 记忆统计 | 无 |
| `reflect` | 反思巩固 | `--days N` |
| `reconsolidate` | 再巩固 | `--days N`, `--dry-run` |
| `dedup` | 去重 | `--similarity F(0-1)`, `--dry-run` |
| `learn-synonyms` | 学习同义词 | `--dry-run`, `--top-k N` |
| `learned` | 查看学习数据 | `--top N`, `--reset` |
| `import` | 导入 | `--source PATH`, `--dry-run`, `--min-importance N` |
| `vacuum` | 清理整理 | 无 |

## 定时维护

| 频率 | 命令 | 说明 |
|------|------|------|
| 每天 22:30 | `hippocampus reflect --days 3` | 反思巩固近期记忆 |
| 每月 1 日 | `hippocampus vacuum` | 清理整理长期记忆 |

## 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `HIPPOCAMPUS_HOME` | 数据目录 | `~/.hippocampus` |
