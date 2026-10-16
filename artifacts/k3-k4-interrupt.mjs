// K3 ④ 打断按钮验收（mock 慢流式）：发消息 → 流中截「打断出现态」→ 点打断 → 截「诚实注记态」。
import {spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';

const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9381;
const APP = 'http://127.0.0.1:5199/';
const OUT = 'C:\\Users\\31683\\Apeireth-rust\\artifacts\\';

const seedConversations = [
  {id: 'demo-1', title: '打断验收会话', createdAt: Date.now() - 3600000, updatedAt: Date.now() - 60000, scope: 'global',
   messages: [{id: 'm0', role: 'user', text: '上一轮占位', time: '21:00', timestamp: Date.now() - 60000}]},
];

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
  '--user-data-dir=' + process.env.TEMP + '\\apeireth-k3-k4b-profile',
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
await ws.send('Runtime.evaluate', {
  expression: `localStorage.setItem('apeireth-conversations', ${JSON.stringify(JSON.stringify(seedConversations))}); localStorage.setItem('apeireth-first-run-done', '1'); 'seeded'`,
});
await ws.send('Page.navigate', {url: APP});
await sleep(6000);

// 点进会话
const click = await ws.send('Runtime.evaluate', {
  expression: `(() => {
    const leaf = [...document.querySelectorAll('body *')].find((el) => el.childElementCount === 0 && el.textContent?.includes('打断验收会话'));
    if (!leaf) return 'NO_ROW';
    const c = leaf.closest('[role=button], button, a, article, [class*=card], [class*=row], [class*=session]');
    if (!c) return 'NO_CLICKABLE';
    c.dispatchEvent(new MouseEvent('click', {bubbles: true, cancelable: true}));
    return 'OK';
  })()`, returnByValue: true,
});
console.log('click session:', click.result.value);
await sleep(2500);

// 关掉可能弹出的工作区选择器（2026-10-06 设计内行为）
await ws.send('Runtime.evaluate', {
  expression: `(() => { const b=[...document.querySelectorAll('button')].find(x=>/取消|跳过|稍后/.test(x.textContent||'')); if(b){b.click(); return 'dismissed'} return 'none' })()`, returnByValue: true,
});
await sleep(800);

// composer 输入并发送
const typed = await ws.send('Runtime.evaluate', {
  expression: `(() => {
    const ta = document.querySelector('textarea') || document.querySelector('[contenteditable=true]');
    if (!ta) return 'NO_COMPOSER';
    ta.focus();
    return 'FOCUSED ' + ta.tagName;
  })()`, returnByValue: true,
});
console.log('composer:', typed.result.value);
await ws.send('Input.insertText', {text: '写一句关于夜色的话'});
await sleep(300);
await ws.send('Input.dispatchKeyEvent', {type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13});
await ws.send('Input.dispatchKeyEvent', {type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13});
await sleep(1800); // 流式进行中

let state = await ws.send('Runtime.evaluate', {
  expression: `(() => {
    const stop = [...document.querySelectorAll('button')].find(b => /打断|停止|stop/i.test((b.title||'') + (b.getAttribute('aria-label')||'') + b.textContent));
    return JSON.stringify({stopFound: !!stop, stopTitle: stop?.title || stop?.getAttribute('aria-label') || ''});
  })()`, returnByValue: true,
});
console.log('streaming state:', state.result.value);
let shot = await ws.send('Page.captureScreenshot', {format: 'png'});
writeFileSync(OUT + 'k3-k4-interrupt-on.png', Buffer.from(shot.data, 'base64'));
console.log('saved k3-k4-interrupt-on.png');

// 点打断
await ws.send('Runtime.evaluate', {
  expression: `(() => {
    const stop = [...document.querySelectorAll('button')].find(b => /打断|停止|stop/i.test((b.title||'') + (b.getAttribute('aria-label')||'') + b.textContent));
    if (stop) { stop.click(); return 'STOPPED'; } return 'NO_STOP';
  })()`, returnByValue: true,
});
await sleep(1200);
shot = await ws.send('Page.captureScreenshot', {format: 'png'});
writeFileSync(OUT + 'k3-k4-interrupt-after.png', Buffer.from(shot.data, 'base64'));
console.log('saved k3-k4-interrupt-after.png');

ws.close();
edge.kill();
process.exit(0);
