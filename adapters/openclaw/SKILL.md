# Hippocampus 记忆系统

本地 Rust CLI 记忆管理工具，路径：`/home/bot/.cargo/bin/hippocampus`，通过 exec 调用。

## 适用场景
用户涉及记忆查询、个人信息、历史对话、偏好回忆、上下文关联时自动匹配。

## 记忆规则

### 对话前：召回（必执行，简单问候除外）
```bash
hippocampus recall --query "用户消息关键词" --top-k 3
```

### 对话后：门控写入（必执行，简单问候除外）
```bash
hippocampus gate --message "用户原始消息" --write
```

### 简单问候例外
纯"你好""早安""晚安"等可跳过 recall 和 gate。

### 手动记忆
用户明确说"记住"时：`hippocampus remember --content "内容" --importance 8`
