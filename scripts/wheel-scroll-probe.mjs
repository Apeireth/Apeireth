#!/usr/bin/env node
// CDP 滚轮探针（wheel-scroll probe）——对**真实 WebView2 应用**发真实滚轮事件，
// 量化"鼠标滚轮 vs 右侧滚动条"的差异，作为滚动修复的复现/验收证据。
//
// 背景（内测反馈）：小窗模式下设置界面、初始对话界面等处只能拖右侧滚动条，
// 鼠标滚轮不动。本探针在 980x640 小窗视口（≤980px 断点生效）下逐点发 wheel，
// 比对滚轮前后目标滚动面板的 scrollTop——不动 = 复现病灶；动 = 修复生效。
//
// 用法（先给应用开 CDP 端口再启动）：
//   $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9223'
//   Start-Process 'C:\Program Files\Apeireth Companion\companion-desktop.exe'
//   node scripts/wheel-scroll-probe.mjs [--port 9223] [--width 980] [--height 640]
//
// 注：Input.dispatchMouseEvent 的 wheel 走真实命中测试（与真人滚轮同语义）；
// Emulation.setDeviceMetricsOverride 把视口钉成小窗，媒体查询随之生效。

const args = process.argv.slice(2);
function argValue(name, fallback) {
  const i = args.indexOf(`--${name}`);
  return i >= 0 && args[i + 1] ? args[i + 1] : fallback;
}
const PORT = Number(argValue('port', '9223'));
const WIDTH = Number(argValue('width', '980'));
const HEIGHT = Number(argValue('height', '640'));

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function listTargets() {
  const res = await fetch(`http://127.0.0.1:${PORT}/json/list`);
  return res.json();
}

function connect(wsUrl) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(wsUrl);
    const pending = new Map();
    let nextId = 1;
    ws.onopen = () =>
      resolve({
        send(method, params = {}) {
          const id = nextId++;
          ws.send(JSON.stringify({id, method, params}));
          return new Promise((res, rej) => {
            pending.set(id, {res, rej});
            setTimeout(() => {
              if (pending.delete(id)) rej(new Error(`CDP timeout: ${method}`));
            }, 8000);
          });
        },
        close: () => ws.close(),
      });
    ws.onerror = (e) => reject(new Error('CDP ws error: ' + (e.message || e.type)));
    ws.onmessage = (ev) => {
      const msg = JSON.parse(ev.data);
      const waiter = msg.id && pending.get(msg.id);
      if (waiter) {
        pending.delete(msg.id);
        if (msg.error) waiter.rej(new Error(msg.error.message));
        else waiter.res(msg.result);
      }
    };
  });
}

/** 找(x,y)下命中元素向上的滚动面板，返回其滚动位置快照（JSON）。 */
function paneAtExpr(x, y) {
  return `(() => {
    let el = document.elementFromPoint(${x}, ${y});
    const chain = [];
    while (el && el !== document.documentElement) {
      const cs = getComputedStyle(el);
      const cls = el.id ? '#' + el.id : (el.className ? '.' + String(el.className).trim().split(/\\s+/).slice(0, 3).join('.') : '');
      chain.push(el.tagName + cls + '[' + cs.overflowY + (cs.pointerEvents === 'none' ? '/pe:none' : '') + ']');
      if ((cs.overflowY === 'auto' || cs.overflowY === 'scroll') && el.scrollHeight > el.clientHeight + 1) {
        return JSON.stringify({found: true, pane: chain[chain.length - 1], top: Math.round(el.scrollTop), clientH: el.clientHeight, scrollH: el.scrollHeight, chain: chain.slice(0, 5)});
      }
      el = el.parentElement;
    }
    return JSON.stringify({found: false, hit: chain[0] || 'none', chain: chain.slice(0, 6)});
  })()`;
}

async function evaluate(cdp, expression) {
  const out = await cdp.send('Runtime.evaluate', {expression, returnByValue: true});
  if (out.exceptionDetails) throw new Error('eval failed: ' + JSON.stringify(out.exceptionDetails).slice(0, 200));
  return out.result.value;
}

async function clickByText(cdp, selector, text) {
  const expr = `(() => {
    const btn = [...document.querySelectorAll('${selector}')].find((b) => b.textContent.includes('${text}'));
    if (!btn) return false;
    btn.click();
    return true;
  })()`;
  return evaluate(cdp, expr);
}

/** 直测期望面板的 scrollTop（验收口径：滚轮后它必须动；没溢出 = 无需滚动，如实中性）。 */
function paneStateExpr(sel) {
  return `(() => {
    const el = document.querySelector('${sel}');
    if (!el) return JSON.stringify({missing: true});
    return JSON.stringify({top: Math.round(el.scrollTop), clientH: el.clientHeight, scrollH: el.scrollHeight, overflow: el.scrollHeight > el.clientHeight + 5});
  })()`;
}

/** 打开抽屉页（rail 是切换语义：已开时再点会关——先点、验面板、缺则再点）。 */
async function openDrawer(cdp, label, sel) {
  for (let i = 0; i < 2; i++) {
    await clickByText(cdp, '.rail-btn', label);
    await sleep(800);
    const ok = await evaluate(cdp, `document.querySelector('${sel}') !== null`);
    if (ok) return true;
  }
  return false;
}

async function probePoint(cdp, label, x, y, expectSel) {
  const beforeChain = JSON.parse(await evaluate(cdp, paneAtExpr(x, y)));
  const before = JSON.parse(await evaluate(cdp, paneStateExpr(expectSel)));
  let verdict;
  if (before.missing) {
    verdict = '⚠️ 期望面板不存在（选择器失效？）';
  } else if (!before.overflow) {
    verdict = '— 内容没溢出（无需滚动，中性）';
  } else {
    // 按余量方向发滚轮（向下没余量就向上）——否则顶在边界上是假阴性。
    const maxTop = before.scrollH - before.clientH;
    const roomDown = maxTop - before.top;
    const dir = roomDown > 5 ? 1 : before.top > 5 ? -1 : 0;
    if (dir === 0) {
      verdict = '— 无滚动余量（中性）';
    } else {
      await cdp.send('Input.dispatchMouseEvent', {type: 'mouseWheel', x, y, deltaX: 0, deltaY: 420 * dir, modifiers: 0});
      await sleep(300);
      const after = JSON.parse(await evaluate(cdp, paneStateExpr(expectSel)));
      verdict =
        Math.abs(after.top - before.top) > 2
          ? `✅ 滚动了 ${after.top - before.top}px（${dir > 0 ? '向下' : '向上'}）`
          : `❌ 滚轮无效（${dir > 0 ? '向下' : '向上'}有余量但 scrollTop 不动）`;
    }
  }
  console.log(`\n[${label}] (${x},${y}) → 盯 ${expectSel}`);
  console.log(`  命中链: ${(beforeChain.chain || []).join(' → ')}`);
  console.log(`  面板: scrollTop ${before.top ?? '-'} / max ${before.missing || !before.overflow ? '-' : before.scrollH - before.clientH} (clientH ${before.clientH ?? '-'}, scrollH ${before.scrollH ?? '-'})`);
  console.log(`  判定: ${verdict}`);
  return {label, moved: verdict.startsWith('✅'), neutral: verdict.startsWith('—'), verdict};
}

const targets = await listTargets();
// 主窗口 = 根路由（无 ?window= 查询）；quick 小窗是 ?window=quick，别连错。
const windowFilter = argValue('window', '');
const page = windowFilter
  ? targets.find((t) => t.type === 'page' && t.url.includes(windowFilter))
  : targets.find((t) => t.type === 'page' && /tauri\.localhost\/(\?|$)/.test(t.url) && !t.url.includes('window=')) ||
    targets.find((t) => t.type === 'page' && !t.url.includes('window=quick'));
if (!page) {
  console.error('未找到应用页面 target。当前 targets:', targets.map((t) => `${t.type} ${t.url}`).join(' | '));
  process.exit(1);
}
console.log(`连接: ${page.title} (${page.url})`);
const cdp = await connect(page.webSocketDebuggerUrl);

await cdp.send('Emulation.setDeviceMetricsOverride', {width: WIDTH, height: HEIGHT, deviceScaleFactor: 1, mobile: false});
await sleep(400);
console.log(`视口钉为 ${WIDTH}x${HEIGHT}（小窗断点 ≤980px 生效）`);

// ---- 场景 1：设置界面（小窗）----
console.log(`\n=== 场景 1：设置界面（${WIDTH}x${HEIGHT}）===`);
const CONTENT = '[class*="settings-content"]';
console.log('打开设置:', await openDrawer(cdp, '设置', CONTENT));
const r1 = await probePoint(cdp, '设置内容中心（R1 原生）', 700, 400, CONTENT);
const r2 = await probePoint(cdp, '设置抽屉头（R4 唯一面板）', 400, 60, CONTENT);
const r3 = await probePoint(cdp, '设置子导航列（R4 唯一面板）', 150, 400, CONTENT);

// ---- 场景 2：初始对话界面 ----
console.log('\n=== 场景 2：初始对话界面 ===');
await openDrawer(cdp, '对话', '[class*="session-col"]'); // 对话无面板可验，靠往来列表兜底
await sleep(300);
console.log('打开往来列表:', await clickByText(cdp, '.col-toggle', '往来'));
await sleep(600);
const r4 = await probePoint(cdp, '会话列表栏（R1 原生）', 200, 400, '[class*="session-col"]');
// 选本机有正文、消息最多的会话打开（空会话/后端独有会话没有溢出余量，测不出滚动）。
const openedCount = await evaluate(
  cdp,
  `(() => {
    const rows = [...document.querySelectorAll('.session-row')];
    let best = null, bestCount = -1;
    for (const r of rows) {
      const prev = r.querySelector('.session-preview');
      if (!prev || prev.classList.contains('ledger') || prev.classList.contains('faint')) continue;
      const cnt = r.querySelector('.session-count');
      const n = cnt ? (parseInt(cnt.textContent, 10) || 0) : 0;
      if (n > bestCount) { bestCount = n; best = r; }
    }
    if (!best) return -1;
    best.click();
    return bestCount;
  })()`,
);
console.log('打开消息最多的本机会话（消息数）:', openedCount);
// 轮询等消息流渲染出真实溢出（异步回填时序），就绪后再发滚轮。
let chatReady = false;
for (let i = 0; i < 10 && !chatReady; i++) {
  await sleep(400);
  chatReady = await evaluate(cdp, `(() => { const el = document.querySelector('#chatScroll'); return !!el && el.scrollHeight > el.clientHeight + 5; })()`);
}
console.log('聊天区就绪（有溢出可滚）:', chatReady);
const r5 = await probePoint(cdp, '聊天区左侧留白（R2 穿透死区）', 346, 300, '#chatScroll');
const r6 = await probePoint(cdp, '聊天区右侧留白（R2 穿透死区）', 900, 500, '#chatScroll');

const results = [r1, r2, r3, r4, r5, r6];
const failed = results.filter((r) => r.verdict.startsWith('❌')).map((r) => r.label);
console.log('\n==== 汇总 ====');
for (const r of results) {
  const mark = r.verdict.startsWith('✅') ? '✅' : r.verdict.startsWith('—') ? '➖' : '❌';
  console.log(`  ${mark} ${r.label} — ${r.verdict}`);
}
console.log(failed.length === 0 ? '滚轮在全部有溢出的面板上可用（修复生效）' : `滚轮失效点: ${failed.join('、')}`);
cdp.close();
process.exit(failed.length === 0 ? 0 : 2);
