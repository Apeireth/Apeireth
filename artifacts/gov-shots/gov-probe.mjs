// 一次性探针：页面实际对 8080 发了哪些请求、直接 fetch capabilities 是否成功。
import {spawn} from 'node:child_process';

const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9346;

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
        const {res} = pending.get(msg.id);
        pending.delete(msg.id);
        res(msg.result);
      }
    };
    socket.onerror = reject;
  });
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const edge = spawn(EDGE, ['--headless=new', `--remote-debugging-port=${DEBUG_PORT}`,
  '--user-data-dir=' + process.env.TEMP + '\\gov-probe-profile', '--window-size=1440,900',
  '--disable-gpu', 'about:blank'], {stdio: 'ignore'});
process.on('exit', () => { try { edge.kill(); } catch {} });

let ws;
for (let i = 0; i < 30; i++) {
  await sleep(500);
  try {
    const list = await (await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/list`)).json();
    const page = list.find((t) => t.type === 'page');
    if (page) { ws = await connect(page.webSocketDebuggerUrl); break; }
  } catch {}
}
if (!ws) { console.error('FATAL: no CDP'); process.exit(1); }

await ws.send('Page.enable');
await ws.send('Runtime.enable');
await ws.send('Page.navigate', {url: 'http://127.0.0.1:1420/?drawer=governance&govtab=approvals'});
await sleep(5000);

const r = await ws.send('Runtime.evaluate', {
  expression: `(async () => {
    const resources = performance.getEntriesByType('resource')
      .map((e) => e.name).filter((n) => n.includes('8080'));
    let direct = 'n/a';
    try {
      const res = await fetch('http://127.0.0.1:8080/v1/apeireth/capabilities');
      const j = await res.json();
      direct = res.status + ' groups=' + (j.capabilities ? j.capabilities.length : 'none');
    } catch (e) { direct = 'FETCH-FAIL: ' + e.message; }
    let health = 'n/a';
    try { health = String((await fetch('http://127.0.0.1:8080/health')).status); }
    catch (e) { health = 'FAIL: ' + e.message; }
    return JSON.stringify({resources, direct, health, ls: Object.keys(localStorage)});
  })()`,
  awaitPromise: true,
});
console.log('probe:', r.result.value);
ws.close();
edge.kill();
process.exit(0);
