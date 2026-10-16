// 铭刻台 · 文本会话与编辑纯逻辑（星港内置应用面 I 批 2，零依赖纯模块）
//
// 契约（I-starport-builtins.md「铭刻台」文本面）：
//   1. 坐标约定（命名即承诺，全文一致）：行号 line 为 1-based（首行为 1）；
//      列号 column 为 0-based 行内字符偏移（0 = 行首插入点）；区间为
//      [start, end) 半开区间（start 位字符起、end 位前止）。
//   2. expected 原文校验 —— replaceRange 必须携带区间原文 expected，
//      与区间实际内容不符即抛 mismatch（防并发写坏，宁拒不写）；
//   3. 0 装 —— 行/列越界、区间倒置、非字符串输入、空查找词、查无会话
//      一律抛错，不猜不修不吞；错误带 code（out-of-bounds / mismatch /
//      invalid-argument / missing-session）。
//   4. 纯函数 —— 编辑操作 content 进 content 出，不触 IO/DOM/时钟；
//      会话容器（store）注入时钟生成 updatedAt，返回快照副本防外部篡改。
//   5. findText 逐行匹配（不跨行），行优先、非重叠命中，返回行号+列号命中表。
//
// 测试：tests/starport-text-editor.mjs（真实模块直测）。

/** 文本位置（行 1-based、列 0-based，见契约 §1）。 */
export interface TextPosition {
  line: number;
  column: number;
}

/** 文本区间 [start, end) 半开区间。 */
export interface TextRange {
  start: TextPosition;
  end: TextPosition;
}

/** 文本会话（铭刻台的一个稿面）。 */
export interface TextSession {
  id: string;
  name: string;
  content: string;
  cursorLine: number;
  updatedAt: number;
}

/** findText 命中表条目。 */
export interface TextHit {
  line: number;
  column: number;
}

export type TextEditorErrorCode =
  | 'out-of-bounds'
  | 'mismatch'
  | 'invalid-argument'
  | 'missing-session';

export class TextEditorError extends Error {
  readonly code: TextEditorErrorCode;

  constructor(code: TextEditorErrorCode, message: string) {
    super(message);
    this.name = 'TextEditorError';
    this.code = code;
  }
}

// ---------------------------------------------------------------------------
// 内部坐标换算（契约 §1）
// ---------------------------------------------------------------------------

function requireString(value: unknown, what: string): string {
  if (typeof value !== 'string') {
    throw new TextEditorError('invalid-argument', `${what} 必须是字符串`);
  }
  return value;
}

function splitLines(content: string): string[] {
  return content.split('\n');
}

function requireLine(content: string, line: number): string[] {
  const lines = splitLines(content);
  if (!Number.isInteger(line) || line < 1 || line > lines.length) {
    throw new TextEditorError('out-of-bounds', `行号越界：${line}（共 ${lines.length} 行）`);
  }
  return lines;
}

function requireColumn(lines: string[], line: number, column: number): void {
  const length = lines[line - 1].length;
  if (!Number.isInteger(column) || column < 0 || column > length) {
    throw new TextEditorError(
      'out-of-bounds',
      `列号越界：行 ${line} 列 ${column}（该行长度 ${length}）`,
    );
  }
}

/** 坐标 → 绝对偏移（校验并抛错；仅内部与 store 校验复用）。 */
function positionToOffset(content: string, position: TextPosition): number {
  const lines = requireLine(content, position.line);
  requireColumn(lines, position.line, position.column);
  let offset = 0;
  for (let i = 0; i < position.line - 1; i += 1) offset += lines[i].length + 1;
  return offset + position.column;
}

// ---------------------------------------------------------------------------
// 文本度量与行操作（纯函数）
// ---------------------------------------------------------------------------

/** 总行数（空内容为 1 行；'\n' 为 2 行）。 */
export function lineCount(content: string): number {
  return splitLines(requireString(content, 'content')).length;
}

/** 取某行文本（不含换行符）。行号越界抛错。 */
export function lineAt(content: string, line: number): string {
  return requireLine(requireString(content, 'content'), line)[line - 1];
}

/**
 * 在 at 处插入 text（text 可含换行；空串为幂等无操作）。
 * at 越界抛错。返回新内容。
 */
export function insertText(content: string, at: TextPosition, text: string): string {
  requireString(content, 'content');
  requireString(text, 'text');
  const offset = positionToOffset(content, at);
  return content.slice(0, offset) + text + content.slice(offset);
}

/**
 * 区间替换（带 expected 原文校验，契约 §2）：区间 [range.start, range.end)
 * 的实际内容必须与 expected 完全一致，否则抛 mismatch——宁拒不写。
 * 区间倒置抛 invalid-argument；任一端越界抛 out-of-bounds。
 */
export function replaceRange(
  content: string,
  range: TextRange,
  replacement: string,
  expected: string,
): string {
  requireString(content, 'content');
  requireString(replacement, 'replacement');
  requireString(expected, 'expected');
  const startOffset = positionToOffset(content, range.start);
  const endOffset = positionToOffset(content, range.end);
  if (endOffset < startOffset) {
    throw new TextEditorError('invalid-argument', '区间倒置：end 早于 start');
  }
  const actual = content.slice(startOffset, endOffset);
  if (actual !== expected) {
    throw new TextEditorError(
      'mismatch',
      `expected 原文校验失败：期望「${expected}」实际「${actual}」`,
    );
  }
  return content.slice(0, startOffset) + replacement + content.slice(endOffset);
}

/** 在 (line, column) 处断行。column 允许 0..行长（两端即行首/行尾断）。 */
export function splitLine(content: string, line: number, column: number): string {
  return splitLineAt(content, {line, column});
}

function splitLineAt(content: string, at: TextPosition): string {
  return insertText(content, at, '\n');
}

/** 合并 line 与 line+1 两行（原样拼接，不加空格——0 装不猜）。
 *  line 必须在 [1, 行数-1]，否则抛错。 */
export function joinLines(content: string, line: number): string {
  const lines = requireLine(requireString(content, 'content'), line);
  if (line >= lines.length) {
    throw new TextEditorError('out-of-bounds', `行 ${line} 无下一行可合并（共 ${lines.length} 行）`);
  }
  const merged = lines[line - 1] + lines[line];
  return [...lines.slice(0, line - 1), merged, ...lines.slice(line + 1)].join('\n');
}

/**
 * 查找命中表：逐行匹配（不跨行）、行优先、非重叠。
 * needle 为空串或非字符串抛 invalid-argument；无命中返回空数组。
 */
export function findText(content: string, needle: string): TextHit[] {
  requireString(content, 'content');
  if (typeof needle !== 'string' || needle === '') {
    throw new TextEditorError('invalid-argument', '查找词必须是非空字符串');
  }
  const hits: TextHit[] = [];
  const lines = splitLines(content);
  for (let index = 0; index < lines.length; index += 1) {
    const text = lines[index];
    let from = 0;
    for (;;) {
      const column = text.indexOf(needle, from);
      if (column === -1) break;
      hits.push({line: index + 1, column});
      from = column + needle.length;
    }
  }
  return hits;
}

// ---------------------------------------------------------------------------
// 会话容器（CRUD；返回快照副本，契约 §4）
// ---------------------------------------------------------------------------

export interface TextEditorStore {
  create(name: string, content?: string): TextSession;
  get(id: string): TextSession;
  list(): TextSession[];
  rename(id: string, name: string): TextSession;
  /** 更新内容；cursorLine 省略时按契约 clamp 到合法范围（编辑后光标回落）。 */
  updateContent(id: string, content: string, cursorLine?: number): TextSession;
  remove(id: string): void;
}

export function createTextEditorStore(options: {now?: () => number; idPrefix?: string} = {}): TextEditorStore {
  const now = options.now ?? Date.now;
  const idPrefix = options.idPrefix ?? 'text-';
  const sessions = new Map<string, TextSession>();
  let seq = 0;

  function snapshot(session: TextSession): TextSession {
    return {...session};
  }

  function requireSession(id: string): TextSession {
    const session = sessions.get(id);
    if (!session) throw new TextEditorError('missing-session', `会话不存在：${id}`);
    return session;
  }

  return {
    create(name: string, content = ''): TextSession {
      if (typeof name !== 'string' || name === '') {
        throw new TextEditorError('invalid-argument', '会话名必须是非空字符串');
      }
      requireString(content, 'content');
      seq += 1;
      const session: TextSession = {
        id: `${idPrefix}${seq}`,
        name,
        content,
        cursorLine: 1,
        updatedAt: now(),
      };
      sessions.set(session.id, session);
      return snapshot(session);
    },

    get(id: string): TextSession {
      return snapshot(requireSession(id));
    },

    list(): TextSession[] {
      return [...sessions.values()].map(snapshot);
    },

    rename(id: string, name: string): TextSession {
      if (typeof name !== 'string' || name === '') {
        throw new TextEditorError('invalid-argument', '会话名必须是非空字符串');
      }
      const session = requireSession(id);
      session.name = name;
      session.updatedAt = now();
      return snapshot(session);
    },

    updateContent(id: string, content: string, cursorLine?: number): TextSession {
      requireString(content, 'content');
      const session = requireSession(id);
      const total = lineCount(content);
      if (cursorLine === undefined) {
        session.cursorLine = Math.min(session.cursorLine, total);
      } else {
        if (!Number.isInteger(cursorLine) || cursorLine < 1 || cursorLine > total) {
          throw new TextEditorError(
            'out-of-bounds',
            `光标行越界：${cursorLine}（内容共 ${total} 行）`,
          );
        }
        session.cursorLine = cursorLine;
      }
      session.content = content;
      session.updatedAt = now();
      return snapshot(session);
    },

    remove(id: string): void {
      if (!sessions.delete(id)) {
        throw new TextEditorError('missing-session', `会话不存在：${id}`);
      }
    },
  };
}
