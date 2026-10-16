// P9d 诊断：day 主题白屏
import {spawn} from 'node:child_process';
const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9411;
const BASE = 'http://127.0.0.1:1420/';
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
const edge = spawn(EDGE, ['--headless=new', `--remote-debugging-port=${DEBUG_PORT}`, '--user-data-dir=' + process.env.TEMP + '\\apeireth-p9d-diag-' + Date.now(), '--window-size=1600,900', '--hide-scrollbars', '--lang=zh-CN', 'about:blank'], {stdio: 'ignore'});
process.on('exit', () => { try { edge.kill(); } catch {} });
let ws;
for (let i = 0; i < 30; i++) { await sleep(500); try { const list = await (await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/list`)).json(); const page = list.find((t) => t.type === 'page'); if (page) { ws = await connect(page.webSocketDebuggerUrl); break; } } catch {} }
if (!ws) { console.error('FATAL'); edge.kill(); process.exit(1); }
await ws.send('Page.enable');
await ws.send('Runtime.enable');
await ws.send('Log.enable');
const logs = [];
ws.socket?.addEventListener?.('message', () => {});
// 直接收事件：用原始 socket 不便，改为轮询 evaluate
await ws.send('Page.navigate', {url: BASE + '?theme=day'});
await sleep(6000);
const r = await ws.send('Runtime.evaluate', {expression: `(function(){
  var app = document.querySelector('.app-root');
  var shell = document.querySelector('.shell');
  var staticBg = document.querySelector('.static-bg');
  return JSON.stringify({
    appRootExists: !!app,
    appClasses: app ? app.className : null,
    shellExists: !!shell,
    bodyChildren: document.body.children.length,
    htmlAttr: document.documentElement.getAttribute('data-theme'),
    err: window.__lastError || null
  });
})()`, returnByValue: true});
console.log('DOM:', r.result?.value);
// console errors via Runtime console API — 直接再导航一次抓 exception
const r2 = await ws.send('Runtime.evaluate', {expression: `(function(){
  try {
    return 'no-sync-error';
  } catch(e) { return String(e); }
})()`, returnByValue: true});
ws.close();
edge.kill();
process.exit(0);
