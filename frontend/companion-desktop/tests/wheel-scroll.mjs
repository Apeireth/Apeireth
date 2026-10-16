// 全局滚轮路由纯逻辑单测（真实模块 src/lib/wheel-scroll.ts；Node 类型擦除直 import）。
//
// 依据 CDP 实测病灶（scripts/wheel-scroll-probe.mjs，980x640 小窗）：
//   死区（聊天留白 pe:none 穿透 / 设置子导航列旁路）里滚轮事件到不了任何滚动面板。
// 盯死四组：
//   ① 路由规则 R1（命中链面板=原生零干预）/ R2（后代几何面板）/ R4（唯一面板）/ none；
//   ② 滚动执行的到界链式传递与 overscroll-contain 即停；
//   ③ 滚轮增量归一化（行/页模式）；
//   ④ 源码镜像：App 真挂路由、设置页小窗响应式（minmax(0,1fr) 行 + 子导航自滚）。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));

const {resolveWheelDelta, pickPaneUnderPoint, pickUniquePane, routeWheel, scrollWithChain, paneVisibleInViewport} =
  await import('../src/lib/wheel-scroll.ts');

// ---- 迷你 DOM 同构假件：祖先表 + contains + parentOf ----
const parents = new Map();
const byId = new Map();
function node(id, parent = null) {
  parents.set(id, parent);
  const n = {
    id,
    contains: (other) => {
      for (let c = other && other.id; c; c = parents.get(c)) if (c === id) return true;
      return false;
    },
  };
  byId.set(id, n);
  return n;
}
function depthOf(id) {
  let d = 1;
  for (let c = parents.get(id); c; c = parents.get(c)) d += 1;
  return d;
}
function hit(id) {
  return {
    id,
    depth: depthOf(id),
    node: byId.get(id),
    contains: byId.get(id).contains,
    parentOf: () => (parents.get(id) ? hit(parents.get(id)) : null),
  };
}
function pane(id, parent, rect, {scrollH = 1000, clientH = 400, contain = false, outer = null} = {}) {
  return {
    id,
    depth: depthOf(id),
    rect,
    scrollTop: 0,
    scrollHeight: scrollH,
    clientHeight: clientH,
    overscrollContain: contain,
    node: node(id, parent),
    contains: byId.get(id).contains,
    outer,
  };
}
const FULL = {left: 0, top: 0, right: 980, bottom: 640};

// ---- ① 滚轮增量归一化 ----
{
  assert.equal(resolveWheelDelta(100, 0, 640), 100, '像素模式原样');
  assert.equal(resolveWheelDelta(3, 1, 640), 48, '行模式 ×16px');
  assert.equal(resolveWheelDelta(2, 2, 640), 1280, '页模式 ×视口高');
  console.log('  ok  ① 增量归一化（pixel/line/page）');
}

// ---- ② R2 后代几何面板（pe:none 穿透死区，复刻聊天留白病灶）----
{
  parents.clear();
  byId.clear();
  node('body');
  node('app', 'body');
  node('chatScroll', 'app'); // 命中体（pe:none 穿透）的后代面板
  const paneChat = pane('chatScroll', 'app', {left: 336, top: 0, right: 980, bottom: 640});
  const index = {panes: [paneChat], hit: hit('app')};
  assert.equal(pickPaneUnderPoint(index, 400, 300)?.id, 'chatScroll', '后代面板几何含光标 → 路由');
  assert.equal(pickPaneUnderPoint(index, 100, 300), null, '光标在面板矩形外 → 不路由');
  // 面板内容没溢出 = 无处可滚（初始对话空屏的正确行为），不冒充滚动。
  const fitted = pane('fitted', 'app', FULL, {scrollH: 400, clientH: 400});
  assert.equal(pickPaneUnderPoint({panes: [fitted], hit: hit('app')}, 400, 300), null, '无溢出 → 无处可滚');
  // 更深的面板优先（内层列表嵌在页面面板里）。
  node('page', 'app');
  node('innerList', 'page');
  const pPage = pane('page', 'app', FULL);
  const pInner = pane('innerList', 'page', {left: 76, top: 0, right: 336, bottom: 640});
  assert.equal(
    pickPaneUnderPoint({panes: [pPage, pInner], hit: hit('app')}, 150, 300)?.id,
    'innerList',
    '同点多面板 → 最深者（内层优先）',
  );
  console.log('  ok  ② R2 后代几何面板（穿透死区复刻 / 矩形外不动 / 无溢出不冒充 / 最深优先）');
}

// ---- ③ R4 唯一面板（复刻设置子导航列病灶）----
{
  parents.clear();
  byId.clear();
  node('body');
  node('settingsLayout', 'body');
  node('subnavGroup', 'settingsLayout');
  node('subnavBtn', 'subnavGroup');
  node('settingsContent', 'settingsLayout');
  const pContent = pane('settingsContent', 'settingsLayout', {left: 276, top: 0, right: 796, bottom: 640});
  const index = {panes: [pContent], hit: hit('subnavBtn')};
  assert.equal(pickUniquePane(index)?.id, 'settingsContent', '子树唯一面板 → 路由到内容列');
  // 同容器两个面板（子导航自身也溢出）→ 不武断：R1 原生处理子导航，R4 在容器层不抢。
  node('subnav', 'settingsLayout');
  parents.set('subnavGroup', 'subnav');
  parents.set('subnavBtn', 'subnavGroup');
  const pSubnav = pane('subnav', 'settingsLayout', {left: 76, top: 0, right: 276, bottom: 640});
  assert.equal(
    pickUniquePane({panes: [pSubnav, pContent], hit: hit('settingsLayout')}),
    null,
    '两面板共存 → R4 不路由（留给原生命中链）',
  );
  console.log('  ok  ③ R4 唯一面板（子导航旁路复刻 / 两面板共存不抢）');
}

// ---- ④ 路由裁决优先级（R1 > R2 > R4 > none）----
{
  parents.clear();
  byId.clear();
  node('body');
  node('scroller', 'body');
  node('content', 'scroller');
  const pScroll = pane('scroller', 'body', FULL);
  // R1：命中链里就有面板 → 原生。
  const d1 = routeWheel([pScroll], {panes: [pScroll], hit: hit('content')}, 500, 300);
  assert.equal(d1.kind, 'native');
  assert.equal(d1.rule, 'R1', '命中链面板 → 原生零干预');
  // R2 胜过 R4。
  node('app2');
  node('viaGeometry', 'app2');
  node('viaUniqueHost', 'app2');
  node('viaUnique', 'viaUniqueHost');
  const pGeo = pane('viaGeometry', 'app2', {left: 0, top: 0, right: 100, bottom: 100});
  const pUni = pane('viaUnique', 'viaUniqueHost', {left: 200, top: 0, right: 980, bottom: 640});
  const d2 = routeWheel([], {panes: [pGeo, pUni], hit: hit('app2')}, 50, 50);
  assert.equal(d2.rule, 'R2');
  assert.equal(d2.pane?.id, 'viaGeometry');
  // none：光标下无面板、也无唯一面板（抽屉外遮罩区复刻）。
  node('scrim');
  const d3 = routeWheel([], {panes: [pGeo], hit: hit('scrim')}, 940, 400);
  assert.equal(d3.rule, 'none', '无解 → 不干预（不乱滚背后的面板）');
  console.log('  ok  ④ 裁决优先级（R1 原生 / R2>R4 / none 不干预）');
}

// ---- ⑤ 到界链式 + overscroll-contain 即停 ----
{
  const mk = (id, contain, outer) => ({
    id,
    depth: 1,
    rect: FULL,
    scrollTop: 0,
    scrollHeight: 1000,
    clientHeight: 400,
    overscrollContain: contain,
    node: node(id),
    outer,
  });
  // 记账式 apply：剩余空间 300px。
  const ledger = [];
  const apply = (p, by) => {
    const room = 300 - p.scrollTop;
    const moved = Math.sign(by) * Math.min(Math.abs(by), Math.abs(room)) * (Math.sign(room) === Math.sign(by) ? 1 : 0);
    p.scrollTop += moved;
    ledger.push([p.id, by, moved]);
    return moved;
  };
  // A: 全量被本层消化。
  const a = mk('a', false, null);
  const outA = scrollWithChain(a, 120, apply);
  assert.equal(outA.moved, 120);
  assert.equal(outA.remaining, 0);
  // B: 本层到界 → 链式到外层。
  const bOuter = mk('bOuter', false, null);
  const b = mk('b', false, bOuter);
  b.scrollTop = 280; // 本层只剩 20px 余量
  const outB = scrollWithChain(b, 100, apply);
  assert.equal(outB.moved, 100, '链式消化全量');
  assert.deepEqual(outB.used, ['b', 'bOuter'], '本层到界后由外层接着滚');
  // C: contain 到界即停（.session-col 语义）。
  const cOuter = mk('cOuter', false, null);
  const c = mk('c', true, cOuter);
  c.scrollTop = 290;
  const outC = scrollWithChain(c, 500, apply);
  assert.equal(outC.moved, 10, 'contain 面板只滚到界');
  assert.equal(outC.remaining, 490, '剩余量不传外层（到界即停）');
  assert.deepEqual(outC.used, ['c'], '外层未被触碰');
  console.log('  ok  ⑤ 到界链式（全量/传外层/contain 即停）');
}

// ---- ⑤b 可见性判定（幽灵面板防污染，复刻窄窗折叠 .session-col 病灶）----
{
  const R = (l, t, r, b) => ({left: l, top: t, right: r, bottom: b});
  assert.equal(paneVisibleInViewport(R(0, 0, 300, 640), 980, 640), true, '面板在视口内 → 参与');
  assert.equal(
    paneVisibleInViewport(R(-197, 0, 63, 640), 980, 640),
    false,
    '滑出面板（右缘压视口但中心在外）→ 不参与 R4 唯一性',
  );
  assert.equal(paneVisibleInViewport(R(700, 0, 1200, 640), 980, 640), true, '半出屏但中心在内（中心 950）→ 参与');
  assert.equal(paneVisibleInViewport(R(700, 0, 1300, 640), 980, 640), false, '半出屏且中心在外（中心 1000）→ 不参与');
  assert.equal(paneVisibleInViewport(R(0, -400, 300, -100), 980, 640), false, '顶部滑出 → 不参与');
  console.log('  ok  ⑤b 可见性判定（中心在视口内才参与）');
}

// ---- ⑥ 源码镜像：App 挂路由 + 设置页小窗响应式 ----
{
  const appSrc = readFileSync(join(here, '../src/App.svelte'), 'utf8');
  assert.ok(appSrc.includes("import {initWheelRouter} from './lib/wheel-scroll'"), 'App 挂载滚轮路由');
  assert.ok(/const disposeWheelRouter = initWheelRouter\(\)/.test(appSrc), 'onMount 真调用');
  assert.ok(/disposeWheelRouter\(\)/.test(appSrc), '卸载真解绑');

  const shellSrc = readFileSync(join(here, '../src/lib/design/shell.css'), 'utf8');
  assert.ok(
    /@media \(max-width: 980px\)[\s\S]{0,600}?\.session-col \{[\s\S]{0,400}?visibility: hidden/.test(shellSrc),
    '窄窗折叠列表栏 visibility:hidden（幽灵面板不污染 R4）',
  );

  const settingsSrc = readFileSync(join(here, '../src/lib/views/SettingsView.svelte'), 'utf8');
  assert.ok(settingsSrc.includes('grid-template-rows: minmax(0, 1fr)'), '设置布局行钉死（小窗内容列成有界面板）');
  assert.ok(/\.settings-subnav[\s\S]{0,600}?overflow: hidden/.test(settingsSrc), '子导航不自滚（自滚会吞 R1/R4，滚轮须落内容列）');
  assert.ok(/\.settings-content[\s\S]{0,300}?min-height: 0/.test(settingsSrc), '内容列有界（min-height: 0）');
  console.log('  ok  ⑥ 源码镜像（App 路由接线 + 设置页响应式加固）');
}

console.log('--- All Wheel Router (responsive scroll) Checks PASSED! ---');
