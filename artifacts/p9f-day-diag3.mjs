// P9f 诊断 round3：visibility / pointer-events / 全屏覆盖层排查 + 截图
import {spawn} from 'node:child_process';
const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9413;
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
const edge = spawn(EDGE, ['--headless=new', `--remote-debugging-port=${DEBUG_PORT}`, '--user-data-dir=' + process.env.TEMP + '\\apeireth-p9f-diag-' + Date.now(), '--window-size=1600,900', '--hide-scrollbars', '--lang=zh-CN', 'about:blank'], {stdio: 'ignore'});
process.on('exit', () => { try { edge.kill(); } catch {} });
let ws;
for (let i = 0; i < 30; i++) { await sleep(500); try { const list = await (await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/list`)).json(); const page = list.find((t) => t.type === 'page'); if (page) { ws = await connect(page.webSocketDebuggerUrl); break; } } catch {} }
if (!ws) { console.error('FATAL'); edge.kill(); process.exit(1); }
await ws.send('Page.enable');
await ws.send('Page.navigate', {url: BASE + '?theme=day'});
await sleep(6000);
const r = await ws.send('Runtime.evaluate', {expression: `(function(){
  function probe(sel) {
    var el = document.querySelector(sel);
    if (!el) return sel + ': MISSING';
    var cs = getComputedStyle(el);
    var rect = el.getBoundingClientRect();
    return sel + ': vis=' + cs.visibility + ' pe=' + cs.pointerEvents + ' disp=' + cs.display +
      ' rect=' + Math.round(rect.width) + 'x' + Math.round(rect.height) +
      ' bg=' + cs.backgroundColor + (cs.backgroundImage !== 'none' ? ' IMG' : '');
  }
  var out = [];
  ['html', 'body', '#app', 'app-root', '.static-bg', '#vignette', '.shell', '#scene-underlay', '#presence'].forEach(function(s){ out.push(probe(s)); });
  var all = document.querySelectorAll('*');
  out.push('total elements: ' + all.length);
  var fixed = [];
  all.forEach(function(el){
    var cs = getComputedStyle(el);
    if (cs.position === 'fixed' && cs.display !== 'none') {
      var rect = el.getBoundingClientRect();
      if (rect.width > 500 && rect.height > 300) {
        fixed.push(String(el.tagName).toLowerCase() + '.' + String(el.className).slice(0,30) + ' z=' + cs.zIndex + ' vis=' + cs.visibility + ' bg=' + cs.backgroundColor + (cs.backgroundImage !== 'none' ? ' IMG' : ''));
      }
    }
  });
  out.push('--- large fixed overlays ---');
  fixed.slice(0, 20).forEach(function(f){ out.push(f); });
  return out.join('\\n');
})()`, returnByValue: true});
console.log(r.result?.value);
// 截图
const shot = await ws.send('Page.captureScreenshot', {format: 'png'});
const {writeFileSync} = await import('node:fs');
writeFileSync('artifacts/p9f-day-diag.png', Buffer.from(shot.data, 'base64'));
ws.close();
edge.kill();
process.exit(0);
