import {spawn} from 'node:child_process';
const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9403;
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
const edge = spawn(EDGE, ['--headless=new', `--remote-debugging-port=${DEBUG_PORT}`, '--user-data-dir=' + process.env.TEMP + '\\apeireth-p2-diag', '--window-size=2549,1352', '--hide-scrollbars', 'about:blank'], {stdio: 'ignore'});
process.on('exit', () => { try { edge.kill(); } catch {} });
let ws;
for (let i = 0; i < 30; i++) { await sleep(500); try { const list = await (await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/list`)).json(); const page = list.find((t) => t.type === 'page'); if (page) { ws = await connect(page.webSocketDebuggerUrl); break; } } catch {} }
if (!ws) { console.error('FATAL'); edge.kill(); process.exit(1); }
await ws.send('Page.enable');
await ws.send('Emulation.setDeviceMetricsOverride', {width: 2549, height: 1352, deviceScaleFactor: 1, mobile: false});
await ws.send('Page.navigate', {url: 'http://127.0.0.1:1420/'});
await sleep(5000);
await ws.send('Runtime.evaluate', {expression: `localStorage.setItem('apeireth-first-run-done','1'); 'ok'`});
await ws.send('Page.navigate', {url: 'http://127.0.0.1:1420/'});
await sleep(6000);
await ws.send('Runtime.evaluate', {expression: `document.querySelector('.cap-btn.pill-model').click(); 'ok'`});
await sleep(800);
const diag = await ws.send('Runtime.evaluate', {expression: `(() => {
  const p = document.querySelector('#panel-model');
  if (!p) return 'NO_PANEL';
  const r = p.getBoundingClientRect();
  const cs = getComputedStyle(p);
  const chain = [];
  let el = p;
  while (el && chain.length < 7) {
    const rr = el.getBoundingClientRect();
    chain.push((el.className.toString().slice(0, 30) || el.tagName) + ' | ' + Math.round(rr.width) + 'x' + Math.round(rr.height) + ' @' + Math.round(rr.left) + ',' + Math.round(rr.top) + ' | pos=' + getComputedStyle(el).position + ' bf=' + (getComputedStyle(el).backdropFilter || getComputedStyle(el).webkitBackdropFilter));
    el = el.parentElement;
  }
  return JSON.stringify({panel: {w: Math.round(r.width), h: Math.round(r.height), x: Math.round(r.left), y: Math.round(r.top), display: cs.display, bf: cs.backdropFilter}, chain}, null, 1);
})()`, returnByValue: true});
console.log(diag.result.value);
ws.close();
edge.kill();
process.exit(0);
