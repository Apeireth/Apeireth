// Apeireth 桌面伙伴 — 核心共享类型定义 (Svelte 5 + Tauri 2)

export type ViewId = 'chat' | 'conversations' | 'activity' | 'tools' | 'memory' | 'settings';
export type Theme = 'night' | 'day' | 'ocean' | 'forest' | 'paper';
export type MemoryCategory =
  | '工作记忆'
  | '近期记忆'
  | '长期记忆'
  | '用户画像'
  | '知识'
  | '事实'
  | '偏好'
  | '事件'
  | '反馈'
  | '参考';

export type ConversationScope = 'global' | 'project';

export interface ToolCallDetails {
  id: string;
  name: string;
  args?: unknown;
  rawArgs?: string;
  status: 'pending' | 'running' | 'succeeded' | 'failed' | 'cancelled';
  resultSummary?: string;
  resultFull?: string;
  error?: string;
  durationMs?: number;
  startTime?: number;
  endTime?: number;
}

export interface ChatMessageEvent {
  id: string;
  kind: 'status' | 'tool' | 'task' | 'mcp' | 'memory' | 'agent' | 'error' | string;
  text: string;
  ts?: number;
  status?: 'pending' | 'running' | 'done' | 'failed' | 'skipped' | 'awaiting_approval' | string;
  action?: string;
  /** 工具风险等级 (T1-T3) */
  tier?: number;
  receipt?: string;
  taskId?: string;
  stepId?: string;
  toolCall?: ToolCallDetails;
}

export interface TaskCardInfo {
  taskId: string;
  title: string;
  status: string;
  detail?: string;
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  text: string;
  time: string;
  timestamp?: number;
  proactive?: string;
  events?: ChatMessageEvent[];
  error?: string;
  streaming?: boolean;
  taskCard?: TaskCardInfo;
  toolCalls?: ToolCallDetails[];
  modelInfo?: {
    id: string;
    provider?: string;
  };
}

export interface Conversation {
  id: string;
  title: string;
  createdAt: number;
  updatedAt: number;
  messages: ChatMessage[];
  archived?: boolean;
  pinned?: boolean;
  scope: ConversationScope;
  projectId?: string;
  model?: string;
}

export interface ModelSetup {
  baseUrl: string;
  apiKey: string;
  model: string;
}

export interface SubsystemStatus {
  name: string;
  key: 'api' | 'companion' | 'memory' | 'tools' | 'events' | 'sessions';
  status: 'ok' | 'degraded' | 'offline' | 'unknown';
  endpoint: string;
  detail?: string;
  latencyMs?: number;
}

export interface RuntimeHealthReport {
  overall: 'connecting' | 'online' | 'degraded' | 'offline' | 'error';
  baseUrl: string;
  latencyMs?: number;
  lastChecked?: number;
  subsystems: SubsystemStatus[];
  model: string;
  error?: string;
}

export type HealthState = 'connecting' | 'online' | 'ready' | 'degraded' | 'generating' | 'error' | 'offline';

export interface ApeirethConfig {
  baseUrl: string;
  apiKey: string;
  model: string;
  theme?: Theme;
}

export interface ActivityItem {
  id: string;
  timestamp: number;
  category: 'conversation' | 'agent' | 'tool' | 'memory' | 'workflow' | 'runtime' | 'error';
  title: string;
  summary: string;
  source: 'sse' | 'audit' | 'runtime' | 'local';
  severity: 'info' | 'success' | 'warning' | 'error';
  detail?: string;
  raw?: unknown;
}

export interface MemoryEpisodeItem {
  id: string;
  timestamp: number;
  role: string;
  content: string;
  sessionId: string;
  category?: string;
  stream?: string;
  importance?: number;
  tags?: string[];
}

export interface ToolItem {
  name: string;
  description?: string;
  argsSchema?: unknown;
  source?: 'builtin' | 'mcp' | 'extension';
  permission?: 'none' | 'prompt' | 'granted' | 'restricted';
  lastUsed?: number;
  available: boolean;
}

export interface ApprovalRequestItem {
  id: string;
  chain?: string;
  rev?: number;
  tool: string;
  reason?: string;
  args_preview?: string;
  summary?: string;
  requestedAt?: number;
  params?: unknown;
  status: 'pending' | 'approved' | 'expired' | 'rejected';
}


export function categoryToWire(category: MemoryCategory | string): string {
  const map: Record<string, string> = {
    工作记忆: 'working',
    近期记忆: 'recent',
    长期记忆: 'long_term',
    用户画像: 'profile',
    知识: 'knowledge',
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
    working: '工作记忆',
    recent: '近期记忆',
    long_term: '长期记忆',
    profile: '用户画像',
    knowledge: '知识',
    fact: '长期记忆',
    preference: '用户画像',
    event: '近期记忆',
    feedback: '近期记忆',
    reference: '知识',
  };
  return map[wire] || '长期记忆';
}

export function importanceStars(value: number): 1 | 2 | 3 {
  if (value >= 0.75) return 3;
  if (value >= 0.4) return 2;
  return 1;
}
