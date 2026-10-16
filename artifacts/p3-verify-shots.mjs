// P3 真机反馈批修复后视觉验收连拍（dev :1420）
// 覆盖：①主题卡片只剩 3 套 ②保存栏进分页底部（模型/记忆两节）③M/◯ 面板宽度
import {spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';
const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9403;
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
// 全新 profile（p2 教训：旧 profile 缓存旧 CSS，验收必须新开）
const edge = spawn(EDGE, ['--headless=new', `--remote-debugging-port=${DEBUG_PORT}`, '--user-data-dir=' + process.env.TEMP + '\\apeireth-p3-verify-' + Date.now(), '--window-size=1600,900', '--hide-scrollbars', '--lang=zh-CN', 'about:blank'], {stdio: 'ignore'});
process.on('exit', () => { try { edge.kill(); } catch {} });
let ws;
for (let i = 0; i < 30; i++) { await sleep(500); try { const list = await (await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/list`)).json(); const page = list.find((t) => t.type === 'page'); if (page) { ws = await connect(page.webSocketDebuggerUrl); break; } } catch {} }
if (!ws) { console.error('FATAL'); edge.kill(); process.exit(1); }
await ws.send('Page.enable');
await ws.send('Emulation.setDeviceMetricsOverride', {width: 1600, height: 900, deviceScaleFactor: 1, mobile: false});
await ws.send('Page.navigate', {url: BASE});
await sleep(5000);
await ws.send('Runtime.evaluate', {expression: `localStorage.setItem('apeireth-first-run-done','1'); 'ok'`});

async function shot(name) {
  const png = await ws.send('Page.captureScreenshot', {format: 'png'});
  writeFileSync(OUT + name, Buffer.from(png.data, 'base64'));
  console.log('saved', name);
}
async function clickSubnav(label, name) {
  const r = await ws.send('Runtime.evaluate', {expression: `(() => {
    const b = [...document.querySelectorAll('.subnav-btn')].find((x) => x.textContent.includes('${label}'));
    if (!b) return 'NOT_FOUND';
    b.click(); return 'clicked';
  })()`, returnByValue: true});
  console.log(name, r.result.value);
}
async function clickSel(sel, name) {
  const r = await ws.send('Runtime.evaluate', {expression: `(() => {
    const el = document.querySelector('${sel}');
    if (!el) return 'NOT_FOUND';
    el.click(); return 'clicked';
  })()`, returnByValue: true});
  console.log(name, r.result.value);
}
async function scrollContentBottom() {
  await ws.send('Runtime.evaluate', {expression: `(() => {
    const c = document.querySelector('.settings-content');
    if (c) c.scrollTop = c.scrollHeight;
    return 'ok';
  })()`, returnByValue: true});
}

// ① 设置页主题卡片：应只剩 遗产星空 / Essence / 深空舰桥 三套
await ws.send('Page.navigate', {url: BASE + '?drawer=settings'});
await sleep(5000);
await shot('p3-theme-cards.png');

// ② 保存栏新位置——模型与提供商分节
await clickSubnav('模型与提供商', 'subnav-models');
await sleep(1200);
await scrollContentBottom();
await sleep(600);
await shot('p3-savebar-models.png');

// ③ 保存栏新位置——记忆策略分节（主人点名比例失调的那页）
await clickSubnav('记忆', 'subnav-memory');
await sleep(1200);
await scrollContentBottom();
await sleep(600);
await shot('p3-savebar-memory.png');

// ④ M / ◯ 面板宽度回归
await ws.send('Page.navigate', {url: BASE});
await sleep(6000);
await clickSel('.cap-btn.pill-model', 'pill-model');
await sleep(900);
await shot('p3-model-panel.png');
await clickSel('.cap-btn.pill-model', 'pill-model-close');
await sleep(500);
await clickSel('.cap-btn.pill-ctx', 'pill-ctx');
await sleep(900);
await shot('p3-ctx-panel.png');
ws.close();
edge.kill();
process.exit(0);
