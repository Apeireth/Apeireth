// 用户印象（User Impression）—— AI 智能体的独立设置栏数据层。
//
// 双重身份（本功能的两条验收边）：
//   - 用户阅读项：设置 ›「用户印象」原样呈现——可读、可手改、可清空、可追问；
//   - AI 自我参考项：作为 system 侧栏随每次对话注入（runtime.ts run() 统一接线），
//     让每个伙伴带着「我对用户的既有印象」继续相处，而不是每次从零认识。
//
// 刻意不进 ApeirethConfig：印象是每个伙伴（persona）自己的私人笔记，不是用户
// 配置面——按 personaId 分键落 localStorage 单键（与 call-logger / 会话本地账
// 同口径），本机存储、无账号、不上传。将来接账号体系只换本模块存取实现。
//
// 纪律（与 user-profile.ts 同）：
//   - 读侧永不抛：脏 JSON / 超长字段 / 非法枚举一律归一到安全缺省；
//   - 写侧如实抛：localStorage 不可用/满时让 UI 显示可读理由，不静默丢改动；
//   - 顶层不触 DOM，Node 直接 import 可测；
//   - 与 runtime.ts 保持单向依赖：runtime 依赖本模块；总结所需的模型补全由调用方
//     以 complete 回调注入，本模块永不反向 import runtime（不引循环、不偷网络）。

/** 本地落盘单键（按 personaId 分键的账本包在这一个键里）。 */
export const USER_IMPRESSION_STORAGE_KEY = 'apeireth-user-impression';

/** 印象全文字符上限（手写与总结同样钳制；200 字总结 + 用户增补留足余量）。 */
export const IMPRESSION_MAX_CHARS = 2000;

/** 送进总结提示词的对话转写字符上限（尾部截断，新信息优先）。 */
export const TRANSCRIPT_MAX_CHARS = 6000;

/** 自动总结节流：距上次总结至少攒够这么多条新用户消息才追一次（成本闸）。 */
export const AUTO_REFRESH_USER_MESSAGES = 4;

/** 首次印象的最低门槛：至少聊过这么多条用户消息才开始总结（空会话不出印象）。 */
export const MIN_USER_MESSAGES_FOR_FIRST_IMPRESSION = 2;

export interface UserImpression {
  /** 归属伙伴（人设）id；缺省伙伴用 'default'。 */
  personaId: string;
  /** 归属伙伴显示名（提示词里自称用；人设删了名字仍留档）。 */
  personaName: string;
  /** 印象全文：用户可读可改，AI 注入 system 自我参考。空 = 还没有印象。 */
  summary: string;
  /** 最近一次成功写入时间（ms epoch；0 = 从未写过）。 */
  updatedAt: number;
  /** 上次总结时已看过的用户消息条数（自动总结的节流游标，不展示）。 */
  seenUserCount: number;
  /** 最近一次来源：auto = 对话中自动总结 / manual = 手动总结或手写。 */
  source: 'auto' | 'manual';
  /** 自动总结开关（默认开；关 = 只保留手动总结与手写）。 */
  autoRefresh: boolean;
}

export function emptyUserImpression(personaId: string, personaName = ''): UserImpression {
  return {
    personaId,
    personaName,
    summary: '',
    updatedAt: 0,
    seenUserCount: 0,
    source: 'manual',
    autoRefresh: true,
  };
}

function clampText(value: unknown, max: number): string {
  return typeof value === 'string' ? value.trim().slice(0, max) : '';
}

function clampCount(value: unknown): number {
  return typeof value === 'number' && Number.isFinite(value) && value >= 0 ? Math.floor(value) : 0;
}

/** 归一：非对象 / 脏字段 / 非法枚举一律收敛或截断，不炸界面。 */
export function normalizeUserImpression(raw: unknown, personaId: string): UserImpression {
  const record = (raw !== null && typeof raw === 'object' ? raw : {}) as Record<string, unknown>;
  return {
    personaId,
    personaName: clampText(record.personaName, 64),
    summary: clampText(record.summary, IMPRESSION_MAX_CHARS),
    updatedAt: clampCount(record.updatedAt),
    seenUserCount: clampCount(record.seenUserCount),
    source: record.source === 'auto' ? 'auto' : 'manual',
    autoRefresh: record.autoRefresh !== false,
  };
}

type UserImpressionListener = (entry: UserImpression) => void;
const listeners = new Set<UserImpressionListener>();

/** 订阅印象写入（设置栏实时跟随）；返回退订闭包。 */
export function subscribeUserImpression(listener: UserImpressionListener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function readStoreRaw(): Record<string, unknown> {
  try {
    const storage = globalThis.localStorage;
    if (!storage) return {};
    const raw = storage.getItem(USER_IMPRESSION_STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as unknown;
    if (!parsed || typeof parsed !== 'object') return {};
    const entries = (parsed as {entries?: unknown}).entries;
    if (!entries || typeof entries !== 'object') return {};
    return entries as Record<string, unknown>;
  } catch {
    return {};
  }
}

/** 读某伙伴的印象；localStorage 不可用/脏数据一律回落空印象（读侧不抛）。 */
export function loadUserImpression(personaId: string, personaName = ''): UserImpression {
  const entry = normalizeUserImpression(readStoreRaw()[personaId], personaId);
  if (!entry.personaName) entry.personaName = personaName;
  return entry;
}

/** 读全部伙伴的印象账本（设置栏换伙伴不反复解析单键）。 */
export function loadUserImpressionLedger(): Map<string, UserImpression> {
  const raw = readStoreRaw();
  const out = new Map<string, UserImpression>();
  for (const [personaId, value] of Object.entries(raw)) {
    out.set(personaId, normalizeUserImpression(value, personaId));
  }
  return out;
}

/**
 * 写印象：归一 + 盖 updatedAt + 落盘 + 广播订阅方。
 * localStorage 不可用/满时如实抛（写侧不静默失败，UI 提示并保住输入）。
 */
export function saveUserImpression(entry: UserImpression): UserImpression {
  const next = normalizeUserImpression({...entry}, entry.personaId);
  const storage = globalThis.localStorage;
  if (!storage) throw new Error('本地存储不可用，印象改动只留在页面里');
  const entries = readStoreRaw();
  entries[next.personaId] = next;
  storage.setItem(USER_IMPRESSION_STORAGE_KEY, JSON.stringify({version: 1, entries})); // 可能抛（配额/隐私模式）
  for (const listener of listeners) listener(next);
  return next;
}

/** 清空某伙伴的印象（保留账本键与自动总结开关，只清正文与节流游标）。 */
export function clearUserImpression(entry: UserImpression): UserImpression {
  return saveUserImpression({...entry, summary: '', seenUserCount: 0});
}

// ============================================================
// 纯逻辑：提示词构建 / 回复解析 / 注入块 / 节流判定
// ============================================================

/** 对话转写（user/assistant 尾部截断；系统提示与事件杂音不进总结）。 */
export function transcriptForSummary(
  messages: ReadonlyArray<{role: string; text: string}>,
  maxChars: number = TRANSCRIPT_MAX_CHARS,
): string {
  const lines = messages
    .filter((m) => (m.role === 'user' || m.role === 'assistant') && m.text.trim())
    .map((m) => `${m.role === 'user' ? '用户' : '伙伴'}：${m.text.trim()}`);
  const out: string[] = [];
  let used = 0;
  for (let i = lines.length - 1; i >= 0; i--) {
    if (used + lines[i].length > maxChars && out.length > 0) break;
    out.unshift(lines[i]);
    used += lines[i].length;
  }
  return out.join('\n');
}

/** 从本地会话账本取最新鲜的对话转写（设置栏「立即总结」的取数面）。
 *  会话按 updatedAt 升序铺开（新对话落在尾部），配合 transcriptForSummary 的
 *  尾部截断，保证越新的对话越优先留下。 */
export function transcriptFromConversations(
  conversations: ReadonlyArray<{
    updatedAt?: number;
    messages: ReadonlyArray<{role: string; text: string}>;
  }>,
  maxChars: number = TRANSCRIPT_MAX_CHARS,
): string {
  const ordered = [...conversations].sort((a, b) => (a.updatedAt ?? 0) - (b.updatedAt ?? 0));
  const messages = ordered.flatMap((c) => c.messages);
  return transcriptForSummary(messages, maxChars);
}

/** 总结提示词：旧印象 + 新对话 → 一份合并后的印象全文（结构稳定，可单测）。 */
export function buildImpressionPrompt(options: {
  personaName: string;
  previousSummary: string;
  transcript: string;
}): string {
  const who = options.personaName.trim() || '这个伙伴';
  const previous = options.previousSummary.trim() || '（还没有印象，这是第一次）';
  return [
    `你是「${who}」，正在更新你对用户的长期印象档案。这份档案用户本人可读可改，也会作为你日后相处时的自我参考。`,
    '请只根据对话内容总结你对用户的印象：性格气质、偏好与雷点、沟通风格、当前关心的事、你们的相处默契与需要注意的地方。',
    '要求：第三人称简述；不编造对话里没有的信息；保留仍然成立的旧印象，合并新信息，撤掉已过时的结论；200 字以内；纯文本，不要标题、列表符号、引号包裹或代码块。',
    '',
    `旧印象：`,
    previous,
    '',
    `最近对话：`,
    options.transcript.trim() || '（本轮没有可读的对话内容）',
    '',
    '直接输出更新后的印象全文。',
  ].join('\n');
}

/** 解析模型回复：剥代码围栏/引号壳、去空白、钳长；空回复归空串（如实）。 */
export function parseImpressionReply(reply: unknown): string {
  if (typeof reply !== 'string') return '';
  let text = reply.trim();
  const fenced = /^```[a-zA-Z]*\s*\n([\s\S]*?)\n```$/.exec(text);
  if (fenced) text = fenced[1].trim();
  if (
    (text.startsWith('「') && text.endsWith('」')) ||
    (text.startsWith('"') && text.endsWith('"')) ||
    (text.startsWith('“') && text.endsWith('”'))
  ) {
    text = text.slice(1, -1).trim();
  }
  return text.slice(0, IMPRESSION_MAX_CHARS);
}

/**
 * AI 自我参考注入块：印象存在时进 system（人设之后），空印象不注水。
 * 文案自报性质（用户可读可改），让模型知道这条的可信边界。
 */
export function impressionSystemBlock(entry: UserImpression | null | undefined): string {
  const summary = entry?.summary?.trim();
  if (!summary) return '';
  const who = entry?.personaName?.trim();
  const speaker = who ? `你（${who}）` : '你';
  return [
    '【用户印象 · 自我参考】',
    `${speaker}对用户的既有印象（用户本人可读可改；与事实不符时以用户当下的表述为准）：`,
    summary,
  ].join('\n');
}

/** 自动总结节流：没印象先看门槛，有印象按新用户消息增量；关了自动则恒不追。 */
export function shouldRefreshImpression(
  entry: UserImpression | null | undefined,
  userMessageCount: number,
): boolean {
  if (entry && entry.autoRefresh === false) return false;
  const seen = entry?.seenUserCount ?? 0;
  const hasSummary = !!entry?.summary?.trim();
  if (!hasSummary) {
    return (
      userMessageCount >= MIN_USER_MESSAGES_FOR_FIRST_IMPRESSION && userMessageCount > seen
    );
  }
  return userMessageCount - seen >= AUTO_REFRESH_USER_MESSAGES;
}

// ============================================================
// 总结动作：补全函数由调用方注入（runtime 注入 chatOnce；本模块不联网）
// ============================================================

export interface SummarizeUserImpressionOptions {
  personaId: string;
  personaName?: string;
  /** 进入总结的对话转写（transcriptForSummary / transcriptFromConversations 的产物）。 */
  transcript: string;
  /** 模型补全缝：收到提示词返回模型文本（调用方决定走哪个通道）。 */
  complete: (prompt: string) => Promise<string>;
  source?: 'auto' | 'manual';
  /** 总结完成后记录的用户消息游标（自动总结传当前条数；缺省沿用旧值）。 */
  seenUserCount?: number;
}

/**
 * 总结并落盘一次用户印象：读旧印象 → 提示词 → 补全 → 解析 → 存 + 广播。
 * 模型空回复如实抛错；存储失败沿写侧纪律如实抛（自动路径由调用方吞掉并保旧值）。
 */
export async function summarizeUserImpression(
  options: SummarizeUserImpressionOptions,
): Promise<UserImpression> {
  const previous = loadUserImpression(options.personaId, options.personaName ?? '');
  const reply = await options.complete(
    buildImpressionPrompt({
      personaName: options.personaName ?? previous.personaName,
      previousSummary: previous.summary,
      transcript: options.transcript,
    }),
  );
  const summary = parseImpressionReply(reply);
  if (!summary) throw new Error('模型没有返回印象内容，旧印象保持不变');
  return saveUserImpression({
    ...previous,
    personaName: options.personaName ?? previous.personaName,
    summary,
    updatedAt: Date.now(),
    seenUserCount:
      options.seenUserCount !== undefined ? options.seenUserCount : previous.seenUserCount,
    source: options.source ?? 'auto',
  });
}
