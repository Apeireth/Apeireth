// P10 档案调深舱反色验收：深四主题（night/heritage-void/ocean/forest）
// + 浅三（essence 默认/day/paper）× 记忆卷宗/日记 连拍
import {spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';
const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9414;
const BASE = 'http://127.0.0.1:1420/';
const OUT = 'C:\\Users\\31683\\Apeireth-rust\\artifacts\\';
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
const edge = spawn(EDGE, ['--headless=new', `--remote-debugging-port=${DEBUG_PORT}`, '--user-data-dir=' + process.env.TEMP + '\\apeireth-p10-arch-' + Date.now(), '--window-size=1600,900', '--hide-scrollbars', '--lang=zh-CN', 'about:blank'], {stdio: 'ignore'});
process.on('exit', () => { try { edge.kill(); } catch {} });
let ws;
for (let i = 0; i < 30; i++) { await sleep(500); try { const list = await (await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/list`)).json(); const page = list.find((t) => t.type === 'page'); if (page) { ws = await connect(page.webSocketDebuggerUrl); break; } } catch {} }
if (!ws) { console.error('FATAL'); edge.kill(); process.exit(1); }
await ws.send('Page.enable');
await ws.send('Emulation.setDeviceMetricsOverride', {width: 1600, height: 900, deviceScaleFactor: 1, mobile: false});
async function shot(name) {
  const png = await ws.send('Page.captureScreenshot', {format: 'png'});
  writeFileSync(OUT + name, Buffer.from(png.data, 'base64'));
  console.log('saved', name);
}
// 首启标记
await ws.send('Page.navigate', {url: BASE});
await sleep(5000);
await ws.send('Runtime.evaluate', {expression: `localStorage.setItem('apeireth-first-run-done','1'); 'ok'`});

const themes = [
  ['night', '?theme=night'],
  ['heritage', '?theme=heritage-void'],
  ['ocean', '?theme=ocean'],
  ['forest', '?theme=forest'],
  ['day', '?theme=day'],
  ['paper', '?theme=paper'],
  ['essence', '?theme=essence'],  // ⚠ essence 也是显式主题，无参数时回落的是 heritage-void
];
for (const [name, q] of themes) {
  await ws.send('Page.navigate', {url: `${BASE}${q}${q ? '&' : '?'}drawer=memory`});
  await sleep(5500);
  await shot(`p10-${name}-memory.png`);
  await ws.send('Page.navigate', {url: `${BASE}${q}${q ? '&' : '?'}drawer=diary`});
  await sleep(5000);
  await shot(`p10-${name}-diary.png`);
}
ws.close();
edge.kill();
process.exit(0);
