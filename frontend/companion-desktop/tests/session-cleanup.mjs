// 会话清理双档纯逻辑单测（真实模块 src/lib/chat-shell/session-cleanup.ts +
// session-list.ts 归并 + settings-live-apply.ts 确认登记表；Node 类型擦除直 import）。
//
// 依据: docs/01-architecture/session-cleanup-options-spec.md。
// 盯死四件套 + 三条红线：
//   ① 复活病灶（只清本地 → 后端陈账行补回来）必须被真删链消灭；
//   ② 近期窗口过滤边界（含"无从判定的时间戳不动手"保守规则）；
//   ③ 部分失败聚合（已成保持删除、失败项还原保留、不谎报清空）；
//   ④ 后端"本就没有这条"= 成功（约定在 runtime.deleteBackendSession，链上认 resolve）。
// 红线：A 两档都不碰记忆（本套件无任何记忆端口 = 机器可检）；
//       B 命名不冒充"记忆"；C 文案必须逐句写明清什么留什么（含"长期记忆不受影响"句）。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));

const {
  purgeSessions,
  withinRecentWindow,
  markSessionsCleared,
  isSessionCleared,
  resetClearedSessions,
  RECENT_WINDOW_DAYS_DEFAULT,
} = await import('../src/lib/chat-shell/session-cleanup.ts');
const {mergeSessionLedger} = await import('../src/lib/chat-shell/session-list.ts');
const {DANGER_ACTION_CONFIRMATIONS} = await import('../src/lib/settings-live-apply.ts');

const NOW = 1_800_000_000_000; // 固定"现在"，窗口测试全用它
const DAY = 86_400_000;

function conv(id, lastActive) {
  return {
    id,
    title: id,
    createdAt: lastActive,
    updatedAt: lastActive,
    messages: [{id: `${id}-m`, role: 'user', text: `hello ${id}`}],
    scope: 'global',
  };
}

function backendRow(id, lastActive) {
  return {id, title: id, started_at: lastActive, last_active_at: lastActive, episode_count: 1};
}

/** 清理链假件：deleteRemote 可按 id 拒绝；persist 可失败；记录全部调用。 */
function makeHarness({local = [], backend = [], failPersistTimes = 0, failRemoteIds = []} = {}) {
  const calls = {setList: 0, persist: 0, remote: [], callLogs: [], errors: []};
  let current = local;
  let failPersist = failPersistTimes;
  const failed = new Set(failRemoteIds);
  return {
    calls,
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
      },
      backend: () => backend,
      deleteRemote: async (id) => {
        calls.remote.push(id);
        if (failed.has(id)) throw new Error(`HTTP 500 删除失败 ${id}`);
      },
      clearCallLogs: (target) => calls.callLogs.push(target),
      onError: (caught, failedIds) => calls.errors.push({caught, failedIds}),
    },
  };
}

// ---- ① 复活病灶：双档都必须打到后端真删，陈账行不再补回 ----
{
  const local = [conv('a', NOW - 1 * DAY), conv('b', NOW - 20 * DAY)];
  const backend = [backendRow('c', NOW - 1 * DAY), backendRow('d', NOW - 20 * DAY)];
  const h = makeHarness({local, backend});
  const outcome = await purgeSessions('all', h.ports, {nowMs: NOW});
  assert.equal(outcome.ok, true, '全量清理成功');
  assert.deepEqual([...h.calls.remote].sort(), ['a', 'b', 'c', 'd'], '本地 ∪ 后端账本逐条真删（不是只动本地）');
  assert.deepEqual(h.current.map((c) => c.id), [], '本地列表清空');
  // 归并控制组：后端陈账行 + 乐观排除 = 不复活（病灶复现在 session-delete.mjs，这里盯双档）
  const merged = mergeSessionLedger({local: [], backend, exclude: new Set(outcome.deletedIds)});
  assert.deepEqual(merged.map((item) => item.id), [], '真删 + 乐观排除后，陈账行不复活');
  console.log('  ok  ① 复活病灶：双档真删后端账本行，陈账行不复活');
}

// ---- ② 近期窗口过滤边界：窗内删、窗外留、无从判定的不动手 ----
{
  assert.equal(withinRecentWindow(NOW - 7 * DAY, NOW, 7), true, '窗口左边界（=cutoff）算窗内');
  assert.equal(withinRecentWindow(NOW - 7 * DAY - 1, NOW, 7), false, '刚出窗口即窗外');
  assert.equal(withinRecentWindow(0, NOW, 7), false, '时间戳为 0（旧数据无从判定）= 不在窗口（保守不动手）');
  assert.equal(withinRecentWindow(Number.NaN, NOW, 7), false, '非法时间戳同保守规则');

  const local = [conv('in1', NOW - 1 * DAY), conv('out1', NOW - 30 * DAY), conv('undatable', 0)];
  const backend = [backendRow('in2', NOW - 6 * DAY), backendRow('out2', NOW - 30 * DAY)];
  const h = makeHarness({local, backend});
  const outcome = await purgeSessions('recent', h.ports, {nowMs: NOW, windowDays: 7});
  assert.equal(outcome.ok, true);
  assert.deepEqual([...outcome.deletedIds].sort(), ['in1', 'in2'], '① 档只删窗口内（本地+后端一致）');
  assert.deepEqual(
    h.current.map((c) => c.id).sort(),
    ['out1', 'undatable'],
    '窗外与无从判定的会话原样保留',
  );
  const h2 = makeHarness({local, backend});
  const all = await purgeSessions('all', h2.ports, {nowMs: NOW});
  assert.deepEqual([...all.deletedIds].sort(), ['in1', 'in2', 'out1', 'out2', 'undatable'], '② 档全量（含无从判定）');
  console.log('  ok  ② 窗口边界：窗内删/窗外留/无从判定不动手；all 档全量');
}

// ---- ③ 部分失败聚合：已成保持删除、失败项还原保留、不谎报清空 ----
{
  const local = [conv('ok1', NOW - 1 * DAY), conv('bad1', NOW - 1 * DAY)];
  const backend = [backendRow('ok1', NOW - 1 * DAY), backendRow('bad1', NOW - 1 * DAY)];
  const h = makeHarness({local, backend, failRemoteIds: ['bad1']});
  const outcome = await purgeSessions('recent', h.ports, {nowMs: NOW});
  assert.equal(outcome.ok, false, '部分失败绝不算 ok（不谎报"已清空"）');
  assert.deepEqual(outcome.deletedIds, ['ok1']);
  assert.deepEqual(outcome.failedIds, ['bad1'], '失败 id 如实列出');
  assert.deepEqual(h.current.map((c) => c.id), ['bad1'], '失败项本地还原保留（删除不算数）');
  assert.equal(h.calls.errors.length, 1, '失败即亮帧');
  assert.match(String(h.calls.errors[0].caught.message), /bad1/, '帧里列出失败会话 id');
  assert.deepEqual(h.calls.callLogs, [new Set(['ok1'])], '调用日志只清真删成的会话（失败项不动）');
  // 重试可行：失败项仍在列表里（不是消失后假装修复）
  const retry = makeHarness({local: h.current, backend: []});
  const again = await purgeSessions('recent', retry.ports, {nowMs: NOW});
  assert.equal(again.ok, true, '失败项可重试直至真删');
  console.log('  ok  ③ 部分失败：已成保持删除 + 失败项还原 + 亮帧列 id + 可重试');
}

// ---- ④ 空转/回滚/账本缺失三种收场 ----
{
  // 空转：没有窗内会话 = ok 且不惊动后端
  const empty = makeHarness({local: [conv('out', NOW - 30 * DAY)], backend: []});
  const none = await purgeSessions('recent', empty.ports, {nowMs: NOW});
  assert.equal(none.ok, true);
  assert.deepEqual(empty.calls.remote, [], '无目标不惊动后端');
  assert.deepEqual(empty.calls.callLogs, [], '无目标不动调用日志');

  // 本地持久失败 = 整体回滚 + 不惊动后端
  const h = makeHarness({local: [conv('a', NOW)], backend: [], failPersistTimes: 1});
  const outcome = await purgeSessions('all', h.ports, {nowMs: NOW});
  assert.equal(outcome.ok, false);
  assert.equal(outcome.fullRollback, true, '持久失败 = 整体回滚');
  assert.deepEqual(h.current.map((c) => c.id), ['a'], '列表还原');
  assert.deepEqual(h.calls.remote, [], '回滚档不惊动后端');
  assert.equal(h.calls.errors.length, 1);

  // 后端账本拉不到 = 整体取消（宁可不动手，不"只清本地"制造复活）
  const h2 = makeHarness({local: [conv('a', NOW)], backend: null});
  const cancelled = await purgeSessions('all', h2.ports, {nowMs: NOW});
  assert.equal(cancelled.ok, false, '账本缺失 = 取消清理');
  assert.deepEqual(h2.current.map((c) => c.id), ['a'], '本地一字未动');
  assert.deepEqual(h2.calls.remote, [], '后端一字未动');
  assert.equal(h2.calls.errors.length, 1, '取消也亮帧（如实告知）');
  console.log('  ok  ④ 空转零动作 / 持久失败整体回滚 / 账本缺失取消清理');
}

// ---- ⑤ 调用日志档位（C2 裁决位）----
{
  const h1 = makeHarness({local: [conv('a', NOW)], backend: []});
  await purgeSessions('all', h1.ports, {nowMs: NOW});
  assert.deepEqual(h1.calls.callLogs, ['all'], '② 档调用日志全清');
  console.log('  ok  ⑤ 调用日志：② 档全清 / ① 档只清已删归属（③ 已验）');
}

// ---- ⑥ 会话已清除登记表（MemoryView 悬空引用占位语义）----
{
  resetClearedSessions();
  assert.equal(isSessionCleared('x'), false, '默认不冒充"已清除"');
  markSessionsCleared(['x']);
  assert.equal(isSessionCleared('x'), true);
  assert.equal(isSessionCleared('y'), false, '未登记 id 不判定（0 装）');
  resetClearedSessions();
  console.log('  ok  ⑥ 登记表：只认真删成的 id，重启/未登记不冒充');
}

// ---- ⑦ 红线 B/C：命名不冒充记忆 + 文案机器断言（真实登记表 + 源码镜像）----
{
  const recentMsg = DANGER_ACTION_CONFIRMATIONS.clearRecentConversations.message;
  const allMsg = DANGER_ACTION_CONFIRMATIONS.clearAllSessionData.message;
  assert.ok(recentMsg.includes('长期记忆') && recentMsg.includes('不受影响'), '① 文案必须含"长期记忆不受影响"句');
  assert.ok(allMsg.includes('长期记忆') && allMsg.includes('不受影响'), '② 文案必须含"长期记忆不受影响"句');
  assert.ok(recentMsg.includes(`${RECENT_WINDOW_DAYS_DEFAULT} 天`), '① 文案的窗口天数与 RECENT_WINDOW_DAYS_DEFAULT 一致（改窗口必改文案）');
  assert.ok(allMsg.includes('记忆遗忘') && allMsg.includes('单独审批'), '② 文案必须指路③（记忆遗忘走单独审批），不代执行');
  for (const key of ['clearRecentConversations', 'clearAllSessionData']) {
    assert.ok(!DANGER_ACTION_CONFIRMATIONS[key].title.includes('记忆') || DANGER_ACTION_CONFIRMATIONS[key].title.includes('长期记忆'),
      `${key} 标题不许用裸"记忆"冒充记忆库操作（红线 B：可带"（保留长期记忆）"限定）`);
    assert.ok(DANGER_ACTION_CONFIRMATIONS[key].confirmText.length > 0);
  }

  const settingsSrc = readFileSync(join(here, '../src/lib/views/SettingsView.svelte'), 'utf8');
  assert.ok(settingsSrc.includes("requestDanger('clearRecentConversations')"), '① 按钮必须走二次确认');
  assert.ok(settingsSrc.includes("requestDanger('clearAllSessionData')"), '② 按钮必须走二次确认');
  assert.ok(/case 'clearRecentConversations'/.test(settingsSrc), 'runDangerAction 收口①');
  assert.ok(/case 'clearAllSessionData'/.test(settingsSrc), 'runDangerAction 收口②');
  assert.ok(settingsSrc.includes('记忆遗忘') && settingsSrc.includes('尚未接线'), '③ 入口灰显如实标注未接线（0 装）');
  assert.ok(settingsSrc.includes('保留长期记忆'), '按钮名逐项写明"保留长期记忆"');

  const memorySrc = readFileSync(join(here, '../src/lib/MemoryView.svelte'), 'utf8');
  assert.ok(memorySrc.includes('会话已清除'), 'MemoryView 悬空引用占位（不报错不删记忆行）');
  assert.ok(memorySrc.includes('这条记忆不受影响'), '占位文案如实说明记忆不受影响');
  // 红线 A 机器可检：清理链本体零记忆端口（不 import runtime、不接任何遗忘 API），
  // 注释里引用 CoordinatedForget 协议名说"不走它"是合法的，不在此列。
  const cleanupSrc = readFileSync(join(here, '../src/lib/chat-shell/session-cleanup.ts'), 'utf8');
  assert.ok(!/forgetMemoryEpisode|forget_episode|coordinated_forget\(/i.test(cleanupSrc),
    '清理链不许接入任何遗忘 API（红线 A：记忆遗忘是另一个动作）');
  assert.ok(!/from '\.\.\/runtime'/.test(cleanupSrc), '清理链不 import runtime（零记忆/网络端口，纯注入端口）');
  console.log('  ok  ⑦ 红线 B/C：双档文案含"长期记忆不受影响"+ ③ 灰显未接线 + 占位不越界');
}

console.log('--- All Session Cleanup (dual-scope purge) Checks PASSED! ---');
