// P9 四主题补全视觉验收：day/paper/ocean/forest × 主页+设置+抽屉
import {spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';
const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9410;
const BASE = 'http://127.0.0.1:1420/';
const OUT = 'C:\\Users\\31683\\Apeireth-rust\\artifacts\\';
function connect(ws) {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(ws);
    let id = 0;
    const pending = new Map();
    socket.onopen = () => resolve({send(m, p = {}) { return new Promise((res, rej) => { const i = ++id; pending.set(i, {res, rej}); socket.send(JSON.stringify({id: i, method: m, params: p})); }); }, close: () => socket.close()});
    socket.onmessage = (e) => { const msg = JSON.parse(String(e.data)); if (msg.id && pending.has(msg.id)) { const {res, rej} = pending.get(msg.id); pending.delete(msg.id); msg.error ? rej(new Error(JSON.stringify(msg.error))) : res(msg.result); } };
    socket.onerror = reject;
  });
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const edge = spawn(EDGE, ['--headless=new', `--remote-debugging-port=${DEBUG_PORT}`, '--user-data-dir=' + process.env.TEMP + '\\apeireth-p9-themes-' + Date.now(), '--window-size=1600,900', '--hide-scrollbars', '--lang=zh-CN', 'about:blank'], {stdio: 'ignore'});
process.on('exit', () => { try { edge.kill(); } catch {} });
let ws;
for (let i = 0; i < 30; i++) { await sleep(500); try { const list = await (await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/list`)).json(); const page = list.find((t) => t.type === 'page'); if (page) { ws = await connect(page.webSocketDebuggerUrl); break; } } catch {} }
if (!ws) { console.error('FATAL'); edge.kill(); process.exit(1); }
await ws.send('Page.enable');
await ws.send('Emulation.setDeviceMetricsOverride', {width: 1600, height: 900, deviceScaleFactor: 1, mobile: false});

async function shot(name) {
  const png = await ws.send('Page.captureScreenshot', {format: 'png'});
  writeFileSync(OUT + name, Buffer.from(png.data, 'base64'));
  console.log('saved', name);
}
// 首启标记（一次即可）
await ws.send('Page.navigate', {url: BASE});
await sleep(5000);
await ws.send('Runtime.evaluate', {expression: `localStorage.setItem('apeireth-first-run-done','1'); 'ok'`});

for (const t of ['day', 'paper', 'ocean', 'forest']) {
  // 主页（composer 全览）
  await ws.send('Page.navigate', {url: `${BASE}?theme=${t}`});
  await sleep(5500);
  await shot(`p9-${t}-home.png`);
  // 设置（主题卡片墙 + 保存栏）
  await ws.send('Page.navigate', {url: `${BASE}?theme=${t}&drawer=settings`});
  await sleep(5000);
  await shot(`p9-${t}-settings.png`);
  // 一个内容抽屉（记忆卷宗）
  await ws.send('Page.navigate', {url: `${BASE}?theme=${t}&drawer=memory`});
  await sleep(5500);
  await shot(`p9-${t}-memory.png`);
}
ws.close();
edge.kill();
process.exit(0);
