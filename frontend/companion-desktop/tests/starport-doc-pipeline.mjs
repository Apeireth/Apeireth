// 星笺源码保持型文档管线（doc-pipeline.ts + doc-app.ts，真实模块）——
// 唯一真源 / 编译索引 / 区间事务 / 编辑不变量 的插口证明。
//
// 契约要点：
//   1. 分段铺满真源：text/code 段首尾相接 ≡ source，零长段不产出；
//      围栏代码块整块（含围栏行）为一个 code 段，未闭合围栏如实到文末；
//   2. 哈希稳定：FNV-1a 32 / UTF-8 字节流（基向量 + 独立镜像双钉）；
//   3. 区间事务：expected 不符必须抛错（防并发写坏），乱区间/非整数抛错；
//   4. 编辑不变量：通过/失败两侧 + 乱区间抛错（调用损坏与不成立分开）；
//   5. 段定位边界：段界归属后段、文末光标位返回 null、越界抛错、乱序重叠抛错；
//   6. 段完整性：content/哈希被篡改一律抛错（0 装）。
import assert from 'node:assert/strict';
import {
  compileDocument,
  applyEdit,
  verifyEditInvariant,
  findSegment,
  verifySegment,
  hashContent,
  fnv1a32,
  DocPipelineError,
} from '../src/lib/starport/apps/doc-pipeline.ts';
import {createDocApp, STARPORT_DOC_APP_ID} from '../src/lib/starport/apps/doc-app.ts';

console.log('--- Starting Starport Doc Pipeline Check ---');

const codeOf = (fn) => {
  try {
    fn();
    return null;
  } catch (error) {
    return error instanceof DocPipelineError ? error.code : 'not-doc-error';
  }
};

// 独立镜像（与实现分开写，交叉钉 FNV-1a 语义；空串 = offset basis）。
function fnv1a32Mirror(content) {
  const bytes = new TextEncoder().encode(content);
  let hash = 2166136261;
  for (const byte of bytes) {
    hash = Math.imul(hash ^ byte, 16777619) >>> 0;
  }
  return hash >>> 0;
}

// ---------------------------------------------------------------------------
// 1. 分段与字符区间（含围栏识别 / 铺满真源 / 未闭合围栏）
// ---------------------------------------------------------------------------
{
  const source = '# 标题\n\n正文一。\n\n```\ncode line\n```\n\n正文二。\n';
  const {segments} = compileDocument(source);

  assert.deepEqual(
    segments.map((s) => s.kind),
    ['text', 'code', 'text'],
    '正文/围栏/正文三段切分',
  );
  assert.equal(segments[0].start, 0, '首段从 0 开始');
  assert.equal(segments.at(-1).end, source.length, '末段铺到文末');
  assert.equal(
    segments.map((s) => s.content).join(''),
    source,
    '各段 content 首尾相接 ≡ 真源',
  );
  for (const segment of segments) {
    assert.equal(segment.content, source.slice(segment.start, segment.end), `${segment.id} 区间与 content 一致`);
  }
  assert.equal(segments[1].content, '```\ncode line\n```\n', '代码段整块含围栏行（受保护区域）');
  assert.equal(segments[1].start, source.indexOf('```'), '代码段起点对齐围栏行');
  assert.match(segments[0].id, /^seg-\d+$/, '段 key 为编译修订内临时 key');
  console.log('  [1] segmentation & ranges OK');
}

{
  // 未闭合围栏如实到文末（不猜闭合）；空源零段；零长段不产出。
  const unclosed = 'x\n```\nabc';
  const {segments} = compileDocument(unclosed);
  assert.deepEqual(segments.map((s) => s.kind), ['text', 'code']);
  assert.equal(segments.at(-1).end, unclosed.length, '未闭合围栏切到文末');
  assert.equal(segments[1].content, '```\nabc');

  assert.deepEqual(compileDocument('').segments, [], '空源零段');

  // 两围栏块之间隔一个空行 → 中间 text 段恰为 '\n'（非零长即产出）。
  const adjacent = '```\na\n```\n\n```\nb\n```\n';
  const adj = compileDocument(adjacent).segments;
  assert.deepEqual(adj.map((s) => s.kind), ['code', 'text', 'code']);
  assert.equal(adj[1].content, '\n');

  // 真正相邻（闭合围栏行已含换行，中间正文零长）→ 零长段不产出。
  const glued = '```\na\n```\n```\nb\n```\n';
  assert.deepEqual(
    compileDocument(glued).segments.map((s) => s.kind),
    ['code', 'code'],
    '零长正文段不产出（契约：分段铺满但零长不立段）',
  );

  // 带语言标注的围栏同样识别为代码段。
  const lang = '```js\nlet x = 1;\n```\n';
  assert.equal(compileDocument(lang).segments[0].kind, 'code');
  console.log('  [2] fence edge cases OK');
}

// ---------------------------------------------------------------------------
// 3. 哈希稳定性（基向量 + 独立镜像 + 跨编译稳定 + 段哈希自洽）
// ---------------------------------------------------------------------------
{
  assert.equal(fnv1a32(''), 2166136261, '空串 = offset basis');
  for (const content of ['hello', '星笺', 'a\n```', '']) {
    assert.equal(fnv1a32(content), fnv1a32Mirror(content), `镜像一致：${JSON.stringify(content)}`);
  }
  assert.equal(hashContent('hello'), hashContent('hello'), '同文同哈希');
  assert.notEqual(hashContent('hello'), hashContent('hellp'), '异文异哈希（非退化常量）');
  assert.match(hashContent('星笺'), /^[0-9a-f]{8}$/, '十六进制 8 位小写');

  const source = '文本\n```\ncode\n```\n';
  const once = compileDocument(source).segments;
  const twice = compileDocument(source).segments;
  assert.deepEqual(
    once.map((s) => s.hash),
    twice.map((s) => s.hash),
    '跨编译哈希稳定',
  );
  for (const segment of once) {
    assert.equal(segment.hash, hashContent(segment.content), `${segment.id} 段哈希自洽`);
  }
  console.log('  [3] hash stability OK');
}

// ---------------------------------------------------------------------------
// 4. 区间事务 applyEdit：正常替换 / 纯插入 / expected 不符 / 乱区间
// ---------------------------------------------------------------------------
{
  const source = 'hello world';
  assert.equal(
    applyEdit(source, {start: 6, end: 11, expected: 'world', insert: '星笺'}),
    'hello 星笺',
    '整段替换级事务',
  );
  assert.equal(
    applyEdit(source, {start: 5, end: 5, expected: '', insert: '，'}),
    'hello， world',
    '纯插入（零长区间）',
  );
  assert.equal(
    applyEdit(source, {start: 0, end: 0, expected: '', insert: '>'}),
    '>hello world',
    '文首插入',
  );

  assert.equal(
    codeOf(() => applyEdit(source, {start: 6, end: 11, expected: 'worlds', insert: 'x'})),
    'expected-mismatch',
    'expected 不符必须抛（防并发写坏）',
  );
  assert.equal(
    codeOf(() => applyEdit('abc', {start: 1, end: 1, expected: 'z', insert: 'x'})),
    'expected-mismatch',
    '零长区间 expected 也校验',
  );
  assert.equal(codeOf(() => applyEdit(source, {start: -1, end: 3, expected: 'hel', insert: 'x'})), 'invalid-range');
  assert.equal(codeOf(() => applyEdit(source, {start: 6, end: 2, expected: '', insert: 'x'})), 'invalid-range', '倒置区间');
  assert.equal(codeOf(() => applyEdit(source, {start: 0, end: 999, expected: '', insert: 'x'})), 'invalid-range', '越界区间');
  assert.equal(codeOf(() => applyEdit(source, {start: 1.5, end: 3, expected: '', insert: 'x'})), 'invalid-range', '非整数端点');
  assert.equal(codeOf(() => applyEdit(source, {start: 0, end: 1, expected: 7, insert: 'x'})), 'invalid-edit', 'expected 非字符串');
  assert.equal(codeOf(() => applyEdit(source, {start: 0, end: 1, expected: 'h', insert: null})), 'invalid-edit', 'insert 非字符串');
  console.log('  [4] range transaction OK');
}

// ---------------------------------------------------------------------------
// 5. 编辑不变量：通过 / 失败两侧 + 调用损坏抛错
// ---------------------------------------------------------------------------
{
  const source = 'abc\ndef';
  assert.equal(verifyEditInvariant('abc', source, {start: 0, end: 3}), true, '临时编辑文本 ≡ 原片段');
  assert.equal(verifyEditInvariant('abX', source, {start: 0, end: 3}), false, '不成立返回 false（拒绝进入编辑）');
  assert.equal(verifyEditInvariant('def', source, {start: 4, end: 7}), true);
  assert.equal(verifyEditInvariant('', source, {start: 3, end: 3}), true, '零长区间空文本成立');

  assert.equal(codeOf(() => verifyEditInvariant('x', source, {start: -1, end: 2})), 'invalid-range');
  assert.equal(codeOf(() => verifyEditInvariant('x', source, {start: 5, end: 2})), 'invalid-range');
  assert.equal(codeOf(() => verifyEditInvariant('x', source, {start: 0, end: 99})), 'invalid-range');
  assert.equal(codeOf(() => verifyEditInvariant(null, source, {start: 0, end: 1})), 'invalid-source');
  console.log('  [5] edit invariant OK');
}

// ---------------------------------------------------------------------------
// 6. findSegment 边界：段内 / 段界归后段 / 文末 null / 越界 / 乱序重叠
// ---------------------------------------------------------------------------
{
  const source = 'a\n```\nx\n```\n';
  const {segments} = compileDocument(source);
  assert.equal(segments.length, 2);
  assert.equal(findSegment(segments, 0), segments[0]);
  assert.equal(findSegment(segments, segments[0].end - 1), segments[0], '段末字符仍属本段');
  assert.equal(findSegment(segments, segments[0].end), segments[1], '段界归属后段');
  assert.equal(findSegment(segments, segments[1].start), segments[1]);
  assert.equal(findSegment(segments, source.length), null, '文末光标位合法插入点，返回 null');

  assert.equal(codeOf(() => findSegment(segments, -1)), 'offset-out-of-range');
  assert.equal(codeOf(() => findSegment(segments, source.length + 1)), 'offset-out-of-range', '超出文末抛错');
  assert.equal(codeOf(() => findSegment(segments, 1.5)), 'offset-out-of-range');

  // 单段文档：文末 = 末段 end → null。
  const single = compileDocument('abc').segments;
  assert.equal(findSegment(single, 3), null);
  assert.equal(findSegment(single, 2), single[0]);

  // 乱序/重叠索引一律抛错（0 装）。
  const overlap = [
    {id: 'seg-0', kind: 'text', start: 0, end: 5, hash: hashContent('12345'), content: '12345'},
    {id: 'seg-1', kind: 'text', start: 3, end: 7, hash: hashContent('4567'), content: '4567'},
  ];
  assert.equal(codeOf(() => findSegment(overlap, 4)), 'invalid-segments');
  const broken = [{id: 'seg-0', kind: 'text', start: 0, end: 5, hash: hashContent('123'), content: '123'}];
  assert.equal(codeOf(() => findSegment(broken, 1)), 'invalid-segments', 'content 长度与区间不符抛错');
  console.log('  [6] findSegment boundaries OK');
}

// ---------------------------------------------------------------------------
// 7. 段完整性 verifySegment：完好 / 篡改 content / 篡改哈希 / 越界
// ---------------------------------------------------------------------------
{
  const source = '文本\n```\ncode\n```\n';
  const [textSeg, codeSeg] = compileDocument(source).segments;
  verifySegment(source, textSeg);
  verifySegment(source, codeSeg);

  const tamperedContent = {...textSeg, content: textSeg.content + '!'};
  assert.equal(codeOf(() => verifySegment(source, tamperedContent)), 'segment-integrity', 'content 被改必须抛');
  const tamperedHash = {...textSeg, hash: 'deadbeef'};
  assert.equal(codeOf(() => verifySegment(source, tamperedHash)), 'segment-integrity', '哈希被改必须抛');
  const overflow = {...textSeg, end: source.length + 10, content: textSeg.content + 'x'.repeat(12)};
  assert.equal(codeOf(() => verifySegment(source, overflow)), 'segment-integrity', '长度伪装一致但真源不符 → 段完整性抛');

  // 编辑后旧段立即过期（事务防线的另一面）。
  const edited = applyEdit(source, {start: 0, end: 2, expected: '文本', insert: 'XY'});
  assert.equal(codeOf(() => verifySegment(edited, textSeg)), 'segment-integrity', '旧索引在他人改后立即失效');
  console.log('  [7] segment integrity OK');
}

// ---------------------------------------------------------------------------
// 8. 往返闭环：事务编辑 → 重编译 → 新索引区间正确
// ---------------------------------------------------------------------------
{
  let source = '# 文档\n\n段落。\n\n```\nold\n```\n';
  let compiled = compileDocument(source);
  const codeSeg = compiled.segments.find((s) => s.kind === 'code');
  source = applyEdit(source, {
    start: codeSeg.start + 4,
    end: codeSeg.end - 5,
    expected: 'old',
    insert: 'new code',
  });
  compiled = compileDocument(source);
  assert.equal(
    compiled.segments.map((s) => s.content).join(''),
    source,
    '重编译后分段仍铺满真源',
  );
  const newCode = compiled.segments.find((s) => s.kind === 'code');
  assert.equal(newCode.content, '```\nnew code\n```\n');
  for (const segment of compiled.segments) verifySegment(source, segment);
  console.log('  [8] edit → recompile round-trip OK');
}

// ---------------------------------------------------------------------------
// 9. 文档件定义面：元数据与挂载协议（注册归装配面，本批不越界）
// ---------------------------------------------------------------------------
{
  const app = createDocApp();
  assert.equal(app.id, STARPORT_DOC_APP_ID);
  assert.equal(app.id, 'starport-doc');
  assert.equal(app.title, '星笺');
  assert.equal(app.kind, 'internal');
  assert.equal(typeof app.mount, 'function', 'mount 挂载协议就位（注册待装配面）');
  console.log('  [9] doc app contract OK');
}

console.log('--- Starport Doc Pipeline Check PASSED ---');
