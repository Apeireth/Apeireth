// T0 聊天壳骨骼 — 会话列表主页（「谁找我了」）纯逻辑
//
// 范式依据: docs/design/00-PHILOSOPHY.md §3.1「消息列表是主页」——
// 打开应用的第一念是"谁找我了", 会话列表直接回答它。
// 数据源: GET /v1/panel/sessions（docs/gateway-api-contract.md §4, canonical 已就绪）
// 与本地 conversations（localStorage, 消息正文所在）归并。
//
// 0 装纪律:
// - 后端账本是真源; 本地有而后端没有的会话 = 尚未对后端说过话的纯本地草稿,
//   照常列出（origin='local'），不伪造后端字段。
// - 后端有而本地没有的会话 = 他那边记得、本机没有消息副本的会话
//   （origin='backend'），预览诚实写"内容在他的账本里"，不伪造消息正文。
// - 未读计数在后端不存在 → 永远没有未读 badge，只有"待签"（审批真实信号）。
//
// 本文件刻意零运行时依赖（不 import svelte / DOM），纯函数可被 Node 直接
// import 测试（tests/chat-shell.mjs），与 presence.ts 同一约定。

import type {Conversation} from '../types';

/** 后端会话账本条目（fetchBackendSessions 的返回形状）。 */
export interface BackendLedgerSession {
  id: string;
  title?: string;
  started_at: number;
  last_active_at: number;
  closed_at?: number;
  episode_count: number;
}

/** 主页会话行（归一化后的渲染模型）。 */
export interface HomeSessionItem {
  id: string;
  title: string;
  /** 最后活跃时刻（epoch ms；缺时为 0，渲染层决定如何兜底）。 */
  lastActiveAt: number;
  /** 消息数：本地会话=本地消息数；后端独有=后端账本 message_count。 */
  messageCount: number;
  /** 最后一条消息预览（仅本地有副本时存在；backend-only 为 null）。 */
  preview: string | null;
  /** 数据来源：both=本地+后端 / local=纯本地草稿 / backend=仅后端账本。 */
  origin: 'both' | 'local' | 'backend';
  /** 该会话有他停下等签的文书（SSE approval_required 真实信号驱动）。 */
  pendingApproval: boolean;
  /** 本地归档标记（后端账本无归档概念）。 */
  archived: boolean;
}

export interface MergeSessionLedgerInput {
  /** 本地会话（localStorage，消息正文所在）。 */
  local: Conversation[];
  /** 后端账本（GET /v1/panel/sessions；拉取失败/不支持时传 null）。 */
  backend: BackendLedgerSession[] | null;
  /** 有待签文书的会话 id 集（SSE approval_required − approval_resolved 推导）。 */
  pendingApprovalSessions?: ReadonlySet<string>;
}

/** 最后一条消息的一行预览（空白折叠，截断 80 字）。 */
export function sessionPreview(conv: Conversation): string | null {
  const last = conv.messages.at(-1);
  const text = last?.text?.replace(/\s+/g, ' ').trim();
  return text ? text.slice(0, 80) : null;
}

/**
 * 归并本地会话与后端账本为主页列表。
 * 排序：最后活跃倒序（「谁最近找我了」自上而下）。
 * 归档会话不进主页（归档是管理动作，留在管理卷宗里）。
 */
export function mergeSessionLedger(input: MergeSessionLedgerInput): HomeSessionItem[] {
  const pending = input.pendingApprovalSessions ?? new Set<string>();
  const byId = new Map<string, HomeSessionItem>();

  for (const conv of input.local) {
    if (conv.archived) continue;
    byId.set(conv.id, {
      id: conv.id,
      title: conv.title || '新对话',
      lastActiveAt: conv.updatedAt ?? conv.createdAt ?? 0,
      messageCount: conv.messages.length,
      preview: sessionPreview(conv),
      origin: 'local',
      pendingApproval: pending.has(conv.id),
      archived: false,
    });
  }

  for (const s of input.backend ?? []) {
    const existing = byId.get(s.id);
    const lastActive = s.last_active_at || s.started_at || 0;
    if (existing) {
      // 两边都有：标题/预览以本地为准（正文在本地），活跃时刻取两者较新。
      existing.origin = 'both';
      existing.lastActiveAt = Math.max(existing.lastActiveAt, lastActive);
      if (!existing.title || existing.title === '新对话') {
        if (s.title) existing.title = s.title;
      }
      if (existing.messageCount === 0 && s.episode_count > 0) {
        existing.messageCount = s.episode_count;
      }
    } else {
      byId.set(s.id, {
        id: s.id,
        title: s.title || '未命名会话',
        lastActiveAt: lastActive,
        messageCount: s.episode_count,
        preview: null,
        origin: 'backend',
        pendingApproval: pending.has(s.id),
        archived: false,
      });
    }
  }

  return [...byId.values()].sort((a, b) => b.lastActiveAt - a.lastActiveAt);
}

/** 微信式相对时间：今天给时刻，昨天给「昨天」，更早给日期。 */
export function formatSessionTime(ts: number, now: number = Date.now()): string {
  if (!ts || !Number.isFinite(ts)) return '';
  const d = new Date(ts);
  if (Number.isNaN(d.getTime())) return '';
  const n = new Date(now);
  const sameDay = d.toDateString() === n.toDateString();
  if (sameDay) {
    return d.toLocaleTimeString('zh-CN', {hour: '2-digit', minute: '2-digit'});
  }
  const yesterday = new Date(now - 86_400_000);
  if (d.toDateString() === yesterday.toDateString()) return '昨天';
  const sameYear = d.getFullYear() === n.getFullYear();
  return d.toLocaleDateString(
    'zh-CN',
    sameYear
      ? {month: 'numeric', day: 'numeric'}
      : {year: 'numeric', month: 'numeric', day: 'numeric'},
  );
}
