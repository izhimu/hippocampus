import { execSync } from 'child_process';

/**
 * Hippocampus Memory Hook Handler
 *
 * Maps OpenClaw hook events to hippocampus CLI commands, injects recalled memories
 * as context into the event messages stream.
 */

export default async function handler(event: any): Promise<void> {
  const { type, action } = event;

  // Map OpenClaw event to hippocampus hook action
  let hookAction: string | null = null;

  if (type === 'message' && action === 'received') {
    // 收到消息 → recall 相关记忆 + gate 评估
    hookAction = 'auto';
  } else if (type === 'message' && action === 'preprocessed') {
    // 预处理消息 → gate 评估
    hookAction = 'record';
  } else if (type === 'message' && action === 'sent') {
    // 成功发送的消息 → gate 评估
    if (event.context?.success === true) {
      hookAction = 'record';
    }
  } else if (type === 'session' && action === 'compact:before') {
    // 会话压缩前 → 摘要归档
    hookAction = 'summarize';
  } else if (type === 'command' && action === 'new') {
    // 新会话 → 反思巩固
    hookAction = 'reflect';
  }

  if (!hookAction) return;

  try {
    const result = execSync(`hippocampus hook ${hookAction} --openclaw`, {
      input: JSON.stringify(event),
      encoding: 'utf-8',
      timeout: 10000,
    });

    if (result?.trim()) {
      const parsed = JSON.parse(result.trim());
      if (parsed.context) {
        event.messages = event.messages || [];
        event.messages.push({ role: 'system', content: parsed.context });
      }
    }
  } catch {
    // Silent error handling — memory failures should not disrupt conversation
  }
}
