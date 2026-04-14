---
name: hippocampus
description: 🧠 仿生认知记忆系统（Rust）— 基于Engram印迹的AI Agent记忆增强
---

# Hippocampus 记忆系统

## 记忆操作
- 记住信息：exec `hippocampus remember --content "内容" --importance N --source dialogue --tags "标签1,标签2"`
- 检索记忆：exec `hippocampus recall --query "关键词" --top-k 5`
- 自动评估+写入：exec `hippocampus gate --message "用户消息" --write`
- 仅评估不写入：exec `hippocampus gate --message "用户消息"`

## 记忆维护
- 反思巩固：exec `hippocampus reflect --days 3`
- 记忆再巩固：exec `hippocampus reconsolidate --days 30`
- 去重检测：exec `hippocampus dedup --dry-run`
- 清理整理：exec `hippocampus vacuum`
- 统计信息：exec `hippocampus stats`

## 数据目录
- 默认：~/.hippocampus
- 可通过 HIPPOCAMPUS_HOME 环境变量覆盖
- JSONL 分层存储：engrams_L1/L2/L3.jsonl

## 使用场景
- 重要对话 → gate --write 自动记忆
- 定时报告摘要 → remember 手动写入
- 回答问题前 → recall 检索相关记忆
- 每日维护 → reflect + vacuum
