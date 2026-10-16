// ⑤ 底部状态条 自查截图（scratch，不提交）：Edge headless CDP。
// 前置：vite(1420) 与 mock-gateway(8080) 都已起（两种模式都要——
//       lost 模式必须先让 mock 活着完成首载，capabilities 到手、
//       subscribePresence 建立后再由脚本杀 mock，否则 SIM 永不置位）。
// 用法: node k5-shot.mjs quiet   —— heritage 默认 + night 场景主题各一张
//       node k5-shot.mjs lost    —— 连通基线后杀 mock，截 重连中(黄) 与 断连+SIM(红)
import {exec, spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {dirname, join} from 'node:path';

const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = Number(process.env.K5_DEBUG_PORT || 9385);
const APP = 'http://127.0.0.1:1420/';
const OUT = dirname(fileURLToPath(import.meta.url));
const MODE = process.argv[2] || 'quiet';

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

// 杀 8080 真监听者（pids.txt 记的是启动壳 PID，不靠谱；按端口找 OwningProcess）
function killMock() {
  return new Promise((resolve) => {
    const ps = '$c = Get-NetTCPConnection -LocalPort 8080 -State Listen -ErrorAction SilentlyContinue | Select-Object -First 1; ' +
      'if ($c) { Stop-Process -Id $c.OwningProcess -Force; Write-Output killed } else { Write-Output none }';
    exec(`powershell -NoProfile -Command "${ps}"`, (err, stdout) => {
      console.log('[kill-mock]', String(stdout).trim() || (err && err.message));
      resolve();
    });
  });
}

const edge = spawn(EDGE, [
  '--headless=new',
  `--remote-debugging-port=${DEBUG_PORT}`,
  '--user-data-dir=' + process.env.TEMP + '\\k5-shot-profile-' + MODE,
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

const PROBE = `(() => {
  const bar = document.querySelector('.status-bar');
  if (!bar) return JSON.stringify({error: 'no-status-bar'});
  return JSON.stringify({
    theme: document.documentElement.getAttribute('data-theme'),
    items: [...bar.querySelectorAll('.sb-item')].map((el) => ({
      text: el.textContent.trim(),
      tone: [...el.classList].find((c) => c.startsWith('tone-')),
      clickable: el.tagName === 'BUTTON',
      sim: !!el.querySelector('.sim-badge'),
    })),
  });
})()`;

async function probe(name) { console.log(`[${name}]`, await ws.send('Runtime.evaluate', {expression: PROBE}).then((r) => r.result.value)); }
async function probeJson() {
  const v = await ws.send('Runtime.evaluate', {expression: PROBE}).then((r) => r.result.value);
  try { return JSON.parse(v); } catch { return {}; }
}
async function shot(name) {
  const data = await ws.send('Page.captureScreenshot', {format: 'png'});
  writeFileSync(join(OUT, name), Buffer.from(data.data, 'base64'));
  console.log('saved', name);
}
// mock 的 SSE 一帧后 ~300ms 收口 → 订阅周期性「连通~0.4s / 退避 2-30s」抖动。
// 截「已连接」静稳态需轮询逮住连通窗口立即截图（生产 canonical 长连接不收口，此为夹具限制）。
async function shotWhenConnected(name, timeoutMs = 60000) {
  const t0 = Date.now();
  for (;;) {
    const p = await probeJson();
    if (p.items && p.items[0] && p.items[0].text === 'SSE 已连接') {
      await shot(name);
      console.log(`[${name}] caught connected window`, JSON.stringify(p.items));
      return;
    }
    if (Date.now() - t0 > timeoutMs) {
      console.log(`[${name}] TIMEOUT — shooting anyway`, JSON.stringify(p.items || p));
      await shot(name);
      return;
    }
    await sleep(150);
  }
}

if (MODE === 'quiet') {
  // heritage 默认主题（无覆写）—— 先等首载数据到位（守卫/记忆计数），再逮 SSE 连通窗口
  await ws.send('Page.navigate', {url: `${APP}?_cb=${Date.now()}`});
  await sleep(8000);
  await probe('heritage-pre');
  await shotWhenConnected('k5-sb-heritage.png');
  // night 场景主题（?theme=night；night 是 CSS 默认态 → data-theme 属性被移除，探针报 null 为正常）
  await ws.send('Page.navigate', {url: `${APP}?theme=night&_cb=${Date.now()}`});
  await sleep(8000);
  await probe('night-pre');
  await shotWhenConnected('k5-sb-night.png');
} else {
  // lost：mock 活着首载 → capabilities 到手 + presence 订阅建立（直连死连时
  // capabilities 恒 null → 不订阅 → simulated 永不置位，截不到 SIM）→ 再杀 mock
  await ws.send('Page.navigate', {url: `${APP}?_cb=${Date.now()}`});
  await sleep(9000);
  await probe('connected-baseline');
  await killMock();
  await sleep(6000);
  await probe('retrying-6s'); // 退避重连中（黄）
  await shot('k5-sb-retrying.png');
  await sleep(30000);
  await probe('lost-36s'); // 断连 > SIM_AFTER_MS(30s) → SIM（红）
  await shot('k5-sb-lost.png');
}

edge.kill();
console.log('done');
process.exit(0);
