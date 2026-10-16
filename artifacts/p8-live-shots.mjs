// P8 真数据浅色验收：gateway :8080 + vite :1420，essence 全视图连拍
import {spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';
const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9409;
const BASE = 'http://127.0.0.1:1420/';
const GW = 'http://127.0.0.1:8080';
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
const edge = spawn(EDGE, ['--headless=new', `--remote-debugging-port=${DEBUG_PORT}`, '--user-data-dir=' + process.env.TEMP + '\\apeireth-p8-live-' + Date.now(), '--window-size=1600,900', '--hide-scrollbars', '--lang=zh-CN', 'about:blank'], {stdio: 'ignore'});
process.on('exit', () => { try { edge.kill(); } catch {} });
let ws;
for (let i = 0; i < 30; i++) { await sleep(500); try { const list = await (await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/list`)).json(); const page = list.find((t) => t.type === 'page'); if (page) { ws = await connect(page.webSocketDebuggerUrl); break; } } catch {} }
if (!ws) { console.error('FATAL'); edge.kill(); process.exit(1); }
await ws.send('Page.enable');
await ws.send('Emulation.setDeviceMetricsOverride', {width: 1600, height: 900, deviceScaleFactor: 1, mobile: false});

// gateway 侧数据面快拍（证据用）
const health = await (await fetch(GW + '/health')).json();
console.log('gateway health:', health.status);
try {
  const grants = await (await fetch(GW + '/v1/apeireth/governance/grants')).text();
  console.log('grants bytes:', grants.length);
} catch (e) { console.log('grants err', String(e)); }

await ws.send('Page.navigate', {url: BASE + '?theme=essence'});
await sleep(6000);
await ws.send('Runtime.evaluate', {expression: `localStorage.setItem('apeireth-first-run-done','1'); 'ok'`});

async function shot(name) {
  const png = await ws.send('Page.captureScreenshot', {format: 'png'});
  writeFileSync(OUT + name, Buffer.from(png.data, 'base64'));
  console.log('saved', name);
}
const targets = [
  ['history', 'p8-essence-history.png', 5000],
  ['memory', 'p8-essence-memory.png', 6000],
  ['diary', 'p8-essence-diary.png', 5000],
  ['tools', 'p8-essence-tools.png', 6000],
  ['governance&govtab=approvals', 'p8-essence-gov-approvals.png', 6000],
  ['governance&govtab=grants', 'p8-essence-gov-grants.png', 6000],
  ['governance&govtab=guard', 'p8-essence-gov-guard.png', 6000],
  ['governance&govtab=audit', 'p8-essence-gov-audit.png', 6000],
  ['status', 'p8-essence-status.png', 6000],
  ['logs', 'p8-essence-logs.png', 6000],
];
for (const [q, name, wait] of targets) {
  await ws.send('Page.navigate', {url: `${BASE}?theme=essence&drawer=${q}`});
  await sleep(wait);
  await shot(name);
}
ws.close();
edge.kill();
process.exit(0);
