#!/usr/bin/env node
// pet-window-smoke.mjs —— 桌宠窗构建产物烟测（?window=pet）。
//
// 做四件事：
//   1) pnpm build 出 dist（--skip-build 可跳过）；
//   2) 静态服务 dist，无头内核（Edge/Chromium，CDP）加载 index.html?window=pet；
//   3) 断言桌宠窗三件事：根视图挂载、默认光态形象渲染出形、切「角色形象」层后
//     （WebGL 可用时）画布出现；收集未捕获异常（网络重试噪声不算）；
//   4) 打印判定，非 0 退出码 = 红。
//
// 用法：node scripts/pet-window-smoke.mjs [--skip-build] [--headed] [--edge <msedge路径>]
// 口径：无头内核与 WebView2 同内核近似（诚实边界同 reports/avatar-perf-probe-2026-10-11.md）。

import {createServer} from 'node:http';
import {readFile} from 'node:fs/promises';
import {existsSync} from 'node:fs';
import {spawn, spawnSync} from 'node:child_process';
import path from 'node:path';
import os from 'node:os';
import {fileURLToPath} from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(__dirname, '..');
const APP_DIR = path.join(REPO, 'frontend', 'companion-desktop');
const DIST = path.join(APP_DIR, 'dist');

const args = process.argv.slice(2);
const argValue = (name, fallback) => {
  const i = args.indexOf(`--${name}`);
  return i >= 0 && args[i + 1] && !args[i + 1].startsWith('--') ? args[i + 1] : fallback;
};
const argFlag = (name) => args.includes(`--${name}`);
const PORT = Number(argValue('port', '8791'));
const CDP_PORT = Number(argValue('cdp-port', '9341'));
const HEADED = argFlag('headed');
const SKIP_BUILD = argFlag('skip-build');

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const log = (m) => console.log(`[pet-smoke] ${m}`);

// ---------------- 1) 构建 ----------------
if (!SKIP_BUILD) {
  log('pnpm build …');
  // Windows 下 pnpm 是 .cmd 包装：显式走 cmd /c，不用 shell:true（弃用告警与注入面双清）
  const pnpm = process.platform === 'win32' ? 'cmd' : 'pnpm';
  const pnpmArgs = process.platform === 'win32' ? ['/c', 'pnpm', 'build'] : ['build'];
  const built = spawnSync(pnpm, pnpmArgs, {cwd: APP_DIR, stdio: 'inherit'});
  if (built.status !== 0) {
    console.error('❌ pnpm build 失败');
    process.exit(1);
  }
}
if (!existsSync(path.join(DIST, 'index.html'))) {
  console.error('❌ dist/index.html 不存在');
  process.exit(1);
}

// ---------------- 2) 静态服务 ----------------
const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.png': 'image/png',
  '.webp': 'image/webp',
  '.svg': 'image/svg+xml',
  '.wasm': 'application/wasm',
};
const server = createServer(async (req, res) => {
  try {
    const url = new URL(req.url, `http://127.0.0.1:${PORT}`);
    const rel = decodeURIComponent(url.pathname).replace(/^\/+/, '') || 'index.html';
    const abs = path.join(DIST, rel);
    if (!abs.startsWith(DIST)) {
      res.writeHead(403).end();
      return;
    }
    const body = await readFile(abs);
    res.writeHead(200, {'content-type': MIME[path.extname(abs)] ?? 'application/octet-stream'});
    res.end(body);
  } catch {
    res.writeHead(404).end();
  }
});
await new Promise((r) => server.listen(PORT, '127.0.0.1', r));
log(`静态服务 :${PORT}`);

// ---------------- 3) 无头内核 + CDP ----------------
function findEdge() {
  if (argValue('edge', '')) return argValue('edge', '');
  const candidates = [
    'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe',
    'C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe',
    '/usr/bin/microsoft-edge',
    '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge',
  ];
  return candidates.find((p) => existsSync(p)) ?? 'msedge';
}

const userDataDir = path.join(os.tmpdir(), `pet-smoke-${Date.now()}`);
const browser = spawn(
  findEdge(),
  [
    ...(HEADED ? [] : ['--headless=new']),
    `--remote-debugging-port=${CDP_PORT}`,
    `--user-data-dir=${userDataDir}`,
    '--no-first-run',
    '--disable-gpu-sandbox',
    '--window-size=420,520',
    'about:blank',
  ],
  {stdio: 'ignore'},
);
log('无头内核启动中…');

async function connectPage() {
  for (let i = 0; i < 60; i++) {
    try {
      const res = await fetch(`http://127.0.0.1:${CDP_PORT}/json/list`);
      const targets = await res.json();
      const page = targets.find((t) => t.type === 'page');
      if (page?.webSocketDebuggerUrl) {
        const ws = new WebSocket(page.webSocketDebuggerUrl);
        await new Promise((resolve, reject) => {
          ws.onopen = resolve;
          ws.onerror = () => reject(new Error('CDP ws error'));
        });
        let seq = 0;
        const pending = new Map();
        ws.onmessage = (event) => {
          const msg = JSON.parse(event.data);
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
      // 未就绪，续等
    }
    await sleep(500);
  }
  throw new Error('CDP 未就绪');
}

const cdp = await connectPage();
const errors = [];
await cdp.send('Runtime.enable');
await cdp.send('Log.enable');
await cdp.send('Page.enable');
// 未捕获异常收集器先装后导航（error + unhandledrejection）；网络重试噪声走
// 事件处理器内 catch，不算未捕获。
await cdp.send('Page.addScriptToEvaluateOnNewDocument', {
  source: `window.__petSmokeUncaught = [];
window.addEventListener('error', (e) => window.__petSmokeUncaught.push(String(e.message || e.error)));
window.addEventListener('unhandledrejection', (e) => window.__petSmokeUncaught.push('rejection: ' + String(e.reason)));`,
});

async function evalJs(expression) {
  const out = await cdp.send('Runtime.evaluate', {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  if (out.exceptionDetails) {
    throw new Error(out.exceptionDetails.text ?? 'evaluate 异常');
  }
  return out.result?.value;
}

async function loadPet(layerId) {
  await cdp.send('Page.navigate', {
    url: `http://127.0.0.1:${PORT}/index.html?window=pet&smoke=${Date.now()}`,
  });
  await sleep(2500);
  if (layerId) {
    await evalJs(`localStorage.setItem('apeireth:pet:avatar-layer', '${layerId}'); true`);
    await cdp.send('Page.navigate', {
      url: `http://127.0.0.1:${PORT}/index.html?window=pet&smoke=${Date.now()}`,
    });
    await sleep(3500);
  }
}

const results = [];
function record(name, ok, detail = '') {
  results.push({name, ok, detail});
  log(`${ok ? '✅' : '❌'} ${name}${detail ? ` — ${detail}` : ''}`);
}

// ---- 用例 1：默认光态形象出形 ----
await loadPet(null);
record(
  '桌宠窗根视图挂载',
  await evalJs(`document.querySelector('.pet-root') !== null`),
);
record(
  '默认光态哨兵渲染出形（SVG）',
  await evalJs(`document.querySelector('.avatar-slot svg') !== null`),
);
record(
  '形象层登记正确（data-layer=light-sentinel）',
  (await evalJs(`document.querySelector('.avatar-slot')?.firstElementChild?.dataset?.layer ?? ''`)) ===
    'light-sentinel',
);

// ---- 用例 2：角色形象层（WebGL 可用时出画布）----
await loadPet('character-form');
const webgl = await evalJs(`(() => {
  const c = document.createElement('canvas');
  return !!(c.getContext('webgl2') || c.getContext('webgl'));
})()`);
record('WebGL 可用性探测', true, webgl ? '可用' : '不可用（预期回落光态）');
if (webgl) {
  record(
    '角色形象层渲染出画布',
    await evalJs(`document.querySelector('.avatar-slot canvas') !== null`),
    '程序化素体（无形象文件）',
  );
  record(
    '角色形象层登记正确（data-layer=character-form）',
    (await evalJs(
      `document.querySelector('.avatar-slot')?.firstElementChild?.dataset?.layer ?? ''`,
    )) === 'character-form',
  );
} else {
  record(
    'WebGL 不可用 → 回落微光简模（降级纪律）',
    (await evalJs(
      `document.querySelector('.avatar-slot')?.firstElementChild?.dataset?.layer ?? ''`,
    )) === 'ember-minimal',
  );
}

// ---- 用例 3：未捕获异常巡检 ----
const uncaught = await evalJs(`window.__petSmokeUncaught ?? []`);
if (Array.isArray(uncaught) && uncaught.length > 0) {
  errors.push(...uncaught);
}
record('无未捕获异常', errors.length === 0, errors.join(' | '));

cdp.close();
browser.kill();
server.close();

const failed = results.filter((r) => !r.ok);
console.log(
  failed.length === 0
    ? `\n✅ 桌宠窗烟测 ${results.length}/${results.length} 全过（口径：无头内核 ≈ WebView2 同内核近似）`
    : `\n❌ 桌宠窗烟测 ${failed.length}/${results.length} 未过`,
);
process.exit(failed.length === 0 ? 0 : 1);
