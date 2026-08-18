// Apeireth 桌面伙伴 — 共享类型 (从 Pattern apps/desktop 移植, 精简)
// 已移除 Computer Use / AgentOS 残留字段 (recovery/screenshotPath/steps[].tier)

export type ViewId = 'chat' | 'conversations' | 'memory' | 'settings';
export type Theme = 'night' | 'day' | 'ocean' | 'forest' | 'paper';
export type MemoryCategory = '事实' | '偏好' | '事件' | '反馈' | '参考';
export type ConversationScope = 'global' | 'project';

export interface ChatMessageEvent {
  id: string;
  kind: 'status' | 'tool' | 'task' | 'mcp' | 'memory' | 'agent' | 'error' | string;
  text: string;
  ts?: number;
  status?: 'pending' | 'running' | 'done' | 'failed' | 'skipped' | 'awaiting_approval' | string;
  action?: string;
  /** 工具风险等级 (T1-T3) — 工具透明 UI 保留展示 */
  tier?: number;
  receipt?: string;
  taskId?: string;
  stepId?: string;
}

export interface TaskCardInfo {
  taskId: string;
  title: string;
  status: string;
  detail?: string;
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  text: string;
  time: string;
  proactive?: string;
  events?: ChatMessageEvent[];
  error?: string;
  streaming?: boolean;
  taskCard?: TaskCardInfo;
}

export interface Conversation {
  id: string;
  title: string;
  createdAt: number;
  updatedAt: number;
  messages: ChatMessage[];
  archived?: boolean;
  scope: ConversationScope;
  projectId?: string;
}

export interface ModelSetup {
  baseUrl: string;
  apiKey: string;
  model: string;
}

export interface RuntimeStatus {
  connected: boolean;
  baseUrl: string;
  model?: string;
}

// Apeireth 端点配置 (持久化到 localStorage)
export interface ApeirethConfig {
  baseUrl: string;
  apiKey: string;
  model: string;
}

export function categoryToWire(category: MemoryCategory | string): string {
  const map: Record<string, string> = {
    事实: 'fact',
    偏好: 'preference',
    事件: 'event',
    反馈: 'feedback',
    参考: 'reference',
  };
  return map[category] || category;
}

export function categoryFromWire(wire: string): MemoryCategory {
  const map: Record<string, MemoryCategory> = {
    fact: '事实',
    preference: '偏好',
    event: '事件',
    feedback: '反馈',
    reference: '参考',
  };
  return map[wire] || '事实';
}

export function importanceStars(value: number): 1 | 2 | 3 {
  if (value >= 0.75) return 3;
  if (value >= 0.4) return 2;
  return 1;
}
