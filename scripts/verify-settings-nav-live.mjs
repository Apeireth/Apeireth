#!/usr/bin/env node
// 设置页两级导航真机验收——对**运行中的已安装应用**（WebView2）发 CDP 指令，
// 验证「大类即导航 + 描述词挪至类别分级下的页面」已在运行中的程序里生效。
//
// 用法（先带调试端口启动应用）：
//   $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9223'
//   Start-Process 'C:\Program Files\Apeireth Companion\companion-desktop.exe'
//   node scripts/verify-settings-nav-live.mjs [--port 9223] [--out artifacts]
//
// 断言：①侧栏只列 5 个大类 ②大类展开才见其下分区页 ③导航列无描述词
//      ④描述词落位页面顶部（category-context）⑤换大类跟随展开
//      ⑥多开多收（同时展开并存、再点即收、不自动收放）⑦全开总高 980×640 放得下。
// 验收后连拍（外观 / 能力与安全 / 多开并存）作实证。

import {writeFileSync} from 'node:fs';

const args = process.argv.slice(2);
function argValue(name, fallback) {
  const i = args.indexOf(`--${name}`);
  return i >= 0 && args[i + 1] ? args[i + 1] : fallback;
}
const PORT = Number(argValue('port', '9223'));
const OUT_DIR = argValue('out', 'artifacts');
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function connect(wsUrl) {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(wsUrl);
    let id = 0;
    const pending = new Map();
    socket.onopen = () =>
      resolve({
        send(method, params = {}) {
          return new Promise((res, rej) => {
            const i = ++id;
            pending.set(i, {res, rej});
            socket.send(JSON.stringify({id: i, method, params}));
          });
        },
        close: () => socket.close(),
      });
    socket.onmessage = (e) => {
      const msg = JSON.parse(String(e.data));
      if (msg.id && pending.has(msg.id)) {
        const {res, rej} = pending.get(msg.id);
        pending.delete(msg.id);
        msg.error ? rej(new Error(JSON.stringify(msg.error))) : res(msg.result);
      }
    };
    socket.onerror = reject;
  });
}

let ws;
for (let i = 0; i < 30; i++) {
  await sleep(500);
  try {
    const list = await (await fetch(`http://127.0.0.1:${PORT}/json/list`)).json();
    // 主窗才是设置页宿主：跳过 ?window=quick 快捷助手小窗。
    const page = list.find((t) => t.type === 'page' && !/window=quick/.test(t.url || ''));
    if (page) {
      ws = await connect(page.webSocketDebuggerUrl);
      break;
    }
  } catch {}
}
if (!ws) {
  console.error('FATAL: CDP 未就绪（应用没带 --remote-debugging-port 启动？）');
  process.exit(1);
}

async function evalJs(expression) {
  const r = await ws.send('Runtime.evaluate', {expression, returnByValue: true, awaitPromise: true});
  return r.result?.value;
}
async function shot(name) {
  const png = await ws.send('Page.captureScreenshot', {format: 'png'});
  writeFileSync(`${OUT_DIR}/${name}`, Buffer.from(png.data, 'base64'));
  console.log('saved', name);
}

// 进设置页（左栏「设置」）。
const entered = await evalJs(`(() => {
  const b = [...document.querySelectorAll('button')].find((x) => (x.title || '').startsWith('设置'));
  if (!b) return 'NO_BTN';
  b.click();
  return 'clicked';
})()`);
console.log('进入设置页:', entered);
await sleep(1200);

const snapshot = await evalJs(`(() => ({
  groupBtns: [...document.querySelectorAll('.subnav-group-btn')].map((b) => b.textContent.trim()),
  pageRows: [...document.querySelectorAll('.subnav-group-pages .subnav-btn')].map((b) => b.textContent.trim()),
  navHasBlurb: !!document.querySelector('.subnav-group-blurb'),
  navText: (document.querySelector('.settings-subnav')?.textContent || '').trim(),
  catTitle: document.querySelector('.category-context-title')?.textContent?.trim() ?? null,
  catBlurb: document.querySelector('.category-context-blurb')?.textContent?.trim() ?? null,
}))()`);
console.log('导航快照:', JSON.stringify(snapshot, null, 2));

const failures = [];
if (!snapshot || snapshot.groupBtns.length !== 5) failures.push(`侧栏应只列 5 个大类（实得 ${snapshot?.groupBtns?.length}）`);
if (!snapshot || snapshot.pageRows.length < 1) failures.push('当前大类应展开其分区页');
if (snapshot?.navHasBlurb) failures.push('导航列不应再有描述词（subnav-group-blurb）');
if (snapshot && /它靠谁|它是谁|它能做什么|怎么布置|落在哪/.test(snapshot.navText)) failures.push('导航列文本仍混入大类描述词');
if (!snapshot?.catBlurb) failures.push('页面顶部应有大类描述词（category-context-blurb）');

await shot('settings-nav-live-appearance.png');

// 切到「能力与安全」大类：展开应跟随到 4 个分区页。
const switched = await evalJs(`(() => {
  const b = [...document.querySelectorAll('.subnav-group-btn')].find((x) => x.textContent.includes('能力与安全'));
  if (!b) return 'NO_BTN';
  b.click();
  return 'clicked';
})()`);
console.log('切大类:', switched);
await sleep(800);
const after = await evalJs(`(() => {
  const grp = [...document.querySelectorAll('.subnav-group')].find((g) => g.querySelector('.subnav-group-btn')?.textContent.includes('能力与安全'));
  return {
    pageRows: grp ? [...grp.querySelectorAll('.subnav-btn')].map((b) => b.textContent.trim()) : [],
    catTitle: document.querySelector('.category-context-title')?.textContent?.trim() ?? null,
    catBlurb: document.querySelector('.category-context-blurb')?.textContent?.trim() ?? null,
  };
})()`);
console.log('切换后快照:', JSON.stringify(after, null, 2));
if (!after || after.pageRows.length !== 4) failures.push(`能力与安全应展开 4 个分区页（实得 ${after?.pageRows?.length}）`);
if (after && after.catTitle !== '能力与安全') failures.push(`页面顶部大类名应跟随切换（实得 ${after?.catTitle}）`);

if (switched === 'clicked') await shot('settings-nav-live-capability.png');
else await shot('settings-nav-live-appearance.png');

// ⑥ 多开多收：多个大类可同时展开并存；再点其中一个即收起，别的不动（不自动收放）。
const multi = await evalJs(`(async () => {
  const btns = [...document.querySelectorAll('.subnav-group-btn')];
  const find = (t) => btns.find((b) => b.textContent.includes(t));
  const app = find('外观'), cap = find('能力与安全'), data = find('数据与工作区');
  if (!app || !cap || !data) return {error: 'NO_BTN'};
  const state = () => ({app: app.getAttribute('aria-expanded'), cap: cap.getAttribute('aria-expanded'), data: data.getAttribute('aria-expanded')});
  if (data.getAttribute('aria-expanded') !== 'true') { data.click(); await new Promise((r) => setTimeout(r, 200)); }
  const bothOpen = state();
  data.click(); await new Promise((r) => setTimeout(r, 200));
  return {bothOpen, afterReclick: state()};
})()`);
console.log('多开多收:', JSON.stringify(multi, null, 2));
if (!multi || multi.bothOpen?.app !== 'true' || multi.bothOpen?.cap !== 'true' || multi.bothOpen?.data !== 'true') failures.push('多个大类应同时展开（多开并存）');
if (!multi || multi.afterReclick?.data !== 'false' || multi.afterReclick?.app !== 'true' || multi.afterReclick?.cap !== 'true') failures.push('再点应收起自己、且不动别的大类（不自动收放）');
await shot('settings-nav-live-multi-open.png');

// ⑦ 全开总高预算（980×640 最小窗）：子导航不自滚（滚轮契约），全开必须原生放得下。
await ws.send('Emulation.setDeviceMetricsOverride', {width: 980, height: 640, deviceScaleFactor: 1, mobile: false});
await sleep(300);
const fit = await evalJs(`(async () => {
  for (const b of [...document.querySelectorAll('.subnav-group-btn')]) {
    if (b.getAttribute('aria-expanded') !== 'true') { b.click(); await new Promise((r) => setTimeout(r, 150)); }
  }
  const aside = document.querySelector('.settings-subnav');
  return {clientHeight: aside.clientHeight, scrollHeight: aside.scrollHeight, fits: aside.scrollHeight <= aside.clientHeight};
})()`);
console.log('全开总高（980×640）:', JSON.stringify(fit));
if (!fit || !fit.fits) failures.push(`全开应原生放得下（scrollHeight ${fit?.scrollHeight} > clientHeight ${fit?.clientHeight}）`);
await ws.send('Emulation.clearDeviceMetricsOverride', {});

if (failures.length) {
  console.error('验收失败:');
  for (const f of failures) console.error('  -', f);
  process.exit(1);
}
console.log('--- 运行中程序里的设置页两级导航验收全部通过 ---');
ws.close();
