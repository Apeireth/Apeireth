// 诊断：页面里直接起 EventSource 打 mock SSE，看 open/error 时序（scratch，不提交）
import {spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {dirname} from 'node:path';

const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9386;
const APP = 'http://127.0.0.1:1420/';
const OUT = dirname(fileURLToPath(import.meta.url));
void OUT;

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

const edge = spawn(EDGE, [
  '--headless=new', `--remote-debugging-port=${DEBUG_PORT}`,
  '--user-data-dir=' + process.env.TEMP + '\\k5-diag-profile',
  '--window-size=1200,800', '--disable-gpu', 'about:blank',
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

await ws.send('Page.navigate', {url: `${APP}?_cb=${Date.now()}`});
await sleep(6000);

// 直连 8080 起 EventSource，记 12 秒时序
await ws.send('Runtime.evaluate', {expression: `
  window.__eslog = [];
  window.__es = new EventSource('http://127.0.0.1:8080/v1/apeireth/events');
  __es.onopen = () => __eslog.push('open ' + (Date.now() % 100000));
  __es.onerror = () => __eslog.push('err  ' + (Date.now() % 100000));
  __es.onmessage = (m) => __eslog.push('msg  ' + (Date.now() % 100000) + ' ' + m.data.slice(0, 40));
  'armed'
`});
await sleep(12000);
const log = await ws.send('Runtime.evaluate', {expression: 'JSON.stringify(window.__eslog)'});
console.log('eslog:', log.result.value);

// 顺带看 config.baseUrl 实际指向
const cfg = await ws.send('Runtime.evaluate', {expression: `localStorage.getItem('apeireth.config') || 'no-config-key'`});
console.log('cfg:', String(cfg.result.value).slice(0, 300));

edge.kill();
process.exit(0);
