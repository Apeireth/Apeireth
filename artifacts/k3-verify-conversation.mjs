// K3 主会话验收工具（不提交）：点击流复核会话视图。
// 前置: gateway :8080 与 vite :5199 已在听。
// 步骤: 注入演示会话(本地 localStorage) → 主页截图 → 点击第一行进入会话 →
//       会话视图截图 → 杀 gateway 后重载 → 断链诚实态截图。
import {spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';

const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9345;
const APP = 'http://127.0.0.1:5199/';
const OUT = 'C:\\Users\\31683\\Apeireth-rust\\artifacts\\';

const seedConversations = [
  {
    id: 'demo-1',
    title: '星野渲染管线的预算重估',
    createdAt: Date.now() - 86400000 * 2,
    updatedAt: Date.now() - 1000 * 60 * 12,
    scope: 'global',
    messages: [
      {id: 'm1', role: 'user', text: '帮我把 T2 场景的帧率预算再压一压', time: '21:03', timestamp: Date.now() - 1000 * 60 * 13},
      {id: 'm2', role: 'assistant', text: '星野三层视差已经按远中近分层，近层数量占比 17% 还可以再让 4 个点。\n\n漂移三组正弦周期互质，47/37/53 秒，节奏永不可预测——这部分不建议动，它是「不可指认」的来源。', time: '21:04', timestamp: Date.now() - 1000 * 60 * 12},
      {id: 'm3', role: 'user', text: '近层让 4 个点会不会显得稀？', time: '21:05', timestamp: Date.now() - 1000 * 60 * 11},
      {id: 'm4', role: 'assistant', text: '不会。近层靠的是视差系数 1.40 的位移感，不是密度。密度降了，单颗星反而更有分量。', time: '21:06', timestamp: Date.now() - 1000 * 60 * 10},
    ],
  },
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
  '--headless=new',
  `--remote-debugging-port=${DEBUG_PORT}`,
  '--user-data-dir=' + process.env.TEMP + '\\apeireth-k3-verify-profile',
  '--window-size=1600,900',
  '--hide-scrollbars',
  'about:blank',
], {stdio: 'ignore'});

let ws;
for (let i = 0; i < 30; i++) {
  await sleep(500);
  try {
    const list = await (await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/list`)).json();
    const page = list.find((t) => t.type === 'page');
    if (page) { ws = await connect(page.webSocketDebuggerUrl); break; }
  } catch { /* retry */ }
}
if (!ws) { console.error('FATAL: cannot connect to Edge CDP'); edge.kill(); process.exit(1); }

await ws.send('Page.enable');
await ws.send('Runtime.enable');
await ws.send('Emulation.setDeviceMetricsOverride', {width: 1600, height: 900, deviceScaleFactor: 1, mobile: false});

// 注入演示会话
await ws.send('Page.navigate', {url: APP});
await sleep(5000);
await ws.send('Runtime.evaluate', {
  expression: `localStorage.setItem('apeireth-conversations', ${JSON.stringify(JSON.stringify(seedConversations))}); localStorage.setItem('apeireth-first-run-done', '1'); 'seeded'`,
});
await ws.send('Page.navigate', {url: APP});
await sleep(7000);

// 未选中态（默认主题 = heritage 静态图）
let shot0 = await ws.send('Page.captureScreenshot', {format: 'png'});
writeFileSync(OUT + 'k3-t0-3col-default.png', Buffer.from(shot0.data, 'base64'));
console.log('saved k3-t0-3col-default.png');

// 点击演示会话行（标题文本定位，避免依赖内部 class 名）
const clickResult = await ws.send('Runtime.evaluate', {
  expression: `(() => {
    const hits = [...document.querySelectorAll('body *')].filter((el) => el.childElementCount === 0 && el.textContent?.includes('星野渲染管线的预算重估'));
    if (!hits.length) return 'NOT_FOUND_TEXT';
    const leaf = hits[0];
    const clickable = leaf.closest('[role=button], button, a, article, [class*=card], [class*=row], [class*=session]');
    if (!clickable) return 'NOT_FOUND_CLICKABLE:' + leaf.tagName + '.' + leaf.className;
    clickable.dispatchEvent(new MouseEvent('click', {bubbles: true, cancelable: true}));
    return 'CLICKED ' + clickable.tagName + '.' + String(clickable.className).slice(0, 60);
  })()`,
  returnByValue: true,
});
console.log('click:', clickResult.result.value);
await sleep(4000);

let shot = await ws.send('Page.captureScreenshot', {format: 'png'});
writeFileSync(OUT + 'k3-t0-conversation.png', Buffer.from(shot.data, 'base64'));
console.log('saved k3-t0-conversation.png');

// 呼吸半周期 1.4s 后第二帧（验金光晕呼吸）
await sleep(1400);
shot = await ws.send('Page.captureScreenshot', {format: 'png'});
writeFileSync(OUT + 'k3-t0-conversation-b.png', Buffer.from(shot.data, 'base64'));
console.log('saved k3-t0-conversation-b.png');

ws.close();
edge.kill();
process.exit(0);
