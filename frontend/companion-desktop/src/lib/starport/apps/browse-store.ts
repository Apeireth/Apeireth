// 深望 · 浏览面纯逻辑（I 批 3，零依赖纯模块）
//
// 契约（I-starport-builtins.md「深望」）：
//   1. 准入第一闸 —— 只放行 HTTP/HTTPS；javascript:/data:/file:/vbscript:/
//      about:/blob:/ftp: 等一律拒绝（0 装：不修不猜，非法即拒）；
//   2. 站点策略 —— 可选域名白名单（精确或 *.后缀 通配），命中不了即拒，
//      理由如实回报（不静默）；
//   3. 抓取转文本 —— MVP 只做粗提取（去脚本/样式/标签 + 基础实体还原），
//      如实标注不等同语义树（语义操控内核在后续批）；
//   4. 历史账本 —— 有界（50 条），越界淘汰最旧，不外发。
//
// 测试：tests/starport-browse.mjs（真实模块直测）。

export interface BrowseTarget {
  href: string;
  host: string;
}

export interface BrowsePolicy {
  /** null = 全部 HTTP/HTTPS 站点放行；否则精确 host 或 *.后缀 通配。 */
  allowedHosts: readonly string[] | null;
}

export type BrowseGateResult =
  | {ok: true; target: BrowseTarget}
  | {ok: false; reason: string};

export interface BrowseHistoryEntry {
  href: string;
  host: string;
  at: number;
  title: string | null;
}

export const BROWSE_HISTORY_LIMIT = 50;

const ALLOWED_PROTOCOLS = new Set(['http:', 'https:']);

/** URL 准入：只放行 HTTP/HTTPS。非法输入返回 null（宁拒不猜）。 */
export function normalizeBrowseUrl(input: string): BrowseTarget | null {
  const raw = typeof input === 'string' ? input.trim() : '';
  if (!raw) return null;
  let parsed: URL;
  try {
    parsed = new URL(raw);
  } catch {
    return null;
  }
  if (!ALLOWED_PROTOCOLS.has(parsed.protocol)) return null;
  if (!parsed.hostname) return null;
  return {href: parsed.href, host: parsed.hostname.toLowerCase()};
}

/** 站点策略判定：命中放行；否则拒绝并如实给理由。 */
export function checkBrowsePolicy(target: BrowseTarget, policy: BrowsePolicy): BrowseGateResult {
  if (policy.allowedHosts === null) return {ok: true, target};
  const host = target.host.toLowerCase();
  for (const rule of policy.allowedHosts) {
    const pattern = rule.trim().toLowerCase();
    if (!pattern) continue;
    if (pattern.startsWith('*.')) {
      const suffix = pattern.slice(1); // ".example.com"
      if (host.endsWith(suffix) || host === pattern.slice(2)) return {ok: true, target};
    } else if (host === pattern) {
      return {ok: true, target};
    }
  }
  return {ok: false, reason: `站点不在放行名单：${host}`};
}

/**
 * 抓取转文本（MVP 粗提取）：去 script/style/标签 + 基础实体还原 + 空行折叠。
 * 不等同语义树（如实口径：语义操控内核在后续批）。
 */
export function extractReadableText(html: string): string {
  const stripped = String(html)
    .replace(/<script[\s\S]*?<\/script>/gi, ' ')
    .replace(/<style[\s\S]*?<\/style>/gi, ' ')
    .replace(/<!--[\s\S]*?-->/g, ' ')
    .replace(/<[^>]+>/g, ' ')
    .replace(/&nbsp;/gi, ' ')
    .replace(/&amp;/gi, '&')
    .replace(/&lt;/gi, '<')
    .replace(/&gt;/gi, '>')
    .replace(/&quot;/gi, '"')
    .replace(/&#39;/gi, "'");
  return stripped
    .split('\n')
    .map((line) => line.replace(/[ \t]+/g, ' ').trim())
    .filter((line, index, lines) => line !== '' || (index > 0 && lines[index - 1] !== ''))
    .join('\n')
    .trim();
}

/** 历史账本：最新在前，超限淘汰最旧。 */
export function pushHistory(
  history: readonly BrowseHistoryEntry[],
  entry: BrowseHistoryEntry,
): BrowseHistoryEntry[] {
  return [entry, ...history.filter((item) => item.href !== entry.href)].slice(
    0,
    BROWSE_HISTORY_LIMIT,
  );
}

// ---------------------------------------------------------------------------
// 用户侧语义树（深望进阶件：抓取 → 去噪结构 → 稳定语义句柄）
// ---------------------------------------------------------------------------

export type SemanticKind = 'heading' | 'link' | 'text' | 'image' | 'form-field' | 'button';

export interface SemanticNode {
  /** 稳定语义句柄（按文档序确定性编号，同 HTML 恒同句柄）。 */
  handle: string;
  kind: SemanticKind;
  /** 标题层级（仅 heading）。 */
  level?: number;
  text: string;
  /** 链接目标（仅 link）。 */
  href?: string;
  /** 表单字段名/类型（仅 form-field）。 */
  name?: string;
}

/** 噪声块（提取前剥除）：脚本/样式/注释。 */
const NOISE_BLOCKS: readonly RegExp[] = [
  /<script[\s\S]*?<\/script>/gi,
  /<style[\s\S]*?<\/style>/gi,
  /<noscript[\s\S]*?<\/noscript>/gi,
  /<!--[\s\S]*?-->/g,
];

function stripTags(html: string): string {
  return html
    .replace(/<[^>]+>/g, ' ')
    .replace(/&nbsp;/gi, ' ')
    .replace(/&amp;/gi, '&')
    .replace(/&lt;/gi, '<')
    .replace(/&gt;/gi, '>')
    .replace(/&quot;/gi, '"')
    .replace(/&#39;/gi, "'")
    .replace(/\s+/g, ' ')
    .trim();
}

interface RawHit {
  at: number;
  kind: SemanticKind;
  level?: number;
  text: string;
  href?: string;
  name?: string;
}

/**
 * 去噪语义提取（MVP：模式匹配抽取标题/段落/链接/图片/表单字段/按钮）。
 * 按出现位置归并、文档序编号命名句柄——同 HTML 恒同句柄（0 装：不猜补）。
 * 复杂 DOM（模板/ShadowDOM）为扩展位，如实标注不等同完整语义树操控。
 */
export function extractSemanticTree(html: string): SemanticNode[] {
  const source = String(html);
  let cleaned = source;
  for (const pattern of NOISE_BLOCKS) cleaned = cleaned.replace(pattern, ' ');

  const hits: RawHit[] = [];
  const collect = (pattern: RegExp, map: (match: RegExpExecArray) => RawHit | null): void => {
    const re = new RegExp(pattern.source, pattern.flags);
    let match: RegExpExecArray | null;
    while ((match = re.exec(cleaned)) !== null) {
      const hit = map(match);
      if (hit && hit.text) hits.push(hit);
    }
  };

  collect(/<h([1-6])[^>]*>([\s\S]*?)<\/h\1>/gi, (m) => {
    const text = stripTags(m[2]);
    return text ? {at: m.index, kind: 'heading', level: Number(m[1]), text} : null;
  });
  collect(/<p[^>]*>([\s\S]*?)<\/p>/gi, (m) => {
    const text = stripTags(m[1]);
    return text ? {at: m.index, kind: 'text', text} : null;
  });
  collect(/<a\b[^>]*href=["']([^"']+)["'][^>]*>([\s\S]*?)<\/a>/gi, (m) => {
    const text = stripTags(m[2]) || m[1];
    return {at: m.index, kind: 'link', text, href: m[1]};
  });
  collect(/<img\b[^>]*>/gi, (m) => {
    const alt = /alt=["']([^"']*)["']/.exec(m[0]);
    const src = /src=["']([^"']+)["']/.exec(m[0]);
    const text = (alt && alt[1]) || (src && src[1]) || '';
    return text ? {at: m.index, kind: 'image', text} : null;
  });
  collect(/<(?:input|textarea|select)\b[^>]*>/gi, (m) => {
    const name = /name=["']([^"']+)["']/.exec(m[0]);
    const type = /type=["']([^"']+)["']/.exec(m[0]);
    const placeholder = /placeholder=["']([^"']*)["']/.exec(m[0]);
    const text = (placeholder && placeholder[1]) || (name && name[1]) || '';
    return text
      ? {at: m.index, kind: 'form-field', text, name: (name && name[1]) || undefined}
      : null;
  });
  collect(/<button[^>]*>([\s\S]*?)<\/button>/gi, (m) => {
    const text = stripTags(m[1]);
    return text ? {at: m.index, kind: 'button', text} : null;
  });

  hits.sort((a, b) => a.at - b.at);
  const counters = new Map<SemanticKind, number>();
  return hits.map((hit) => {
    const ordinal = (counters.get(hit.kind) ?? 0) + 1;
    counters.set(hit.kind, ordinal);
    const node: SemanticNode = {handle: `${hit.kind}:${ordinal}`, kind: hit.kind, text: hit.text};
    if (hit.level !== undefined) node.level = hit.level;
    if (hit.href !== undefined) node.href = hit.href;
    if (hit.name !== undefined) node.name = hit.name;
    return node;
  });
}

/** 句柄 → 节点（找不到返回 null，不猜）。 */
export function findSemanticNode(
  tree: readonly SemanticNode[],
  handle: string,
): SemanticNode | null {
  return tree.find((node) => node.handle === handle) ?? null;
}
