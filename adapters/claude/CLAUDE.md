# Hippocampus 记忆系统

本项目集成 Hippocampus 仿生认知记忆系统。

## 可用命令
- `hippocampus recall --query "关键词" --top-k 5` — 检索相关记忆
- `hippocampus remember --content "内容" --importance N --source claude-code --tags "标签"` — 写入记忆
- `hippocampus gate --message "用户消息" --write` — 自动评估并记忆
- `hippocampus gate --message "用户消息"` — 仅评估不写入
- `hippocampus stats` — 查看记忆统计
- `hippocampus reflect --days 3` — 反思巩固

## 记忆规则
- 用户表达偏好、决策、重要信息时 → 用 remember 写入（importance 6-9）
- 用户说"记住" → 用 gate --write 强制记忆
- 开始复杂任务前 → 先 recall 相关历史
- 完成重要任务后 → remember 记录结果
- 情绪强烈的对话 → gate --write 自动记忆
- 每日结束工作 → reflect --days 1

## 数据目录
- 默认：~/.hippocampus
- 环境变量：HIPPOCAMPUS_HOME

## 重要性参考
- 1-3: 日常闲聊、临时信息
- 4-5: 一般信息、工作记录
- 6-7: 用户偏好、重要决策
- 8-9: 核心实体、关键配置
- 10: 永久记忆（不会遗忘）
