// P6 视觉验收：essence 面板清晰度 / 保存栏双模式 / ctx 面板
import {spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';
const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9406;
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
const edge = spawn(EDGE, ['--headless=new', `--remote-debugging-port=${DEBUG_PORT}`, '--user-data-dir=' + process.env.TEMP + '\\apeireth-p6-verify-' + Date.now(), '--window-size=1600,900', '--hide-scrollbars', '--lang=zh-CN', 'about:blank'], {stdio: 'ignore'});
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
async function evl(expr) {
  const r = await ws.send('Runtime.evaluate', {expression: expr, returnByValue: true});
  return r.result?.value;
}

// ① essence：M 面板 + ◯ 面板
await ws.send('Page.navigate', {url: BASE + '?theme=essence'});
await sleep(6000);
await evl(`localStorage.setItem('apeireth-first-run-done','1'); 'ok'`);
await ws.send('Page.navigate', {url: BASE + '?theme=essence'});
await sleep(6000);
await evl(`document.querySelector('.cap-btn.pill-model')?.click(); 'ok'`);
await sleep(900);
await shot('p6-essence-model-panel.png');
await evl(`document.querySelector('.cap-btn.pill-model')?.click(); 'ok'`);
await sleep(400);
await evl(`document.querySelector('.cap-btn.pill-ctx')?.click(); 'ok'`);
await sleep(900);
await shot('p6-essence-ctx-panel.png');
await evl(`document.querySelector('.cap-btn.pill-ctx')?.click(); 'ok'`);
await sleep(400);

// ② essence：设置保存栏（浅色应亮）
await ws.send('Page.navigate', {url: BASE + '?theme=essence&drawer=settings'});
await sleep(5000);
await evl(`(() => { const c = document.querySelector('.settings-content'); if (c) c.scrollTop = c.scrollHeight; return 'ok'; })()`);
await sleep(700);
await shot('p6-essence-savebar.png');

// ③ night（默认）：设置保存栏（深色应保持）
await ws.send('Page.navigate', {url: BASE + '?drawer=settings'});
await sleep(5000);
await evl(`(() => { const c = document.querySelector('.settings-content'); if (c) c.scrollTop = c.scrollHeight; return 'ok'; })()`);
await sleep(700);
await shot('p6-night-savebar.png');
ws.close();
edge.kill();
process.exit(0);
