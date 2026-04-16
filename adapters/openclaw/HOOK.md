---
name: hippocampus
version: 0.1.0
description: >
  Hippocampus 仿生认知记忆系统 Hook。自动在对话中召回相关记忆、门控评估消息是否值得记忆、
  会话压缩前摘要归档、新会话启动时反思巩固。为 Agent 提供持久化的认知记忆能力。
events:
  - message:received
  - message:sent
  - message:preprocessed
  - session:compact:before
  - command:new
---

# Hippocampus Memory Hook

集成 Hippocampus 仿生认知记忆系统到 OpenClaw。

## 事件映射

| 事件 | Hippocampus 动作 | 说明 |
|------|-----------------|------|
| `message:received` | auto (recall + gate) | 收到消息时召回相关记忆并评估 |
| `message:preprocessed` | record (gate) | 预处理后的消息进行门控评估 |
| `message:sent` | record (gate) | 成功发送的消息进行门控评估 |
| `session:compact:before` | summarize | 会话压缩前将对话摘要归档 |
| `command:new` | reflect | 新会话启动时执行记忆反思巩固 |

## 依赖

- `hippocampus` CLI 已安装并在 PATH 中
- `HIPPOCAMPUS_HOME` 环境变量已配置
