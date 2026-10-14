// 星港多标签壳（shell.ts + builtins.ts，真实模块）——视图生命周期与浮层配对证明。
//
// 契约要点：
//   1. openApp 挂载入标签体系；closeView 清理恰好一次（含异常路径与迟到清理）；
//   2. openOrFocusApp 单实例语义：重复打开只换焦点，不叠标签；
//   3. 浮层 acquire/release 成对，release 未知令牌抛错，重复 acquire 同名抛错；
//   4. whenSettled 在挂载队列清空后兑现（含异步挂载）；
//   5. 0 装：开未注册应用 / 关未知视图 / 置未知焦点一律抛错。
import assert from 'node:assert/strict';
import {ContributionRegistry} from '../src/lib/starport/contribution-registry.ts';
import {
  createStarportShell,
  StarportShellError,
} from '../src/lib/starport/shell.ts';
import {
  createHelloApp,
  installStarportBuiltins,
  STARPORT_HELLO_APP_ID,
  STARPORT_HELLO_COMMAND_ID,
} from '../src/lib/starport/builtins.ts';

console.log('--- Starting Starport Shell Check ---');

function makeHarness() {
  const registry = new ContributionRegistry();
  const mounted = [];
  const cleanups = [];
  const host = {
    mountApp: (app, context) => {
      mounted.push(`${app.id}@${context.viewId}`);
      return () => cleanups.push(`${app.id}@${context.viewId}`);
    },
  };
  const shell = createStarportShell({registry, host});
  return {registry, shell, mounted, cleanups};
}

// ---------------------------------------------------------------------------
// 1. hello 契约件全链路：安装 → 挂载 → 回执文本 → 关闭清理
// ---------------------------------------------------------------------------
{
  const registry = new ContributionRegistry();
  const container = {textContent: ''};
  let opened = 0;
  installStarportBuiltins(registry, {onOpenHello: () => (opened += 1)});
  assert.ok(registry.getApp(STARPORT_HELLO_APP_ID), 'hello 契约件必须注册');
  assert.ok(registry.getCommand(STARPORT_HELLO_COMMAND_ID), 'hello 命令必须注册');

  const shell = createStarportShell({
    registry,
    host: {
      mountApp: (app, context) => app.mount(container, context),
    },
  });
  const viewId = shell.openApp(STARPORT_HELLO_APP_ID);
  assert.match(container.textContent, /星港在线/, '挂载回执必须写入容器');
  assert.match(container.textContent, new RegExp(viewId.replace(':', '\\:')), '回执含视图 id');

  shell.closeView(viewId);
  assert.equal(container.textContent, '', 'cleanup 必须清空容器');

  registry.getCommand(STARPORT_HELLO_COMMAND_ID).handler();
  assert.equal(opened, 1, '命令 handler 必须被真实调用');
  console.log('  [1] hello contract app OK');
}

// ---------------------------------------------------------------------------
// 2. 视图生命周期：挂载/焦点/关闭/清理恰好一次
// ---------------------------------------------------------------------------
{
  const {registry, shell, mounted, cleanups} = makeHarness();
  registry.registerApp(createHelloApp());
  registry.registerApp({...createHelloApp(), id: 'starport-second', title: '第二件'});

  const viewA = shell.openApp(STARPORT_HELLO_APP_ID);
  const viewB = shell.openApp('starport-second');
  assert.deepEqual(mounted, [
    `starport-hello@${viewA}`,
    `starport-second@${viewB}`,
  ]);
  let snap = shell.getSnapshot();
  assert.equal(snap.views.length, 2);
  assert.equal(snap.activeViewId, viewB, '新视图自动聚焦');

  shell.setActiveView(viewA);
  assert.equal(shell.getSnapshot().activeViewId, viewA);

  shell.closeView(viewB);
  assert.deepEqual(cleanups, [`starport-second@${viewB}`], '关闭必须触发清理恰好一次');
  snap = shell.getSnapshot();
  assert.equal(snap.views.length, 1);
  assert.equal(snap.activeViewId, viewA, '关闭活动视图后焦点回落到余下视图');

  shell.closeView(viewA);
  assert.equal(shell.getSnapshot().activeViewId, null, '全关后焦点为空');
  assert.throws(
    () => shell.closeView(viewA),
    (error) => error instanceof StarportShellError && error.code === 'view-missing',
    '重复关闭必须抛 view-missing',
  );
  console.log('  [2] view lifecycle OK');
}

// ---------------------------------------------------------------------------
// 3. 单实例语义：openOrFocusApp 复用视图不叠标签
// ---------------------------------------------------------------------------
{
  const {registry, shell, mounted} = makeHarness();
  registry.registerApp(createHelloApp());
  const viewA = shell.openApp(STARPORT_HELLO_APP_ID);
  shell.openApp(STARPORT_HELLO_APP_ID);
  assert.equal(shell.getSnapshot().views.length, 2, 'openApp 显式多开允许');

  const reused = shell.openOrFocusApp(STARPORT_HELLO_APP_ID);
  assert.equal(reused, viewA, 'openOrFocus 必须复用第一个视图');
  assert.equal(shell.getSnapshot().views.length, 2, 'openOrFocus 不叠标签');
  assert.equal(shell.getSnapshot().activeViewId, viewA, '复用即聚焦');
  assert.equal(mounted.length, 2, '复用不再挂载');
  console.log('  [3] single-instance semantics OK');
}

// ---------------------------------------------------------------------------
// 4. 浮层配对纪律
// ---------------------------------------------------------------------------
{
  const {shell} = makeHarness();
  const tokenA = shell.acquireOverlay('帮助');
  const tokenB = shell.acquireOverlay('关于');
  assert.deepEqual(shell.getSnapshot().openOverlays, ['帮助', '关于']);

  assert.throws(
    () => shell.acquireOverlay('帮助'),
    (error) =>
      error instanceof StarportShellError && error.code === 'overlay-duplicate',
    '同名浮层重复 acquire 必须拒绝',
  );
  shell.releaseOverlay(tokenA);
  assert.deepEqual(shell.getSnapshot().openOverlays, ['关于']);
  assert.throws(
    () => shell.releaseOverlay(tokenA),
    (error) =>
      error instanceof StarportShellError && error.code === 'overlay-missing',
    '未知/已释放令牌必须拒绝',
  );
  shell.releaseOverlay(tokenB);
  assert.deepEqual(shell.getSnapshot().openOverlays, []);
  console.log('  [4] overlay pairing OK');
}

// ---------------------------------------------------------------------------
// 5. 异步挂载 + whenSettled + 迟到清理（挂载在途关视图不留孤儿）
// ---------------------------------------------------------------------------
{
  const registry = new ContributionRegistry();
  const cleanups = [];
  let resolveMount = null;
  const shell = createStarportShell({
    registry,
    host: {
      mountApp: () =>
        new Promise((resolve) => {
          resolveMount = (cleanup) => resolve(cleanup);
        }),
    },
  });
  registry.registerApp(createHelloApp());

  const viewId = shell.openApp(STARPORT_HELLO_APP_ID);
  assert.equal(shell.getSnapshot().views[0].status, 'mounting', '异步挂载在途状态为 mounting');

  // 挂载在途即关闭视图：迟到的清理必须立即执行，不留孤儿。
  shell.closeView(viewId);
  resolveMount(() => cleanups.push('late-cleanup'));
  await shell.whenSettled();
  assert.deepEqual(cleanups, ['late-cleanup'], '迟到清理必须恰好执行一次');
  assert.equal(shell.getSnapshot().views.length, 0);

  // 正常异步挂载：settle 后状态 mounted。
  const view2 = shell.openApp(STARPORT_HELLO_APP_ID);
  resolveMount(() => cleanups.push('normal-cleanup'));
  await shell.whenSettled();
  assert.equal(shell.getSnapshot().views[0].status, 'mounted');
  shell.closeView(view2);
  assert.deepEqual(cleanups, ['late-cleanup', 'normal-cleanup']);
  console.log('  [5] async mount / settle / late cleanup OK');
}

// ---------------------------------------------------------------------------
// 6. 0 装边界：未注册应用 / 未知视图 / dispose 全清
// ---------------------------------------------------------------------------
{
  const {registry, shell} = makeHarness();
  registry.registerApp(createHelloApp());
  assert.throws(
    () => shell.openApp('starport-missing'),
    (error) => error instanceof StarportShellError && error.code === 'app-missing',
  );
  assert.throws(
    () => shell.closeView('view:999'),
    (error) => error instanceof StarportShellError && error.code === 'view-missing',
  );
  assert.throws(
    () => shell.setActiveView('view:999'),
    (error) => error instanceof StarportShellError && error.code === 'view-missing',
  );

  shell.openApp(STARPORT_HELLO_APP_ID);
  shell.openApp(STARPORT_HELLO_APP_ID);
  shell.acquireOverlay('帮助');
  shell.dispose();
  const snap = shell.getSnapshot();
  assert.equal(snap.views.length, 0, 'dispose 必须全关视图');
  assert.deepEqual(snap.openOverlays, [], 'dispose 必须全清浮层');
  console.log('  [6] zero-fabrication boundaries OK');
}

console.log('--- Starport Shell Check PASSED ---');
