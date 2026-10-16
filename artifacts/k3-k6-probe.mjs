// K3 ⑥ 补验：精确点第一行记忆看右栏详情。
import {spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';

const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9399;
const APP = 'http://127.0.0.1:5199/';
const OUT = 'C:\\Users\\31683\\Apeireth-rust\\artifacts\\';

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
      close: () => socket.close(),
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
  '--user-data-dir=' + process.env.TEMP + '\\apeireth-k3-k6b-profile',
  '--window-size=1600,900', '--hide-scrollbars', 'about:blank',
], {stdio: 'ignore'});
process.on('exit', () => { try { edge.kill(); } catch {} });

let ws;
for (let i = 0; i < 30; i++) {
  await sleep(500);
  try {
    const list = await (await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/list`)).json();
    const page = list.find((t) => t.type === 'page');
    if (page) { ws = await connect(page.webSocketDebuggerUrl); break; }
  } catch { }
}
if (!ws) { console.error('FATAL'); edge.kill(); process.exit(1); }
await ws.send('Page.enable');
await ws.send('Runtime.enable');
await ws.send('Emulation.setDeviceMetricsOverride', {width: 1600, height: 900, deviceScaleFactor: 1, mobile: false});
await ws.send('Page.navigate', {url: APP});
await sleep(4000);
await ws.send('Runtime.evaluate', {expression: `localStorage.setItem('apeireth-first-run-done', '1'); 'ok'`});
await ws.send('Page.navigate', {url: APP});
await sleep(6000);
await ws.send('Runtime.evaluate', {
  expression: `(() => { const leaf = [...document.querySelectorAll('body *')].find((el) => el.childElementCount === 0 && el.textContent?.trim() === '记忆'); const c = leaf?.closest('button, [role=button], a'); c?.click(); return c ? 'OK' : 'NO'; })()`, returnByValue: true,
});
await sleep(5000);

// 打印左栏候选行结构
const structure = await ws.send('Runtime.evaluate', {
  expression: `(() => {
    const view = document.querySelector('[class*=memory]');
    if (!view) return 'NO_VIEW';
    const rows = [...view.querySelectorAll('button, [role=button], [class*=row], [class*=item], li')].slice(0, 6)
      .map((el) => el.tagName + '.' + String(el.className).slice(0, 50) + ' | ' + (el.textContent || '').trim().slice(0, 24));
    return JSON.stringify(rows, null, 1);
  })()`, returnByValue: true,
});
console.log('rows:', structure.result.value);
