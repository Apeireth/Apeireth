#!/usr/bin/env node
// avatar-perf-probe.mjs —— 桌面常驻形象渲染性能探针一键采集器。
//
// 做四件事：
//   1) 起本地静态服务（serve frontend/perf-probe/，带 COOP/COEP 头以启用
//      performance.measureUserAgentSpecificMemory 的可选口径）；
//   2) 用本机 Edge/Chromium 无头模式（CDP）打开探针页，等三线测项跑完
//      （3×60s 稳态 + hidden 纪律 + WebGL 降级 + 5min 浸泡，约 9~10 分钟）；
//   3) 从 window.__AVATAR_PERF_PROBE__ 取结果存成
//      frontend/perf-probe/results/result.json（含浏览器进程树工作集采样），
//      再以 --disable-3d-apis 跑一遍降级验证 → results/result.webgl-off.json；
//   4) 打印摘要表。
//
// 用法：
//   node scripts/avatar-perf-probe.mjs                 # 全量采集（约 10 分钟）
//   node scripts/avatar-perf-probe.mjs --serve         # 只起服务，手动开页自测
//   node scripts/avatar-perf-probe.mjs --skip-soak --fps-sec 10   # 快速冒烟
//   node scripts/avatar-perf-probe.mjs --skip-degrade --headed    # 有头调试
// 其他参数：--port --cdp-port --soak-sec --hidden-sec --edge <msedge路径> --timeout-sec

import {createServer} from 'node:http';
import {readFile, writeFile, mkdir, stat} from 'node:fs/promises';
import {existsSync, mkdirSync, writeFileSync, readFileSync} from 'node:fs';
import {spawn, spawnSync} from 'node:child_process';
import path from 'node:path';
import os from 'node:os';
import {fileURLToPath} from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(__dirname, '..');
const PROBE_DIR = path.join(REPO, 'frontend', 'perf-probe');
const RESULTS_DIR = path.join(PROBE_DIR, 'results');

const args = process.argv.slice(2);
function argValue(name, fallback) {
  const i = args.indexOf(`--${name}`);
  return i >= 0 && args[i + 1] && !args[i + 1].startsWith('--') ? args[i + 1] : fallback;
}
function argFlag(name) {
  return args.includes(`--${name}`);
}
const PORT = Number(argValue('port', '8765'));
const CDP_PORT = Number(argValue('cdp-port', '9333'));
const FPS_SEC = Number(argValue('fps-sec', '60'));
const SOAK_SEC = Number(argValue('soak-sec', '300'));
const HIDDEN_SEC = Number(argValue('hidden-sec', '3'));
const TIMEOUT_SEC = Number(argValue('timeout-sec', '1500'));
const SERVE_ONLY = argFlag('serve');
const SKIP_DEGRADE = argFlag('skip-degrade');
const SKIP_SOAK = argFlag('skip-soak');
const HEADED = argFlag('headed');
const TAG = argValue('tag', '');
const SUFFIX = TAG ? `.${TAG}` : '';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const log = (m) => console.log(`[probe] ${m}`);
const nowISO = () => new Date().toISOString();

// ---------------- 静态服务 ----------------
const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.map': 'application/json; charset=utf-8',
  '.png': 'image/png',
  '.svg': 'image/svg+xml',
};
const posted = {}; // /api/result → 探针页 POST 回传的备份通道

function startServer() {
  return new Promise((resolve) => {
    const server = createServer(async (req, res) => {
      const url = new URL(req.url, `http://127.0.0.1:${PORT}`);
      if (req.method === 'POST' && url.pathname.startsWith('/api/')) {
        let body = '';
        for await (const chunk of req) body += chunk;
        try {
          posted[url.pathname] = JSON.parse(body);
          res.writeHead(200, {'content-type': 'application/json'});
          res.end('{"ok":true}');
        } catch (e) {
          res.writeHead(400);
          res.end('bad json');
        }
        return;
      }
      if (url.pathname === '/api/ping') {
        res.writeHead(200, {'content-type': 'application/json'});
        res.end('{"ok":true}');
        return;
      }
      const rel = url.pathname === '/' ? 'index.html' : url.pathname.replace(/^\/+/, '');
      const file = path.join(PROBE_DIR, path.normalize(rel));
      if (!file.startsWith(PROBE_DIR)) {
        res.writeHead(403);
        res.end('forbidden');
        return;
      }
      try {
        await stat(file);
        const buf = await readFile(file);
        res.writeHead(200, {
          'content-type': MIME[path.extname(file).toLowerCase()] || 'application/octet-stream',
          // 允许 performance.measureUserAgentSpecificMemory（可选口径，不受支持时探针自动留空）
          'Cross-Origin-Opener-Policy': 'same-origin',
          'Cross-Origin-Embedder-Policy': 'require-corp',
          'cache-control': 'no-store',
        });
        res.end(buf);
      } catch {
        res.writeHead(404);
        res.end('not found');
      }
    });
    server.listen(PORT, '127.0.0.1', () => resolve(server));
  });
}

// ---------------- 浏览器发现与启动 ----------------
function findBrowser() {
  const fromEnv = process.env.AVATAR_PERF_PROBE_BROWSER || process.env.EDGE_PATH || process.env.CHROME_PATH;
  const candidates = [
    fromEnv,
    'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe',
    'C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe',
    'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
    'C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe',
    path.join(os.homedir(), 'AppData\\Local\\Google\\Chrome\\Application\\chrome.exe'),
    path.join(os.homedir(), 'AppData\\Local\\Microsoft\\Edge\\Application\\msedge.exe'),
  ].filter(Boolean);
  return candidates.find((p) => existsSync(p)) || null;
}

function launchBrowser(browser, url, {extraArgs = [], label = 'run'} = {}) {
  const userDataDir = path.join(os.tmpdir(), `avatar-perf-probe-${label}-${Date.now()}`);
  mkdirSync(userDataDir, {recursive: true});
  const browserArgs = [
    ...(HEADED ? [] : ['--headless=new']),
    `--remote-debugging-port=${CDP_PORT}`,
    `--user-data-dir=${userDataDir}`,
    '--window-size=1000,920',
    // 反节流：保证 rAF 采样口径稳定
    '--disable-background-timer-throttling',
    '--disable-renderer-backgrounding',
    '--disable-backgrounding-occluded-windows',
    // 暴露 gc()，让探针能在同一条件下取"GC 后堆"口径
    '--js-flags=--expose-gc',
    // 无头环境下的软件 WebGL 兜底（有 GPU 时优先走 GPU）
    '--enable-unsafe-swiftshader',
    '--no-first-run',
    '--no-default-browser-check',
    '--disable-extensions',
    '--disable-component-update',
    '--hide-scrollbars',
    '--autoplay-policy=no-user-gesture-required',
    ...extraArgs,
    url,
  ];
  log(`启动无头浏览器：${path.basename(browser)}（${userDataDir}）`);
  const child = spawn(browser, browserArgs, {stdio: 'ignore', detached: false});
  return {child, userDataDir};
}

function killTree(child) {
  if (!child || child.exitCode !== null) return;
  try {
    spawnSync('taskkill', ['/pid', String(child.pid), '/T', '/F'], {stdio: 'ignore'});
  } catch {
    try {
      child.kill();
    } catch {
      /* 已退出 */
    }
  }
}

// ---------------- CDP（复用 scripts/wheel-scroll-probe.mjs 的连接形态） ----------------
async function listTargets(port = CDP_PORT) {
  const res = await fetch(`http://127.0.0.1:${port}/json/list`);
  return res.json();
}

function connect(wsUrl) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(wsUrl);
    const pending = new Map();
    let nextId = 1;
    ws.onopen = () =>
      resolve({
        send(method, params = {}) {
          const id = nextId++;
          ws.send(JSON.stringify({id, method, params}));
          return new Promise((res, rej) => {
            pending.set(id, {res, rej});
            setTimeout(() => {
              if (pending.delete(id)) rej(new Error(`CDP timeout: ${method}`));
            }, 30000);
          });
        },
        close: () => ws.close(),
      });
    ws.onerror = (e) => reject(new Error('CDP ws error: ' + (e.message || e.type)));
    ws.onmessage = (ev) => {
      const msg = JSON.parse(ev.data);
      const waiter = msg.id && pending.get(msg.id);
      if (waiter) {
        pending.delete(msg.id);
        if (msg.error) waiter.rej(new Error(msg.error.message));
        else waiter.res(msg.result);
      }
    };
  });
}

async function connectToProbePage() {
  for (let i = 0; i < 60; i++) {
    try {
      const targets = await listTargets();
      const page = targets.find((t) => t.type === 'page' && t.url.includes('127.0.0.1')) || targets.find((t) => t.type === 'page');
      if (page && page.webSocketDebuggerUrl) return connect(page.webSocketDebuggerUrl);
    } catch {
      /* 浏览器还没起来 */
    }
    await sleep(500);
  }
  throw new Error('CDP 未就绪（浏览器没起来或调试端口被拒）');
}

async function evaluate(cdp, expression) {
  const out = await cdp.send('Runtime.evaluate', {expression, returnByValue: true, awaitPromise: true});
  if (out.exceptionDetails) {
    throw new Error('eval failed: ' + JSON.stringify(out.exceptionDetails).slice(0, 200));
  }
  return out.result.value;
}

// ---------------- 浏览器进程树工作集采样（Windows） ----------------
const PS_SCRIPT = `
param([int]$RootProcId, [string]$OutFile)
$ErrorActionPreference = 'SilentlyContinue'
$all = Get-CimInstance Win32_Process
$byId = @{}
foreach ($p in $all) { $byId[[int]$p.ProcessId] = $p }
$seen = New-Object 'System.Collections.Generic.HashSet[int]'
function Walk([int]$procId) {
  if (-not $seen.Add($procId)) { return }
  foreach ($p in $byId.Values) {
    if ([int]$p.ParentProcessId -eq $procId) { Walk ([int]$p.ProcessId) }
  }
}
Walk $RootProcId
$sum = [long]0
$procs = @()
foreach ($id in $seen) {
  $p = $byId[$id]
  if ($p) {
    $sum += [long]$p.WorkingSetSize
    $ptype = 'browser'
    if ($p.CommandLine -match '--type=([a-z-]+)') { $ptype = $Matches[1] }
    $procs += [pscustomobject]@{ pid = $id; name = $p.Name; type = $ptype; wsMB = [math]::Round([long]$p.WorkingSetSize / 1MB, 1) }
  }
}
$result = [pscustomobject]@{
  ts = (Get-Date).ToString('o')
  rootPid = $RootProcId
  processCount = $seen.Count
  totalWsMB = [math]::Round($sum / 1MB, 1)
  processes = $procs
}
$result | ConvertTo-Json -Depth 4 | Out-File -Encoding utf8 $OutFile
`;

let psScriptPath = null;
function ensurePsScript() {
  if (!psScriptPath) {
    psScriptPath = path.join(os.tmpdir(), `avatar-perf-probe-ws-${Date.now()}.ps1`);
    writeFileSync(psScriptPath, PS_SCRIPT, 'utf8');
  }
  return psScriptPath;
}

function sampleProcessTree(rootPid) {
  const outFile = path.join(os.tmpdir(), `avatar-perf-probe-ws-${Date.now()}.json`);
  try {
    spawnSync(
      'powershell',
      ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', ensurePsScript(), '-RootProcId', String(rootPid), '-OutFile', outFile],
      {stdio: 'ignore', timeout: 20000},
    );
    const raw = readFileSync(outFile, 'utf8');
    return JSON.parse(raw.replace(/^\uFEFF/, ''));
  } catch (e) {
    return {error: String(e).slice(0, 160)};
  }
}

// ---------------- 跑一轮探针 ----------------
async function runProbeSession({browser, url, label, expectDone = true, extraArgs = []}) {
  log(`打开探针页：${url}${extraArgs.length ? '（' + extraArgs.join(' ') + '）' : ''}`);
  for (const k of Object.keys(posted)) delete posted[k]; // 会话隔离：别拿上一轮的回传结果
  const {child} = launchBrowser(browser, url, {label, extraArgs});
  const samples = [];
  let result = null;
  try {
    const cdp = await connectToProbePage();
    await sleep(2500);
    // 留证截图（验证形象确实画出来了）：预热后一张、收尾一张
    let shotTaken = false;
    const takeShot = async (tag) => {
      try {
        const shot = await cdp.send('Page.captureScreenshot', {format: 'png', fromSurface: true});
        const file = path.join(RESULTS_DIR, `shot-${label}-${tag}.png`);
        await writeFile(file, Buffer.from(shot.data, 'base64'));
        log(`截图：${file}`);
      } catch (e) {
        log('截图失败（不影响采集）：' + String(e).slice(0, 120));
      }
    };
    // 基线：页面刚起来（预热前）的浏览器进程树工作集
    const baseline = sampleProcessTree(child.pid);
    const t0 = Date.now();
    let lastPhase = '';
    let lastSampleAt = -999;
    while ((Date.now() - t0) / 1000 < TIMEOUT_SEC) {
      const el = Math.round((Date.now() - t0) / 1000);
      let status = null;
      try {
        status = JSON.parse(
          await evaluate(cdp, `(() => { const w = window.__AVATAR_PERF_PROBE__; return w ? JSON.stringify({done: !!w.done, phase: (w.meta && w.meta.phase) || ''}) : 'null'; })()`),
        );
      } catch (e) {
        log('CDP 轮询失败（重试）：' + String(e).slice(0, 120));
      }
      if (status && status.phase !== lastPhase) {
        lastPhase = status.phase;
        log(`[${el}s] ${status.phase}`);
      }
      if (status && status.done && expectDone) {
        await takeShot('done');
        const raw = await evaluate(cdp, 'JSON.stringify(window.__AVATAR_PERF_PROBE__)');
        result = JSON.parse(raw);
        break;
      }
      if (!shotTaken && el >= 10) {
        shotTaken = true;
        await takeShot('mid');
      }
      if (el - lastSampleAt >= 15) {
        lastSampleAt = el;
        const ws = sampleProcessTree(child.pid);
        if (!ws.error) {
          samples.push({tSec: el, phase: lastPhase, totalWsMB: ws.totalWsMB, processCount: ws.processCount, processes: ws.processes});
        }
      }
      await sleep(5000);
    }
    if (!result && posted['/api/result']) result = posted['/api/result'];
    if (!result && posted['/api/result-degrade']) result = posted['/api/result-degrade'];
    if (!result) throw new Error(`超时 ${TIMEOUT_SEC}s 未拿到 window.__AVATAR_PERF_PROBE__`);
    result.hostProcess = {
      note: '浏览器进程树工作集（WorkingSetSize 合计，含浏览器/GPU/渲染器等多进程；是宿主壳+内核近似，不是 WebView2 实体）',
      rootPid: child.pid,
      baseline,
      samples,
      minWsMB: samples.length ? Math.min(...samples.map((s) => s.totalWsMB)) : null,
      maxWsMB: samples.length ? Math.max(...samples.map((s) => s.totalWsMB)) : null,
    };
    result.runner = {
      collectedAt: nowISO(),
      browser: path.basename(browser),
      browserPath: browser,
      headless: !HEADED,
      cdpPort: CDP_PORT,
      staticPort: PORT,
      url,
    };
    return result;
  } finally {
    killTree(child);
  }
}

// ---------------- 摘要 ----------------
function printSummary(result) {
  const L = result.lines || {};
  const row = (label, l) => {
    if (!l) return `${label.padEnd(8)}  未测`;
    if (!l.available) return `${label.padEnd(8)}  不可用（${l.reason || 'n/a'}）`;
    const f = l.fps || {};
    const h = l.heap || {};
    return (
      `${label.padEnd(8)}  FPS ${String(f.fpsMean ?? '—').padStart(6)} | 1% low ${String(f.fps1Low ?? '—').padStart(6)} | ` +
      `每帧JS ${String(l.frameJsMs?.mean ?? '—').padStart(6)}ms | JS堆稳态 ${String(h.steadyMedianMB ?? '—').padStart(5)}MB | ` +
      `堆峰值 ${String(h.maxMB ?? '—').padStart(5)}MB`
    );
  };
  console.log('\n================ 摘要 ================');
  console.log(row('3D', L.three3d));
  console.log(row('2D', L.canvas2d));
  console.log(row('光态', L.lightCss));
  const hs = result.disciplines?.hiddenStop || {};
  console.log('纪律一 document.hidden 停渲染：');
  for (const [k, v] of Object.entries(hs)) {
    console.log(
      `  ${k.padEnd(9)} 隐藏期渲染帧=${v.framesDuringHidden}（应 0）  同期 rAF 心跳=${v.rafTicksDuringHidden}（应 >0）  恢复后帧=${v.framesAfterResume}  → ${v.pass ? 'PASS' : 'FAIL'}`,
    );
  }
  const fb = result.disciplines?.webglFallback?.inPageStub;
  const real = result.disciplines?.webglFallback?.realRun;
  console.log('纪律二 WebGL 不可用降级：');
  if (fb) console.log(`  页内 stub：3D 报不可用=${fb.three3dReportsUnavailable}，2D 照常=${fb.twoDStillRuns}，光态照常=${fb.lightStillRuns} → ${fb.pass ? 'PASS' : 'FAIL'}`);
  if (real) console.log(`  真实 --disable-3d-apis：3D 报不可用=${real.three3dReportsUnavailable}（${real.reason3d || '—'}）→ ${real.pass ? 'PASS' : 'FAIL'}`);
  const s = result.soak;
  if (s) {
    console.log(`浸泡 ${s.durationSec}s（${(s.loadsRunning || []).join('+')}）：JS 堆 ${s.heapStartMB}MB → ${s.heapEndMB}MB，漂移 ${s.driftMB}MB（约 ${s.slopeMBPerMin}MB/min）`);
  }
  const hp = result.hostProcess;
  if (hp && hp.samples?.length) {
    console.log(`浏览器进程树工作集：基线 ${hp.baseline?.totalWsMB ?? '—'}MB，采样 min ${hp.minWsMB}MB / max ${hp.maxWsMB}MB（${hp.samples.length} 个样本）`);
  }
  console.log('=======================================\n');
}

// ---------------- 主流程 ----------------
async function main() {
  if (!existsSync(path.join(PROBE_DIR, 'index.html'))) {
    console.error(`FATAL: 探针页不存在：${path.join(PROBE_DIR, 'index.html')}`);
    process.exit(1);
  }
  await mkdir(RESULTS_DIR, {recursive: true});
  const server = await startServer();
  const base = `http://127.0.0.1:${PORT}/index.html`;
  log(`静态服务已起：${base}`);
  if (SERVE_ONLY) {
    log('手动自测模式：浏览器打开上面的地址，跑完后读 window.__AVATAR_PERF_PROBE__ 或看页面结果面板。Ctrl+C 退出。');
    return;
  }
  const browser = findBrowser();
  if (!browser) {
    console.error('FATAL: 找不到 Edge/Chrome。请手动自测：node scripts/avatar-perf-probe.mjs --serve 后用浏览器打开页面。');
    server.close();
    process.exit(2);
  }
  log(`浏览器：${browser}`);

  const fullUrl = `${base}?mode=full&fpsSec=${FPS_SEC}&soakSec=${SKIP_SOAK ? 5 : SOAK_SEC}&hiddenSec=${HIDDEN_SEC}`;
  const full = await runProbeSession({browser, url: fullUrl, label: TAG ? `full-${TAG}` : 'full'});
  const outPath = path.join(RESULTS_DIR, `result${SUFFIX}.json`);
  await writeFile(outPath, JSON.stringify(full, null, 2), 'utf8');
  log(`结果已存：${outPath}`);
  printSummary(full);

  if (!SKIP_DEGRADE) {
    const degUrl = `${base}?mode=degrade&hiddenSec=${HIDDEN_SEC}`;
    const deg = await runProbeSession({
      browser,
      url: degUrl,
      label: TAG ? `degrade-${TAG}` : 'degrade',
      expectDone: true,
      // 真实禁用 WebGL 启动：验证 WebGL 不可用时的优雅降级
      extraArgs: ['--disable-3d-apis'],
    });
    deg.runner = {...deg.runner, extraArgs: ['--disable-3d-apis'], note: '本轮以 --disable-3d-apis 启动，验证 WebGL 不可用时的优雅降级'};
    const degPath = path.join(RESULTS_DIR, `result.webgl-off${SUFFIX}.json`);
    await writeFile(degPath, JSON.stringify(deg, null, 2), 'utf8');
    log(`降级验证结果已存：${degPath}`);
  }
  server.close();
  log('采集完成。');
}

main().catch((e) => {
  console.error('FATAL:', e);
  process.exit(1);
});
