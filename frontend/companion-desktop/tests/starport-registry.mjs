// 星港贡献点注册表（contribution-registry.ts，真实模块）——数据优先注册面的插口证明。
//
// 契约要点：
//   1. 数据优先注册：应用/命令纯数据定义，id 唯一（小写连字符）；
//   2. 防注入面：html/innerHTML/template/url/channel 字段一律拒绝，零例外；
//   3. owner 生命周期：失活 owner 拒注册，releaseOwner 连带注销不留孤儿；
//   4. 0 装：非法定义抛错（不猜不修），注销 owner 不匹配一律拒绝。
import assert from 'node:assert/strict';
import {
  ContributionRegistry,
  ContributionRegistryError,
  FORBIDDEN_DEFINITION_FIELDS,
} from '../src/lib/starport/contribution-registry.ts';

console.log('--- Starting Starport Contribution Registry Check ---');

const dummyMount = () => () => {};
const helloDef = () => ({
  id: 'starport-hello',
  title: '星港回执',
  kind: 'internal',
  mount: dummyMount,
});

// ---------------------------------------------------------------------------
// 1. 应用注册 / 查询 / 列表 / 订阅
// ---------------------------------------------------------------------------
{
  const registry = new ContributionRegistry();
  const events = [];
  const unsubscribe = registry.subscribe((event) =>
    events.push(`${event.kind}:${event.id}:${event.action}`),
  );
  registry.registerApp(helloDef());
  assert.equal(registry.getApp('starport-hello')?.title, '星港回执');
  assert.equal(registry.listApps().length, 1);
  assert.deepEqual(events, ['app:starport-hello:registered']);
  unsubscribe();
  registry.registerApp({...helloDef(), id: 'starport-second', title: '第二件'});
  assert.equal(events.length, 1, '退订后不再收事件');
  console.log('  [1] register/query/list/subscribe OK');
}

// ---------------------------------------------------------------------------
// 2. id 与定义校验（0 装：非法即抛）
// ---------------------------------------------------------------------------
{
  const registry = new ContributionRegistry();
  assert.throws(
    () => registry.registerApp({...helloDef(), id: 'Bad_Id'}),
    (error) =>
      error instanceof ContributionRegistryError && error.code === 'invalid-id',
    '大写/下划线 id 必须拒绝',
  );
  assert.throws(
    () => registry.registerApp({...helloDef(), id: 'x'}),
    (error) =>
      error instanceof ContributionRegistryError && error.code === 'invalid-id',
    '单字符 id 必须拒绝',
  );
  assert.throws(
    () => registry.registerApp({...helloDef(), title: ''}),
    (error) =>
      error instanceof ContributionRegistryError && error.code === 'invalid-definition',
  );
  assert.throws(
    () => registry.registerApp({...helloDef(), mount: undefined}),
    (error) =>
      error instanceof ContributionRegistryError && error.code === 'invalid-definition',
  );
  assert.throws(
    () => registry.registerApp({...helloDef(), kind: 'external'}),
    (error) =>
      error instanceof ContributionRegistryError && error.code === 'invalid-definition',
    '批 1 只收 internal',
  );
  assert.throws(
    () => registry.registerCommand({id: 'cmd-no-handler', title: '缺 handler'}),
    (error) =>
      error instanceof ContributionRegistryError && error.code === 'invalid-definition',
  );
  console.log('  [2] id/definition validation OK');
}

// ---------------------------------------------------------------------------
// 3. 防注入面：禁带字段全覆盖
// ---------------------------------------------------------------------------
{
  const registry = new ContributionRegistry();
  for (const field of FORBIDDEN_DEFINITION_FIELDS) {
    assert.throws(
      () => registry.registerApp({...helloDef(), [field]: '<b>x</b>'}),
      (error) =>
        error instanceof ContributionRegistryError && error.code === 'forbidden-field',
      `禁带字段 ${field} 必须拒绝`,
    );
    assert.throws(
      () =>
        registry.registerCommand({
          id: 'cmd-forbidden',
          title: 'x',
          handler: () => {},
          [field]: 'x',
        }),
      (error) =>
        error instanceof ContributionRegistryError && error.code === 'forbidden-field',
      `命令禁带字段 ${field} 必须拒绝`,
    );
  }
  console.log('  [3] forbidden injection fields OK');
}

// ---------------------------------------------------------------------------
// 4. 重复 id 一律拒绝（跨 app/command 同名也拒）
// ---------------------------------------------------------------------------
{
  const registry = new ContributionRegistry();
  registry.registerApp(helloDef());
  assert.throws(
    () => registry.registerApp(helloDef()),
    (error) => error instanceof ContributionRegistryError && error.code === 'duplicate',
  );
  assert.throws(
    () =>
      registry.registerCommand({id: 'starport-hello', title: '撞名', handler: () => {}}),
    (error) => error instanceof ContributionRegistryError && error.code === 'duplicate',
    'app/command 跨类撞名也必须拒绝',
  );
  console.log('  [4] duplicate id rejection OK');
}

// ---------------------------------------------------------------------------
// 5. owner 生命周期：失活拒注册 / releaseOwner 连带注销 / owner 不匹配拒注销
// ---------------------------------------------------------------------------
{
  const registry = new ContributionRegistry();
  const owner = {id: 'view-host', active: true};
  registry.registerApp(helloDef(), {owner});
  registry.registerCommand({id: 'cmd-owned', title: 'owner 命令', handler: () => {}}, {owner});

  assert.throws(
    () => registry.registerApp({...helloDef(), id: 'late-app'}, {owner: {...owner, active: false}}),
    (error) =>
      error instanceof ContributionRegistryError && error.code === 'owner-inactive',
    '失活 owner 必须拒绝注册',
  );

  assert.throws(
    () => registry.unregister('app', 'starport-hello', {owner: {id: 'other'}}),
    (error) =>
      error instanceof ContributionRegistryError && error.code === 'owner-mismatch',
    '他人 owner 注销必须拒绝',
  );

  const removed = registry.releaseOwner('view-host');
  assert.equal(removed, 2, 'releaseOwner 必须连带注销其全部贡献点');
  assert.equal(registry.getApp('starport-hello'), null);
  assert.equal(registry.getCommand('cmd-owned'), null);
  assert.equal(registry.listApps().length + registry.listCommands().length, 0, '不留孤儿');
  console.log('  [5] owner lifecycle OK');
}

// ---------------------------------------------------------------------------
// 6. 无 owner 贡献点：可直接注销；注销不存在的抛 missing
// ---------------------------------------------------------------------------
{
  const registry = new ContributionRegistry();
  registry.registerApp(helloDef());
  registry.unregister('app', 'starport-hello');
  assert.equal(registry.getApp('starport-hello'), null);
  assert.throws(
    () => registry.unregister('app', 'starport-hello'),
    (error) => error instanceof ContributionRegistryError && error.code === 'missing',
  );
  console.log('  [6] ownerless unregister OK');
}

console.log('--- Starport Contribution Registry Check PASSED ---');
