// ⑥ 记忆卷宗主从化 + 日记纸面调 自查截图（scratch，不提交）：Edge headless CDP。
// 前置：vite(1420) 已起；mock(8080) 按模式起（main=有数据夹具 / empty=GOV_EMPTY=1）。
// 用法: node k6-shot.mjs main   —— 主从有数据/详情+图谱/forget 红边卡/409 冲突/日记纸面
//       node k6-shot.mjs empty  —— 记忆卷宗空态契约
import {spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {dirname, join} from 'node:path';

const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = Number(process.env.K6_DEBUG_PORT || 9387);
const APP = 'http://127.0.0.1:1420/';
const MOCK = 'http://127.0.0.1:8080';
const OUT = dirname(fileURLToPath(import.meta.url));
const MODE = process.argv[2] || 'main';

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
  '--user-data-dir=' + process.env.TEMP + '\\k6-shot-profile-' + MODE,
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
  const r = await ws.send('Runtime.evaluate', {expression, awaitPromise: true, returnByValue: true});
  return r.result ? r.result.value : undefined;
}
async function shot(name) {
  const data = await ws.send('Page.captureScreenshot', {format: 'png'});
  writeFileSync(join(OUT, name), Buffer.from(data.data, 'base64'));
  console.log('saved', name);
}
const PROBE = `(() => {
  const rows = [...document.querySelectorAll('.mv-row')].map((r) => r.textContent.trim().slice(0, 24));
  const chips = [...document.querySelectorAll('.mv-chip')].map((c) => c.textContent.trim());
  return JSON.stringify({
    rows,
    chips,
    ghost: document.querySelector('.mv-ghost')?.textContent || null,
    detailEmpty: !!document.querySelector('.mv-detail-empty'),
    graphEdges: document.querySelectorAll('.mv-graph-edges li').length,
    conflict: !!document.querySelector('.mv-conflict'),
    forgetArm: !!document.querySelector('.mv-forget-arm'),
    diaryVoice: document.querySelector('.diary-voice')?.textContent.trim() || null,
    emptyState: document.querySelector('.mv-list .empty-title, .mv-list [class*=empty]')?.textContent?.slice(0, 40) || null,
  });
})()`;
async function probe(name) { console.log(`[${name}]`, await evalJs(PROBE)); }

if (MODE === 'main') {
  // 1) 主从有数据态（未选中 = 右栏空态三件套）
  await ws.send('Page.navigate', {url: `${APP}?drawer=memory&_cb=${Date.now()}`});
  await sleep(8000);
  await probe('list');
  await shot('k6-mem-list.png');

  // 2) 选中第 1 行（ep-m1：protected + 有图谱边 + category 真值）→ 详情 + 幽灵编号 + 图谱关联
  await evalJs(`document.querySelectorAll('.mv-row')[0].click()`);
  await sleep(1500); // ensureGraph 拉取窗
  await probe('detail-ep1');
  await shot('k6-mem-detail.png');

  // 3) forget 红边内联确认卡（§4.4-13；不确认，只截 armed 态）
  await evalJs(`document.querySelectorAll('.mv-row')[3].click()`); // ep-m4（未保护才可遗忘）
  await sleep(700);
  await evalJs(`[...document.querySelectorAll('.mv-actions button')].find((b) => b.textContent.includes('遗忘'))?.click()`);
  await sleep(500);
  await probe('forget-armed');
  await shot('k6-mem-forget-arm.png');

  // 4) 409 冲突态：先由「别处」（直接调 mock）protect ep-m2（rev 0→1），
  //    UI 账本仍记 0 → 再点保护必 409 → 真实冲突卡
  await evalJs(`[...document.querySelectorAll('.mv-forget-arm-btns button')].find((b) => b.textContent.includes('取消'))?.click()`);
  await evalJs(`document.querySelectorAll('.mv-row')[1].click()`); // ep-m2
  await sleep(700);
  const pre = await fetch(`${MOCK}/v1/apeireth/memory/episodes/ep-m2d4e5f6/protect`, {
    method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({expected_rev: 0}),
  });
  console.log('[elsewhere-protect]', pre.status); // 200，rev → 1
  await evalJs(`[...document.querySelectorAll('.mv-actions button')].find((b) => b.textContent.includes('保护'))?.click()`);
  await sleep(2200); // 409 + 自动重拉对齐
  await probe('conflict');
  await shot('k6-mem-conflict.png');

  // 5) 他的日记（纸面调空态契约页）
  await ws.send('Page.navigate', {url: `${APP}?drawer=diary&_cb=${Date.now()}`});
  await sleep(6000);
  await probe('diary');
  await shot('k6-diary.png');
} else {
  // empty：GOV_EMPTY=1 mock —— 空态契约「当他记住什么时，这里会出现……」
  await ws.send('Page.navigate', {url: `${APP}?drawer=memory&_cb=${Date.now()}`});
  await sleep(8000);
  await probe('empty');
  await shot('k6-mem-empty.png');
}

edge.kill();
console.log('done');
process.exit(0);
