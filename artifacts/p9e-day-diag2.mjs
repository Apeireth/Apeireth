// P9e 诊断 round2：白屏顶层元素 + 关键元素计算样式
import {spawn} from 'node:child_process';
const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9412;
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
const edge = spawn(EDGE, ['--headless=new', `--remote-debugging-port=${DEBUG_PORT}`, '--user-data-dir=' + process.env.TEMP + '\\apeireth-p9e-diag-' + Date.now(), '--window-size=1600,900', '--hide-scrollbars', '--lang=zh-CN', 'about:blank'], {stdio: 'ignore'});
process.on('exit', () => { try { edge.kill(); } catch {} });
let ws;
for (let i = 0; i < 30; i++) { await sleep(500); try { const list = await (await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/list`)).json(); const page = list.find((t) => t.type === 'page'); if (page) { ws = await connect(page.webSocketDebuggerUrl); break; } } catch {} }
if (!ws) { console.error('FATAL'); edge.kill(); process.exit(1); }
await ws.send('Page.enable');
await ws.send('Page.navigate', {url: BASE + '?theme=day'});
await sleep(6000);
const r = await ws.send('Runtime.evaluate', {expression: `(function(){
  var px = Math.floor(window.innerWidth / 2), py = Math.floor(window.innerHeight / 2);
  var hit = document.elementFromPoint(px, py);
  var chain = [];
  chain.push('viewport ' + window.innerWidth + 'x' + window.innerHeight);
  var el = hit;
  while (el && el !== document.documentElement) {
    var cs = getComputedStyle(el);
    chain.push(String(el.className).slice(0,40) + ' z=' + cs.zIndex + ' bg=' + cs.backgroundColor + (cs.backgroundImage !== 'none' ? ' IMG' : ''));
    el = el.parentElement;
  }
  var vignette = document.querySelector('#vignette');
  var vcs = vignette ? getComputedStyle(vignette) : null;
  return JSON.stringify({
    hit: hit ? hit.tagName + '.' + String(hit.className).slice(0,40) : null,
    chain: chain,
    vignette: vcs ? {z: vcs.zIndex, bg: vcs.backgroundImage.slice(0, 80), opacity: vcs.opacity} : null
  }, null, 1);
})()`, returnByValue: true});
console.log(r.result?.value);
ws.close();
edge.kill();
process.exit(0);
