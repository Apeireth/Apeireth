// P5 实证：面板打开时 scrim 是否真的盖在面板上（点击拦截 + 层叠顺序）
import {spawn} from 'node:child_process';
const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9405;
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
const edge = spawn(EDGE, ['--headless=new', `--remote-debugging-port=${DEBUG_PORT}`, '--user-data-dir=' + process.env.TEMP + '\\apeireth-p5-diag-' + Date.now(), '--window-size=1600,900', '--hide-scrollbars', '--lang=zh-CN', 'about:blank'], {stdio: 'ignore'});
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
const r = await ws.send('Runtime.evaluate', {expression: `(() => {
  document.querySelector('.cap-btn.pill-model')?.click();
  return 'opened';
})()`, returnByValue: true});
await sleep(900);
const raw = await ws.send('Runtime.evaluate', {expression: `(function(){ try {
  var p = document.querySelector('.panel.show');
  if (!p) return {err: 'panel not open'};
  var r = p.getBoundingClientRect();
  var cx = r.left + r.width / 2, cy = r.top + r.height / 2;
  var hit = document.elementFromPoint(cx, cy);
  var chain = [];
  var el = p;
  while (el && el !== document.body) {
    var cs = getComputedStyle(el);
    if (cs.position !== 'static' || cs.zIndex !== 'auto' || cs.backdropFilter !== 'none' || cs.transform !== 'none' || cs.filter !== 'none' || cs.opacity !== '1') {
      chain.push(String(el.className).slice(0, 30) + ' | pos=' + cs.position + ' z=' + cs.zIndex + ' bf=' + (cs.backdropFilter !== 'none'));
    }
    el = el.parentElement;
  }
  return {
    hit: hit ? hit.tagName + '.' + String(hit.className).slice(0, 40) : null,
    hitIsScrim: !!(hit && hit.id === 'scrim'),
    panelRect: {x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height)},
    chain: chain
  };
} catch (e) { return {threw: String(e)}; } })()`, returnByValue: true});
console.log(JSON.stringify(raw.result?.value ?? raw, null, 1));
ws.close();
edge.kill();
process.exit(0);
