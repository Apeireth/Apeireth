// 信使 · 邮件面纯逻辑（I 批次推进，零依赖纯模块）
//
// 契约（I-starport-builtins.md「信使」）：
//   1. 切片边界如实 —— MVP 是「本地邮件台」：邮件模型 + .eml 序列化/解析 +
//      收发件箱账本（导入/导出走真实文件面）；SMTP/IMAP 传输协议面是接线缝
//      （需原生套接字组装，届时点亮）；
//   2. 头注入防御 —— From/To/Subject 含 CR/LF 一律拒绝（0 装：不清洗、
//      不猜测意图，直接拒）；
//   3. .eml 口径 —— 头部行 + 空行 + 正文，CRLF 行界；非 ASCII 直存 UTF-8
//      （RFC 6532 口径，MVP 不做编码字）；解析缺必备头抛错（缺谁报谁）；
//   4. 账本纪律 —— 状态机 draft→queued→sent / received 固定，非法迁移抛错。
//
// 测试：tests/starport-mail.mjs（真实模块直测）。

export type MailDirection = 'in' | 'out';
export type MailStatus = 'draft' | 'queued' | 'sent' | 'received';

export interface EmailMessage {
  id: string;
  direction: MailDirection;
  from: string;
  to: string[];
  subject: string;
  body: string;
  /** Unix 毫秒。 */
  at: number;
  status: MailStatus;
}

export const MAIL_SUBJECT_MAX = 500;
export const MAIL_BODY_MAX = 100_000;

export class MailError extends Error {
  readonly code: 'header-injection' | 'invalid-address' | 'invalid-argument' | 'limit-exceeded' | 'missing-header' | 'missing-message' | 'duplicate-id' | 'invalid-transition';

  constructor(
    code: MailError['code'],
    message: string,
  ) {
    super(message);
    this.name = 'MailError';
    this.code = code;
  }
}

function assertNoCrlf(value: string, field: string): void {
  if (value.includes('\r') || value.includes('\n')) {
    throw new MailError('header-injection', `${field} 含换行符——头注入拒绝`);
  }
}

/** 地址口径（MVP）：含 @、无空白、无换行。换行先按头注入拒（安全码优先）。 */
function assertAddress(value: string, field: string): string {
  const raw = typeof value === 'string' ? value.trim() : '';
  assertNoCrlf(typeof value === 'string' ? value : String(value), field);
  if (!raw || !raw.includes('@') || /\s/.test(raw)) {
    throw new MailError('invalid-address', `${field} 地址非法：${String(value)}`);
  }
  return raw;
}

function validateMessage(input: {
  from: string;
  to: readonly string[];
  subject: string;
  body: string;
  at: number;
}): {from: string; to: string[]; subject: string; body: string; at: number} {
  const from = assertAddress(input.from, 'From');
  if (!Array.isArray(input.to) || input.to.length === 0) {
    throw new MailError('invalid-address', 'To 至少一个收件人');
  }
  const to = input.to.map((entry) => assertAddress(entry, 'To'));
  const subject = typeof input.subject === 'string' ? input.subject : '';
  assertNoCrlf(subject, 'Subject');
  if (subject.length > MAIL_SUBJECT_MAX) {
    throw new MailError('limit-exceeded', `Subject 超上限 ${MAIL_SUBJECT_MAX}`);
  }
  const body = typeof input.body === 'string' ? input.body : '';
  if (body.length > MAIL_BODY_MAX) {
    throw new MailError('limit-exceeded', `正文超上限 ${MAIL_BODY_MAX}`);
  }
  if (!Number.isFinite(input.at)) {
    throw new MailError('invalid-argument', '时间戳非法');
  }
  return {from, to, subject, body, at: input.at};
}

// ---------------------------------------------------------------------------
// .eml 序列化 / 解析（真实文件面的进出闸）
// ---------------------------------------------------------------------------

/** 序列化为 .eml 文本（头 + CRLF 空行 + 正文，行界 CRLF）。 */
export function toEml(message: EmailMessage): string {
  const safe = validateMessage(message);
  const date = new Date(safe.at);
  if (Number.isNaN(date.getTime())) throw new MailError('invalid-argument', '时间戳无法转日期');
  const headers = [
    `Date: ${date.toUTCString()}`,
    `From: ${safe.from}`,
    `To: ${safe.to.join(', ')}`,
    `Subject: ${safe.subject}`,
    `X-Mail-Status: ${message.status}`,
    `X-Mail-Id: ${message.id}`,
  ];
  const bodyLines = safe.body.split('\n').map((line) => line.replace(/\r$/, ''));
  return `${headers.join('\r\n')}\r\n\r\n${bodyLines.join('\r\n')}\r\n`;
}

/** 解析 .eml 文本。缺 From/To/Subject/Date 任一必备头抛 missing-header（缺谁报谁）。 */
export function parseEml(raw: string, fallbackId: string): EmailMessage {
  if (typeof raw !== 'string' || !raw) {
    throw new MailError('invalid-argument', 'eml 文本必须是非空字符串');
  }
  const normalized = raw.replace(/\r\n/g, '\n');
  const splitAt = normalized.indexOf('\n\n');
  const headerBlock = splitAt === -1 ? normalized : normalized.slice(0, splitAt);
  // 往返对称：序列化补的那一个尾行界，回读时剥掉恰一个（正文真实尾行不受损）。
  const bodyRaw = splitAt === -1 ? '' : normalized.slice(splitAt + 2);
  const body = (bodyRaw.endsWith('\n') ? bodyRaw.slice(0, -1) : bodyRaw)
    .split('\n')
    .map((line) => line.replace(/\r$/, ''))
    .join('\r\n');

  const headers = new Map<string, string>();
  for (const line of headerBlock.split('\n')) {
    const colon = line.indexOf(':');
    if (colon <= 0) continue;
    headers.set(line.slice(0, colon).trim().toLowerCase(), line.slice(colon + 1).trim());
  }
  for (const required of ['from', 'to', 'subject', 'date']) {
    if (!headers.has(required)) {
      throw new MailError('missing-header', `缺必备头：${required}`);
    }
  }
  const dateMs = Date.parse(headers.get('date') as string);
  if (Number.isFinite(dateMs)) {
    // 通过校验后再落模型（validateMessage 会拒头注入与坏地址）。
  } else {
    throw new MailError('missing-header', '缺必备头：date（无法解析）');
  }
  const validated = validateMessage({
    from: headers.get('from') as string,
    to: (headers.get('to') as string).split(',').map((entry) => entry.trim()).filter(Boolean),
    subject: headers.get('subject') as string,
    body,
    at: dateMs,
  });
  return {
    id: headers.get('x-mail-id') || fallbackId,
    direction: 'in',
    ...validated,
    status: 'received',
  };
}

// ---------------------------------------------------------------------------
// 收发件箱账本
// ---------------------------------------------------------------------------

const STATUS_FLOW: Record<MailStatus, readonly MailStatus[]> = {
  draft: ['queued'],
  queued: ['sent', 'draft'],
  sent: [],
  received: [],
};

export function transitionStatus(message: EmailMessage, next: MailStatus): EmailMessage {
  if (!STATUS_FLOW[message.status].includes(next)) {
    throw new MailError('invalid-transition', `状态不允许：${message.status} → ${next}`);
  }
  return {...message, status: next};
}

export function createMailbox(): {
  add: (message: EmailMessage) => EmailMessage[];
  remove: (id: string) => EmailMessage[];
  update: (id: string, patch: Partial<Pick<EmailMessage, 'from' | 'to' | 'subject' | 'body'>>) => EmailMessage[];
  list: (filter?: {direction?: MailDirection; status?: MailStatus}) => EmailMessage[];
} {
  let messages: EmailMessage[] = [];
  return {
    add(message) {
      if (messages.some((item) => item.id === message.id)) {
        throw new MailError('duplicate-id', `邮件 id 冲突：${message.id}`);
      }
      validateMessage(message);
      messages = [...messages, {...message}];
      return messages;
    },
    remove(id) {
      if (!messages.some((item) => item.id === id)) {
        throw new MailError('missing-message', `邮件不存在：${id}`);
      }
      messages = messages.filter((item) => item.id !== id);
      return messages;
    },
    update(id, patch) {
      const current = messages.find((item) => item.id === id);
      if (!current) {
        throw new MailError('missing-message', `邮件不存在：${id}`);
      }
      const merged = validateMessage({...current, ...patch});
      messages = messages.map((item) => (item.id === id ? {...item, ...merged} : item));
      return messages;
    },
    list(filter) {
      const filtered = messages.filter(
        (item) =>
          (!filter?.direction || item.direction === filter.direction) &&
          (!filter?.status || item.status === filter.status),
      );
      return [...filtered].sort((a, b) => b.at - a.at);
    },
  };
}
