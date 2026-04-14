# Claude Code 集成指南

## 安装步骤

```bash
cd /home/bot/code/hippocampus
cargo install --path .
hippocampus init  # 初始化数据目录
```

## CLAUDE.md 放置

### 项目级集成
将 `adapters/claude/CLAUDE.md` 复制到项目根目录：

```bash
cp adapters/claude/CLAUDE.md ./CLAUDE.md
```

### 全局集成（推荐）
复制到 Claude Code 全局配置：

```bash
mkdir -p ~/.claude
cp adapters/claude/CLAUDE.md ~/.claude/CLAUDE.md
```

## Hooks 配置

在项目的 `.claude/settings.json` 或全局 `~/.claude/settings.json` 中配置：

```json
{
  "hooks": {
    "Stop": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "hippocampus gate --message \"$CLAUDE_LAST_USER_MESSAGE\" --write 2>/dev/null || true"
          }
        ]
      }
    ]
  }
}
```

参考 `hooks-example.json` 获取完整配置示例。

## MCP Server（规划中）

未来计划提供 MCP Server 接口，支持：
- `memory_recall` tool — 检索记忆
- `memory_remember` tool — 写入记忆
- `memory_gate` tool — 自动评估并记忆

## 数据目录配置

```bash
export HIPPOCAMPUS_HOME=~/.hippocampus  # 默认
# 或自定义路径
export HIPPOCAMPUS_HOME=/path/to/cognitive_memory
```

## 验证

```bash
hippocampus stats          # 查看记忆统计
hippocampus recall --query "测试" --top-k 3  # 测试检索
```
