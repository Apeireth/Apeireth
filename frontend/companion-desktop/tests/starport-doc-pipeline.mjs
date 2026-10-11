// 星笺源码保持型文档管线（doc-pipeline.ts + doc-app.ts，真实模块）——
// 唯一真源 / 编译索引 / 区间事务 / 编辑不变量 的插口证明。
//
// 契约要点：
//   1. 分段铺满真源：text/code/island 段首尾相接 ≡ source，零长段不产出；
//      围栏块整块（含围栏行）为一个受保护段，未闭合围栏如实到文末；
//   2. 哈希稳定：FNV-1a 32 / UTF-8 字节流（基向量 + 独立镜像双钉）；
//   3. 区间事务：expected 不符必须抛错（防并发写坏），乱区间/非整数抛错；
//   4. 编辑不变量：通过/失败两侧 + 乱区间抛错（调用损坏与不成立分开）；
//   5. 段定位边界：段界归属后段、文末光标位返回 null、越界抛错、乱序重叠抛错；
//   6. 段完整性：content/哈希被篡改一律抛错（0 装）；
//   7. 渲染态直改：段内偏移映射回真源绝对区间（手算核对）、expectedLocal
//      不符/段过期/哈希篡改一律拒绝、越界与形状矩阵、最小段内编辑前后缀裁剪；
//   8. 可编程岛：岛围栏识别（与普通围栏区分）、stableIslandId 确定性与
//      内容/区间敏感性、islandPayload 原样载荷（不解析执行）、IslandRegistry
//      dispose 恰好一次（二次抛错）/重登记回路/disposeAll 不漏放不吞错。
//   既有 [1]/[2] 的 kind 字面（text/code）无需演进：info 串不含 island 词的
//   围栏（空信息串/纯语言标注）仍是 code——island 词是唯一分岔开关。
import assert from 'node:assert/strict';
import {
  compileDocument,
  applyEdit,
  verifyEditInvariant,
  findSegment,
  verifySegment,
  visualSlice,
  mapRenderedEdit,
  minimalSegmentEdit,
  stableIslandId,
  islandPayload,
  IslandRegistry,
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

// ---------------------------------------------------------------------------
// 10. 渲染态直改：段内偏移映射手算核对 + 最小段内编辑归约
// ---------------------------------------------------------------------------
{
  const source = 'hello world\n```\ncode\n```\n';
  const {segments} = compileDocument(source);
  const [textSeg, codeSeg] = segments;

  assert.equal(visualSlice(source, textSeg), 'hello world\n', '渲染态文本 ≡ 源片段（MVP 恒等）');
  assert.equal(visualSlice(source, codeSeg), '```\ncode\n```\n');

  // 段内替换手算：offset 6 删 5 插 '星笺' → 绝对 [6, 11)，只动真源对应片段。
  const replaced = mapRenderedEdit(source, segments, {
    segmentId: textSeg.id,
    offsetInSegment: 6,
    deleteLength: 5,
    insertText: '星笺',
    expectedLocal: 'world',
  });
  assert.equal(replaced, 'hello 星笺\n```\ncode\n```\n', '段内替换只动真源对应片段');

  // 段内纯插入（零长删除）。
  const inserted = mapRenderedEdit(source, segments, {
    segmentId: textSeg.id,
    offsetInSegment: 0,
    deleteLength: 0,
    insertText: '> ',
    expectedLocal: '',
  });
  assert.equal(inserted, '> hello world\n```\ncode\n```\n');

  // 受保护代码段同样走段内事务。
  const recoded = mapRenderedEdit(source, segments, {
    segmentId: codeSeg.id,
    offsetInSegment: 4,
    deleteLength: 4,
    insertText: 'Rust',
    expectedLocal: 'code',
  });
  assert.equal(recoded, 'hello world\n```\nRust\n```\n');

  // 最小段内编辑归约：前后缀裁剪手算核对。
  const trimmed = minimalSegmentEdit('hello world', 'hello 星笺');
  assert.deepEqual(
    [trimmed.offsetInSegment, trimmed.deleteLength, trimmed.insertText, trimmed.expectedLocal],
    [6, 5, '星笺', 'world'],
    '中段替换只留变化片段',
  );
  const tiny = minimalSegmentEdit('abc', 'abXc');
  assert.deepEqual(
    [tiny.offsetInSegment, tiny.deleteLength, tiny.insertText, tiny.expectedLocal],
    [2, 0, 'X', ''],
    '纯插入位（前后缀夹出零长删除）',
  );
  const noop = minimalSegmentEdit('same', 'same');
  assert.equal(noop.deleteLength, 0);
  assert.equal(noop.insertText, '', '相同文本 → 零操作事务');
  const whole = minimalSegmentEdit('abc', 'xyz');
  assert.deepEqual(
    [whole.offsetInSegment, whole.deleteLength, whole.insertText, whole.expectedLocal],
    [0, 3, 'xyz', 'abc'],
    '全异文本退化为整段替换',
  );
  const shrink = minimalSegmentEdit('aaa', 'aa');
  assert.deepEqual([shrink.offsetInSegment, shrink.deleteLength, shrink.insertText], [2, 1, ''], '重叠前缀不越界（尾删一个 a）');

  // 组合闭环：归约 + 映射 ≡ 效果等于整段文本替换，重编译仍铺满真源。
  const doc = 'para\n```\nx\n```\n';
  const compiled = compileDocument(doc);
  const para = compiled.segments[0];
  const edit = minimalSegmentEdit(visualSlice(doc, para), 'para EDITED\n');
  const next = mapRenderedEdit(doc, compiled.segments, {segmentId: para.id, ...edit});
  assert.equal(next, 'para EDITED\n```\nx\n```\n', '组合闭环：渲染态编辑落回真源');
  assert.equal(compileDocument(next).segments.map((s) => s.content).join(''), next);
  console.log('  [10] rendered edit mapping OK');
}

// ---------------------------------------------------------------------------
// 11. 渲染态直改拒绝面：expectedLocal 不符 / 段过期 / 哈希篡改
// ---------------------------------------------------------------------------
{
  const source = 'hello world\n```\ncode\n```\n';
  const {segments} = compileDocument(source);
  const [textSeg, codeSeg] = segments;

  assert.equal(
    codeOf(() =>
      mapRenderedEdit(source, segments, {
        segmentId: textSeg.id,
        offsetInSegment: 6,
        deleteLength: 5,
        insertText: 'x',
        expectedLocal: 'worlds',
      }),
    ),
    'segment-integrity',
    'expectedLocal 与段内原文不符必须抛',
  );

  // 段过期：真源被改后旧索引立即失效。
  const edited = applyEdit(source, {start: 0, end: 5, expected: 'hello', insert: 'HELLO'});
  assert.equal(
    codeOf(() =>
      mapRenderedEdit(edited, segments, {
        segmentId: textSeg.id,
        offsetInSegment: 0,
        deleteLength: 0,
        insertText: 'x',
        expectedLocal: '',
      }),
    ),
    'segment-integrity',
    '旧索引在他人改后立即失效',
  );

  // 哈希被篡改（长度伪装一致）→ 段过期拒绝。
  const tampered = [{...textSeg, hash: 'deadbeef'}, codeSeg];
  assert.equal(
    codeOf(() =>
      mapRenderedEdit(source, tampered, {
        segmentId: textSeg.id,
        offsetInSegment: 0,
        deleteLength: 0,
        insertText: 'x',
        expectedLocal: '',
      }),
    ),
    'segment-integrity',
    '段哈希过期拒绝',
  );

  assert.equal(codeOf(() => visualSlice(edited, textSeg)), 'segment-integrity', 'visualSlice 同样拒绝过期段');
  console.log('  [11] rendered edit rejection OK');
}

// ---------------------------------------------------------------------------
// 12. 渲染态直改越界与形状矩阵（0 装：坏调用一律抛）
// ---------------------------------------------------------------------------
{
  const source = 'hello world\n```\ncode\n```\n';
  const {segments} = compileDocument(source);
  const [textSeg] = segments; // 段长 12
  const at = (patch) => () =>
    mapRenderedEdit(source, segments, {
      segmentId: textSeg.id,
      offsetInSegment: 0,
      deleteLength: 0,
      insertText: '',
      expectedLocal: '',
      ...patch,
    });

  // 偏移类 → offset-out-of-range；长度类 → invalid-range。
  assert.equal(codeOf(at({offsetInSegment: 13, deleteLength: 0})), 'offset-out-of-range', '偏移越出段体');
  assert.equal(codeOf(at({offsetInSegment: 8, deleteLength: 5})), 'offset-out-of-range', '偏移+长度越出段体');
  assert.equal(codeOf(at({offsetInSegment: -1})), 'offset-out-of-range', '负偏移');
  assert.equal(codeOf(at({offsetInSegment: 1.5})), 'offset-out-of-range', '非整数偏移');
  assert.equal(codeOf(at({deleteLength: -1})), 'invalid-range', '负长度');
  assert.equal(codeOf(at({deleteLength: 2.5})), 'invalid-range', '非整数长度');
  // 段尾插入点（offset=段长、零删除）是合法边界。
  assert.equal(
    typeof at({offsetInSegment: 12, deleteLength: 0, insertText: '!', expectedLocal: ''})(),
    'string',
    '段尾插入点合法',
  );

  // 形状畸形 → invalid-edit；索引/真源畸形走各自家族码。
  assert.equal(codeOf(at({segmentId: 'seg-ghost'})), 'invalid-edit', '未知段 id');
  assert.equal(codeOf(at({segmentId: ''})), 'invalid-edit');
  assert.equal(codeOf(at({insertText: 7})), 'invalid-edit');
  assert.equal(codeOf(at({expectedLocal: null})), 'invalid-edit');
  assert.equal(codeOf(() => mapRenderedEdit(source, segments, null)), 'invalid-edit', '编辑非对象');
  assert.equal(
    codeOf(() =>
      mapRenderedEdit('abc', 'not-array', {segmentId: 'seg-0', offsetInSegment: 0, deleteLength: 0, insertText: '', expectedLocal: ''}),
    ),
    'invalid-segments',
    '索引非数组',
  );
  assert.equal(
    codeOf(() => mapRenderedEdit(null, segments, {segmentId: 'seg-0', offsetInSegment: 0, deleteLength: 0, insertText: '', expectedLocal: ''})),
    'invalid-source',
    '真源非字符串',
  );
  assert.equal(codeOf(() => visualSlice(null, textSeg)), 'invalid-source');
  assert.equal(codeOf(() => minimalSegmentEdit('a', 7)), 'invalid-edit', '归约输入非字符串');
  console.log('  [12] rendered edit boundary matrix OK');
}

// ---------------------------------------------------------------------------
// 13. 岛分段识别：island 词分岔开关 / 与普通围栏区分 / 铺满真源 / 未闭合
// ---------------------------------------------------------------------------
{
  const source =
    '前文\n```island js\nalert(1)\n```\n中间\n```\nplain\n```\n```island name=my-widget\n<b>岛</b>\n```\n尾\n';
  const {segments} = compileDocument(source);
  assert.deepEqual(
    segments.map((s) => s.kind),
    ['text', 'island', 'text', 'code', 'island', 'text'],
    'island 围栏识别为岛段，普通围栏仍 code',
  );
  assert.equal(segments.map((s) => s.content).join(''), source, '岛段同样铺满真源');

  // 分岔开关只有 island 词——既有 [1]/[2] kind 字面不动的依据。
  assert.equal(compileDocument('```js\nx\n```\n').segments[0].kind, 'code', '纯语言标注仍是 code');
  assert.equal(compileDocument('```\nx\n```\n').segments[0].kind, 'code', '空信息串仍是 code');
  assert.equal(compileDocument('```myisland\nx\n```\n').segments[0].kind, 'code', '词边界：myisland 不是 island');
  assert.equal(compileDocument('```js island\nx\n```\n').segments[0].kind, 'island', '信息串含 island 词即岛');
  assert.equal(compileDocument('```island name=my-widget\nx\n```\n').segments[0].kind, 'island', 'island name=… 即岛');

  // 未闭合岛围栏如实到文末（不猜闭合）。
  const unclosed = '```island\nabc';
  const u = compileDocument(unclosed).segments;
  assert.deepEqual(u.map((s) => s.kind), ['island']);
  assert.equal(u[0].end, unclosed.length);
  console.log('  [13] island segmentation OK');
}

// ---------------------------------------------------------------------------
// 14. stableIslandId：确定性 / 内容敏感 / 区间敏感 / 0 装
// ---------------------------------------------------------------------------
{
  const source = '```island js\nalert(1)\n```\n';
  const range = {start: 0, end: source.length};
  const id = stableIslandId(source, range);
  assert.match(id, /^isl-[0-9a-f]{8}$/, '格式 isl-<FNV-1a 8 位小写十六进制>');
  assert.equal(stableIslandId(source, range), id, '同源同区间恒同');

  // 跨编译稳定：两次编译由段区间派生出同一 ID。
  const segA = compileDocument(source).segments[0];
  const segB = compileDocument(source).segments[0];
  assert.equal(stableIslandId(source, {start: segA.start, end: segA.end}), id);
  assert.equal(stableIslandId(source, {start: segB.start, end: segB.end}), id, '跨编译恒同');

  // 内容敏感：改载荷换 ID。
  const edited = '```island js\nalert(2)\n```\n';
  assert.notEqual(stableIslandId(edited, {start: 0, end: edited.length}), id, '改内容换 ID');

  // 区间敏感：同内容不同位置换 ID（ID 承诺「这一处这一段内容」）。
  const moved = 'z\n```island js\nalert(1)\n```\n';
  const movedRange = {start: moved.indexOf('```'), end: moved.length};
  assert.notEqual(stableIslandId(moved, movedRange), id, '区间移位换 ID');

  // 0 装。
  assert.equal(codeOf(() => stableIslandId(7, range)), 'invalid-source');
  assert.equal(codeOf(() => stableIslandId(source, null)), 'invalid-range');
  assert.equal(codeOf(() => stableIslandId(source, {start: 1.5, end: 2})), 'invalid-range');
  assert.equal(codeOf(() => stableIslandId(source, {start: 3, end: 1})), 'invalid-range');
  assert.equal(codeOf(() => stableIslandId(source, {start: 0, end: 999})), 'invalid-range');
  console.log('  [14] stable island id OK');
}

// ---------------------------------------------------------------------------
// 15. 岛载荷提取 + IslandRegistry 生命周期（dispose 恰好一次 / 不漏放不吞错）
// ---------------------------------------------------------------------------
{
  // 载荷 = 两围栏行之间的原样正文（含行尾换行）；未闭合如实到段末。
  const closed = compileDocument('```island js\nalert(1)\n```\n').segments[0];
  assert.equal(islandPayload(closed), 'alert(1)\n');
  const emptyIsland = compileDocument('```island\n```\n').segments[0];
  assert.equal(islandPayload(emptyIsland), '', '空岛空载荷');
  const unclosedSeg = compileDocument('```island js\nalert(1)').segments[0];
  assert.equal(islandPayload(unclosedSeg), 'alert(1)', '未闭合到段末（不猜闭合）');

  // 0 装：非岛屿段 / 损坏段拒绝。
  const codeSeg = compileDocument('```\nx\n```\n').segments[0];
  assert.equal(codeOf(() => islandPayload(codeSeg)), 'invalid-island', '代码段不产载荷');
  assert.equal(codeOf(() => islandPayload(null)), 'invalid-island');
  const corrupt = {...closed, content: 'not a fence'};
  assert.equal(codeOf(() => islandPayload(corrupt)), 'invalid-island', '首行非岛屿围栏（损坏）拒绝');

  // 生命周期账本：恰好一次 / 二次抛错 / 重登记回路 / disposeAll。
  const registry = new IslandRegistry();
  const calls = [];
  registry.register('isl-a', () => calls.push('a'));
  registry.register('isl-b', () => calls.push('b'));
  assert.equal(registry.activeCount(), 2);
  assert.equal(codeOf(() => registry.register('isl-a', () => {})), 'duplicate-island', '在册未释放重复登记拒绝');
  registry.dispose('isl-a');
  assert.deepEqual(calls, ['a'], 'dispose 回调恰一次');
  assert.equal(registry.activeCount(), 1);
  assert.equal(codeOf(() => registry.dispose('isl-a')), 'island-disposed', '二次 dispose 抛错（恰好一次）');
  assert.deepEqual(calls, ['a'], '二次 dispose 不回调');

  registry.register('isl-a', () => calls.push('a2'));
  assert.equal(registry.activeCount(), 2, '释放后同 id 重登记（重渲染回路）合法');
  assert.equal(codeOf(() => registry.dispose('isl-ghost')), 'island-missing', '未登记 dispose 抛错');
  assert.equal(codeOf(() => registry.register('', () => {})), 'invalid-island', '空 id 拒绝');
  assert.equal(codeOf(() => registry.register('isl-x', null)), 'invalid-island', '非函数 dispose 拒绝');

  const released = registry.disposeAll();
  assert.equal(released, 2);
  assert.equal(registry.activeCount(), 0);
  assert.deepEqual(calls, ['a', 'a2', 'b'], 'disposeAll 按册序释放');
  assert.equal(codeOf(() => registry.dispose('isl-a')), 'island-disposed', 'disposeAll 后再 dispose 仍是二次');
  assert.equal(registry.disposeAll(), 0, '空转零释放');

  // 回调抛错：不漏放（后继照常）+ 不吞错（首错如实抛）+ 不二启。
  const r2 = new IslandRegistry();
  let boomCalls = 0;
  r2.register('boom', () => {
    boomCalls += 1;
    throw new Error('岛释放失败');
  });
  r2.register('ok', () => calls.push('ok'));
  assert.throws(() => r2.disposeAll(), /岛释放失败/, '首个错误如实抛出（不吞错）');
  assert.equal(r2.activeCount(), 0, '抛错也全数标记释放（不漏放）');
  assert.deepEqual(calls, ['a', 'a2', 'b', 'ok'], '后继岛照常释放');
  assert.equal(boomCalls, 1);
  assert.equal(codeOf(() => r2.dispose('boom')), 'island-disposed', '抛错回调也不二启');
  assert.equal(boomCalls, 1);
  console.log('  [15] island payload & registry OK');
}

console.log('--- Starport Doc Pipeline Check PASSED ---');
