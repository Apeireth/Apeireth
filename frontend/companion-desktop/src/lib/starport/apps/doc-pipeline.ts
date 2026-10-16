// 星笺 · 源码保持型文档管线（内置应用面 I 批 2 MVP，纯逻辑零依赖）
//
// 契约（调研 reports/embedded-app-suite-research-2026-10-11.md §3.2 的独立重建）：
//   1. 唯一真源 —— source 字符串即文档本体；一切派生物（分段索引）可随时
//      丢弃重算，绝不反向覆盖真源；
//   2. 编译索引 —— compileDocument 产出编辑段：每段带源码字符区间
//      [start, end)、内容哈希、原片段 content；分段必须铺满真源
//      （各段 content 首尾相接 ≡ source，零长段不产出）；
//   3. 区间事务 —— applyEdit 只替换 [start, end) 且必须带 expected 原文
//      校验，不符即抛（防并发写坏：他人改过同一区间时旧索引立即失效）；
//   4. 编辑不变量 —— verifyEditInvariant 校验「临时编辑文本 ≡ 原源码片段」，
//      不成立即拒绝进入编辑（宁可不编辑，不冒险写回）；
//   5. 0 装 —— 越界 / 乱区间 / 哈希不符 / 段完整性破坏一律抛 DocPipelineError，
//      不猜不修不吞；
//   6. 渲染态直改 —— mapRenderedEdit 把「渲染态段内编辑」映射为绝对区间事务
//      （段完整性先行 + expectedLocal 校验），minimalSegmentEdit 前后缀裁剪
//      保证只替换真正变化的源码片段；
//   7. 可编程岛 —— 信息串含 `island` 的围栏块为岛段（kind 'island'）：
//      stableIslandId 稳定语义 ID（源码区间+内容哈希派生）、islandPayload
//      原样提载荷（不解析不执行——执行在装配层沙箱）、IslandRegistry
//      强制生命周期（dispose 恰好一次，二次抛错）。
//
// MVP 边界（本批只做这些，扩展位随批接）：
//   - 分段粒度：Markdown 正文（text）、围栏代码块（code）与可编程岛围栏
//     （island）；未闭合围栏如实切到文末（不猜闭合）；
//   - 编辑粒度：段内增量区间事务（渲染态直改，见下方映射族）；跨段联编与
//     选择级富文本为扩展位；
//   - 渲染层：本模块不产 HTML（分段结构即渲染契约，杜绝 XSS 面）；
//     样式化渲染是文档件（doc-app）的表现层职责；岛的执行沙箱同理归装配层；
//   - 扩展位：行内标记细化切分、公式/样式受保护区、
//     版本分支与提案合入（对接既有审批链）——均不在本 MVP 内。
//
// 哈希算法（注明）：FNV-1a 32 位，按 UTF-8 字节流计算，十六进制 8 位小写。
//   offset basis = 2166136261 (0x811C9DC5)，prime = 16777619 (0x01000193)。
//   用途是「编辑索引 ↔ 真源片段」的一致性哨兵，不是密码学承诺。
//
// 本文件刻意零运行时依赖（不 import svelte / DOM），Node 可直接 import
// 测试（tests/starport-doc-pipeline.mjs），与 gateway-events.ts 同一约定。
// 跨模块导入带 .ts 扩展（Node strip-types 直测要求，tsconfig 已允许）。

export type SegmentKind = 'text' | 'code' | 'island';

/** 编辑段：编译修订内的临时 key + 源码字符区间 + 内容哈希 + 原片段。 */
export interface DocumentSegment {
  /** `seg-<序号>` —— 编译修订内临时 key，重新编译后旧 key 作废。 */
  id: string;
  kind: SegmentKind;
  /** 源码字符区间 [start, end)（UTF-16 码元偏移，与 String.slice 同口径）。 */
  start: number;
  end: number;
  /** FNV-1a 32（UTF-8 字节流）十六进制 8 位小写。 */
  hash: string;
  /** 原源码片段（逐字符保真，≡ source.slice(start, end)）。 */
  content: string;
}

export interface CompiledDocument {
  segments: DocumentSegment[];
}

/** 区间事务编辑：只替换 [start, end)，expected 为该区间当前原文。 */
export interface DocEdit {
  start: number;
  end: number;
  expected: string;
  insert: string;
}

export class DocPipelineError extends Error {
  readonly code:
    | 'invalid-source'
    | 'invalid-edit'
    | 'invalid-range'
    | 'expected-mismatch'
    | 'offset-out-of-range'
    | 'segment-integrity'
    | 'invalid-segments'
    | 'invalid-island'
    | 'duplicate-island'
    | 'island-missing'
    | 'island-disposed';

  constructor(
    code:
      | 'invalid-source'
      | 'invalid-edit'
      | 'invalid-range'
      | 'expected-mismatch'
      | 'offset-out-of-range'
      | 'segment-integrity'
      | 'invalid-segments'
      | 'invalid-island'
      | 'duplicate-island'
      | 'island-missing'
      | 'island-disposed',
    message: string,
  ) {
    super(message);
    this.name = 'DocPipelineError';
    this.code = code;
  }
}

// ---------------------------------------------------------------------------
// 内容哈希（FNV-1a 32 / UTF-8 字节流）
// ---------------------------------------------------------------------------

const FNV_OFFSET_BASIS = 2166136261;
const FNV_PRIME = 16777619;

/** FNV-1a 32 位原始值（无符号）。 */
export function fnv1a32(content: string): number {
  if (typeof content !== 'string') {
    throw new DocPipelineError('invalid-source', '哈希输入必须是字符串');
  }
  const bytes = new TextEncoder().encode(content);
  let hash = FNV_OFFSET_BASIS;
  for (const byte of bytes) {
    hash = Math.imul(hash ^ byte, FNV_PRIME) >>> 0;
  }
  return hash >>> 0;
}

/** 段内容哈希（十六进制 8 位小写）。 */
export function hashContent(content: string): string {
  return fnv1a32(content).toString(16).padStart(8, '0');
}

// ---------------------------------------------------------------------------
// 编译：真源 → 编辑段索引
// ---------------------------------------------------------------------------

/** 围栏开启行：行首（允许空格/制表符缩进）连续 ≥3 个反引号。 */
function isFenceOpen(line: string): boolean {
  return /^[ \t]*`{3,}/.test(line);
}

/** 围栏闭合行：整行只有空格/制表符与连续 ≥3 个反引号。 */
function isFenceClose(line: string): boolean {
  return /^[ \t]*`{3,}[ \t]*$/.test(line);
}

/** 围栏信息串（开启行反引号后的文本）按空白切词。 */
function fenceInfoTokens(line: string): string[] {
  const rest = line.replace(/^[ \t]*`{3,}/, '').trim();
  return rest ? rest.split(/\s+/) : [];
}

/** 岛屿围栏：信息串含 `island` 词（如 ```island js 或 ```island name=my-widget）。 */
function isIslandFence(line: string): boolean {
  return fenceInfoTokens(line).includes('island');
}

/**
 * 编译唯一真源为编辑段索引。
 * 分段铺满真源：各段首尾相接（零长段不产出）；
 * 围栏块整块（含开启/闭合围栏行）为一个受保护段——编译器不改写其内容；
 * 开启行信息串含 `island` 词者为可编程岛（kind 'island'），否则为代码段
 * （kind 'code'，普通语言标注/空信息串均属此列）；未闭合围栏如实切到文末。
 */
export function compileDocument(source: string): CompiledDocument {
  if (typeof source !== 'string') {
    throw new DocPipelineError('invalid-source', '文档真源必须是字符串');
  }
  const segments: DocumentSegment[] = [];
  let textStart = 0;
  let index = 0;

  const push = (kind: SegmentKind, start: number, end: number): void => {
    if (end <= start) return;
    const content = source.slice(start, end);
    segments.push({
      id: `seg-${index}`,
      kind,
      start,
      end,
      hash: hashContent(content),
      content,
    });
    index += 1;
  };

  let lineStart = 0;
  while (lineStart < source.length) {
    const newline = source.indexOf('\n', lineStart);
    const lineStop = newline === -1 ? source.length : newline + 1; // 含换行
    const lineText = source.slice(lineStart, newline === -1 ? source.length : newline);

    if (isFenceOpen(lineText)) {
      // 收掉围栏前的正文段，再找闭合围栏（找不到即如实到文末）。
      push('text', textStart, lineStart);
      let scan = lineStop;
      let blockStop = source.length;
      while (scan < source.length) {
        const nextNewline = source.indexOf('\n', scan);
        const nextStop = nextNewline === -1 ? source.length : nextNewline + 1;
        const nextText = source.slice(scan, nextNewline === -1 ? source.length : nextNewline);
        if (isFenceClose(nextText)) {
          blockStop = nextStop;
          break;
        }
        scan = nextStop;
      }
      push(isIslandFence(lineText) ? 'island' : 'code', lineStart, blockStop);
      textStart = blockStop;
      lineStart = blockStop;
    } else {
      lineStart = lineStop;
    }
  }
  push('text', textStart, source.length);
  return {segments};
}

// ---------------------------------------------------------------------------
// 区间事务：带 expected 原文校验的替换
// ---------------------------------------------------------------------------

function assertEdit(source: string, edit: DocEdit): void {
  if (typeof source !== 'string') {
    throw new DocPipelineError('invalid-source', '文档真源必须是字符串');
  }
  if (!edit || typeof edit !== 'object') {
    throw new DocPipelineError('invalid-edit', '编辑事务必须是对象');
  }
  if (!Number.isInteger(edit.start) || !Number.isInteger(edit.end)) {
    throw new DocPipelineError('invalid-range', '区间端点必须是整数');
  }
  if (edit.start < 0 || edit.end < edit.start || edit.end > source.length) {
    throw new DocPipelineError('invalid-range', `区间越界或倒置：[${edit.start}, ${edit.end}) / 真源长 ${source.length}`);
  }
  if (typeof edit.expected !== 'string' || typeof edit.insert !== 'string') {
    throw new DocPipelineError('invalid-edit', 'expected 与 insert 必须是字符串');
  }
}

/**
 * 应用一次区间事务：只替换 [start, end)。
 * expected 与区间当前原文不符必须抛错——这是并发写坏的最后防线
 * （旧索引在他人改过同一区间后立即失效，宁可失败不可写坏）。
 */
export function applyEdit(source: string, edit: DocEdit): string {
  assertEdit(source, edit);
  const current = source.slice(edit.start, edit.end);
  if (current !== edit.expected) {
    throw new DocPipelineError('expected-mismatch', '区间原文与 expected 不符——索引已过期，拒绝写回');
  }
  return source.slice(0, edit.start) + edit.insert + source.slice(edit.end);
}

// ---------------------------------------------------------------------------
// 编辑不变量：临时编辑文本 ≡ 原源码片段
// ---------------------------------------------------------------------------

/**
 * 校验「临时编辑文本 ≡ 原源码片段」。成立返回 true，调用方方可进入
 * 视觉编辑态；不成立返回 false（拒绝进入编辑，不冒险写回）。
 * 区间非法直接抛错——调用损坏与不变量不成立是两回事，不许混为一谈。
 */
export function verifyEditInvariant(
  visualText: string,
  source: string,
  range: {start: number; end: number},
): boolean {
  if (typeof source !== 'string' || typeof visualText !== 'string') {
    throw new DocPipelineError('invalid-source', '校验双方必须是字符串');
  }
  if (!range || !Number.isInteger(range.start) || !Number.isInteger(range.end)) {
    throw new DocPipelineError('invalid-range', '区间端点必须是整数');
  }
  if (range.start < 0 || range.end < range.start || range.end > source.length) {
    throw new DocPipelineError('invalid-range', `区间越界或倒置：[${range.start}, ${range.end}) / 真源长 ${source.length}`);
  }
  return visualText === source.slice(range.start, range.end);
}

// ---------------------------------------------------------------------------
// 段定位与完整性
// ---------------------------------------------------------------------------

function assertSegments(segments: readonly DocumentSegment[]): void {
  if (!Array.isArray(segments)) {
    throw new DocPipelineError('invalid-segments', '段索引必须是数组');
  }
  let prevEnd = -1;
  for (const segment of segments) {
    if (
      !segment ||
      !Number.isInteger(segment.start) ||
      !Number.isInteger(segment.end) ||
      segment.start < 0 ||
      segment.end < segment.start ||
      typeof segment.content !== 'string' ||
      segment.content.length !== segment.end - segment.start
    ) {
      throw new DocPipelineError('invalid-segments', '段结构非法（区间或 content 长度不符）');
    }
    if (segment.start < prevEnd) {
      throw new DocPipelineError('invalid-segments', '段索引乱序或重叠');
    }
    prevEnd = segment.end;
  }
}

/**
 * 段完整性校验：区间内原片段 ≡ segment.content 且哈希一致。
 * 不符抛错（0 装：索引过期就承认过期，不带病运行）。
 */
export function verifySegment(source: string, segment: DocumentSegment): void {
  if (typeof source !== 'string') {
    throw new DocPipelineError('invalid-source', '文档真源必须是字符串');
  }
  // verify 的职责就是完整性：参数畸形走 invalid-range，其余任何不一致
  // （长度伪装/越界/切片不符/哈希不符）都是段完整性问题，统一 segment-integrity。
  if (
    !segment ||
    !Number.isInteger(segment.start) ||
    !Number.isInteger(segment.end) ||
    segment.start < 0 ||
    segment.end < segment.start
  ) {
    throw new DocPipelineError('invalid-range', '段区间非法');
  }
  if (typeof segment.content !== 'string' || segment.content.length !== segment.end - segment.start) {
    throw new DocPipelineError('segment-integrity', `段 ${segment.id} 的 content 与区间长度不符——段被篡改`);
  }
  if (segment.end > source.length) {
    throw new DocPipelineError('segment-integrity', `段 ${segment.id} 超出真源长度——索引已过期`);
  }
  const current = source.slice(segment.start, segment.end);
  if (current !== segment.content) {
    throw new DocPipelineError('segment-integrity', `段 ${segment.id} 的 content 与真源不符——索引已过期`);
  }
  if (hashContent(segment.content) !== segment.hash) {
    throw new DocPipelineError('segment-integrity', `段 ${segment.id} 哈希不符——段内容被篡改或索引损坏`);
  }
}

/**
 * 偏移 → 段定位（点击/选区映射回源码的入口）。
 * - 命中段体 [start, end) 返回该段；段界处归属后段（与区间口径一致）；
 * - 文末光标位（offset === 末段 end）返回 null——合法插入点，不属于任何段体；
 * - 负数 / 非整数 / 超出文末，或段索引乱序重叠，一律抛错（0 装）。
 */
export function findSegment(
  segments: readonly DocumentSegment[],
  offset: number,
): DocumentSegment | null {
  assertSegments(segments);
  if (!Number.isInteger(offset) || offset < 0) {
    throw new DocPipelineError('offset-out-of-range', `偏移非法：${String(offset)}`);
  }
  for (const segment of segments) {
    if (offset >= segment.start && offset < segment.end) return segment;
  }
  const lastEnd = segments.length > 0 ? segments[segments.length - 1].end : 0;
  if (offset > lastEnd) {
    throw new DocPipelineError('offset-out-of-range', `偏移超出文末：${offset} > ${lastEnd}`);
  }
  return null;
}

// ---------------------------------------------------------------------------
// 渲染态直改：渲染态局部编辑 → 真源区间事务
// ---------------------------------------------------------------------------

/** 渲染态局部编辑：段内偏移/长度 + 新文本 + 段内原文校验。 */
export interface RenderedEdit {
  /** 目标段 id（编译修订内临时 key）。 */
  segmentId: string;
  /** 段内偏移（UTF-16 码元，与 String.slice 同口径）。 */
  offsetInSegment: number;
  /** 段内删除长度。 */
  deleteLength: number;
  /** 替换文本。 */
  insertText: string;
  /** 被删区间的段内原文（不符即拒绝写回）。 */
  expectedLocal: string;
}

/** 段内编辑（不带段 id——由调用方补，供 minimalSegmentEdit 归约产出）。 */
export type LocalEdit = Omit<RenderedEdit, 'segmentId'>;

/**
 * 段的渲染态文本。MVP 与源片段恒等（接口留白：未来样式化渲染变换只改
 * 本函数，不改调用方）。段完整性先行——过期/被篡改即抛，不带病外显。
 */
export function visualSlice(source: string, segment: DocumentSegment): string {
  verifySegment(source, segment);
  return source.slice(segment.start, segment.end);
}

/**
 * 渲染态局部编辑 → 真源区间事务：段内偏移映射为绝对区间，走带 expected
 * 校验的区间事务写回（只替换 [start, end)，绝不整段重写）。
 *
 * 拒绝面（0 装，码位与既有家族对齐）：
 *   - 形状畸形（缺字段/类型不符/未知段 id）→ invalid-edit；
 *   - 偏移类（负数/非整数/越出段体）→ offset-out-of-range；
 *   - 长度类（负数/非整数）→ invalid-range；
 *   - 段过期（哈希/内容与真源不符）→ segment-integrity；
 *   - expectedLocal 与段内原文不符 → segment-integrity（渲染态已过期）。
 */
export function mapRenderedEdit(
  source: string,
  segments: readonly DocumentSegment[],
  edit: RenderedEdit,
): string {
  if (typeof source !== 'string') {
    throw new DocPipelineError('invalid-source', '文档真源必须是字符串');
  }
  assertSegments(segments);
  if (!edit || typeof edit !== 'object') {
    throw new DocPipelineError('invalid-edit', '渲染态编辑必须是对象');
  }
  if (typeof edit.segmentId !== 'string' || !edit.segmentId) {
    throw new DocPipelineError('invalid-edit', 'segmentId 必须是非空字符串');
  }
  if (!Number.isInteger(edit.offsetInSegment) || edit.offsetInSegment < 0) {
    throw new DocPipelineError('offset-out-of-range', `段内偏移非法：${String(edit.offsetInSegment)}`);
  }
  if (!Number.isInteger(edit.deleteLength) || edit.deleteLength < 0) {
    throw new DocPipelineError('invalid-range', `删除长度非法：${String(edit.deleteLength)}`);
  }
  if (typeof edit.insertText !== 'string' || typeof edit.expectedLocal !== 'string') {
    throw new DocPipelineError('invalid-edit', 'insertText 与 expectedLocal 必须是字符串');
  }
  const segment = segments.find((item) => item.id === edit.segmentId);
  if (!segment) {
    throw new DocPipelineError('invalid-edit', `编辑事务引用未知段：${edit.segmentId}`);
  }
  // 段过期（哈希/内容与真源不符）→ segment-integrity，不带病写回。
  verifySegment(source, segment);
  const localLength = segment.end - segment.start;
  if (edit.offsetInSegment + edit.deleteLength > localLength) {
    throw new DocPipelineError(
      'offset-out-of-range',
      `段内区间越出段体：offset ${edit.offsetInSegment} + delete ${edit.deleteLength} > 段长 ${localLength}`,
    );
  }
  const start = segment.start + edit.offsetInSegment;
  const end = start + edit.deleteLength;
  if (source.slice(start, end) !== edit.expectedLocal) {
    throw new DocPipelineError('segment-integrity', 'expectedLocal 与段内原文不符——渲染态已过期，拒绝写回');
  }
  // 实际写回仍走区间事务（expected 再校验一次，防线不减）。
  return applyEdit(source, {start, end, expected: edit.expectedLocal, insert: edit.insertText});
}

/**
 * 渲染态编辑归约：整段新旧文本 → 最小段内编辑（公共前缀/后缀裁剪，
 * 只保留真正变化的片段——源码保持型管线的核心纪律）。
 * 新旧相同 → 零操作事务（偏移落旧文末，空删空插）。
 */
export function minimalSegmentEdit(oldLocal: string, newLocal: string): LocalEdit {
  if (typeof oldLocal !== 'string' || typeof newLocal !== 'string') {
    throw new DocPipelineError('invalid-edit', '新旧文本必须是字符串');
  }
  const shorter = Math.min(oldLocal.length, newLocal.length);
  let prefix = 0;
  while (prefix < shorter && oldLocal[prefix] === newLocal[prefix]) {
    prefix += 1;
  }
  let suffix = 0;
  const maxSuffix = shorter - prefix;
  while (
    suffix < maxSuffix &&
    oldLocal[oldLocal.length - 1 - suffix] === newLocal[newLocal.length - 1 - suffix]
  ) {
    suffix += 1;
  }
  return {
    offsetInSegment: prefix,
    deleteLength: oldLocal.length - prefix - suffix,
    insertText: newLocal.slice(prefix, newLocal.length - suffix),
    expectedLocal: oldLocal.slice(prefix, oldLocal.length - suffix),
  };
}

// ---------------------------------------------------------------------------
// 可编程岛：稳定语义 ID / 载荷提取 / 生命周期登记
// ---------------------------------------------------------------------------

/**
 * 岛稳定语义 ID：由源码区间 + 内容哈希派生（FNV-1a 二次聚合）。
 * 口径：同源同区间跨编译恒同 ID；内容变即换 ID；区间移位即换 ID——
 * ID 的承诺是「这一处这一段内容」，不是「岛的名字」（名字在信息串自携）。
 * 格式 `isl-<FNV-1a 8 位小写十六进制>`。
 */
export function stableIslandId(source: string, range: {start: number; end: number}): string {
  if (typeof source !== 'string') {
    throw new DocPipelineError('invalid-source', '文档真源必须是字符串');
  }
  if (!range || !Number.isInteger(range.start) || !Number.isInteger(range.end)) {
    throw new DocPipelineError('invalid-range', '区间端点必须是整数');
  }
  if (range.start < 0 || range.end < range.start || range.end > source.length) {
    throw new DocPipelineError(
      'invalid-range',
      `区间越界或倒置：[${range.start}, ${range.end}) / 真源长 ${source.length}`,
    );
  }
  const contentHash = hashContent(source.slice(range.start, range.end));
  return `isl-${hashContent(`${range.start}:${range.end}:${contentHash}`)}`;
}

/**
 * 岛载荷提取：开启围栏行之后、闭合围栏行之前的正文**原样**返回（含行尾
 * 换行；未闭合如实到段末——不猜闭合）。载荷即脚本/标记载荷，本模块不解析
 * 不执行（执行在装配层沙箱 iframe）。
 * 0 装：非岛屿段 / 首行非岛屿围栏（段结构损坏）一律抛 invalid-island。
 */
export function islandPayload(segment: DocumentSegment): string {
  if (!segment || typeof segment !== 'object' || segment.kind !== 'island') {
    throw new DocPipelineError('invalid-island', '只有岛屿段可提取载荷');
  }
  if (typeof segment.content !== 'string') {
    throw new DocPipelineError('invalid-island', '岛屿段 content 必须是字符串');
  }
  const firstNewline = segment.content.indexOf('\n');
  const firstLine = firstNewline === -1 ? segment.content : segment.content.slice(0, firstNewline);
  if (!isFenceOpen(firstLine) || !isIslandFence(firstLine)) {
    throw new DocPipelineError('invalid-island', '岛屿段首行必须是岛屿围栏开启行（段结构损坏）');
  }
  if (firstNewline === -1) return '';
  const body = segment.content.slice(firstNewline + 1);
  let cursor = 0;
  while (cursor < body.length) {
    const next = body.indexOf('\n', cursor);
    const stop = next === -1 ? body.length : next + 1;
    const lineText = body.slice(cursor, next === -1 ? body.length : next);
    if (isFenceClose(lineText)) {
      return body.slice(0, cursor);
    }
    cursor = stop;
  }
  return body;
}

/**
 * 岛生命周期登记（切页/重编译/卸载链路的强制释放账本）。
 * 纪律：
 *   - dispose **恰好一次**——先标记后调用（回调抛错也不二启）；
 *   - 二次 dispose 抛 island-disposed（0 装：不静默吞）；
 *   - 在册未释放的同 id 重复登记抛 duplicate-island；已释放的 id 可重登记
 *     （重渲染同内容岛走同一稳定 ID 的合法回路）；
 *   - disposeAll 逐个释放（一个回调抛错不漏放其余岛），完成全部释放后
 *     如实抛出首个错误（不吞错）。
 */
export class IslandRegistry {
  private readonly entries = new Map<string, {dispose: () => void; disposed: boolean}>();

  register(id: string, dispose: () => void): void {
    if (typeof id !== 'string' || !id || typeof dispose !== 'function') {
      throw new DocPipelineError('invalid-island', '登记需非空 id 与 dispose 函数');
    }
    const existing = this.entries.get(id);
    if (existing && !existing.disposed) {
      throw new DocPipelineError('duplicate-island', `岛已在册未释放：${id}`);
    }
    this.entries.set(id, {dispose, disposed: false});
  }

  dispose(id: string): void {
    const entry = this.entries.get(id);
    if (!entry) {
      throw new DocPipelineError('island-missing', `岛未登记：${id}`);
    }
    if (entry.disposed) {
      throw new DocPipelineError('island-disposed', `岛已释放（dispose 恰好一次）：${id}`);
    }
    entry.disposed = true;
    entry.dispose();
  }

  activeCount(): number {
    let count = 0;
    for (const entry of this.entries.values()) {
      if (!entry.disposed) count += 1;
    }
    return count;
  }

  /** 释放全部在册岛。返回本次完成释放标记的个数（含回调抛错者）。 */
  disposeAll(): number {
    let count = 0;
    let firstError: unknown = null;
    for (const entry of this.entries.values()) {
      if (!entry.disposed) {
        entry.disposed = true;
        try {
          entry.dispose();
        } catch (caught) {
          if (firstError === null) firstError = caught;
        }
        count += 1;
      }
    }
    if (firstError !== null) throw firstError;
    return count;
  }
}
