// Apeireth 桌面伙伴 — Agent Runtime Contract (Phase 4, §15)
//
// 目标形态 (§12):  UI → Agent Runtime Contract → Adapter → Provider
// UI 不直接依赖 OpenAI SDK / 裸 fetch, 只面对本文件定义的 contract.
// 本文件 = contract 类型 + Apeireth HTTP/SSE adapter (后端 0 改动).
//
// §15 RuntimeEvent 至少预留: run-start / message-start / text-delta /
//   reasoning-delta / tool-call / tool-result / message-end / run-error / run-end
// §16 future: Commander/Worker 边界 — run() 的事件流形态天然可挂多节点/多设备.

import type {ApeirethConfig, ChatMessage, ModelSetup, RuntimeStatus} from './types';

const STORAGE_KEY = 'apeireth-config';

// ============================================================
// Runtime Contract 类型 (§15)
// ============================================================

/** 模型引用 — 不暴露 SDK provider 细节, 只给模型 id + 提供方名. */
export interface ModelReference {
  id: string;
  provider?: string;
  /** 本地昵称 (可选) */
  label?: string;
}

/** Agent 消息 — 标准化输入/输出消息. */
export interface AgentMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  id?: string;
  timestamp?: number;
}

/** Agent 运行请求. */
export interface AgentRunRequest {
  messages: AgentMessage[];
  model: ModelReference;
  /** 会话上下文 ID (future: 多设备共享会话) */
  sessionId?: string;
  /** 用户/人格上下文 (future: persona / long-term memory 注入点) */
  context?: {
    persona?: string;
    user?: string;
  };
  signal?: AbortSignal;
}

/** 运行期事件 — 可辨识联合, UI 用 switch 分支渲染. */
export type RuntimeEvent =
  | {type: 'run-start'; requestId: string}
  | {type: 'message-start'; requestId: string; messageId: string}
  | {type: 'text-delta'; requestId: string; text: string}
  | {type: 'reasoning-delta'; requestId: string; text: string}
  | {type: 'tool-call'; requestId: string; tool: string; args?: unknown}
  | {type: 'tool-result'; requestId: string; tool: string; ok: boolean; summary?: string}
  | {type: 'message-end'; requestId: string; messageId: string; fullText: string}
  | {type: 'run-error'; requestId: string; error: RuntimeError}
  | {type: 'run-end'; requestId: string; aborted: boolean};

/** 标准化运行时错误. */
export interface RuntimeError {
  code: 'http' | 'network' | 'auth' | 'timeout' | 'aborted' | 'unknown';
  message: string;
  /** 原始 HTTP 状态码 (若适用) */
  status?: number;
}

/** Agent Runtime 接口 — UI 的唯一对话入口. */
export interface AgentRuntime {
  /** 发起一次 agent run, 事件经 onEvent 推送. 返回完整文本. */
  run(request: AgentRunRequest, onEvent: (event: RuntimeEvent) => void): Promise<string>;
  /** 中断当前 run. */
  abort(): void;
  /** 当前是否在运行. */
  readonly running: boolean;
  /** 健康检查. */
  health(): Promise<boolean>;
}

/** 从 HTTP 状态码推断标准化错误码. */
export function classifyHttpError(status: number): RuntimeError['code'] {
  if (status === 401 || status === 403) return 'auth';
  if (status === 404) return 'http';
  if (status >= 500) return 'http';
  return 'http';
}

/** 带 HTTP 状态码的传输错误 — streamChat/chatOnce 抛此, toRuntimeError 可分类. */
export class HttpError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.name = 'HttpError';
    this.status = status;
  }
}

export function toRuntimeError(caught: unknown): RuntimeError {
  if (caught instanceof DOMException && caught.name === 'AbortError') {
    return {code: 'aborted', message: '已中止'};
  }
  if (caught instanceof TypeError) {
    return {code: 'network', message: '网络错误: 后端不可达'};
  }
  if (caught instanceof HttpError) {
    return {
      code: classifyHttpError(caught.status),
      message: caught.message,
      status: caught.status,
    };
  }
  const message = caught instanceof Error ? caught.message : String(caught);
  return {code: 'unknown', message};
}

export function loadConfig(): ApeirethConfig {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw) as ApeirethConfig;
  } catch {
    // ignore corrupted config
  }
  return {baseUrl: 'http://127.0.0.1:8090', apiKey: '', model: 'MiniMax-M3'};
}

export function saveConfig(config: ApeirethConfig): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(config));
}

export function toModelSetup(config: ApeirethConfig): ModelSetup {
  return {baseUrl: config.baseUrl, apiKey: config.apiKey, model: config.model};
}

function normalizeBaseUrl(baseUrl: string): string {
  return baseUrl.replace(/\/+$/, '');
}

async function checkJson(response: Response): Promise<unknown> {
  if (!response.ok) {
    const text = await response.text().catch(() => '');
    throw new HttpError(response.status, `HTTP ${response.status} ${text.slice(0, 300)}`);
  }
  return response.json();
}

export async function checkHealth(baseUrl: string): Promise<boolean> {
  try {
    const response = await fetch(`${normalizeBaseUrl(baseUrl)}/health`, {signal: AbortSignal.timeout(3000)});
    return response.ok;
  } catch {
    return false;
  }
}

export async function listModels(baseUrl: string, apiKey: string): Promise<string[]> {
  const response = await fetch(`${normalizeBaseUrl(baseUrl)}/v1/models`, {
    headers: {Authorization: `Bearer ${apiKey}`},
  });
  const data = await checkJson(response) as {data?: Array<{id: string}>};
  return (data.data || []).map((item) => item.id);
}

/**
 * 流式聊天: 通过 SSE 拉取 OpenAI 兼容 chat completion.
 * 每个 text delta 回调 onDelta; 结束回调 onDone.
 */
export async function streamChat(
  config: ApeirethConfig,
  messages: Array<{role: 'user' | 'assistant' | 'system'; content: string}>,
  onDelta: (text: string) => void,
  signal?: AbortSignal,
): Promise<string> {
  const base = normalizeBaseUrl(config.baseUrl);
  const response = await fetch(`${base}/v1/chat/completions`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${config.apiKey}`,
    },
    body: JSON.stringify({
      model: config.model,
      messages,
      stream: true,
    }),
    signal,
  });

  if (!response.ok) {
    const text = await response.text().catch(() => '');
    throw new HttpError(response.status, `HTTP ${response.status} ${text.slice(0, 300)}`);
  }
  if (!response.body) throw new Error('响应无 body');

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';
  let full = '';

  try {
    while (true) {
      const {done, value} = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, {stream: true});
      const lines = buffer.split('\n');
      buffer = lines.pop() || '';

      for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed || !trimmed.startsWith('data:')) continue;
        const payload = trimmed.slice(5).trim();
        if (payload === '[DONE]') return full;
        try {
          const json = JSON.parse(payload) as {
            choices?: Array<{delta?: {content?: string}; message?: {content?: string}}>;
          };
          const delta = json.choices?.[0]?.delta?.content;
          if (delta) {
            full += delta;
            onDelta(delta);
          }
        } catch {
          // skip malformed SSE line
        }
      }
    }
  } finally {
    reader.releaseLock();
  }
  return full;
}

/** 非流式聊天 (用于简单问答/健康检查). */
export async function chatOnce(config: ApeirethConfig, prompt: string): Promise<string> {
  const response = await fetch(`${normalizeBaseUrl(config.baseUrl)}/v1/chat/completions`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${config.apiKey}`,
    },
    body: JSON.stringify({
      model: config.model,
      messages: [{role: 'user', content: prompt}],
      stream: false,
    }),
  });
  const data = await checkJson(response) as {
    choices?: Array<{message?: {content?: string}}>;
  };
  return data.choices?.[0]?.message?.content || '';
}

export function runtimeStatus(baseUrl: string, model?: string): RuntimeStatus {
  return {connected: false, baseUrl, model};
}

// ============================================================
// AgentRuntime 工厂 — contract 的 Apeireth HTTP/SSE adapter
// ============================================================

/**
 * 创建 AgentRuntime 实例. UI 只面对这个接口, 不直接碰 fetch/SSE.
 *
 * §16 future: 此形态预留 Commander/Worker — run() 的事件流可经 socket/
 * bus 透传到多节点/多设备; context 注入点 (persona/memory) 已留位.
 */
export function createAgentRuntime(config: ApeirethConfig): AgentRuntime {
  let abortController: AbortController | null = null;
  let _running = false;

  const runtime: AgentRuntime = {
    get running() {
      return _running;
    },

    async run(request, onEvent) {
      _running = true;
      abortController = new AbortController();
      const requestId = crypto.randomUUID();

      try {
        onEvent({type: 'run-start', requestId});
        onEvent({type: 'message-start', requestId, messageId: requestId});

        // Adapter: contract → Apeireth HTTP/SSE. future 可替换为其他 provider.
        const full = await streamChat(
          config,
          request.messages.map((m) => ({role: m.role, content: m.content})),
          (delta) => onEvent({type: 'text-delta', requestId, text: delta}),
          request.signal ?? abortController.signal,
        );

        onEvent({type: 'message-end', requestId, messageId: requestId, fullText: full});
        onEvent({type: 'run-end', requestId, aborted: false});
        return full;
      } catch (caught) {
        const error = toRuntimeError(caught);
        if (error.code !== 'aborted') {
          onEvent({type: 'run-error', requestId, error});
        }
        onEvent({type: 'run-end', requestId, aborted: error.code === 'aborted'});
        throw error;
      } finally {
        _running = false;
        abortController = null;
      }
    },

    abort() {
      abortController?.abort();
    },

    async health() {
      return checkHealth(config.baseUrl);
    },
  };

  return runtime;
}

/**
 * Apeireth V2 记忆端点 — 记忆可视 (docs/frontend-guide.md P1-2)
 * GET /v1/memory/episodes?limit=N&session=X → {items: Episode[]}
 */
export interface MemoryEpisode {
  id: string;
  timestamp: number;
  role: string;
  content: string;
  session_id: string;
}

export async function fetchEpisodes(config: ApeirethConfig, limit = 50): Promise<MemoryEpisode[]> {
  const response = await fetch(`${normalizeBaseUrl(config.baseUrl)}/v1/memory/episodes?limit=${limit}`, {
    headers: {Authorization: `Bearer ${config.apiKey}`},
  });
  const data = await checkJson(response) as {items?: MemoryEpisode[]};
  return data.items || [];
}

/**
 * Apeireth V2 工具端点 — 工具透明 (docs/frontend-guide.md P1-3)
 * GET /v1/tools/list → 工具注册表
 */
export interface ToolInfo {
  name: string;
  description?: string;
  args_schema?: unknown;
}

export async function fetchTools(config: ApeirethConfig): Promise<ToolInfo[]> {
  const response = await fetch(`${normalizeBaseUrl(config.baseUrl)}/v1/tools/list`, {
    headers: {Authorization: `Bearer ${config.apiKey}`},
  });
  const data = await checkJson(response) as {tools?: ToolInfo[]};
  return data.tools || [];
}

/**
 * Apeireth V2 器官端点 — 器官状态
 * GET /v1/organs → 器官列表
 */
export async function fetchOrgans(config: ApeirethConfig): Promise<unknown[]> {
  const response = await fetch(`${normalizeBaseUrl(config.baseUrl)}/v1/organs`, {
    headers: {Authorization: `Bearer ${config.apiKey}`},
  });
  return checkJson(response) as Promise<unknown[]>;
}

/**
 * 会话持久化 — 存 localStorage (前端侧). Apeireth 后端记忆走 companion memory,
 * 这里只存 UI 会话历史.
 */
export function loadConversations(): Array<{id: string; title: string; createdAt: number; updatedAt: number; messages: ChatMessage[]; archived?: boolean; scope: 'global' | 'project'; projectId?: string}> {
  try {
    const raw = localStorage.getItem('apeireth-conversations');
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}

export function saveConversations(conversations: unknown): void {
  localStorage.setItem('apeireth-conversations', JSON.stringify(conversations));
}
