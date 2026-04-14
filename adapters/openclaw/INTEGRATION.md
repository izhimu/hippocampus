# OpenClaw 集成指南

## 安装步骤

### 1. 编译安装

```bash
cd /home/bot/code/hippocampus
cargo install --path .
```

### 2. 配置 SKILL.md

```bash
mkdir -p ~/.openclaw/workspace/skills/hippocampus
ln -sf /home/bot/code/hippocampus/adapters/openclaw/SKILL.md ~/.openclaw/workspace/skills/hippocampus/SKILL.md
```

### 3. 配置数据目录

在 `~/.openclaw/openclaw.json` 的 `env.vars` 中添加：

```json
"HIPPOCAMPUS_HOME": "/home/bot/.openclaw/workspace/cognitive_memory"
```

### 4. 初始化数据

```bash
export HIPPOCAMPUS_HOME=/home/bot/.openclaw/workspace/cognitive_memory
hippocampus init
```

## AGENTS.md 记忆规则更新建议

将原有 `python3 memorize.py` 调用替换为：

| 旧方式 | 新方式 |
|--------|--------|
| `python3 memorize.py "内容"` | `hippocampus remember --content "内容" --importance 7 --source dialogue` |
| `cognitive_search "关键词"` | `hippocampus recall --query "关键词" --top-k 5` |
| `reflect 22:30` | `hippocampus reflect --days 3` |

建议更新 AGENTS.md 第四章：
- 记忆检索：优先使用 `hippocampus recall` 替代 `memory_search`
- 记忆写入：使用 `hippocampus remember` 替代 `memorize`
- 自动记忆：使用 `hippocampus gate --message "..." --write`

## 定时任务配置示例

在 OpenClaw 中配置以下定时任务：

### 每日反思巩固（22:30）

```json
{
  "name": "Task_Hippocampus_Reflect_2230",
  "cron": "30 22 * * *",
  "task": "执行 hippocampus reflect --days 3 进行记忆巩固，然后 hippocampus vacuum 清理。将结果发送给栀暮主人（QQ: 1334642674）。"
}
```

### 每月再巩固（1号凌晨）

```json
{
  "name": "Task_Hippocampus_Reconsolidate_Monthly",
  "cron": "0 3 1 * *",
  "task": "执行 hippocampus reconsolidate --days 30 --dry-run 检查，然后执行 hippoccampus reconsolidate --days 30。将结果发送给栀暮主人。"
}
```

### 每周去重检测（周日 23:00）

```json
{
  "name": "Task_Hippocampus_Dedup_Weekly",
  "cron": "0 23 * * 0",
  "task": "执行 hippocampus dedup --dry-run 检查重复记忆数量，如果超过 50 条则执行 hippocampus dedup。"
}
```

## 心跳自动记忆

在心跳逻辑中，可以使用 `hippocampus gate` 自动评估对话是否值得记忆：

```bash
# 评估当前对话是否值得记忆
hippocampus gate --message "用户消息内容" --write
```

`gate` 会通过 4 个脑区协同判断（杏仁核·海马体·前额叶·颞叶），自动决定是否写入记忆。
