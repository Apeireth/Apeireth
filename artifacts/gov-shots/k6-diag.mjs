// k6 诊断：读 memory 抽屉实际渲染内容与页面错误（scratch）
import {spawn} from 'node:child_process';

const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9388;
const APP = 'http://127.0.0.1:1420/';

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
  '--user-data-dir=' + process.env.TEMP + '\\k6-diag-profile',
  '--window-size=1440,900', '--disable-gpu', 'about:blank',
], {stdio: 'ignore'});
process.on('exit', () => { try { edge.kill(); } catch {} });

let ws;
const consoleLogs = [];
for (let i = 0; i < 30; i++) {
  await sleep(500);
  try {
    const list = await (await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/list`)).json();
    const page = list.find((t) => t.type === 'page');
    if (page) { ws = await connect(page.webSocketDebuggerUrl); break; }
  } catch { /* retry */ }
}
if (!ws) { console.error('FATAL no CDP'); process.exit(1); }
await ws.send('Page.enable');
await ws.send('Runtime.enable');

// 收 console 与异常
const sock = ws; // 简化：用 evaluate 读页面内状态即可
void sock;

await ws.send('Page.navigate', {url: `${APP}?drawer=memory&_cb=${Date.now()}`});
await sleep(8000);

const evalJs = async (expression) => {
  const r = await ws.send('Runtime.evaluate', {expression, awaitPromise: true, returnByValue: true});
  return r.result ? (r.result.value ?? JSON.stringify(r.result)) : JSON.stringify(r);
};

console.log('list-text:', await evalJs(`document.querySelector('.mv-list')?.textContent?.trim().slice(0, 300) || 'NO .mv-list'`));
console.log('toolbar-text:', await evalJs(`document.querySelector('.mv-toolbar')?.textContent?.trim().slice(0, 200) || 'NO toolbar'`));
console.log('view-html-head:', await evalJs(`document.querySelector('.memory-view')?.innerHTML?.slice(0, 200) || 'NO .memory-view'`));
console.log('direct-fetch:', await evalJs(`fetch('http://127.0.0.1:8080/v1/panel/memory/episodes?limit=120').then(r => r.json()).then(d => 'episodes=' + (d.episodes || []).length).catch(e => 'FAIL ' + e.message)`));
console.log('caps-memory:', await evalJs(`fetch('http://127.0.0.1:8080/v1/apeireth/capabilities').then(r => r.json()).then(d => JSON.stringify((d.capabilities || []).find(c => c.name === 'memory'))).catch(e => 'FAIL ' + e.message)`));

edge.kill();
process.exit(0);
