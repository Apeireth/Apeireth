// K3 治理卷宗验收（不提交）：真后端下四 tab 截图。
import {spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';

const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9361;
const APP = 'http://127.0.0.1:5199/';
const OUT = 'C:\\Users\\31683\\Apeireth-rust\\artifacts\\';
const TABS = ['approvals', 'grants', 'guard', 'audit'];

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
  '--user-data-dir=' + process.env.TEMP + '\\apeireth-k3-gov-profile',
  '--window-size=1600,900', '--hide-scrollbars', 'about:blank',
], {stdio: 'ignore'});

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

// 预热原点 + 跳过首启向导
await ws.send('Page.navigate', {url: APP});
await sleep(4000);
await ws.send('Runtime.evaluate', {expression: `localStorage.setItem('apeireth-first-run-done', '1'); 'ok'`});

for (const tab of TABS) {
  await ws.send('Page.navigate', {url: `${APP}?drawer=governance&govtab=${tab}`});
  await sleep(6000);
  const shot = await ws.send('Page.captureScreenshot', {format: 'png'});
  writeFileSync(`${OUT}k3-gov-${tab}.png`, Buffer.from(shot.data, 'base64'));
  console.log(`saved k3-gov-${tab}.png`);
}
ws.close();
edge.kill();
process.exit(0);
