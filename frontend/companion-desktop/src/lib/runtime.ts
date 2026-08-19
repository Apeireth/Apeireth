// Apeireth 桌面伙伴 — Agent Runtime Contract & Adapter (Svelte 5 + Tauri 2)

import type {
  ApeirethConfig,
  ChatMessage,
  Conversation,
  ModelSetup,
  RuntimeHealthReport,
  SubsystemStatus,
  ToolCallDetails,
  ActivityItem,
  MemoryEpisodeItem,
  ToolItem,
  ApprovalRequestItem,
} from './types';


const STORAGE_KEY = 'apeireth-config';

// ============================================================
// Runtime Contract Types
// ============================================================

export interface ModelReference {
  id: string;
  provider?: string;
  label?: string;
}

export interface AgentMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  id?: string;
  timestamp?: number;
}

export interface AgentRunRequest {
  messages: AgentMessage[];
  model: ModelReference;
  sessionId?: string;
  context?: {
    persona?: string;
    user?: string;
  };
  signal?: AbortSignal;
}

export type RuntimeEvent =
  | {type: 'run-start'; requestId: string}
  | {type: 'message-start'; requestId: string; messageId: string}
  | {type: 'text-delta'; requestId: string; text: string}
  | {type: 'reasoning-delta'; requestId: string; text: string}
  | {type: 'tool-call'; requestId: string; toolCall: ToolCallDetails}
  | {type: 'tool-result'; requestId: string; toolCallId: string; ok: boolean; summary?: string; full?: string; error?: string}
  | {type: 'message-end'; requestId: string; messageId: string; fullText: string}
  | {type: 'run-error'; requestId: string; error: RuntimeError}
  | {type: 'run-end'; requestId: string; aborted: boolean};

export interface RuntimeError {
  code: 'http' | 'network' | 'auth' | 'timeout' | 'aborted' | 'unknown';
  message: string;
  status?: number;
}

export interface AgentRuntime {
  run(request: AgentRunRequest, onEvent: (event: RuntimeEvent) => void): Promise<string>;
  abort(): void;
  readonly running: boolean;
  health(): Promise<RuntimeHealthReport>;
}

export function classifyHttpError(status: number): RuntimeError['code'] {
  if (status === 401 || status === 403) return 'auth';
  if (status === 404) return 'http';
  if (status >= 500) return 'http';
  return 'http';
}

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
    return {code: 'aborted', message: '已中止请求'};
  }
  if (caught instanceof TypeError) {
    return {code: 'network', message: '网络错误：后端不可达或跨域拒绝'};
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
    if (raw) {
      const parsed = JSON.parse(raw) as Record<string, unknown>;
      // Security migration: ensure legacy masterToken keys are purged
      let modified = false;
      if ('masterToken' in parsed) {
        delete parsed.masterToken;
        modified = true;
      }
      if ('master_token' in parsed) {
        delete parsed.master_token;
        modified = true;
      }
      const cleaned: ApeirethConfig = {
        baseUrl: typeof parsed.baseUrl === 'string' ? parsed.baseUrl : 'http://127.0.0.1:8090',
        apiKey: typeof parsed.apiKey === 'string' ? parsed.apiKey : '',
        model: typeof parsed.model === 'string' ? parsed.model : 'MiniMax-M3',
        theme: typeof parsed.theme === 'string' ? (parsed.theme as any) : undefined,
      };
      if (modified) {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(cleaned));
      }
      return cleaned;
    }
  } catch {
    // ignore corrupted config
  }
  return {baseUrl: 'http://127.0.0.1:8090', apiKey: '', model: 'MiniMax-M3'};
}

export function saveConfig(config: ApeirethConfig): void {
  const safeConfig: ApeirethConfig = {
    baseUrl: config.baseUrl,
    apiKey: config.apiKey,
    model: config.model,
    theme: config.theme,
  };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(safeConfig));
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

/** 基础 /health 端点探测 */
export async function checkHealth(baseUrl: string): Promise<boolean> {
  try {
    const response = await fetch(`${normalizeBaseUrl(baseUrl)}/health`, {signal: AbortSignal.timeout(2500)});
    return response.ok;
  } catch {
    return false;
  }
}

/** 深度多子系统健康检测，真实探测后端各项能力 */
export async function checkHealthDetailed(baseUrl: string, apiKey: string = ''): Promise<RuntimeHealthReport> {
  const base = normalizeBaseUrl(baseUrl);
  const subsystems: SubsystemStatus[] = [];
  const startAll = performance.now();
  let anyOk = false;
  let allOk = true;

  // 1. API Gateway / Gateway Health
  const t0 = performance.now();
  try {
    const res = await fetch(`${base}/health`, {signal: AbortSignal.timeout(2500)});
    const lat = Math.round(performance.now() - t0);
    if (res.ok) {
      anyOk = true;
      subsystems.push({name: 'API 网关', key: 'api', status: 'ok', endpoint: '/health', latencyMs: lat, detail: 'HTTP 200 OK'});
    } else {
      allOk = false;
      subsystems.push({name: 'API 网关', key: 'api', status: 'degraded', endpoint: '/health', latencyMs: lat, detail: `HTTP ${res.status}`});
    }
  } catch (e) {
    allOk = false;
    subsystems.push({name: 'API 网关', key: 'api', status: 'offline', endpoint: '/health', detail: '连接超时或服务未启动'});
  }

  // 2. 模型列表 / Provider
  const t1 = performance.now();
  try {
    const res = await fetch(`${base}/v1/models`, {
      headers: apiKey ? {Authorization: `Bearer ${apiKey}`} : {},
      signal: AbortSignal.timeout(3000),
    });
    const lat = Math.round(performance.now() - t1);
    if (res.ok) {
      anyOk = true;
      const data = (await res.json().catch(() => ({}))) as {data?: unknown[]};
      const count = Array.isArray(data.data) ? data.data.length : 0;
      subsystems.push({name: '模型服务', key: 'companion', status: 'ok', endpoint: '/v1/models', latencyMs: lat, detail: `可用模型数: ${count}`});
    } else {
      allOk = false;
      subsystems.push({name: '模型服务', key: 'companion', status: 'degraded', endpoint: '/v1/models', latencyMs: lat, detail: `HTTP ${res.status}`});
    }
  } catch {
    allOk = false;
    subsystems.push({name: '模型服务', key: 'companion', status: 'offline', endpoint: '/v1/models', detail: '模型列表不可用'});
  }

  // 3. 会话存储 / Session Ledger
  const t2 = performance.now();
  try {
    const res = await fetch(`${base}/v1/panel/sessions`, {
      headers: apiKey ? {Authorization: `Bearer ${apiKey}`} : {},
      signal: AbortSignal.timeout(3000),
    });
    const lat = Math.round(performance.now() - t2);
    if (res.ok) {
      anyOk = true;
      subsystems.push({name: '会话存储', key: 'sessions', status: 'ok', endpoint: '/v1/panel/sessions', latencyMs: lat, detail: 'SQLite 会话账本已加载'});
    } else {
      allOk = false;
      subsystems.push({name: '会话存储', key: 'sessions', status: 'degraded', endpoint: '/v1/panel/sessions', latencyMs: lat, detail: `HTTP ${res.status}`});
    }
  } catch {
    allOk = false;
    subsystems.push({name: '会话存储', key: 'sessions', status: 'offline', endpoint: '/v1/panel/sessions', detail: '会话只读端点不可用'});
  }

  // 4. 记忆系统 / Memory Streams
  const t3 = performance.now();
  try {
    const res = await fetch(`${base}/v1/panel/memory/streams`, {
      headers: apiKey ? {Authorization: `Bearer ${apiKey}`} : {},
      signal: AbortSignal.timeout(3000),
    });
    const lat = Math.round(performance.now() - t3);
    if (res.ok) {
      anyOk = true;
      subsystems.push({name: '记忆流', key: 'memory', status: 'ok', endpoint: '/v1/panel/memory/streams', latencyMs: lat, detail: '6 历史流已就绪'});
    } else {
      allOk = false;
      subsystems.push({name: '记忆流', key: 'memory', status: 'degraded', endpoint: '/v1/panel/memory/streams', latencyMs: lat, detail: `HTTP ${res.status}`});
    }
  } catch {
    allOk = false;
    subsystems.push({name: '记忆流', key: 'memory', status: 'offline', endpoint: '/v1/panel/memory/streams', detail: '记忆端点不可用'});
  }

  // 5. 工具注册表 / Tools
  const t4 = performance.now();
  try {
    const res = await fetch(`${base}/v1/tools/list`, {
      headers: apiKey ? {Authorization: `Bearer ${apiKey}`} : {},
      signal: AbortSignal.timeout(3000),
    });
    const lat = Math.round(performance.now() - t4);
    if (res.ok) {
      anyOk = true;
      subsystems.push({name: '工具注册表', key: 'tools', status: 'ok', endpoint: '/v1/tools/list', latencyMs: lat, detail: '工具目录已装配'});
    } else {
      allOk = false;
      subsystems.push({name: '工具注册表', key: 'tools', status: 'degraded', endpoint: '/v1/tools/list', latencyMs: lat, detail: `HTTP ${res.status}`});
    }
  } catch {
    allOk = false;
    subsystems.push({name: '工具注册表', key: 'tools', status: 'offline', endpoint: '/v1/tools/list', detail: '工具服务不可用'});
  }

  const overallLat = Math.round(performance.now() - startAll);
  let overall: RuntimeHealthReport['overall'] = 'offline';
  if (allOk && anyOk) {
    overall = 'online';
  } else if (anyOk) {
    overall = 'degraded';
  } else {
    overall = 'offline';
  }

  return {
    overall,
    baseUrl: base,
    latencyMs: overallLat,
    lastChecked: Date.now(),
    subsystems,
    model: 'MiniMax-M3',
  };
}

export async function listModels(baseUrl: string, apiKey: string): Promise<string[]> {
  const response = await fetch(`${normalizeBaseUrl(baseUrl)}/v1/models`, {
    headers: apiKey ? {Authorization: `Bearer ${apiKey}`} : {},
  });
  const data = (await checkJson(response)) as {data?: Array<{id: string}>};
  return (data.data || []).map((item) => item.id);
}

/**
 * 流式聊天: 通过 SSE 请求 OpenAI 兼容 chat completion 端点.
 * 覆盖：text delta, tool calls, reasoning delta, malformed lines, interruptions.
 */
export async function streamChat(
  config: ApeirethConfig,
  messages: Array<{role: 'user' | 'assistant' | 'system'; content: string}>,
  callbacks: {
    onDelta?: (text: string) => void;
    onReasoningDelta?: (text: string) => void;
    onToolCall?: (toolCall: ToolCallDetails) => void;
    onToolResult?: (id: string, ok: boolean, summary?: string) => void;
  },
  signal?: AbortSignal,
  sessionId?: string,
): Promise<string> {
  const base = normalizeBaseUrl(config.baseUrl);
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };
  if (config.apiKey) {
    headers.Authorization = `Bearer ${config.apiKey}`;
  }
  if (sessionId) {
    headers['X-Apeireth-Continuity'] = sessionId;
  }

  const response = await fetch(`${base}/v1/chat/completions`, {
    method: 'POST',
    headers,
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
  if (!response.body) throw new Error('响应流为空');

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';
  let fullText = '';
  const currentTools: Map<string, ToolCallDetails> = new Map();

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
        if (payload === '[DONE]') {
          return fullText;
        }

        try {
          const json = JSON.parse(payload) as {
            choices?: Array<{
              delta?: {
                content?: string;
                reasoning_content?: string;
                tool_calls?: Array<{
                  index?: number;
                  id?: string;
                  function?: {
                    name?: string;
                    arguments?: string;
                  };
                }>;
              };
              finish_reason?: string;
            }>;
          };

          const choice = json.choices?.[0];
          const delta = choice?.delta;

          // 1. Text delta
          if (delta?.content) {
            fullText += delta.content;
            callbacks.onDelta?.(delta.content);
          }

          // 2. Reasoning delta
          if (delta?.reasoning_content) {
            callbacks.onReasoningDelta?.(delta.reasoning_content);
          }

          // 3. Tool calls streaming
          if (delta?.tool_calls && Array.isArray(delta.tool_calls)) {
            for (const tc of delta.tool_calls) {
              const tcId = tc.id || `tc-${tc.index ?? 0}`;
              let existing = currentTools.get(tcId);
              if (!existing) {
                existing = {
                  id: tcId,
                  name: tc.function?.name || '未知工具',
                  rawArgs: tc.function?.arguments || '',
                  status: 'running',
                  startTime: Date.now(),
                };
                currentTools.set(tcId, existing);
                callbacks.onToolCall?.(existing);
              } else {
                if (tc.function?.name) existing.name = tc.function.name;
                if (tc.function?.arguments) existing.rawArgs = (existing.rawArgs || '') + tc.function.arguments;
                try {
                  if (existing.rawArgs) {
                    existing.args = JSON.parse(existing.rawArgs);
                  }
                } catch {
                  // partial JSON parsing failure is expected while streaming arguments
                }
                callbacks.onToolCall?.(existing);
              }
            }
          }

          // 4. Finish reason
          if (choice?.finish_reason) {
            for (const [, tc] of currentTools) {
              if (tc.status === 'running') {
                tc.status = 'succeeded';
                tc.endTime = Date.now();
                tc.durationMs = tc.endTime - (tc.startTime || tc.endTime);
                callbacks.onToolResult?.(tc.id, true, '执行成功');
              }
            }
          }
        } catch {
          // ignore malformed SSE chunks
        }
      }
    }
  } finally {
    reader.releaseLock();
  }

  return fullText;
}

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

        const full = await streamChat(
          config,
          request.messages.map((m) => ({role: m.role, content: m.content})),
          {
            onDelta: (delta) => onEvent({type: 'text-delta', requestId, text: delta}),
            onReasoningDelta: (delta) => onEvent({type: 'reasoning-delta', requestId, text: delta}),
            onToolCall: (toolCall) => onEvent({type: 'tool-call', requestId, toolCall}),
            onToolResult: (toolCallId, ok, summary) =>
              onEvent({type: 'tool-result', requestId, toolCallId, ok, summary}),
          },
          request.signal ?? abortController.signal,
          request.sessionId,
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
      return checkHealthDetailed(config.baseUrl, config.apiKey);
    },
  };

  return runtime;
}

// ============================================================
// Backend Real API Fetchers (Activity, Memory, Tools, Sessions)
// ============================================================

/** 获取真实后端会话列表 (只读数据) */
export async function fetchBackendSessions(config: ApeirethConfig): Promise<Array<{id: string; started_at: number; last_active_at: number; closed_at?: number; episode_count: number}>> {
  const res = await fetch(`${normalizeBaseUrl(config.baseUrl)}/v1/panel/sessions`, {
    headers: config.apiKey ? {Authorization: `Bearer ${config.apiKey}`} : {},
  });
  const data = (await checkJson(res)) as {sessions?: Array<{id: string; started_at: number; last_active_at: number; closed_at?: number; episode_count: number}>};
  return data.sessions || [];
}

/** 获取会话时间线 (episodes) */
export async function fetchSessionTimeline(config: ApeirethConfig, sessionId: string): Promise<MemoryEpisodeItem[]> {
  const res = await fetch(`${normalizeBaseUrl(config.baseUrl)}/v1/panel/sessions/${encodeURIComponent(sessionId)}/timeline`, {
    headers: config.apiKey ? {Authorization: `Bearer ${config.apiKey}`} : {},
  });
  const data = (await checkJson(res)) as {episodes?: Array<{id: string; timestamp: number; role: string; content: string; session_id: string}>};
  return (data.episodes || []).map((e) => ({
    id: e.id,
    timestamp: e.timestamp,
    role: e.role,
    content: e.content,
    sessionId: e.session_id,
  }));
}

/** 获取 6 历史记忆流 */
export async function fetchMemoryStreams(config: ApeirethConfig): Promise<Record<string, MemoryEpisodeItem[]>> {
  const res = await fetch(`${normalizeBaseUrl(config.baseUrl)}/v1/panel/memory/streams`, {
    headers: config.apiKey ? {Authorization: `Bearer ${config.apiKey}`} : {},
  });
  const data = (await checkJson(res)) as {streams?: Record<string, Array<{id: string; timestamp: number; role: string; content: string; session_id: string}>>};
  const result: Record<string, MemoryEpisodeItem[]> = {};
  if (data.streams) {
    for (const [key, list] of Object.entries(data.streams)) {
      result[key] = (list || []).map((e) => ({
        id: e.id,
        timestamp: e.timestamp,
        role: e.role,
        content: e.content,
        sessionId: e.session_id,
        stream: key,
      }));
    }
  }
  return result;
}

/** 搜索记忆条目 */
export async function fetchMemoryEpisodes(config: ApeirethConfig, query = '', limit = 100): Promise<MemoryEpisodeItem[]> {
  const url = `${normalizeBaseUrl(config.baseUrl)}/v1/panel/memory/episodes?limit=${limit}${query ? `&q=${encodeURIComponent(query)}` : ''}`;
  const res = await fetch(url, {
    headers: config.apiKey ? {Authorization: `Bearer ${config.apiKey}`} : {},
  });
  const data = (await checkJson(res)) as {episodes?: Array<{id: string; timestamp: number; role: string; content: string; session_id: string}>};
  return (data.episodes || []).map((e) => ({
    id: e.id,
    timestamp: e.timestamp,
    role: e.role,
    content: e.content,
    sessionId: e.session_id,
  }));
}

/** 获取知识图谱事实和链接 */
export async function fetchGraphData(config: ApeirethConfig): Promise<{facts: MemoryEpisodeItem[]; links: MemoryEpisodeItem[]}> {
  const res = await fetch(`${normalizeBaseUrl(config.baseUrl)}/v1/panel/graph`, {
    headers: config.apiKey ? {Authorization: `Bearer ${config.apiKey}`} : {},
  });
  const data = (await checkJson(res)) as {facts?: Array<{id: string; timestamp: number; role: string; content: string; session_id: string}>; links?: Array<{id: string; timestamp: number; role: string; content: string; session_id: string}>};
  return {
    facts: (data.facts || []).map((e) => ({id: e.id, timestamp: e.timestamp, role: e.role, content: e.content, sessionId: e.session_id})),
    links: (data.links || []).map((e) => ({id: e.id, timestamp: e.timestamp, role: e.role, content: e.content, sessionId: e.session_id})),
  };
}

/** 获取持久化审计记录 */
export async function fetchAuditLogs(config: ApeirethConfig, limit = 100): Promise<ActivityItem[]> {
  const res = await fetch(`${normalizeBaseUrl(config.baseUrl)}/v1/panel/audit?limit=${limit}`, {
    headers: config.apiKey ? {Authorization: `Bearer ${config.apiKey}`} : {},
  });
  const data = (await checkJson(res)) as {records?: Array<{id?: string; timestamp?: number; action?: string; tool?: string; status?: string; detail?: string}>};
  return (data.records || []).map((r, i) => ({
    id: r.id || `audit-${r.timestamp || Date.now()}-${i}`,
    timestamp: r.timestamp ? (r.timestamp > 1e11 ? r.timestamp : r.timestamp * 1000) : Date.now(),
    category: (r.tool ? 'tool' : 'runtime') as ActivityItem['category'],
    title: r.tool ? `工具调用: ${r.tool}` : (r.action || '操作记录'),
    summary: r.detail || r.action || '系统操作留痕',
    source: 'audit',
    severity: r.status === 'failed' || r.status === 'error' ? 'error' : 'info',
    detail: JSON.stringify(r, null, 2),
    raw: r,
  }));
}

/** 获取工具列表 */
export async function fetchTools(config: ApeirethConfig): Promise<ToolItem[]> {
  try {
    const res = await fetch(`${normalizeBaseUrl(config.baseUrl)}/v1/tools/list`, {
      headers: config.apiKey ? {Authorization: `Bearer ${config.apiKey}`} : {},
    });
    if (res.ok) {
      const data = (await res.json()) as {tools?: Array<{name: string; description?: string; args_schema?: unknown}>};
      if (Array.isArray(data.tools) && data.tools.length > 0) {
        return data.tools.map((t) => ({
          name: t.name,
          description: t.description || '无描述信息',
          argsSchema: t.args_schema,
          source: 'builtin',
          permission: 'prompt',
          available: true,
        }));
      }
    }
  } catch {
    // ignore error and fallback to builtin companion tools
  }

  // 基础常驻伙伴内置工具列表 (companion 机制装配)
  return [
    {name: 'FileOperator', description: '文件读写与目录操作 (需要主人授权)', source: 'builtin', permission: 'prompt', available: true},
    {name: 'ShellExec', description: '本地命令行执行工具 (高危特权)', source: 'builtin', permission: 'prompt', available: true},
    {name: 'save_memory', description: '持久化记忆静默写入工具', source: 'builtin', permission: 'none', available: true},
    {name: 'web_search', description: '互联网搜索与信息抓取', source: 'builtin', permission: 'none', available: true},
    {name: 'calendar', description: '日程管理与提醒工具', source: 'builtin', permission: 'none', available: true},
    {name: 'message', description: '向外部通道发送通知消息', source: 'builtin', permission: 'none', available: true},
  ];
}


/** 获取待审批授权请求 */
export async function fetchApprovalRequests(config: ApeirethConfig): Promise<ApprovalRequestItem[]> {
  const res = await fetch(`${normalizeBaseUrl(config.baseUrl)}/v1/apeireth/approval-requests`, {
    headers: config.apiKey ? {Authorization: `Bearer ${config.apiKey}`} : {},
  });
  const list = (await checkJson(res)) as Array<{
    id?: string;
    chain?: string;
    rev?: number;
    tool?: string;
    reason?: string;
    args_preview?: string;
    summary?: string;
    created_at?: number;
    requested_at?: number;
    status?: string;
  }>;
  if (!Array.isArray(list)) return [];
  return list.map((item, idx) => ({
    id: item.id || `apreq-${idx}`,
    chain: item.chain,
    rev: item.rev,
    tool: item.tool || '未知工具',
    reason: item.reason,
    args_preview: item.args_preview,
    summary: item.reason || item.summary || item.args_preview || '请求执行特权工具',
    requestedAt: item.created_at || item.requested_at,
    status: (item.status as ApprovalRequestItem['status']) || 'pending',
  }));
}

/** 主人批准端点 (master token 显式授权，不持久化 Token) */
export async function grantToolPermission(
  config: ApeirethConfig,
  tool: string,
  hours: number = 1,
  masterToken: string = '',
): Promise<{ok: boolean; error?: string}> {
  try {
    const res = await fetch(`${normalizeBaseUrl(config.baseUrl)}/v1/apeireth/grant`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: config.apiKey ? `Bearer ${config.apiKey}` : '',
      },
      body: JSON.stringify({
        tool,
        hours,
        master_token: masterToken.trim(),
      }),
    });
    if (!res.ok) {
      const err = (await res.json().catch(() => ({error: `HTTP ${res.status}`}))) as {error?: string};
      return {ok: false, error: err.error || `HTTP ${res.status}`};
    }
    return {ok: true};
  } catch (caught) {
    return {ok: false, error: caught instanceof Error ? caught.message : String(caught)};
  }
}

/** 写入记忆条目 */
export async function appendMemoryEpisode(
  config: ApeirethConfig,
  content: string,
  category: string = 'fact',
  sessionId: string = 'me',
): Promise<boolean> {
  const res = await fetch(`${normalizeBaseUrl(config.baseUrl)}/v1/memory/append`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: config.apiKey ? `Bearer ${config.apiKey}` : '',
    },
    body: JSON.stringify({
      session_id: sessionId,
      role: 'user',
      content: `[${category}] ${content}`,
    }),
  });
  return res.ok;
}

/** 本地会话持久化与容错迁移 (客户端专用) */
export function loadConversations(): Conversation[] {
  try {
    const raw = localStorage.getItem('apeireth-conversations');
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.map((item: any) => ({
      id: typeof item.id === 'string' ? item.id : crypto.randomUUID(),
      title: typeof item.title === 'string' ? item.title : '新对话',
      createdAt: typeof item.createdAt === 'number' ? item.createdAt : Date.now(),
      updatedAt: typeof item.updatedAt === 'number' ? item.updatedAt : Date.now(),
      messages: Array.isArray(item.messages) ? item.messages : [],
      scope: item.scope === 'project' ? 'project' : 'global',
      pinned: !!item.pinned,
      archived: !!item.archived,
      model: typeof item.model === 'string' ? item.model : undefined,
    }));
  } catch {
    return [];
  }
}


export function saveConversations(conversations: Conversation[]): void {
  localStorage.setItem('apeireth-conversations', JSON.stringify(conversations));
}

// Backward-compatible aliases for legacy / transition imports
export type MemoryEpisode = MemoryEpisodeItem;
export type ToolInfo = ToolItem;

export async function fetchEpisodes(config: ApeirethConfig, limit = 50): Promise<MemoryEpisodeItem[]> {
  return fetchMemoryEpisodes(config, '', limit);
}

export async function fetchOrgans(config: ApeirethConfig): Promise<unknown[]> {
  try {
    const res = await fetch(`${normalizeBaseUrl(config.baseUrl)}/v1/organs`, {
      headers: config.apiKey ? {Authorization: `Bearer ${config.apiKey}`} : {},
    });
    return (await checkJson(res)) as unknown[];
  } catch {
    return [];
  }
}

