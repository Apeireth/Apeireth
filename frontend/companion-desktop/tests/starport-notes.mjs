// 星屑便签（notes-store.ts + notes-app.ts，真实模块）——CRUD/排序/校验/持久化/清理的插口证明。
//
// 契约要点：
//   1. CRUD 往返 + 缺失即抛（missing）；置顶切换不动 updatedAt（独立坐标轴）；
//   2. 排序即承诺：置顶优先 → updatedAt 倒序 → createdAt 倒序 → id 升序；
//   3. 校验 0 装：空内容（含纯空白）/超限（title 200 / body 20000）/id 冲突
//      一律抛错，不猜不修不截断；
//   4. 持久化适配器往返：每次变更 save、新实例 load 还原、null=新起步、损坏即抛；
//   5. 挂载/清理：假 DOM 直测（Node 无宿主 DOM，容器走同构 ownerDocument 契约）——
//      cleanup 零残留（DOM/监听清空），持久化数据不灭（视图死，数据活），双清理安全。
import assert from 'node:assert/strict';
import {
  NotesStore,
  NotesStoreError,
  compareNotes,
  parseNoteList,
  NOTE_TITLE_MAX,
  NOTE_BODY_MAX,
} from '../src/lib/starport/apps/notes-store.ts';
import {
  createNotesApp,
  STARPORT_NOTES_APP_ID,
} from '../src/lib/starport/apps/notes-app.ts';

console.log('--- Starting Starport Notes Check ---');

// ---------------------------------------------------------------------------
// 0. 假 DOM（Node 无宿主 DOM）：只实现 notes-app 用到的 DOM 面
// ---------------------------------------------------------------------------
function makeFakeDocument() {
  const listeners = [];
  function createElement(tag) {
    const node = {
      tag,
      className: '',
      textContent: '',
      value: '',
      hidden: false,
      type: '',
      placeholder: '',
      children: [],
      parent: null,
      get firstChild() {
        return this.children.length ? this.children[0] : null;
      },
      appendChild(child) {
        child.parent = this;
        this.children.push(child);
        return child;
      },
      removeChild(child) {
        const index = this.children.indexOf(child);
        if (index >= 0) this.children.splice(index, 1);
        child.parent = null;
        return child;
      },
      addEventListener(type, listener) {
        listeners.push({node: this, type, listener});
      },
      removeEventListener(type, listener) {
        const index = listeners.findIndex(
          (entry) => entry.node === this && entry.type === type && entry.listener === listener,
        );
        if (index >= 0) listeners.splice(index, 1);
      },
      remove() {
        if (this.parent) this.parent.removeChild(this);
      },
    };
    return node;
  }
  return {createElement, listeners};
}

function findByClass(node, className) {
  if (node.className && node.className.split(' ').includes(className)) return node;
  for (const child of node.children) {
    const hit = findByClass(child, className);
    if (hit) return hit;
  }
  return null;
}

function findAllByClass(node, className, out) {
  if (node.className && node.className.split(' ').includes(className)) out.push(node);
  for (const child of node.children) findAllByClass(child, className, out);
  return out;
}

function click(doc, node) {
  for (const entry of [...doc.listeners]) {
    if (entry.node === node && entry.type === 'click') entry.listener();
  }
}

function makeAdapter() {
  const saved = [];
  return {
    saved,
    adapter: {
      load: () => (saved.length ? saved[saved.length - 1] : null),
      save: (notes) => saved.push(notes.map((n) => ({...n}))),
    },
  };
}

// ---------------------------------------------------------------------------
// 1. CRUD：新建/更新/删除/置顶 + 缺失即抛
// ---------------------------------------------------------------------------
{
  const store = new NotesStore();
  const added = store.add({id: 'n1', title: '标题', body: '内容'}, 1000);
  assert.deepEqual(added, {
    id: 'n1',
    title: '标题',
    body: '内容',
    createdAt: 1000,
    updatedAt: 1000,
    pinned: false,
  });

  const updated = store.update('n1', {body: '内容二'}, 2000);
  assert.equal(updated.title, '标题', '未传字段不动');
  assert.equal(updated.body, '内容二');
  assert.equal(updated.updatedAt, 2000);
  assert.equal(updated.createdAt, 1000, 'createdAt 不动');

  const pinned = store.togglePin('n1');
  assert.equal(pinned.pinned, true);
  assert.equal(pinned.updatedAt, 2000, '置顶切换不动 updatedAt（独立坐标轴）');
  assert.equal(store.togglePin('n1').pinned, false);

  const removed = store.remove('n1');
  assert.equal(removed.id, 'n1');
  assert.deepEqual(store.list(), [], '删后为空');

  for (const call of [
    () => store.update('n1', {title: 'x'}, 1),
    () => store.remove('n1'),
    () => store.togglePin('n1'),
  ]) {
    assert.throws(call, (error) => error instanceof NotesStoreError && error.code === 'missing');
  }
  assert.throws(
    () => store.add({id: '', title: 'x', body: 'y'}, 1),
    (error) => error instanceof NotesStoreError && error.code === 'invalid-id',
    '空 id 必须拒绝（invalid-id，非 missing）',
  );
  console.log('  [1] CRUD + missing rejection OK');
}

// ---------------------------------------------------------------------------
// 2. 排序即承诺：置顶优先 → updatedAt 倒序 → createdAt 倒序 → id 升序
// ---------------------------------------------------------------------------
{
  const store = new NotesStore();
  store.add({id: 'n1', title: 'a', body: 'x'}, 7); // createdAt 7 / updatedAt 7
  store.update('n1', {body: 'x2'}, 3000); // updatedAt 3000 / createdAt 7
  store.add({id: 'n2', title: 'b', body: 'x'}, 7);
  store.update('n2', {body: 'x2'}, 3000); // updatedAt 3000 / createdAt 7（与 n1 同刻，id 裁决）
  store.add({id: 'n3', title: 'c', body: 'x'}, 1);
  store.update('n3', {body: 'x2'}, 5000);
  store.togglePin('n3'); // 置顶
  store.add({id: 'n4', title: 'd', body: 'x'}, 7);
  store.update('n4', {body: 'x2'}, 3000); // updatedAt 3000 / createdAt 7
  store.add({id: 'n5', title: 'e', body: 'x'}, 1);
  store.update('n5', {body: 'x2'}, 4000);

  assert.deepEqual(
    store.list().map((n) => n.id),
    ['n3', 'n5', 'n1', 'n2', 'n4'],
    '全序：置顶 → updatedAt 倒序 → createdAt 倒序 → id 升序',
  );

  // compareNotes 纯函数与 list 同一全序（钉死不许漂）。
  const a = {id: 'x', title: '', body: '', createdAt: 1, updatedAt: 1, pinned: true};
  const b = {id: 'y', title: '', body: '', createdAt: 9, updatedAt: 9, pinned: false};
  assert.ok(compareNotes(a, b) < 0, '置顶恒在前');
  console.log('  [2] committed ordering OK');
}

// ---------------------------------------------------------------------------
// 3. 校验 0 装：空内容/超限/id 冲突一律抛，边界值放行
// ---------------------------------------------------------------------------
{
  const store = new NotesStore();
  const emptyCases = [
    {id: 'e1', title: '', body: ''},
    {id: 'e2', title: '   ', body: '\t\n'},
    {id: 'e3', title: '', body: undefined},
    {id: 'e4', title: undefined, body: ''},
  ];
  for (const input of emptyCases) {
    assert.throws(
      () => store.add(input, 1),
      (error) => error instanceof NotesStoreError && error.code === 'empty-content',
      `空内容必须拒绝：${input.id}`,
    );
  }

  assert.throws(
    () => store.add({id: 't1', title: 'x'.repeat(NOTE_TITLE_MAX + 1), body: 'y'}, 1),
    (error) => error instanceof NotesStoreError && error.code === 'limit-exceeded',
    'title 超限必须拒绝',
  );
  assert.throws(
    () => store.add({id: 't2', title: 'x', body: 'y'.repeat(NOTE_BODY_MAX + 1)}, 1),
    (error) => error instanceof NotesStoreError && error.code === 'limit-exceeded',
    'body 超限必须拒绝',
  );
  // 边界值放行（恰好等于上限合法）。
  const edge = store.add(
    {id: 't3', title: 'x'.repeat(NOTE_TITLE_MAX), body: 'y'.repeat(NOTE_BODY_MAX)},
    1,
  );
  assert.equal(edge.title.length, NOTE_TITLE_MAX);

  assert.throws(
    () => store.add({id: 't3', title: '撞名', body: 'z'}, 2),
    (error) => error instanceof NotesStoreError && error.code === 'duplicate-id',
    'id 冲突必须拒绝',
  );
  assert.throws(
    () => store.update('t3', {title: '', body: ''}, 3),
    (error) => error instanceof NotesStoreError && error.code === 'empty-content',
    '更新后整体重新校验',
  );
  assert.throws(
    () => store.update('t3', {body: 'y'.repeat(NOTE_BODY_MAX + 1)}, 3),
    (error) => error instanceof NotesStoreError && error.code === 'limit-exceeded',
  );
  console.log('  [3] strict validation OK');
}

// ---------------------------------------------------------------------------
// 4. 持久化适配器往返：变更即 save、新实例还原、null=新起步、损坏即抛
// ---------------------------------------------------------------------------
{
  const {saved, adapter} = makeAdapter();
  const store = new NotesStore({adapter});
  assert.deepEqual(store.list(), [], 'load() 返回 null = 全新起步');

  store.add({id: 'p1', title: '持久', body: '化'}, 100);
  assert.equal(saved.length, 1, 'add 触发 save');
  store.update('p1', {body: '化二'}, 200);
  assert.equal(saved.length, 2, 'update 触发 save');
  store.togglePin('p1');
  assert.equal(saved.length, 3, 'togglePin 触发 save');

  const restored = new NotesStore({adapter});
  assert.deepEqual(restored.list(), saved[2], '新实例按最后保存还原');
  restored.remove('p1');
  assert.equal(saved.length, 4);
  assert.deepEqual(new NotesStore({adapter}).list(), [], '删除也持久化');

  // 损坏载荷一律抛（0 装：不半信半疑）。
  const corruptCases = [
    {},
    'str',
    [{id: 'x'}],
    [{id: 'x', title: 't', body: 'b', createdAt: 1, updatedAt: 1, pinned: 'yes'}],
    [
      {id: 'dup', title: 't', body: 'b', createdAt: 1, updatedAt: 1, pinned: false},
      {id: 'dup', title: 't2', body: 'b2', createdAt: 2, updatedAt: 2, pinned: false},
    ],
  ];
  for (const payload of corruptCases) {
    assert.throws(
      () => parseNoteList(payload),
      (error) =>
        error instanceof NotesStoreError &&
        (error.code === 'corrupt-persistence' || error.code === 'duplicate-id'),
      '损坏载荷必须拒绝',
    );
  }
  console.log('  [4] persistence round-trip OK');
}

// ---------------------------------------------------------------------------
// 5. 应用挂载/清理：假 DOM 全链路 + cleanup 零残留 + 数据不灭 + 双清理安全
// ---------------------------------------------------------------------------
{
  const {saved, adapter} = makeAdapter();
  let clock = 1000;
  const app = createNotesApp({adapter, now: () => clock});
  assert.equal(app.id, STARPORT_NOTES_APP_ID, '贡献点 id 即承诺');
  assert.equal(app.id, 'starport-notes');
  assert.equal(app.title, '星屑');
  assert.equal(app.kind, 'internal');
  assert.equal(typeof app.mount, 'function');

  const doc = makeFakeDocument();
  const container = doc.createElement('div');
  container.ownerDocument = doc;
  const context = {viewId: 'view:1', appId: app.id, close: () => {}};

  const cleanup = app.mount(container, context);
  assert.equal(typeof cleanup, 'function', 'mount 必须返回清理函数');
  const root = findByClass(container, 'stardust');
  assert.ok(root, '挂载必须造出根节点');
  assert.ok(findByClass(container, 'stardust-empty'), '空态外显「暂无便签」');

  // —— UI 新建：填表 → 存 → 上屏 + 持久化 ——
  const newBtn = findAllByClass(container, 'stardust-btn', []).find((b) => b.textContent === '新建');
  click(doc, newBtn);
  const editor = findByClass(container, 'stardust-editor');
  assert.equal(editor.hidden, false, '点新建展开编辑器');
  const titleInput = findByClass(container, 'stardust-input');
  const bodyInput = findByClass(container, 'stardust-area');
  titleInput.value = '第一条';
  bodyInput.value = '正文';
  const saveBtn = findAllByClass(container, 'stardust-btn', []).find((b) => b.textContent === '存');
  click(doc, saveBtn);
  assert.equal(editor.hidden, true, '存后收起编辑器');
  assert.equal(saved.length, 1);
  assert.equal(saved[0][0].id, 'note-1000-1', 'id 由应用生成（时钟+序号，可复算）');
  assert.equal(saved[0][0].createdAt, 1000);
  const titles = findAllByClass(container, 'stardust-note-title', []).map((n) => n.textContent);
  assert.deepEqual(titles, ['第一条'], '新便签上屏');

  // —— UI 置顶（拨钟 2000 后置顶：updatedAt 不动，钉住「置顶不改时间」） ——
  clock = 2000;
  const card = findByClass(container, 'stardust-note');
  const pinBtn = findAllByClass(card, 'stardust-btn', []).find((b) => b.textContent === '置顶');
  click(doc, pinBtn);
  assert.equal(saved[1][0].pinned, true, '置顶落账');
  assert.equal(saved[1][0].updatedAt, 1000, '置顶不动 updatedAt');
  const pinnedCard = findAllByClass(container, 'stardust-note', []).find((n) =>
    n.className.split(' ').includes('pinned'),
  );
  assert.ok(pinnedCard, '置顶样式外显');

  // —— UI 编辑：回填 → 改 → 存 ——
  const editBtn = findAllByClass(container, 'stardust-btn', []).find((b) => b.textContent === '编辑');
  click(doc, editBtn);
  assert.equal(editor.hidden, false);
  assert.equal(titleInput.value, '第一条', '编辑回填标题');
  assert.equal(bodyInput.value, '正文', '编辑回填内容');
  titleInput.value = '第一条·改';
  clock = 3000;
  click(doc, saveBtn);
  assert.equal(saved[2][0].title, '第一条·改');
  assert.equal(saved[2][0].updatedAt, 3000);

  // —— 校验错误原文外显（0 装：不吞不粉饰） ——
  click(doc, newBtn);
  titleInput.value = '';
  bodyInput.value = '';
  click(doc, saveBtn);
  assert.equal(editor.hidden, false, '校验失败编辑器不收');
  const errorLine = findByClass(container, 'stardust-error');
  assert.match(errorLine.textContent, /便签拒绝执行/, '错误原文外显');

  // —— UI 删除 ——
  const deleteBtn = findAllByClass(container, 'stardust-btn', []).find((b) => b.textContent === '删除');
  click(doc, deleteBtn);
  assert.equal(saved[3].length, 0, '删除落账');
  assert.ok(findByClass(container, 'stardust-empty'), '删后回空态');

  // —— 多视图共享账本（应用实例级存储，契约 §4） ——
  const container2 = doc.createElement('div');
  container2.ownerDocument = doc;
  const cleanup2 = app.mount(container2, context);
  assert.ok(findByClass(container2, 'stardust-empty'), '共享账本：第二视图见同一份数据（现为空）');

  // —— cleanup 零残留：DOM/监听清空，持久化数据不灭（视图死，数据活） ——
  cleanup();
  assert.equal(container.children.length, 0, 'cleanup 后根节点不残留');
  cleanup();
  assert.equal(container.children.length, 0, '双清理安全（幂等）');
  cleanup2();
  assert.equal(container2.children.length, 0);
  assert.equal(doc.listeners.length, 0, 'cleanup 后零监听残留');
  assert.ok(saved.length >= 4, '持久化数据不灭（视图死，数据活）');
  console.log('  [5] app mount/cleanup lifecycle OK');
}

console.log('--- Starport Notes Check PASSED ---');
