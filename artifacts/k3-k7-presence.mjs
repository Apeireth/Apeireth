// K3 ⑦ 显影接线验收（真后端）：等真实 presence_state 心跳帧（60s 一拍），
// 对比到达前/后的余烬与光晕 CSS 变量与画面。
import {spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';

const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9411;
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
  '--user-data-dir=' + process.env.TEMP + '\\apeireth-k3-k7-profile',
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

const PROBE = `(() => {
  const ember = document.querySelector('[class*=ember]');
  const cs = ember ? getComputedStyle(ember) : null;
  const root = getComputedStyle(document.documentElement);
  return JSON.stringify({
    emberPeriod: cs?.getPropertyValue('--ember-period') || root.getPropertyValue('--ember-period') || 'n/a',
    emberAmp: cs?.getPropertyValue('--ember-amp') || root.getPropertyValue('--ember-amp') || 'n/a',
    glow: root.getPropertyValue('--presence-glow') || 'n/a',
  });
})()`;

// night 主题（场景在），先采基线（心跳未到）
await ws.send('Page.navigate', {url: APP + '?theme=night&hour=22'});
await sleep(4000);
await ws.send('Runtime.evaluate', {expression: `localStorage.setItem('apeireth-first-run-done', '1'); 'ok'`});
await ws.send('Page.navigate', {url: APP + '?theme=night&hour=22'});
await sleep(6000);
const before = await ws.send('Runtime.evaluate', {expression: PROBE, returnByValue: true});
console.log('baseline:', before.result.value);
let shot = await ws.send('Page.captureScreenshot', {format: 'png'});
writeFileSync(OUT + 'k3-k7-before.png', Buffer.from(shot.data, 'base64'));

// 等真实心跳（gateway 60s 一拍；页面开至今约 10s，再等到 75s 保险）
console.log('waiting for real presence_state heartbeat (~65s)…');
await sleep(65000);
const after = await ws.send('Runtime.evaluate', {expression: PROBE, returnByValue: true});
console.log('after heartbeat:', after.result.value);
shot = await ws.send('Page.captureScreenshot', {format: 'png'});
writeFileSync(OUT + 'k3-k7-after.png', Buffer.from(shot.data, 'base64'));
console.log('saved k3-k7-before/after.png');

ws.close();
edge.kill();
process.exit(0);
