// Call Logger: 记录每一轮 LLM 调用的审计与监控系统
// 记录请求时间、端点、模型、协议、上下文消息、响应、CoT 思考过程、延迟及报错详情。

export interface CallLogEntry {
  id: string;
  timestamp: number;
  timeFormatted: string;
  conversationId?: string;
  protocol: 'openai' | 'anthropic' | 'gateway';
  endpoint: string;
  model: string;
  status: 'success' | 'error' | 'aborted';
  latencyMs: number;
  requestMessages: Array<{role: string; content: string}>;
  systemPrompt?: string;
  responseContent?: string;
  reasoningContent?: string;
  toolCalls?: Array<{id?: string; name?: string; args?: unknown}>;
  errorMessage?: string;
  httpStatus?: number;
}

const STORAGE_KEY = 'ap_call_logs_v1';
const MAX_LOGS = 150;

let memoryLogs: CallLogEntry[] = [];
let isLoaded = false;
type LogListener = (logs: CallLogEntry[]) => void;
const listeners: Set<LogListener> = new Set();

function loadLogs(): CallLogEntry[] {
  if (isLoaded) return memoryLogs;
  if (typeof window === 'undefined' || !window.localStorage) return [];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      memoryLogs = JSON.parse(raw);
      if (!Array.isArray(memoryLogs)) memoryLogs = [];
    }
  } catch (err) {
    console.warn('[call-logger] Failed to load logs:', err);
    memoryLogs = [];
  }
  isLoaded = true;
  return memoryLogs;
}

function persistLogs(): void {
  if (typeof window === 'undefined' || !window.localStorage) return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(memoryLogs.slice(0, MAX_LOGS)));
  } catch (err) {
    console.warn('[call-logger] Failed to persist logs:', err);
  }
}

function notifyListeners(): void {
  const copy = [...memoryLogs];
  for (const listener of listeners) {
    try {
      listener(copy);
    } catch (err) {
      console.error('[call-logger] Listener error:', err);
    }
  }
}

export function recordCallLog(entry: Omit<CallLogEntry, 'id' | 'timestamp' | 'timeFormatted'>): CallLogEntry {
  loadLogs();
  const now = Date.now();
  const fullEntry: CallLogEntry = {
    ...entry,
    id: crypto.randomUUID(),
    timestamp: now,
    timeFormatted: new Date(now).toLocaleTimeString('zh-CN', {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    }),
  };

  memoryLogs = [fullEntry, ...memoryLogs].slice(0, MAX_LOGS);
  persistLogs();
  notifyListeners();
  return fullEntry;
}

export function getCallLogs(): CallLogEntry[] {
  return loadLogs();
}

export function clearCallLogs(): void {
  memoryLogs = [];
  persistLogs();
  notifyListeners();
}

export function subscribeCallLogs(listener: LogListener): () => void {
  listeners.add(listener);
  listener(loadLogs());
  return () => {
    listeners.delete(listener);
  };
}

export function exportCallLogsJson(): string {
  return JSON.stringify(loadLogs(), null, 2);
}
