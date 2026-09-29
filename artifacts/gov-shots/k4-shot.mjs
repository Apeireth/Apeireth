// ④ 命令面板 + 打断按钮 自查截图（scratch，不提交）：Edge headless CDP。
// 前置：mock-gateway(8080, 含慢速 /v1/chat/completions) 与 vite(1420) 已起。
// 用法: node k4-shot.mjs
// 真键路径：Ctrl+K / Esc / Enter 都走 window 合成 KeyboardEvent（svelte:window 与
// textarea onkeydown 都是普通 DOM 监听，合成事件可触发，且不依赖焦点猜测）。
import {spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {dirname, join} from 'node:path';

const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = Number(process.env.K4_DEBUG_PORT || 9375);
const APP = 'http://127.0.0.1:1420/';
const OUT = dirname(fileURLToPath(import.meta.url));

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
  '--headless=new',
  `--remote-debugging-port=${DEBUG_PORT}`,
  '--user-data-dir=' + process.env.TEMP + '\\k4-shot-profile',
  '--window-size=1440,900',
  '--hide-scrollbars',
  '--disable-gpu',
  'about:blank',
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
await ws.send('Emulation.setDeviceMetricsOverride', {width: 1440, height: 900, deviceScaleFactor: 1, mobile: false});

async function evalJs(expression) {
  const r = await ws.send('Runtime.evaluate', {expression, awaitPromise: true});
  return r.result?.value;
}
async function shot(name) {
  const data = await ws.send('Page.captureScreenshot', {format: 'png'});
  const file = join(OUT, name);
  writeFileSync(file, Buffer.from(data.data, 'base64'));
  console.log('saved', file);
}

// ---- 载入壳（等健康链与字体 settle）----
await ws.send('Page.navigate', {url: `${APP}?_cb=${Date.now()}`});
await sleep(10000);

// ---- ① Ctrl+K 打开面板 ----
await evalJs(`window.dispatchEvent(new KeyboardEvent('keydown', {key:'k', ctrlKey:true, bubbles:true, cancelable:true})); 'sent'`);
await sleep(800);
console.log('[palette-open] probe:', await evalJs(`(() => {
  const p = document.querySelector('.palette');
  if (!p) return JSON.stringify({error: 'no-palette'});
  const items = [...p.querySelectorAll('.palette-item')];
  return JSON.stringify({
    items: items.length,
    disabled: [...p.querySelectorAll('.palette-item.disabled')].length,
    focused: document.activeElement?.className,
    first: items[0]?.textContent?.trim().slice(0, 40),
    recentTag: !!p.querySelector('.item-recent'),
  });
})()`));
await shot('k4-palette-open.png');

// ---- ② 搜索过滤态（拼音别名 'zhil' → 治理卷宗）----
await evalJs(`(() => {
  const input = document.querySelector('.palette-input');
  const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
  setter.call(input, 'zhil');
  input.dispatchEvent(new Event('input', {bubbles: true}));
  return 'typed';
})()`);
await sleep(500);
console.log('[palette-filter] probe:', await evalJs(`(() => {
  const items = [...document.querySelectorAll('.palette-item')];
  return JSON.stringify({count: items.length, texts: items.map((i) => i.textContent.trim().slice(0, 30))});
})()`));
await shot('k4-palette-filter.png');

// 关面板（Esc）
await evalJs(`window.dispatchEvent(new KeyboardEvent('keydown', {key:'Escape', bubbles:true, cancelable:true})); 'esc'`);
await sleep(500);

// ---- ③ 打断按钮出现态：新对话 → 发一条消息 → busy 中截 ----
console.log('new conv:', await evalJs(`(() => {
  const btn = document.querySelector('.round-btn[title="新对话"]');
  if (!btn) return 'no-new-btn';
  btn.click();
  return 'clicked';
})()`));
await sleep(900);
// 新会话自动弹工作区选择器（2026-10-06 需求，设计内行为）——截图前诚实关掉它
console.log('dismiss picker:', await evalJs(`(() => {
  const cancel = document.querySelector('.picker-backdrop .quiet-btn');
  if (!cancel) return 'no-picker';
  cancel.click();
  return 'cancelled';
})()`));
await sleep(600);
console.log('send msg:', await evalJs(`(() => {
  const ta = document.querySelector('.composer textarea');
  if (!ta) return 'no-textarea';
  const setter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value').set;
  setter.call(ta, '给我讲讲星野的远山。');
  ta.dispatchEvent(new Event('input', {bubbles: true}));
  ta.dispatchEvent(new KeyboardEvent('keydown', {key:'Enter', bubbles:true, cancelable:true}));
  return 'enter-sent';
})()`));
await sleep(1600); // 慢速流 400ms/帧，此时 busy 中、stop 钮在
console.log('[interrupt-visible] probe:', await evalJs(`(() => {
  const stop = document.querySelector('.send.stop');
  return JSON.stringify({
    stopBtn: !!stop,
    stopTitle: stop?.getAttribute('title') || '',
    halo: !!document.querySelector('.presence-halo'),
    streaming: !!document.querySelector('.md-body.streaming'),
  });
})()`));
await shot('k4-interrupt-visible.png');

// ---- ④ 点打断 → aborted 诚实注记 ----
console.log('click stop:', await evalJs(`(() => {
  const stop = document.querySelector('.send.stop');
  if (!stop) return 'no-stop-btn';
  stop.click();
  return 'clicked';
})()`));
await sleep(1200);
console.log('[interrupt-after] probe:', await evalJs(`(() => {
  const note = document.querySelector('.message-aborted');
  return JSON.stringify({
    abortedNote: note?.textContent?.trim() || 'none',
    stopGone: !document.querySelector('.send.stop'),
    sendBack: !!document.querySelector('.send:not(.stop)'),
  });
})()`));
await shot('k4-interrupt-after.png');

ws.close?.();
edge.kill();
console.log('done');
process.exit(0);
