// Apeireth 桌面伙伴 — Runtime 适配层
// 连接 apeireth-companion companion_serve (:8090) 的 OpenAI 兼容端点.
// 重写自 Pattern sidecar 的 WS 协议 → Apeireth HTTP/SSE.
//
// 目标形态 (§12):  UI → Apeireth Runtime Contract → Runtime/Provider/Tools
// 本文件是 UI 侧的适配层, 后端 0 改动.

import type {ApeirethConfig, ChatMessage, ModelSetup, RuntimeStatus} from './types';

const STORAGE_KEY = 'apeireth-config';

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
    throw new Error(`HTTP ${response.status} ${text.slice(0, 300)}`);
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
  messages: Array<{role: 'user' | 'assistant'; content: string}>,
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
    throw new Error(`HTTP ${response.status} ${text.slice(0, 300)}`);
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
