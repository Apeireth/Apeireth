// P1 修复后视觉验收连拍（dev :1420）
import {spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';

const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9400;
const BASE = 'http://127.0.0.1:1420/';
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
  '--user-data-dir=' + process.env.TEMP + '\\apeireth-p1-verify',
  '--window-size=1600,900', '--hide-scrollbars', '--lang=zh-CN', 'about:blank',
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
await ws.send('Emulation.setDeviceMetricsOverride', {width: 1600, height: 900, deviceScaleFactor: 1, mobile: false});
await ws.send('Page.navigate', {url: BASE});
await sleep(5000);
await ws.send('Runtime.evaluate', {expression: `localStorage.setItem('apeireth-first-run-done','1'); 'ok'`});

// ① 设置页主题卡片（P1-5）——默认即「外观与主题」分节
await ws.send('Page.navigate', {url: BASE + '?drawer=settings'});
await sleep(5000);
let png = await ws.send('Page.captureScreenshot', {format: 'png'});
writeFileSync(`${OUT}p1-theme-cards.png`, Buffer.from(png.data, 'base64'));
console.log('saved p1-theme-cards');

// ② 对话主页列表空态（P1-6）：清掉本地会话 + 无网关态
await ws.send('Runtime.evaluate', {expression: `localStorage.removeItem('apeireth-conversations'); 'ok'`});
await ws.send('Page.navigate', {url: BASE});
await sleep(6000);
png = await ws.send('Page.captureScreenshot', {format: 'png'});
writeFileSync(`${OUT}p1-home-empty.png`, Buffer.from(png.data, 'base64'));
console.log('saved p1-home-empty');

// ③ 工具抽屉错误态（P1-7）：错误态下不应再出现「已装配工具 N 个」
await ws.send('Page.navigate', {url: BASE + '?drawer=tools'});
await sleep(6000);
png = await ws.send('Page.captureScreenshot', {format: 'png'});
writeFileSync(`${OUT}p1-tools-error.png`, Buffer.from(png.data, 'base64'));
console.log('saved p1-tools-error');

ws.close();
edge.kill();
process.exit(0);
