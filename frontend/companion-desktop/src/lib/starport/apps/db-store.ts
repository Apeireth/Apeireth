// 矿脉 · 数据库面纯逻辑（批次推进，零依赖纯模块）
//
// 契约（I-starport-builtins.md「矿脉」）：
//   1. 只读口径 —— classifySql 只放行读类语句（SELECT/WITH/PRAGMA/EXPLAIN，
//      前导空白/注释容忍），写类/DDL/多语句一律拒绝（0 装：不改写不放行）；
//   2. CSV 导出真件 —— toCsv 标准转义（引号双写、含分隔/换行/引号必裹引号），
//      任何行集都能无损往返到表格软件；
//   3. 行集账本 —— 行上限与截断如实标注（原生侧查询有界，纯件钉导出口径）。
//
// 测试：tests/starport-db.mjs（真实模块直测）。

export class DbError extends Error {
  readonly code: 'write-rejected' | 'empty-sql' | 'invalid-argument';

  constructor(code: DbError['code'], message: string) {
    super(message);
    this.name = 'DbError';
    this.code = code;
  }
}

const READ_PREFIXES = ['select', 'with', 'pragma', 'explain'];

/**
 * 语句分类（只读口径）：返回 'read' 或抛错。
 * 多语句（分号后带实质内容）拒绝；前导注释/空白容忍。
 */
export function classifySql(sql: string): 'read' {
  const raw = typeof sql === 'string' ? sql : '';
  // 剥前导注释与空白（-- 行注释、/* */ 块注释）。
  let text = raw.trim();
  let changed = true;
  while (changed) {
    changed = false;
    if (text.startsWith('--')) {
      const nl = text.indexOf('\n');
      text = nl === -1 ? '' : text.slice(nl + 1).trim();
      changed = true;
    } else if (text.startsWith('/*')) {
      const end = text.indexOf('*/');
      text = end === -1 ? '' : text.slice(end + 2).trim();
      changed = true;
    }
  }
  if (!text) throw new DbError('empty-sql', 'SQL 为空');
  // 多语句拒绝：分号之后仅允许空白/注释。
  const semi = text.indexOf(';');
  if (semi !== -1 && text.slice(semi + 1).trim().replace(/--[^\n]*/g, '').trim()) {
    throw new DbError('write-rejected', '多语句拒绝（一次只读一条）');
  }
  const head = (semi === -1 ? text : text.slice(0, semi)).trim().toLowerCase();
  if (READ_PREFIXES.some((prefix) => head.startsWith(prefix))) return 'read';
  throw new DbError('write-rejected', '只读口径：仅放行 SELECT/WITH/PRAGMA/EXPLAIN');
}

/** CSV 转义（RFC 4180 口径）：引号双写，含 " , \n \r 必裹引号。 */
export function toCsv(columns: readonly string[], rows: readonly (readonly unknown[])[]): string {
  if (!Array.isArray(columns) || !Array.isArray(rows)) {
    throw new DbError('invalid-argument', 'columns/rows 必须是数组');
  }
  const escape = (value: unknown): string => {
    const text = value === null || value === undefined ? '' : String(value);
    return /["\n\r,]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text;
  };
  const lines = [columns.map(escape).join(',')];
  for (const row of rows) {
    if (!Array.isArray(row)) throw new DbError('invalid-argument', '每行必须是数组');
    if (row.length !== columns.length) {
      throw new DbError('invalid-argument', '行列数与表头不齐');
    }
    lines.push(row.map(escape).join(','));
  }
  return `${lines.join('\r\n')}\r\n`;
}
