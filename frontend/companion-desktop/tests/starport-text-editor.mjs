// 铭刻台文本面（text-editor-store.ts + text-editor-app.ts，真实模块）——
// 文本会话与编辑纯逻辑 + 内置件挂载/清理链的插口证明。
//
// 契约要点：
//   1. 坐标约定：行 1-based、列 0-based、区间 [start, end)；
//   2. replaceRange 的 expected 原文校验：不符抛 mismatch（防并发写坏）；
//   3. 0 装：越界/区间倒置/非字符串/空查找词/查无会话一律抛错；
//   4. findText 逐行非重叠命中表（行:列）；
//   5. cleanup：会话删除 + DOM 清空 + 幂等可重入，不留孤儿。
import assert from 'node:assert/strict';
import {
  createTextEditorStore,
  lineAt,
  lineCount,
  insertText,
  replaceRange,
  splitLine,
  joinLines,
  findText,
  TextEditorError,
} from '../src/lib/starport/apps/text-editor-store.ts';
import {
  createTextEditorApp,
  STARPORT_TEXT_APP_ID,
  STARPORT_TEXT_APP_TITLE,
} from '../src/lib/starport/apps/text-editor-app.ts';

console.log('--- Starting Starport Text Editor Check ---');

// ---------------------------------------------------------------------------
// 1. 会话 CRUD（快照副本、命名/内容校验、查无会话抛错）
// ---------------------------------------------------------------------------
{
  let clock = 1000;
  const store = createTextEditorStore({now: () => clock, idPrefix: 'draft-'});
  const a = store.create('草稿甲', '第一行\n第二行');
  assert.equal(a.id, 'draft-1', 'id 由注入前缀+序号生成');
  assert.equal(a.cursorLine, 1);
  assert.equal(a.updatedAt, 1000);

  clock = 2000;
  const b = store.create('草稿乙');
  assert.equal(b.id, 'draft-2');
  assert.equal(b.content, '', '内容缺省为空稿');
  assert.equal(store.list().length, 2);

  assert.throws(() => store.create(''), (error) => error instanceof TextEditorError && error.code === 'invalid-argument');
  assert.throws(() => store.create('x', 42), (error) => error instanceof TextEditorError && error.code === 'invalid-argument');
  assert.throws(() => store.get('draft-9'), (error) => error instanceof TextEditorError && error.code === 'missing-session');
  assert.throws(() => store.remove('draft-9'), (error) => error instanceof TextEditorError && error.code === 'missing-session');

  clock = 3000;
  const renamed = store.rename('draft-1', '草稿甲改');
  assert.equal(renamed.name, '草稿甲改');
  assert.equal(renamed.updatedAt, 3000, 'rename 刷新时间戳');
  assert.equal(store.get('draft-1').name, '草稿甲改');

  const updated = store.updateContent('draft-1', 'a\nb\nc', 3);
  assert.equal(updated.cursorLine, 3);
  assert.equal(store.get('draft-1').content, 'a\nb\nc');
  assert.throws(() => store.updateContent('draft-1', 'a', 9), (error) => error instanceof TextEditorError && error.code === 'out-of-bounds');
  // cursorLine 省略：clamp 到合法范围（编辑后回落，契约写明的唯一例外）。
  assert.equal(store.updateContent('draft-1', 'x').cursorLine, 1);

  // 快照副本：外部改返回值不污染容器。
  const copy = store.get('draft-1');
  copy.content = '篡改';
  assert.equal(store.get('draft-1').content, 'x', '返回快照，外部不可篡改容器');

  store.remove('draft-1');
  assert.equal(store.list().length, 1);
  console.log('  [1] session CRUD OK');
}

// ---------------------------------------------------------------------------
// 2. 文本度量与行操作（lineAt/lineCount/splitLine/joinLines + 越界抛错）
// ---------------------------------------------------------------------------
{
  const content = 'alpha\nbeta\ngamma';
  assert.equal(lineCount(content), 3);
  assert.equal(lineCount(''), 1, '空内容为 1 行');
  assert.equal(lineCount('a\n'), 2);
  assert.equal(lineAt(content, 1), 'alpha');
  assert.equal(lineAt(content, 3), 'gamma');
  assert.throws(() => lineAt(content, 0), (error) => error instanceof TextEditorError && error.code === 'out-of-bounds');
  assert.throws(() => lineAt(content, 4), (error) => error instanceof TextEditorError && error.code === 'out-of-bounds');
  assert.throws(() => lineCount(42), (error) => error instanceof TextEditorError && error.code === 'invalid-argument');

  // splitLine：行 2 列 2 处断开 bet|a。
  assert.equal(splitLine(content, 2, 2), 'alpha\nbe\nta\ngamma');
  assert.equal(splitLine(content, 1, 0), '\nalpha\nbeta\ngamma', '列 0 = 行首断');
  assert.throws(() => splitLine(content, 2, 99), (error) => error instanceof TextEditorError && error.code === 'out-of-bounds');
  assert.throws(() => splitLine(content, 9, 0), (error) => error instanceof TextEditorError && error.code === 'out-of-bounds');

  // joinLines：合并行 2 与行 3。
  assert.equal(joinLines(content, 2), 'alpha\nbetagamma');
  assert.equal(joinLines('a\nb', 1), 'ab');
  assert.throws(() => joinLines(content, 3), (error) => error instanceof TextEditorError && error.code === 'out-of-bounds', '末行无下一行可合并');
  assert.throws(() => joinLines('solo', 1), (error) => error instanceof TextEditorError && error.code === 'out-of-bounds');
  console.log('  [2] line operations OK');
}

// ---------------------------------------------------------------------------
// 3. insertText / replaceRange（坐标约定 + expected 原文校验 + 越界/倒置抛错）
// ---------------------------------------------------------------------------
{
  const content = 'alpha\nbeta\ngamma';
  assert.equal(insertText(content, {line: 1, column: 5}, '!'), 'alpha!\nbeta\ngamma');
  assert.equal(insertText(content, {line: 2, column: 0}, 'X\n'), 'alpha\nX\nbeta\ngamma', '插入可含换行');
  assert.equal(insertText(content, {line: 3, column: 5}, ''), content, '空串幂等无操作');
  assert.throws(() => insertText(content, {line: 1, column: 6}, 'x'), (error) => error instanceof TextEditorError && error.code === 'out-of-bounds');
  assert.throws(() => insertText(content, {line: 4, column: 0}, 'x'), (error) => error instanceof TextEditorError && error.code === 'out-of-bounds');
  assert.throws(() => insertText(content, {line: 1, column: 0}, 7), (error) => error instanceof TextEditorError && error.code === 'invalid-argument');

  // expected 校验通过：跨行替换 beta\ngam → BETA。
  const ok = replaceRange(
    content,
    {start: {line: 2, column: 0}, end: {line: 3, column: 3}},
    'BETA',
    'beta\ngam',
  );
  assert.equal(ok, 'alpha\nBETAma');

  // expected 不符：mismatch 拒写（宁拒不写，防并发写坏）。
  assert.throws(
    () => replaceRange(content, {start: {line: 1, column: 0}, end: {line: 1, column: 5}}, 'X', 'beta'),
    (error) => error instanceof TextEditorError && error.code === 'mismatch',
    '区间原文与 expected 不符必须拒绝',
  );

  // 空区间：expected 必须是空串。
  assert.equal(
    replaceRange(content, {start: {line: 1, column: 2}, end: {line: 1, column: 2}}, '|', ''),
    'al|pha\nbeta\ngamma',
  );
  assert.throws(
    () => replaceRange(content, {start: {line: 1, column: 2}, end: {line: 1, column: 2}}, '|', 'l'),
    (error) => error instanceof TextEditorError && error.code === 'mismatch',
    '空区间带非空 expected 必须拒绝',
  );

  // 区间倒置与越界。
  assert.throws(
    () => replaceRange(content, {start: {line: 3, column: 0}, end: {line: 1, column: 0}}, 'x', ''),
    (error) => error instanceof TextEditorError && error.code === 'invalid-argument',
    '区间倒置必须拒绝',
  );
  assert.throws(
    () => replaceRange(content, {start: {line: 1, column: 0}, end: {line: 9, column: 0}}, 'x', ''),
    (error) => error instanceof TextEditorError && error.code === 'out-of-bounds',
  );
  console.log('  [3] insert/replace with expected-check OK');
}

// ---------------------------------------------------------------------------
// 4. findText 命中表（行优先、非重叠、行:列坐标、空词抛错）
// ---------------------------------------------------------------------------
{
  const content = 'abcabc\nxx\nabc';
  assert.deepEqual(findText(content, 'abc'), [
    {line: 1, column: 0},
    {line: 1, column: 3},
    {line: 3, column: 0},
  ]);
  assert.deepEqual(findText(content, 'zz'), [], '无命中返回空表');
  assert.deepEqual(findText('aaaa', 'aa'), [
    {line: 1, column: 0},
    {line: 1, column: 2},
  ], '非重叠命中');
  assert.deepEqual(findText('a\nba', 'a'), [
    {line: 1, column: 0},
    {line: 2, column: 1},
  ]);
  assert.throws(() => findText(content, ''), (error) => error instanceof TextEditorError && error.code === 'invalid-argument');
  assert.throws(() => findText(content, 42), (error) => error instanceof TextEditorError && error.code === 'invalid-argument');
  console.log('  [4] findText hit table OK');
}

// ---------------------------------------------------------------------------
// 5. 内置件挂载/清理链（注入 dom 桩：三件套结构、查找栏行为、cleanup 幂等无孤儿）
// ---------------------------------------------------------------------------
{
  function makeEl(tag) {
    return {
      tag,
      className: '',
      textContent: '',
      value: '',
      style: {},
      children: [],
      parent: null,
      listeners: {},
      appendChild(node) {
        this.children.push(node);
        node.parent = this;
        return node;
      },
      remove() {
        if (this.parent) {
          this.parent.children = this.parent.children.filter((child) => child !== this);
          this.parent = null;
        }
      },
      addEventListener(type, handler) {
        (this.listeners[type] ??= []).push(handler);
      },
      dispatch(type) {
        for (const handler of this.listeners[type] ?? []) handler({target: this});
      },
    };
  }
  const fakeDom = {createElement: (tag) => makeEl(tag)};
  const container = makeEl('div');
  const store = createTextEditorStore({idPrefix: 'app-'});

  const app = createTextEditorApp({dom: fakeDom, store});
  assert.equal(app.id, STARPORT_TEXT_APP_ID);
  assert.equal(app.title, STARPORT_TEXT_APP_TITLE);
  assert.equal(app.kind, 'internal');

  const cleanup = app.mount(container, {viewId: 'view:7', appId: app.id, close: () => {}});
  assert.equal(typeof cleanup, 'function', 'mount 必须返回清理函数');
  assert.equal(container.children.length, 1, '容器恰好一个包装节点');
  const wrapper = container.children[0];
  assert.equal(wrapper.className, 'starport-text-editor');
  const [gutter, area, findBar] = wrapper.children;
  assert.equal(gutter.className, 'starport-text-gutter');
  assert.equal(area.className, 'starport-text-area');
  assert.equal(findBar.className, 'starport-text-find');
  assert.equal(gutter.textContent, '1', '初始行号栏');
  assert.equal(store.list().length, 1, '挂载即建会话');
  assert.equal(store.list()[0].name, '铭刻台-view:7');

  // 编辑 → 行号刷新 + 会话同步。
  area.value = 'one\ntwo';
  area.dispatch('input');
  assert.equal(gutter.textContent, '1\n2', '编辑后行号栏刷新');
  assert.equal(store.list()[0].content, 'one\ntwo', '编辑同步进会话');

  // 查找栏：命中表按 行:列 外显；空词失败即帧。
  const findInput = findBar.children[0];
  const findButton = findBar.children[1];
  const findResult = findBar.children[2];
  findInput.value = 'o';
  findButton.dispatch('click');
  assert.match(findResult.textContent, /命中 2/, '命中计数外显（one/two 各一）');
  assert.match(findResult.textContent, /1:0/, '命中坐标 行:列');
  assert.match(findResult.textContent, /2:2/, '命中坐标 行:列');
  findInput.value = '';
  findButton.dispatch('click');
  assert.match(findResult.textContent, /查找失败/, '空查找词失败即帧（不吞错）');

  // cleanup：会话删 + DOM 清 + 幂等可重入。
  cleanup();
  assert.equal(store.list().length, 0, 'cleanup 不留孤儿会话');
  assert.equal(container.children.length, 0, 'cleanup 不留孤儿 DOM');
  assert.equal(container.textContent, '', '容器清空');
  cleanup();
  assert.equal(container.children.length, 0, 'cleanup 幂等可重入');

  // 无 dom 且无全局 document：抛错不猜（Node 环境天然验证）。
  const bareApp = createTextEditorApp({store});
  assert.throws(
    () => bareApp.mount(makeEl('div'), {viewId: 'v', appId: bareApp.id, close: () => {}}),
    (error) => error instanceof TextEditorError && error.code === 'invalid-argument',
    '无 DOM 环境必须抛错',
  );
  console.log('  [5] app mount/cleanup chain OK');
}

console.log('--- Starport Text Editor Check PASSED ---');
