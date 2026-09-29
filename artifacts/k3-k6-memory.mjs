// K3 ⑥ 记忆卷宗 + 日记验收（真后端）：记忆主从 → 日记纸面。
import {spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';

const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9395;
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
  '--user-data-dir=' + process.env.TEMP + '\\apeireth-k3-k6-profile',
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
if (!ws) { console.error('FATAL: no CDP'); edge.kill(); process.exit(1); }
await ws.send('Page.enable');
await ws.send('Runtime.enable');
await ws.send('Emulation.setDeviceMetricsOverride', {width: 1600, height: 900, deviceScaleFactor: 1, mobile: false});

await ws.send('Page.navigate', {url: APP});
await sleep(4000);
await ws.send('Runtime.evaluate', {expression: `localStorage.setItem('apeireth-first-run-done', '1'); 'ok'`});

// 记忆视图（rail 点「记忆」）
await ws.send('Page.navigate', {url: APP});
await sleep(6000);
let r = await ws.send('Runtime.evaluate', {
  expression: `(() => {
    const leaf = [...document.querySelectorAll('nav *, aside *, [class*=rail] *')].find((el) => el.childElementCount === 0 && el.textContent?.trim() === '记忆');
    if (!leaf) return 'NO_RAIL_ITEM';
    const c = leaf.closest('button, [role=button], a');
    if (!c) return 'NO_CLICKABLE';
    c.click(); return 'OK';
  })()`, returnByValue: true,
});
console.log('open memory:', r.result.value);
await sleep(5000);
let shot = await ws.send('Page.captureScreenshot', {format: 'png'});
writeFileSync(OUT + 'k3-k6-memory.png', Buffer.from(shot.data, 'base64'));
console.log('saved k3-k6-memory.png');

// 点第一行看详情主从
r = await ws.send('Runtime.evaluate', {
  expression: `(() => {
    const rows = [...document.querySelectorAll('[class*=episode], [class*=memory] [role=button], [class*=mem-row], li, article')];
    const row = rows.find((el) => el.textContent && el.textContent.trim().length > 20);
    if (!row) return 'NO_ROW';
    (row.closest('[role=button], button, li, article') ?? row).dispatchEvent(new MouseEvent('click', {bubbles: true, cancelable: true}));
    return 'OK';
  })()`, returnByValue: true,
});
console.log('open episode:', r.result.value);
await sleep(3000);
shot = await ws.send('Page.captureScreenshot', {format: 'png'});
writeFileSync(OUT + 'k3-k6-memory-detail.png', Buffer.from(shot.data, 'base64'));
console.log('saved k3-k6-memory-detail.png');

// 日记（命令面板或 drawer 参数都试）
await ws.send('Page.navigate', {url: APP + '?drawer=diary'});
await sleep(5000);
shot = await ws.send('Page.captureScreenshot', {format: 'png'});
writeFileSync(OUT + 'k3-k6-diary.png', Buffer.from(shot.data, 'base64'));
console.log('saved k3-k6-diary.png');

ws.close();
edge.kill();
process.exit(0);
