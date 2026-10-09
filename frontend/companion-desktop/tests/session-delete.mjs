// 会话删除链纯逻辑单测（真实模块 src/lib/chat-shell/session-delete.ts +
// session-list.ts 归并 + runtime.ts 后端真删契约；Node 类型擦除直 import）。
//
// 病灶（内测实测）：确认删除 → 列表不消失（"像没删"）→ 重启又复活。根因 =
// 删除只动本地账，后端账本行经归并立刻补回来。本套件盯死链上三段：
//   ① 前端乐观移除（确认后立即消失）② 后端真删（重启不复活）③ 失败即回滚 + 亮帧。
import assert from 'node:assert/strict';

// runtime.ts 的持久化助手读 localStorage：先摆好假件再 import。
const storage = new Map();
globalThis.localStorage = {
  getItem: (key) => storage.get(key) ?? null,
  setItem: (key, value) => storage.set(key, String(value)),
  removeItem: (key) => storage.delete(key),
};

const {deleteSession} = await import('../src/lib/chat-shell/session-delete.ts');
const {mergeSessionLedger} = await import('../src/lib/chat-shell/session-list.ts');
const runtime = await import('../src/lib/runtime.ts');

function conv(id) {
  return {
    id,
    title: id,
    createdAt: 1,
    updatedAt: 2,
    messages: [{id: `${id}-m`, role: 'user', text: `hello ${id}`}],
    scope: 'global',
  };
}

/** 删除链假件：记录每次调用，persist 即"落盘"（persisted 模拟重启读回的账本）。 */
function makeHarness(list, opts = {}) {
  const calls = {setList: 0, persist: 0, remote: [], errors: []};
  const persisted = [];
  let current = list;
  let failPersist = opts.failPersistTimes ?? 0;
  const harness = {
    calls,
    persisted,
    get current() {
      return current;
    },
    ports: {
      list: () => current,
      setList: (next) => {
        current = next;
        calls.setList += 1;
      },
      persist: () => {
        calls.persist += 1;
        if (failPersist > 0) {
          failPersist -= 1;
          throw new Error('persist failed');
        }
        persisted.push(current);
      },
      deleteRemote: async (id) => {
        calls.remote.push(id);
        if (opts.remoteInFlight) opts.remoteInFlight(id, () => current);
        if (opts.failRemote) throw new Error('HTTP 500 删除失败');
      },
      onError: (caught) => calls.errors.push(caught),
    },
  };
  return harness;
}

// ---- ① 确认后立即消失 + 持久 + 后端真删 + 重启不复活 ----
{
  const h = makeHarness([conv('a'), conv('b')]);
  let duringRemote = null;
  h.ports.deleteRemote = async (id) => {
    h.calls.remote.push(id);
    duringRemote = h.current.map((c) => c.id); // 远端在途时列表必须已经移除
  };
  const outcome = await deleteSession('a', h.ports);
  assert.equal(outcome.ok, true, '真删成功');
  assert.equal(outcome.rolledBack, false);
  assert.deepEqual(duringRemote, ['b'], '确认后立即从列表消失（乐观移除不等网络）');
  assert.deepEqual(h.calls.remote, ['a'], '后端删除端点被真的调用（真删，不是只动本地）');
  assert.deepEqual(h.current.map((c) => c.id), ['b']);
  assert.deepEqual(h.persisted.at(-1).map((c) => c.id), ['b'], '删除已持久化');
  // "重启" = 从持久账本读回
  assert.ok(!h.persisted.at(-1).some((c) => c.id === 'a'), '重启后不复活（持久账本里已真删）');
  console.log('  ok  确认即消 + 持久 + 后端真删 + 重启不复活');
}

// ---- ② 后端陈账行不残留（归并侧的"删了又回来"病灶复现 + 修复）----
{
  const local = [conv('b')];
  const backend = [{id: 'a', title: 'A', started_at: 1, last_active_at: 2, episode_count: 3}];
  const control = mergeSessionLedger({local, backend});
  assert.ok(
    control.some((item) => item.id === 'a'),
    '控制组：后端陈账行默认会被归并回来（这就是"像没删"的病灶）',
  );
  const excluded = mergeSessionLedger({local, backend, exclude: new Set(['a'])});
  assert.ok(!excluded.some((item) => item.id === 'a'), '已确认删除的 id 被乐观排除，陈账行不残留');
  console.log('  ok  后端陈账行不残留（乐观排除 + 病灶控制组复现）');
}

// ---- ③ 后端真删失败 → 列表回滚 + 立即亮错误帧 ----
{
  const h = makeHarness([conv('a'), conv('b')], {failRemote: true});
  const outcome = await deleteSession('a', h.ports);
  assert.equal(outcome.ok, false);
  assert.equal(outcome.rolledBack, true, '失败即回滚');
  assert.deepEqual(h.current.map((c) => c.id), ['a', 'b'], '列表还原到删除前');
  assert.deepEqual(h.persisted.at(-1).map((c) => c.id), ['a', 'b'], '回滚也落盘（会话不丢）');
  assert.equal(h.calls.errors.length, 1, '失败即亮帧（错误原样交出，不吞不粉饰）');
  assert.match(String(h.calls.errors[0].message), /500/, '帧里是真实失败原因');
  console.log('  ok  后端真删失败 → 列表回滚 + 错误帧');
}

// ---- ④ 本地持久失败 → 回滚且不再惊动后端 ----
{
  const h = makeHarness([conv('a')], {failPersistTimes: 1});
  const outcome = await deleteSession('a', h.ports);
  assert.equal(outcome.ok, false);
  assert.equal(outcome.rolledBack, true);
  assert.deepEqual(h.calls.remote, [], '本地都没删成，不谎报后端真删');
  assert.equal(h.calls.errors.length, 1);
  assert.deepEqual(h.current.map((c) => c.id), ['a'], '列表已回滚');
  console.log('  ok  持久失败 → 回滚 + 亮帧 + 不惊动后端');
}

// ---- ⑤ 后端删除端点契约（真实模块 runtime.deleteBackendSession）----
{
  const cfg = {baseUrl: 'http://gateway.test', apiKey: ''};
  const realFetch = globalThis.fetch;
  const respond = (status, body) =>
    new Response(body === undefined ? '' : JSON.stringify(body), {status});

  const call = async (status, body) => {
    globalThis.fetch = async () => respond(status, body);
    try {
      await runtime.deleteBackendSession(cfg, 'sess-1');
      return 'resolved';
    } catch (err) {
      return `rejected:${err.status ?? 'net'}`;
    } finally {
      globalThis.fetch = realFetch;
    }
  };

  assert.equal(await call(200, {deleted: true}), 'resolved', '200 = 真删成功');
  assert.equal(
    await call(404, {error: {message: 'gone', code: 'session_not_found', solution: 'x'}}),
    'resolved',
    '404 session_not_found = 本机草稿没有后端记录，算成功',
  );
  assert.equal(
    await call(400, {error: {message: 'bad id', code: 'invalid_request', solution: 'x'}}),
    'resolved',
    '400 invalid_request = 该 id 不可能是后端会话 id，无记录可删',
  );
  assert.equal(await call(404, ''), 'rejected:404', '裸 404（路由不存在）必须失败——不许假删');
  assert.equal(
    await call(500, {error: {message: 'boom', code: 'internal', solution: 'x'}}),
    'rejected:500',
    '后端 5xx 必须失败（调用方回滚）',
  );
  console.log('  ok  后端删除端点契约（200 / 404 不存在 / 400 非法 id / 裸 404 与 5xx 失败）');
}

console.log('session delete chain: all assertions passed');
