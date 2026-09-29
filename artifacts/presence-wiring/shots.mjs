// presence_state 接线视觉自查（不提交）：vite dev + Edge headless CDP 截屏。
// 用法: node shots.mjs（脚本自己起/杀 vite 与 Edge，无后台遗留）。
// 四张: heritage-void / night × 无帧 / 有帧。有帧经 import('/src/lib/presence.ts')
// 注入 presenceStore.ingest(契约 §8a turn 帧)，等平滑收敛后截屏 + 数值探针。
import {spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {dirname, join} from 'node:path';

const OUT = dirname(fileURLToPath(import.meta.url));
const FRONTEND = 'C:\\Users\\31683\\Apeireth-rust\\frontend\\companion-desktop';
const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const DEBUG_PORT = 9337;
const VITE_PORT = 1420;
const APP = `http://localhost:${VITE_PORT}/`;

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// 契约 §8a 实例（让差异可见：breath 2s/1.0 快深呼吸；PAD/intensity 拉满抬高光环）
const FRAME = {
  type: 'presence_state',
  at: Date.now(),
  pad: {p: 0.9, a: 0.8, d: 0.3},
  dominant: 'luminous_joy',
  intensity: 0.95,
  stance: 'attentive_presence',
  breath: {period_secs: 2, amplitude: 1.0},
  significance: 'turn',
  source: {kind: 'heuristic_v0', confidence: 0.5},
};

function connect(wsUrl) {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(wsUrl);
    let id = 0;
    const pending = new Map();
    socket.onopen = () =>
      resolve({
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

// 探针：余烬点 CSS 变量 / glow 钩子 / presenceStore 快照
const PROBE = `(async () => {
  const m = await import('/src/lib/presence.ts');
  const s = m.presenceStore.get();
  const ember = document.querySelector('.ember-dot');
  const root = document.querySelector('[style*="--presence-glow"]');
  return JSON.stringify({
    theme: document.documentElement.getAttribute('data-theme'),
    emberFound: !!ember,
    emberPeriod: ember ? ember.style.getPropertyValue('--ember-period') : null,
    emberAmp: ember ? ember.style.getPropertyValue('--ember-amp') : null,
    emberAnimDur: ember ? getComputedStyle(ember).animationDuration : null,
    emberOpacityNow: ember ? getComputedStyle(ember).opacity : null,
    glowVar: root ? root.style.getPropertyValue('--presence-glow') : 'hook-missing',
    current: s.current ? {p: s.current.p, a: s.current.a, intensity: s.current.intensity, stance: s.current.stance, mode: s.current.mode, src: s.current.sourceKind} : null,
    breath: s.breath,
    lastSignificance: s.lastSignificance,
  });
})()`;

let vite = null;
let edge = null;
try {
  // 起 vite dev（脚本子进程，退出时一并杀掉）
  vite = spawn('node', [join(FRONTEND, 'node_modules', 'vite', 'bin', 'vite.js'), '--port', String(VITE_PORT), '--strictPort'], {
    cwd: FRONTEND,
    stdio: 'ignore',
  });
  let up = false;
  for (let i = 0; i < 60; i++) {
    await sleep(500);
    try {
      const r = await fetch(APP);
      if (r.ok) {
        up = true;
        break;
      }
    } catch { /* retry */ }
  }
  if (!up) throw new Error('vite dev 未就绪');

  edge = spawn(EDGE, [
    '--headless=new',
    `--remote-debugging-port=${DEBUG_PORT}`,
    '--user-data-dir=' + process.env.TEMP + '\\apeireth-presence-shot-profile',
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
      if (page) {
        ws = await connect(page.webSocketDebuggerUrl);
        break;
      }
    } catch { /* retry */ }
  }
  if (!ws) throw new Error('cannot connect to Edge CDP');

  await ws.send('Page.enable');
  await ws.send('Runtime.enable');
  await ws.send('Emulation.setDeviceMetricsOverride', {width: 1600, height: 900, deviceScaleFactor: 1, mobile: false});

  // 首跑向导跳过（localStorage 按原点隔离，先载入原点再写）
  await ws.send('Page.navigate', {url: APP});
  await sleep(4000);
  await ws.send('Runtime.evaluate', {
    expression: `localStorage.setItem('apeireth-first-run-done', '1'); 'ok'`,
  });

  for (const theme of ['heritage-void', 'night']) {
    // ① 无帧静息态
    await ws.send('Page.navigate', {url: `${APP}?theme=${theme}`});
    await sleep(6000); // 场景层首帧 + 字体 + 静态背景淡入
    let probe = await ws.send('Runtime.evaluate', {expression: PROBE, awaitPromise: true});
    console.log(`[${theme}] 无帧探针:`, probe.result.value);
    let shot = await ws.send('Page.captureScreenshot', {format: 'png'});
    writeFileSync(join(OUT, `${theme}-noframe.png`), Buffer.from(shot.data, 'base64'));
    console.log(`saved ${theme}-noframe.png`);

    // ② 注入契约 §8a turn 帧（经真实模块路径 presenceStore.ingest）
    const injected = await ws.send('Runtime.evaluate', {
      expression: `(async () => {
        const m = await import('/src/lib/presence.ts');
        m.presenceStore.ingest(${JSON.stringify(FRAME)});
        return 'ingested';
      })()`,
      awaitPromise: true,
    });
    console.log(`[${theme}] 注入:`, injected.result.value);
    await sleep(3000); // rate 2.4/s 平滑收敛 (~7 个时间常数) + breath CSS 变量生效
    probe = await ws.send('Runtime.evaluate', {expression: PROBE, awaitPromise: true});
    console.log(`[${theme}] 有帧探针:`, probe.result.value);
    shot = await ws.send('Page.captureScreenshot', {format: 'png'});
    writeFileSync(join(OUT, `${theme}-frame.png`), Buffer.from(shot.data, 'base64'));
    console.log(`saved ${theme}-frame.png`);

    // 重置 store，避免下一主题继承上一轮的帧
    await ws.send('Runtime.evaluate', {
      expression: `(async () => { const m = await import('/src/lib/presence.ts'); m.presenceStore.reset(); return 'reset'; })()`,
      awaitPromise: true,
    });
  }
  ws.close();
} finally {
  if (edge) edge.kill();
  if (vite) vite.kill();
}
process.exit(0);
