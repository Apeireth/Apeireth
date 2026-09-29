// 治理卷宗四 tab 自查截图（scratch，不提交）：Edge headless CDP。
// 前置：mock-gateway(8080) 与 vite(1420) 已起。用法: node gov-shot.mjs [outPrefix]
// 对每个 tab：导航 ?drawer=governance&govtab=<tab> → 等首载 → DOM 探针 → 截图。
import {spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {dirname, join} from 'node:path';

const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = Number(process.env.GOV_DEBUG_PORT || 9345);
const APP = 'http://127.0.0.1:1420/';
const OUT = dirname(fileURLToPath(import.meta.url));
const PREFIX = process.argv[2] || 'tab';

const TABS = process.argv[3] ? process.argv[3].split(',') : ['approvals', 'grants', 'guard', 'audit'];

function connect(ws) {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(ws);
    let id = 0;
    const pending = new Map();
    socket.onopen = () => resolve({
      send(method, params = {}) {
        return new Promise((res, rej) => {
          const msgId = ++id;
          pending.set(msgId, {res, rej});
          socket.send(JSON.stringify({id: msgId, method, params}));
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

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// 通用探针：不看实现细节类名，只数「有内容的卡片/行」与关键字样
const PROBE = `(() => {
  const view = document.querySelector('.governance-view');
  if (!view) return JSON.stringify({error: 'no-governance-view', body: document.body.innerText.slice(0, 120)});
  const txt = view.innerText;
  return JSON.stringify({
    tab: view.querySelector('[aria-selected="true"], .tab.active, .gov-tab.active')?.textContent?.trim() || '?',
    cards: view.querySelectorAll('.gov-card, .approval-card, .grant-card, .event-row, .trace-row').length,
    ssePill: (document.querySelector('.sse-pill, .conn-pill')?.textContent || 'none').trim(),
    hasEmpty: /暂无|没有|空/.test(txt),
    textHead: txt.slice(0, 200),
  });
})()`;

const edge = spawn(EDGE, [
  '--headless=new',
  `--remote-debugging-port=${DEBUG_PORT}`,
  '--user-data-dir=' + process.env.TEMP + '\\gov-shot-profile-' + PREFIX,
  '--window-size=1440,900',
  '--hide-scrollbars',
  '--disable-gpu',
  'about:blank',
], {stdio: 'ignore'});

process.on('exit', () => { try { edge.kill(); } catch {} });

let ws;
for (let i = 0; i < 30; i++) {
  await sleep(500);
  try {
    const list = await (await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/list`)).json();
    const page = list.find((t) => t.type === 'page');
    if (page) { ws = await connect(page.webSocketDebuggerUrl); break; }
  } catch { /* retry */ }
}
if (!ws) { console.error('FATAL: no CDP'); edge.kill(); process.exit(1); }

await ws.send('Page.enable');
await ws.send('Runtime.enable');
await ws.send('Emulation.setDeviceMetricsOverride', {width: 1440, height: 900, deviceScaleFactor: 1, mobile: false});

for (const tab of TABS) {
  await ws.send('Page.navigate', {url: `${APP}?drawer=governance&govtab=${tab}&_cb=${Date.now()}`});
  await sleep(12000); // health(含提供商探测 5s 超时) → capabilities → tab 首载 + 字体
  const probe = await ws.send('Runtime.evaluate', {expression: PROBE});
  console.log(`[${tab}] probe:`, probe.result.value);
  const shot = await ws.send('Page.captureScreenshot', {format: 'png'});
  const file = join(OUT, `${PREFIX}-${tab}.png`);
  writeFileSync(file, Buffer.from(shot.data, 'base64'));
  console.log(`[${tab}] saved ${file}`);
}

ws.close();
edge.kill();
console.log('done');
process.exit(0);
