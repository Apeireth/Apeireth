#!/usr/bin/env node
// pet-window-real-smoke.mjs —— 桌宠窗【真 Tauri 窗 + 真 WebView2】实测。
//
// 与 pet-window-smoke.mjs（构建产物 + 无头内核近似）互补：本脚本起真壳
// （pnpm tauri dev，侧车已 staged），把调试口挂进真 WebView2
// （WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS），经主窗 IPC 唤起桌宠窗，
// 对真窗做断言 + 实拍截图（reports/pet-real-smoke-2026-10-11/）。
//
// 用法：node scripts/pet-window-real-smoke.mjs [--port 9351] [--headed]
// 口径：真 WebView2 实体（比无头近似高一档）；窗口拖拽/穿透手感仍需人眼。
import {createServer} from 'node:http';
import {mkdir, writeFile} from 'node:fs/promises';
import {existsSync} from 'node:fs';
import {spawn, spawnSync} from 'node:child_process';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(__dirname, '..');
const APP_DIR = path.join(REPO, 'frontend', 'companion-desktop');
const OUT_DIR = path.join(REPO, 'reports', 'pet-real-smoke-2026-10-11');

const args = process.argv.slice(2);
const argValue = (name, fallback) => {
  const i = args.indexOf(`--${name}`);
  return i >= 0 && args[i + 1] && !args[i + 1].startsWith('--') ? args[i + 1] : fallback;
};
const CDP_PORT = Number(argValue('port', '9351'));

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const log = (m) => console.log(`[real-smoke] ${m}`);

// 端口占用预检（1420 = vite devUrl；被占则 tauri dev 会错配，如实早退）
async function portBusy(port) {
  return new Promise((resolve) => {
    const probe = createServer();
    probe.once('error', () => resolve(true));
    probe.once('listening', () => probe.close(() => resolve(false)));
    probe.listen(port, '127.0.0.1');
  });
}
if (await portBusy(1420)) {
  console.error('❌ 端口 1420 已被占用（vite devUrl 错配风险）——先停掉占用者再跑');
  process.exit(1);
}

// ---------------- 起真壳 ----------------
log('起真壳（pnpm tauri dev，调试口 ' + CDP_PORT + '）…');
const shell = spawn('cmd', ['/c', 'pnpm', 'tauri', 'dev'], {
  cwd: APP_DIR,
  stdio: 'ignore',
  env: {
    ...process.env,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${CDP_PORT} --remote-allow-origins=*`,
  },
});
const killTree = () => {
  try {
    spawnSync('taskkill', ['/pid', String(shell.pid), '/T', '/F'], {stdio: 'ignore'});
  } catch {
    // 尽力而为
  }
};
process.on('exit', killTree);

// ---------------- CDP ----------------
async function connectTarget(match) {
  for (let i = 0; i < 120; i++) {
    try {
      const res = await fetch(`http://127.0.0.1:${CDP_PORT}/json/list`);
      const targets = await res.json();
      const t = targets.find((x) => x.type === 'page' && match(x.url ?? ''));
      if (t?.webSocketDebuggerUrl) {
        const ws = new WebSocket(t.webSocketDebuggerUrl);
        await new Promise((resolve, reject) => {
          ws.onopen = resolve;
          ws.onerror = () => reject(new Error('ws error'));
        });
        let seq = 0;
        const pending = new Map();
        ws.onmessage = (e) => {
          const msg = JSON.parse(e.data);
          if (msg.id && pending.has(msg.id)) {
            const {resolve, reject, timer} = pending.get(msg.id);
            clearTimeout(timer);
            pending.delete(msg.id);
            msg.error ? reject(new Error(msg.error.message)) : resolve(msg.result);
          }
        };
        return {
          send(method, params = {}) {
            const id = ++seq;
            return new Promise((resolve, reject) => {
              const timer = setTimeout(() => {
                pending.delete(id);
                reject(new Error(`CDP timeout: ${method}`));
              }, 30000);
              pending.set(id, {resolve, reject, timer});
              ws.send(JSON.stringify({id, method, params}));
            });
          },
          close: () => ws.close(),
        };
      }
    } catch {
      // 壳未就绪，续等
    }
    await sleep(1000);
  }
  throw new Error('等待 CDP 目标超时（壳没起来？）');
}

async function evalJs(cdp, expression) {
  const out = await cdp.send('Runtime.evaluate', {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  if (out.exceptionDetails) throw new Error(out.exceptionDetails.text ?? 'evaluate 异常');
  return out.result?.value;
}

const results = [];
const record = (name, ok, detail = '') => {
  results.push({name, ok});
  log(`${ok ? '✅' : '❌'} ${name}${detail ? ` — ${detail}` : ''}`);
};

try {
  await mkdir(OUT_DIR, {recursive: true});

  // 主窗（devUrl）→ 经真 IPC 唤起桌宠窗（toggle：不可见 → 唤起）
  const main = await connectTarget((url) => url.includes('127.0.0.1:1420') && !url.includes('window='));
  await main.send('Runtime.enable');
  const invoked = await evalJs(
    main,
    `window.__TAURI_INTERNALS__ ? (window.__TAURI_INTERNALS__.invoke('toggle_pet_window'), true) : false`,
  );
  record('主窗真 IPC 可用并唤起桌宠窗', invoked === true, invoked ? '' : '__TAURI_INTERNALS__ 缺失');

  // 桌宠窗（真 WebView2）
  const pet = await connectTarget((url) => url.includes('window=pet'));
  await pet.send('Runtime.enable');
  await pet.send('Page.enable');
  await pet.send('Page.addScriptToEvaluateOnNewDocument', {
    source: `window.__petSmokeUncaught = [];
window.addEventListener('error', (e) => window.__petSmokeUncaught.push(String(e.message || e.error)));
window.addEventListener('unhandledrejection', (e) => window.__petSmokeUncaught.push('rejection: ' + String(e.reason)));`,
  });
  await sleep(2500);
  // 状态钉死：显式设光态层再验（排除历史 localStorage 残留）
  await evalJs(pet, `localStorage.setItem('apeireth:pet:avatar-layer', 'light-sentinel'); true`);
  await pet.send('Page.navigate', {url: `http://127.0.0.1:1420/index.html?window=pet&rs=${Date.now()}`});
  await sleep(1500);
  // 首查前等挂载（dev 首载含 vite 编译，固定 sleep 不可靠——轮询到挂载为止）
  let mounted = false;
  for (let i = 0; i < 30; i++) {
    mounted = await evalJs(pet, `document.querySelector('.pet-root') !== null`);
    if (mounted) break;
    await sleep(1000);
  }
  record('真窗根视图挂载', mounted, mounted ? '' : '15s 内未挂载');
  // 形象层是懒装载（动态 import），出形前留轮询窗；失败带诊断（装载链掉在哪）
  let svgReady = false;
  let svgDiag = '';
  for (let i = 0; i < 10; i++) {
    svgReady = await evalJs(pet, `document.querySelector('.avatar-slot svg') !== null`);
    if (svgReady) break;
    await sleep(1000);
  }
  if (!svgReady) {
    svgDiag = await evalJs(
      pet,
      `JSON.stringify({
        layerAttr: document.querySelector('.avatar-slot > div')?.dataset?.layer ?? null,
        slotHtml: (document.querySelector('.avatar-slot')?.innerHTML ?? '').slice(0, 200),
        storedLayer: localStorage.getItem('apeireth:pet:avatar-layer'),
        uncaught: window.__petSmokeUncaught ?? [],
      })`,
    );
  }
  record('真窗默认光态哨兵出形（SVG）', svgReady, svgReady ? '' : svgDiag);

  // 真 WebView2 WebGL 画布（角色形象层）
  await evalJs(pet, `localStorage.setItem('apeireth:pet:avatar-layer', 'character-form'); true`);
  await pet.send('Page.navigate', {url: `http://127.0.0.1:1420/index.html?window=pet&rs=${Date.now()}`});
  await sleep(4000);
  record(
    '真窗角色形象层出画布（WebGL）',
    await evalJs(pet, `document.querySelector('.avatar-slot canvas') !== null`),
    '程序化素体',
  );
  const shot1 = await pet.send('Page.captureScreenshot', {format: 'png'});
  await writeFile(path.join(OUT_DIR, 'pet-real-character.png'), Buffer.from(shot1.data, 'base64'));

  // 2D 骨骼层
  await evalJs(pet, `localStorage.setItem('apeireth:pet:avatar-layer', 'skeletal-2d'); true`);
  await pet.send('Page.navigate', {url: `http://127.0.0.1:1420/index.html?window=pet&rs=${Date.now()}`});
  await sleep(3000);
  record(
    '真窗 2D 骨骼层出画布',
    await evalJs(pet, `document.querySelector('.avatar-slot canvas') !== null`),
  );
  const shot2 = await pet.send('Page.captureScreenshot', {format: 'png'});
  await writeFile(path.join(OUT_DIR, 'pet-real-skeletal2d.png'), Buffer.from(shot2.data, 'base64'));

  // 光态哨兵实拍
  await evalJs(pet, `localStorage.setItem('apeireth:pet:avatar-layer', 'light-sentinel'); true`);
  await pet.send('Page.navigate', {url: `http://127.0.0.1:1420/index.html?window=pet&rs=${Date.now()}`});
  await sleep(2500);
  const shot3 = await pet.send('Page.captureScreenshot', {format: 'png'});
  await writeFile(path.join(OUT_DIR, 'pet-real-light-sentinel.png'), Buffer.from(shot3.data, 'base64'));

  const uncaught = await evalJs(pet, `window.__petSmokeUncaught ?? []`);
  record('真窗无未捕获异常', Array.isArray(uncaught) && uncaught.length === 0, (uncaught || []).join(' | '));

  // 互斥礼让真验（确定性序）：先把焦点挪去快捷窗（主窗失焦），再唤回主窗
  // —— Focused(true) 真发 → 桌宠必须礼让隐藏。
  // 测量仪 = 已被单测钉死的开关决策表：礼让隐藏后 toggle = 唤起（窗还在）；
  // 没隐藏则 toggle = 销毁（窗消失）——看目标存活性，不看 DOM 可见性口径。
  await evalJs(main, `window.__TAURI_INTERNALS__.invoke('toggle_quick_window'), true`);
  await sleep(1200);
  await evalJs(main, `window.__TAURI_INTERNALS__.invoke('open_settings'), true`);
  await sleep(1500);
  const petDiag = await evalJs(
    pet,
    `JSON.stringify({visibility: document.visibilityState, hidden: document.hidden})`,
  );
  await evalJs(main, `window.__TAURI_INTERNALS__.invoke('toggle_pet_window'), true`);
  await sleep(1200);
  let petStillAlive = false;
  try {
    const res = await fetch(`http://127.0.0.1:${CDP_PORT}/json/list`);
    const targets = await res.json();
    petStillAlive = targets.some((x) => (x.url ?? '').includes('window=pet'));
  } catch {
    petStillAlive = false;
  }
  record(
    '互斥礼让：主窗聚焦 → 桌宠隐藏（决策表测量仪：toggle 后窗存=已隐藏）',
    petStillAlive,
    `pet 诊断=${petDiag}`,
  );
  await evalJs(main, `window.__TAURI_INTERNALS__.invoke('close_quick_window'), true`);

  main.close();
  pet.close();
} catch (caught) {
  record('实测流程完整性', false, String(caught));
} finally {
  killTree();
}

const failed = results.filter((r) => !r.ok);
console.log(
  failed.length === 0
    ? `\n✅ 真窗实测 ${results.length}/${results.length} 全过；实拍图在 reports/pet-real-smoke-2026-10-11/`
    : `\n❌ 真窗实测 ${failed.length}/${results.length} 未过`,
);
process.exit(failed.length === 0 ? 0 : 1);
