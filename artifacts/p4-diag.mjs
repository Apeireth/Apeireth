// P4 诊断：essence 浅色下 M/◯ 面板发虚实证 + 保存栏深色实证
import {spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';
const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9404;
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
const edge = spawn(EDGE, ['--headless=new', `--remote-debugging-port=${DEBUG_PORT}`, '--user-data-dir=' + process.env.TEMP + '\\apeireth-p4-diag-' + Date.now(), '--window-size=1600,900', '--hide-scrollbars', '--lang=zh-CN', 'about:blank'], {stdio: 'ignore'});
process.on('exit', () => { try { edge.kill(); } catch {} });
let ws;
for (let i = 0; i < 30; i++) { await sleep(500); try { const list = await (await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/list`)).json(); const page = list.find((t) => t.type === 'page'); if (page) { ws = await connect(page.webSocketDebuggerUrl); break; } } catch {} }
if (!ws) { console.error('FATAL'); edge.kill(); process.exit(1); }
await ws.send('Page.enable');
await ws.send('Emulation.setDeviceMetricsOverride', {width: 1600, height: 900, deviceScaleFactor: 1, mobile: false});
await ws.send('Page.navigate', {url: BASE + '?theme=essence'});
await sleep(6000);
await ws.send('Runtime.evaluate', {expression: `localStorage.setItem('apeireth-first-run-done','1'); 'ok'`});
await ws.send('Page.navigate', {url: BASE + '?theme=essence'});
await sleep(6000);

async function shot(name) {
  const png = await ws.send('Page.captureScreenshot', {format: 'png'});
  writeFileSync(OUT + name, Buffer.from(png.data, 'base64'));
  console.log('saved', name);
}
async function evl(expr) {
  const r = await ws.send('Runtime.evaluate', {expression: expr, returnByValue: true});
  return r.result.value;
}

// 打开 M 面板并 dump 面板内文字计算样式
await evl(`document.querySelector('.cap-btn.pill-model')?.click(); 'ok'`);
await sleep(900);
console.log(JSON.stringify(await evl(`(() => {
  const p = document.querySelector('.panel.show') || document.querySelector('.panel');
  if (!p) return {err: 'no panel'};
  const cs = getComputedStyle(p);
  const first = p.querySelector('h2, .kv, .model, li, span, div');
  const fcs = first ? getComputedStyle(first) : null;
  return {
    panelBg: cs.backgroundColor,
    panelFilter: cs.backdropFilter || cs.webkitBackdropFilter,
    panelOpacity: cs.opacity,
    rect: p.getBoundingClientRect().toJSON(),
    firstText: first ? first.textContent.slice(0, 20) : null,
    firstColor: fcs ? fcs.color : null,
    firstOpacity: fcs ? fcs.opacity : null,
    firstFilter: fcs ? (fcs.filter || 'none') : null,
  };
})()`), null, 1));
await shot('p4-essence-model-panel.png');

// 保存栏计算样式（设置 drawer，essence 下）
await evl(`document.querySelector('.cap-btn.pill-model')?.click(); 'ok'`);
await sleep(400);
await ws.send('Page.navigate', {url: BASE + '?theme=essence&drawer=settings'});
await sleep(5000);
console.log(JSON.stringify(await evl(`(() => {
  const b = document.querySelector('.settings-save-bar');
  if (!b) return {err: 'no save bar'};
  const cs = getComputedStyle(b);
  return {bg: cs.backgroundColor, border: cs.borderColor, rect: b.getBoundingClientRect().toJSON()};
})()`), null, 1));
await shot('p4-essence-savebar.png');
ws.close();
edge.kill();
process.exit(0);
