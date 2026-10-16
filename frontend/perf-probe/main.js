// avatar-perf-probe / main.js
// 桌面常驻小窗（220×260 透明置顶窗，WebView2/Chromium 语义）形象渲染性能探针。
// 三条形象负载同页顺序实测：
//   [3D]  3D 卡通渲染路线 —— three.js 程序化人形（胶囊/球体拼装 + 卡通渐变着色 + 待机呼吸/口型/视线）
//   [2D]  2D 形象路线   —— 纯 Canvas2D 骨骼小人（三十余节骨骼、四十余描画段、呼吸/口型/视线/眨眼）
//   [光]  光态基线      —— CSS/SVG 呼吸光点（合成器驱动，JS 每帧近零开销）
// 测量口径：
//   稳态 FPS = rAF 采样（均值 = 帧数/时长；1% low = 瞬时帧率最低 1% 帧的均值）
//   JS 堆   = performance.memory.usedJSHeapSize（稳态中位数；采样期每 1s 一记）
//   两条纪律 = document.hidden 时渲染循环必须停（帧计数 + 全局 rAF 心跳对照）；
//             WebGL 不可用时优雅降级（页内 stub 注入 + 真实 --disable-3d-apis 两路验证）
// 全部人形/骨骼均为代码程序化生成，无任何下载美术资产。

const qs = new URLSearchParams(location.search);
const MODE = qs.get('mode') === 'degrade' ? 'degrade' : 'full';
const FPS_SEC = clampNum(qs.get('fpsSec'), 60, 5, 600);
const SOAK_SEC = clampNum(qs.get('soakSec'), 300, 5, 1800);
const HIDDEN_SEC = clampNum(qs.get('hiddenSec'), 3, 1, 30);
const STAGE = { w: 220, h: 260 };

function clampNum(v, dflt, lo, hi) {
  const n = Number(v);
  return Number.isFinite(n) && n > 0 ? Math.min(hi, Math.max(lo, n)) : dflt;
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const $ = (id) => document.getElementById(id);

// ---------------- 数值助手 ----------------
function mean(arr) {
  if (!arr.length) return 0;
  return arr.reduce((a, b) => a + b, 0) / arr.length;
}
function median(arr) {
  if (!arr.length) return 0;
  const s = [...arr].sort((a, b) => a - b);
  return s[Math.floor(s.length / 2)];
}
function percentile(arr, p) {
  if (!arr.length) return 0;
  const s = [...arr].sort((a, b) => a - b);
  return s[Math.min(s.length - 1, Math.floor(s.length * p))];
}
const r2 = (x) => Math.round(x * 100) / 100;

// ---------------- 帧记录（rAF 采样口径） ----------------
class FrameRecorder {
  constructor() {
    this.enabled = false;
    this.frames = 0;
    this.tFirst = 0;
    this.tLast = 0;
    this.fpsInst = [];
    this.jsMs = [];
  }
  begin() {
    this.enabled = true;
    this.frames = 0;
    this.tFirst = 0;
    this.tLast = 0;
    this.fpsInst = [];
    this.jsMs = [];
  }
  end() {
    this.enabled = false;
  }
  tick(t, jsMs) {
    if (!this.enabled) return;
    if (!this.tFirst) this.tFirst = t;
    if (this.tLast) this.fpsInst.push(1000 / Math.max(t - this.tLast, 0.1));
    this.tLast = t;
    this.frames++;
    if (this.jsMs.length < 300000) this.jsMs.push(jsMs);
  }
  stats() {
    const elapsedSec = this.tLast && this.tFirst ? (this.tLast - this.tFirst) / 1000 : 0;
    const fpsMean = elapsedSec > 0 ? (this.frames - 1) / elapsedSec : 0;
    const sorted = [...this.fpsInst].sort((a, b) => a - b);
    const k = Math.max(1, Math.floor(sorted.length * 0.01));
    const low1 = sorted.slice(0, k).reduce((a, b) => a + b, 0) / k;
    return {
      frames: this.frames,
      elapsedSec: r2(elapsedSec),
      fpsMean: r2(fpsMean),
      fps1Low: r2(low1),
      fpsMin: r2(sorted[0] || 0),
      jsMsMean: Math.round(mean(this.jsMs) * 1000) / 1000,
      jsMsP95: Math.round(percentile(this.jsMs, 0.95) * 1000) / 1000,
    };
  }
}

// ---------------- 全局 rAF 心跳：不看 hidden，用来证明"停帧"是循环自律而非浏览器停发 ----------------
let globalTicks = 0;
(function heartbeat() {
  globalTicks++;
  requestAnimationFrame(heartbeat);
})();

// ---------------- 内存 ----------------
function heapMB() {
  const m = performance.memory;
  return m ? m.usedJSHeapSize / 1048576 : null;
}
function gcNow() {
  if (typeof window.gc === 'function') window.gc();
}
async function totalMemoryMB() {
  try {
    if (performance.measureUserAgentSpecificMemory && window.crossOriginIsolated) {
      const r = await performance.measureUserAgentSpecificMemory();
      return r2(r.bytes / 1048576);
    }
  } catch (e) {
    /* 无跨源隔离或不受支持 → 留空，如实记 null */
  }
  return null;
}

// ---------------- hidden 强制注入（模拟 document.hidden，供无头环境验证渲染循环自律） ----------------
let hiddenForced = false;
function forceHidden(v) {
  if (v) {
    Object.defineProperty(document, 'hidden', { configurable: true, get: () => true });
    Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => 'hidden' });
    hiddenForced = true;
  } else {
    delete document.hidden;
    delete document.visibilityState;
    hiddenForced = false;
  }
  document.dispatchEvent(new Event('visibilitychange'));
}
// 真实可见性切换（手动测试时最小化窗口可触发）也留档
document.addEventListener('visibilitychange', () => {
  log(`[visibilitychange] hidden=${document.hidden}${hiddenForced ? '（强制注入）' : '（真实切换）'}`);
});

// ---------------- 渲染线控制器（每条负载一个自己的 rAF 循环 + hidden 纪律） ----------------
function makeLine(id, update, render) {
  const line = {
    id,
    update,
    render,
    rafId: 0,
    running: false,
    wasRunningBeforeHidden: false,
    frames: 0, // 已渲染帧（隐藏期间必须为 0）
    callbacks: 0, // rAF 回调次数
    hiddenSkips: 0, // 保险丝：hidden 时跳过渲染的次数
    rec: new FrameRecorder(),
    start() {
      if (this.running) return;
      this.running = true;
      this.rafId = requestAnimationFrame(step);
    },
    stop() {
      this.running = false;
      if (this.rafId) cancelAnimationFrame(this.rafId);
      this.rafId = 0;
    },
  };
  function step(t) {
    if (!line.running) return;
    line.rafId = requestAnimationFrame(step);
    line.callbacks++;
    if (document.hidden) {
      // 保险丝：即便 visibilitychange 没触发（极端时序），也不渲染
      line.hiddenSkips++;
      return;
    }
    const j0 = performance.now();
    if (line.update) line.update(t);
    if (line.render) line.render();
    const j1 = performance.now();
    line.frames++;
    line.rec.tick(t, j1 - j0);
  }
  // 纪律：document.hidden → 停循环；恢复可见 → 重启循环
  document.addEventListener('visibilitychange', () => {
    if (document.hidden) {
      if (line.running) {
        line.wasRunningBeforeHidden = true;
        line.stop();
      }
    } else if (line.wasRunningBeforeHidden) {
      line.wasRunningBeforeHidden = false;
      line.start();
    }
  });
  return line;
}

// ---------------- [3D] 3D 卡通渲染路线（three.js，程序化人形） ----------------
function toonGradient(THREE) {
  const data = new Uint8Array([90, 175, 255]); // 3 阶卡通明暗
  const tex = new THREE.DataTexture(data, 3, 1, THREE.RedFormat);
  tex.minFilter = THREE.NearestFilter;
  tex.magFilter = THREE.NearestFilter;
  tex.needsUpdate = true;
  return tex;
}

async function build3D(canvas) {
  // three 运行库由 importmap 解析；未安装/不可达时如实报不可用
  let THREE;
  try {
    THREE = await import('three');
  } catch (e) {
    return { available: false, reason: 'three-not-installed: ' + String(e).slice(0, 160) };
  }
  let renderer;
  try {
    renderer = new THREE.WebGLRenderer({
      canvas,
      alpha: true,
      antialias: true,
      powerPreference: 'low-power',
      preserveDrawingBuffer: false,
    });
  } catch (e) {
    // WebGL 不可用（禁用/无上下文）→ 报不可用，由宿主回落，不得抛穿
    return { available: false, reason: 'webgl-unavailable: ' + String(e).slice(0, 160) };
  }
  if (!renderer || !renderer.getContext()) {
    return { available: false, reason: 'webgl-context-null' };
  }

  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
  renderer.setSize(STAGE.w, STAGE.h, false);
  renderer.setClearColor(0x000000, 0);

  const gl = renderer.getContext();
  const dbg = gl.getExtension('WEBGL_debug_renderer_info');
  const glInfo = {
    renderer: dbg ? gl.getParameter(dbg.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER),
    vendor: dbg ? gl.getParameter(dbg.UNMASKED_VENDOR_WEBGL) : gl.getParameter(gl.VENDOR),
    version: gl.getParameter(gl.VERSION),
  };

  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(28, STAGE.w / STAGE.h, 0.1, 50);
  camera.position.set(0, 1.02, 4.3);
  camera.lookAt(0, 1.02, 0);

  const grad = toonGradient(THREE);
  const toon = (color, extra = {}) =>
    new THREE.MeshToonMaterial({ color, gradientMap: grad, ...extra });

  // 光照：主光 + 半球 + 轮廓光（卡通渲染常见三灯）
  scene.add(new THREE.HemisphereLight(0xbfd4ff, 0x2b2233, 0.9));
  const key = new THREE.DirectionalLight(0xfff1dc, 1.5);
  key.position.set(2.2, 3.2, 2.6);
  scene.add(key);
  const rim = new THREE.DirectionalLight(0x9fb8ff, 0.7);
  rim.position.set(-2.4, 1.4, -2.2);
  scene.add(rim);

  // ---- 程序化人形：关节层级 + 胶囊/球体拼装（约 20 节关节、30+ 网格） ----
  const matSkin = toon(0xf0c8a0);
  const matCloth = toon(0x4f6fb0);
  const matCloth2 = toon(0x39518a);
  const matHair = toon(0x5a4632);
  const matDark = toon(0x241d28);
  const matWhite = toon(0xf6f2ee);
  const matMouth = toon(0xa8544e);
  const matScarf = toon(0xd8b25a);

  const cap = (r, h, mat) => new THREE.Mesh(new THREE.CapsuleGeometry(r, h, 5, 12), mat);
  const sph = (r, mat, ws = 16, hs = 12) => new THREE.Mesh(new THREE.SphereGeometry(r, ws, hs), mat);
  const box = (w, h, d, mat) => new THREE.Mesh(new THREE.BoxGeometry(w, h, d), mat);

  const root = new THREE.Group();
  scene.add(root);
  const J = {};
  const joint = (name, parent, x, y, z) => {
    const g = new THREE.Group();
    g.position.set(x, y, z);
    (parent || root).add(g);
    J[name] = g;
    return g;
  };

  // 躯干链
  joint('pelvis', root, 0, 0.95, 0).add(box(0.32, 0.2, 0.22, matCloth2));
  const spine = joint('spine', J.pelvis, 0, 0.16, 0);
  const chest = joint('chest', spine, 0, 0.16, 0);
  chest.add((() => { const m = cap(0.185, 0.26, matCloth); m.position.y = 0.08; return m; })());
  joint('neck', chest, 0, 0.3, 0).add((() => { const m = cap(0.055, 0.06, matSkin); return m; })());
  const head = joint('head', J.neck, 0, 0.14, 0);
  const headMesh = sph(0.17, matSkin, 24, 18);
  headMesh.scale.set(1, 1.08, 0.98);
  head.add(headMesh);
  // 头发（程序化：球壳 + 刘海条）
  const hair = joint('hair', head, 0, 0.02, -0.01);
  const hairMesh = sph(0.183, matHair, 20, 14);
  hairMesh.scale.set(1, 0.95, 1);
  hairMesh.position.y = 0.045;
  hair.add(hairMesh);
  for (let i = -2; i <= 2; i++) {
    const b = box(0.052, 0.1, 0.03, matHair);
    b.position.set(i * 0.058, 0.09 - Math.abs(i) * 0.012, 0.155);
    b.rotation.x = -0.25;
    hair.add(b);
  }
  // 眼睛（gaze 组）+ 眉毛
  const gaze = joint('gaze', head, 0, 0, 0);
  for (const sgn of [-1, 1]) {
    const eye = sph(0.029, matWhite, 12, 10);
    eye.position.set(sgn * 0.062, 0.028, 0.152);
    gaze.add(eye);
    const pupil = sph(0.016, matDark, 10, 8);
    pupil.position.set(sgn * 0.062, 0.028, 0.172);
    gaze.add(pupil);
    const brow = box(0.055, 0.011, 0.012, matHair);
    brow.position.set(sgn * 0.064, 0.082, 0.157);
    brow.rotation.z = sgn * -0.12;
    head.add(brow);
  }
  const mouth = box(0.062, 0.018, 0.014, matMouth);
  mouth.position.set(0, -0.062, 0.163);
  head.add(mouth);

  // 手臂（肩/肘/腕/手）
  for (const sgn of [-1, 1]) {
    const sh = joint(sgn < 0 ? 'shoulderL' : 'shoulderR', chest, sgn * 0.215, 0.2, 0);
    sh.add(sph(0.062, matCloth));
    const ua = cap(0.055, 0.16, matCloth);
    ua.position.y = -0.14;
    sh.add(ua);
    const el = joint(sgn < 0 ? 'elbowL' : 'elbowR', sh, 0, -0.28, 0);
    const fa = cap(0.048, 0.15, matSkin);
    fa.position.y = -0.13;
    el.add(fa);
    const wr = joint(sgn < 0 ? 'wristL' : 'wristR', el, 0, -0.26, 0);
    const hand = sph(0.055, matSkin);
    hand.scale.set(1, 1.15, 0.7);
    hand.position.y = -0.055;
    wr.add(hand);
  }

  // 腿（髋/膝/踝/脚）
  for (const sgn of [-1, 1]) {
    const hip = joint(sgn < 0 ? 'hipL' : 'hipR', J.pelvis, sgn * 0.105, -0.06, 0);
    const th = cap(0.077, 0.2, matCloth2);
    th.position.y = -0.19;
    hip.add(th);
    const kn = joint(sgn < 0 ? 'kneeL' : 'kneeR', hip, 0, -0.38, 0);
    const sn = cap(0.06, 0.18, matDark);
    sn.position.y = -0.17;
    kn.add(sn);
    const an = joint(sgn < 0 ? 'ankleL' : 'ankleR', kn, 0, -0.36, 0);
    const ft = box(0.095, 0.055, 0.2, matDark);
    ft.position.set(0, -0.025, 0.045);
    an.add(ft);
  }

  // 围巾（toon 材质在曲面上的明暗交界更明显）
  const scarf = new THREE.Mesh(new THREE.TorusGeometry(0.115, 0.042, 10, 22), matScarf);
  scarf.position.set(0, 1.32, 0);
  scarf.rotation.x = Math.PI / 2 - 0.12;
  root.add(scarf);

  // 脚下伪影（blob 影，不占 shadow map）
  const blob = new THREE.Mesh(new THREE.CircleGeometry(0.42, 24), new THREE.MeshBasicMaterial({ color: 0x000000, transparent: true, opacity: 0.28 }));
  blob.rotation.x = -Math.PI / 2;
  blob.position.y = 0.012;
  root.add(blob);

  let drawCalls = 0;
  let triangles = 0;

  function update(t) {
    const s = t / 1000;
    const breath = Math.sin(s * 1.6);
    // 待机呼吸：胸腔起伏 + 肩部微抬
    chest.scale.set(1 + breath * 0.022, 1 + breath * 0.038, 1 + breath * 0.028);
    chest.position.y = 0.16 + breath * 0.006;
    // 重心摇摆 + 轻微起伏
    root.rotation.y = Math.sin(s * 0.35) * 0.13;
    root.rotation.z = Math.sin(s * 0.52) * 0.018;
    root.position.y = Math.sin(s * 0.7) * 0.012;
    // 头部随动
    head.rotation.y = Math.sin(s * 0.5 + 0.8) * 0.19;
    head.rotation.x = Math.sin(s * 0.9) * 0.05;
    head.rotation.z = Math.sin(s * 0.43 + 1.7) * 0.035;
    // 视线摆动
    gaze.rotation.y = Math.sin(s * 0.72 + 1.1) * 0.13;
    gaze.rotation.x = Math.sin(s * 0.55 + 2.2) * 0.07;
    // 口型开合（伪语音包络）
    const talk = Math.max(0, Math.sin(s * 6.9) * Math.sin(s * 2.1) * 1.4);
    mouth.scale.y = 1 + talk * 5.2;
    mouth.position.y = -0.062 - talk * 0.008;
    // 手臂摆动 + 发丝/围巾物理近似
    J.shoulderL.rotation.x = Math.sin(s * 0.8) * 0.09;
    J.shoulderR.rotation.x = Math.sin(s * 0.8 + Math.PI) * 0.09;
    J.shoulderL.rotation.z = 0.16 + Math.sin(s * 0.6) * 0.035;
    J.shoulderR.rotation.z = -0.16 - Math.sin(s * 0.6 + 0.4) * 0.035;
    J.elbowL.rotation.x = -0.22 + Math.sin(s * 0.9 + 1) * 0.06;
    J.elbowR.rotation.x = -0.22 + Math.sin(s * 0.9 + 2) * 0.06;
    hair.rotation.z = Math.sin(s * 1.1) * 0.035;
    hair.rotation.x = Math.sin(s * 0.9 + 0.6) * 0.025;
    scarf.rotation.z = Math.sin(s * 1.25 + 0.3) * 0.05;
  }
  function render() {
    renderer.render(scene, camera);
    drawCalls = renderer.info.render.calls;
    triangles = renderer.info.render.triangles;
  }

  const line = makeLine('three3d', update, render);
  return {
    available: true,
    line,
    glInfo,
    extra: () => ({
      drawCalls,
      triangles,
      glRenderer: glInfo.renderer,
      glVendor: glInfo.vendor,
      glVersion: glInfo.version,
    }),
    dispose() {
      line.stop();
      renderer.dispose();
    },
  };
}

// ---------------- [2D] 2D 形象路线：程序化 Canvas2D 骨骼小人 ----------------
function build2D(canvas) {
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  canvas.width = STAGE.w * dpr;
  canvas.height = STAGE.h * dpr;
  const ctx = canvas.getContext('2d');
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

  // 骨架（模型空间 220×260，脚底 ≈ y238）：35 节关节 / 34 条骨骼 + 面部与发丝描画段
  const joints = [
    { id: 'pelvis', p: null, x: 110, y: 150 },
    { id: 'spine', p: 'pelvis', x: 0, y: -18 },
    { id: 'chest', p: 'spine', x: 0, y: -20 },
    { id: 'neck', p: 'chest', x: 0, y: -14 },
    { id: 'head', p: 'neck', x: 0, y: -14 },
    { id: 'shoulderL', p: 'chest', x: -19, y: -7 },
    { id: 'shoulderR', p: 'chest', x: 19, y: -7 },
    { id: 'elbowL', p: 'shoulderL', x: -7, y: 25 },
    { id: 'elbowR', p: 'shoulderR', x: 7, y: 25 },
    { id: 'wristL', p: 'elbowL', x: -3, y: 23 },
    { id: 'wristR', p: 'elbowR', x: 3, y: 23 },
    { id: 'fL1', p: 'wristL', x: -4, y: 10 },
    { id: 'fL2', p: 'wristL', x: -1, y: 11 },
    { id: 'fL3', p: 'wristL', x: 2, y: 10 },
    { id: 'fL4', p: 'wristL', x: 5, y: 8 },
    { id: 'fR1', p: 'wristR', x: 4, y: 10 },
    { id: 'fR2', p: 'wristR', x: 1, y: 11 },
    { id: 'fR3', p: 'wristR', x: -2, y: 10 },
    { id: 'fR4', p: 'wristR', x: -5, y: 8 },
    { id: 'hipL', p: 'pelvis', x: -10, y: 6 },
    { id: 'hipR', p: 'pelvis', x: 10, y: 6 },
    { id: 'kneeL', p: 'hipL', x: -2, y: 32 },
    { id: 'kneeR', p: 'hipR', x: 2, y: 32 },
    { id: 'ankleL', p: 'kneeL', x: -1, y: 31 },
    { id: 'ankleR', p: 'kneeR', x: 1, y: 31 },
    { id: 'toeL', p: 'ankleL', x: -9, y: 5 },
    { id: 'toeR', p: 'ankleR', x: 9, y: 5 },
  ];
  // 发丝（8 节，挂在头上）
  for (let i = 0; i < 8; i++) {
    const ang = -1.25 + (i / 7) * 2.5;
    joints.push({
      id: 'hair' + i,
      p: 'head',
      x: Math.sin(ang) * 26,
      y: -Math.cos(ang) * 20 + 4,
    });
  }
  const bones = [
    ['pelvis', 'spine', 17, '#39518a'],
    ['spine', 'chest', 19, '#4f6fb0'],
    ['chest', 'neck', 12, '#4f6fb0'],
    ['neck', 'head', 9, '#f0c8a0'],
    ['shoulderL', 'elbowL', 10, '#4f6fb0'],
    ['elbowL', 'wristL', 8, '#f0c8a0'],
    ['shoulderR', 'elbowR', 10, '#4f6fb0'],
    ['elbowR', 'wristR', 8, '#f0c8a0'],
    ['hipL', 'kneeL', 13, '#39518a'],
    ['kneeL', 'ankleL', 10, '#241d28'],
    ['hipR', 'kneeR', 13, '#39518a'],
    ['kneeR', 'ankleR', 10, '#241d28'],
    ['ankleL', 'toeL', 7, '#241d28'],
    ['ankleR', 'toeR', 7, '#241d28'],
    ['wristL', 'fL1', 3.4, '#f0c8a0'],
    ['wristL', 'fL2', 3.4, '#f0c8a0'],
    ['wristL', 'fL3', 3.4, '#f0c8a0'],
    ['wristL', 'fL4', 3.2, '#f0c8a0'],
    ['wristR', 'fR1', 3.4, '#f0c8a0'],
    ['wristR', 'fR2', 3.4, '#f0c8a0'],
    ['wristR', 'fR3', 3.4, '#f0c8a0'],
    ['wristR', 'fR4', 3.2, '#f0c8a0'],
  ];
  for (let i = 0; i < 8; i++) {
    bones.push(['head', 'hair' + i, 5.5, '#5a4632']);
  }

  // 缓存渐变（避免每帧分配）
  const glow = ctx.createRadialGradient(110, 120, 10, 110, 120, 130);
  glow.addColorStop(0, 'rgba(216,178,90,0.20)');
  glow.addColorStop(1, 'rgba(216,178,90,0)');

  const world = {};
  const angles = {};

  function solve(t) {
    const s = t / 1000;
    const breath = Math.sin(s * 1.7);
    const sway = Math.sin(s * 0.42);
    // 待机呼吸 + 重心摇摆 + 口型开合 + 视线摆动 + 眨眼
    angles.pelvis = sway * 0.055;
    angles.spine = -sway * 0.03 + breath * 0.012;
    angles.chest = breath * 0.02;
    angles.neck = Math.sin(s * 0.5 + 0.8) * 0.06;
    angles.head = Math.sin(s * 0.5 + 0.8) * 0.12 + breath * 0.01;
    angles.shoulderL = Math.sin(s * 0.8) * 0.1 + 0.1;
    angles.shoulderR = Math.sin(s * 0.8 + Math.PI) * 0.1 - 0.1;
    angles.elbowL = Math.sin(s * 0.9 + 1) * 0.12 - 0.12;
    angles.elbowR = Math.sin(s * 0.9 + 2) * 0.12 - 0.12;
    angles.wristL = Math.sin(s * 1.1) * 0.14;
    angles.wristR = Math.sin(s * 1.1 + 1.5) * 0.14;
    angles.hipL = -sway * 0.02;
    angles.hipR = -sway * 0.02;
    angles.kneeL = 0.03 + Math.sin(s * 0.8) * 0.02;
    angles.kneeR = 0.03 - Math.sin(s * 0.8) * 0.02;
    for (let i = 0; i < 8; i++) {
      angles['hair' + i] = Math.sin(s * 1.15 + i * 0.7) * 0.09;
    }
    const bob = Math.sin(s * 0.7) * 1.6 + breath * 1.1;
    for (const j of joints) {
      const a0 = angles[j.id] || 0;
      if (!j.p) {
        world[j.id] = { x: j.x + sway * 2.2, y: j.y + bob, a: a0 };
      } else {
        const par = world[j.p];
        const a = par.a + a0;
        const c = Math.cos(par.a);
        const sn = Math.sin(par.a);
        world[j.id] = {
          x: par.x + j.x * c - j.y * sn,
          y: par.y + j.x * sn + j.y * c,
          a,
        };
      }
    }
    return { s, breath, sway };
  }

  function render() {
    const t = performance.now();
    const { s, breath } = solve(t);
    const headW = world.head;

    ctx.clearRect(0, 0, STAGE.w, STAGE.h);
    ctx.fillStyle = glow;
    ctx.fillRect(0, 0, STAGE.w, STAGE.h);

    // 骨骼描画（每条两遍：暗描边 + 本体，近似卡通勾线）
    for (const [a, b, w, color] of bones) {
      const pa = world[a];
      const pb = world[b];
      ctx.lineCap = 'round';
      ctx.strokeStyle = 'rgba(20,16,26,0.55)';
      ctx.lineWidth = w + 2.6;
      ctx.beginPath();
      ctx.moveTo(pa.x, pa.y);
      ctx.lineTo(pb.x, pb.y);
      ctx.stroke();
      ctx.strokeStyle = color;
      ctx.lineWidth = w;
      ctx.beginPath();
      ctx.moveTo(pa.x, pa.y);
      ctx.lineTo(pb.x, pb.y);
      ctx.stroke();
    }
    // 关节球
    for (const j of joints) {
      const w = world[j.id];
      ctx.fillStyle = j.id.startsWith('hair') ? '#5a4632' : '#f0c8a0';
      ctx.beginPath();
      ctx.arc(w.x, w.y, j.id.startsWith('f') ? 3.4 : 5.2, 0, Math.PI * 2);
      ctx.fill();
    }

    // 头部与面部
    const hx = headW.x;
    const hy = headW.y - 12;
    const tilt = headW.a;
    ctx.save();
    ctx.translate(hx, hy);
    ctx.rotate(tilt);
    // 头发后层
    ctx.fillStyle = '#5a4632';
    for (let i = 0; i < 8; i++) {
      const w = world['hair' + i];
      ctx.strokeStyle = '#5a4632';
      ctx.lineWidth = 6.5;
      ctx.lineCap = 'round';
      ctx.beginPath();
      ctx.moveTo(0, -6);
      ctx.lineTo(w.x - hx, w.y - hy);
      ctx.stroke();
    }
    // 脸
    ctx.fillStyle = '#f0c8a0';
    ctx.strokeStyle = 'rgba(20,16,26,0.55)';
    ctx.lineWidth = 2.4;
    ctx.beginPath();
    ctx.ellipse(0, 0, 22, 24, 0, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
    // 刘海
    ctx.fillStyle = '#5a4632';
    ctx.beginPath();
    ctx.ellipse(0, -14, 22, 13, 0, Math.PI, Math.PI * 2);
    ctx.fill();
    for (let i = -2; i <= 2; i++) {
      ctx.beginPath();
      ctx.moveTo(i * 8 - 3.5, -14);
      ctx.lineTo(i * 8, -2 + Math.abs(i) * 1.5);
      ctx.lineTo(i * 8 + 3.5, -14);
      ctx.fill();
    }
    // 视线摆动（瞳孔偏移）
    const gx = Math.sin(s * 0.9) * 3.1 + Math.sin(s * 0.31) * 1.6;
    const gy = Math.sin(s * 0.7 + 1.3) * 2.1;
    // 眨眼
    const blinkPhase = s % 4.3;
    const blink = blinkPhase > 4.16 ? Math.max(0, 1 - Math.abs(blinkPhase - 4.23) * 22) : 0;
    for (const sgn of [-1, 1]) {
      const ex = sgn * 8.2;
      ctx.fillStyle = '#fbf7f2';
      ctx.beginPath();
      ctx.ellipse(ex, -2, 5.2, 5.6 * (1 - blink * 0.92), 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = '#241d28';
      ctx.beginPath();
      ctx.arc(ex + gx * 0.6, -2 + gy * 0.6, 2.7 * (1 - blink * 0.9), 0, Math.PI * 2);
      ctx.fill();
      // 眉毛
      ctx.strokeStyle = '#4a3826';
      ctx.lineWidth = 2.2;
      ctx.beginPath();
      ctx.moveTo(ex - 5, -10);
      ctx.quadraticCurveTo(ex, -12.4, ex + 5, -10.4);
      ctx.stroke();
    }
    // 腮红
    ctx.fillStyle = 'rgba(228,138,120,0.35)';
    for (const sgn of [-1, 1]) {
      ctx.beginPath();
      ctx.ellipse(sgn * 13, 5, 5.4, 3.1, 0, 0, Math.PI * 2);
      ctx.fill();
    }
    // 口型开合
    const talk = Math.max(0, Math.sin(s * 6.9) * Math.sin(s * 2.1) * 1.4);
    const mouthOpen = 1.6 + talk * 8.2 + breath * 0.5;
    ctx.fillStyle = '#a8544e';
    ctx.strokeStyle = 'rgba(20,16,26,0.5)';
    ctx.lineWidth = 1.6;
    ctx.beginPath();
    ctx.ellipse(0, 10.5, 5.4 + talk * 1.6, mouthOpen, 0, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
    ctx.restore();
  }

  return makeLine('canvas2d', null, render);
}

// ---------------- [光] 光态基线：CSS/SVG 呼吸光点（合成器驱动） ----------------
function buildLight() {
  // 呼吸动画由 CSS keyframes 驱动（见 index.html .breathe），JS 每帧仅计时采样
  return makeLine('light-css', null, null);
}

// ---------------- WebGL 探测 ----------------
function probeGL() {
  try {
    const c2 = document.createElement('canvas');
    const c1 = document.createElement('canvas');
    const gl2 = c2.getContext('webgl2');
    const gl = gl2 || c1.getContext('webgl');
    if (!gl) {
      PROBE.meta.gl = { available: false };
      return;
    }
    const dbg = gl.getExtension('WEBGL_debug_renderer_info');
    PROBE.meta.gl = {
      available: true,
      webgl2: !!gl2,
      renderer: dbg ? gl.getParameter(dbg.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER),
      vendor: dbg ? gl.getParameter(dbg.UNMASKED_VENDOR_WEBGL) : gl.getParameter(gl.VENDOR),
    };
    const lose = gl.getExtension('WEBGL_lose_context');
    if (lose) lose.loseContext();
    if (gl2 && gl !== gl2) {
      const lose2 = gl2.getExtension('WEBGL_lose_context');
      if (lose2) lose2.loseContext();
    }
  } catch (e) {
    PROBE.meta.gl = { available: false, error: String(e).slice(0, 160) };
  }
}

// ---------------- 步骤：单线稳态 FPS + JS 堆 ----------------
async function measureLine(key, label, entry, line, extraFn) {
  if (!line) {
    PROBE.lines[key] = { id: key, label, available: false, reason: entry && entry.reason || 'not-built' };
    log(`[跳过] ${label}：不可用（${PROBE.lines[key].reason}）`);
    return;
  }
  setPhase(`${label}：预热 3s`);
  gcNow();
  line.rec.end();
  line.start();
  await sleep(3000);

  setPhase(`${label}：稳态采样 ${FPS_SEC}s`);
  const heapSamples = [];
  const t0 = performance.now();
  line.rec.begin();
  while (performance.now() - t0 < FPS_SEC * 1000) {
    await sleep(1000);
    const h = heapMB();
    if (h !== null) heapSamples.push(r2(h));
    const el = (performance.now() - t0) / 1000;
    setPhase(`${label}：稳态采样 ${FPS_SEC}s（已测 ${Math.round(el)}s / ${FPS_SEC}s，瞬时帧计数 ${line.frames}）`);
  }
  const stats = line.rec.stats();
  line.rec.end();
  line.stop();

  gcNow();
  await sleep(250);
  const heapEnd = heapMB();
  const totalMB = await totalMemoryMB();

  PROBE.lines[key] = {
    id: key,
    label,
    available: true,
    fps: {
      fpsMean: stats.fpsMean,
      fps1Low: stats.fps1Low,
      fpsMin: stats.fpsMin,
      frames: stats.frames,
      elapsedSec: stats.elapsedSec,
    },
    frameJsMs: { mean: stats.jsMsMean, p95: stats.jsMsP95 },
    heap: {
      startMB: heapSamples.length ? heapSamples[0] : null,
      steadyMedianMB: heapSamples.length ? r2(median(heapSamples)) : null,
      maxMB: heapSamples.length ? r2(Math.max(...heapSamples)) : null,
      endMB: heapEnd !== null ? r2(heapEnd) : null,
      samplesMB: heapSamples,
    },
    totalMemoryMB: totalMB,
    extra: extraFn ? extraFn() : null,
  };
  log(
    `[完成] ${label}：FPS 均值 ${stats.fpsMean}、1% low ${stats.fps1Low}；` +
      `JS 堆稳态 ${PROBE.lines[key].heap.steadyMedianMB}MB；每帧 JS ${stats.jsMsMean}ms`
  );
}

// ---------------- 步骤：hidden 纪律（帧计数证明） ----------------
async function hiddenTest(key, label, line) {
  setPhase(`${label}：hidden 纪律验证`);
  line.rec.end();
  line.start();
  await sleep(600);
  const fBase = line.frames;
  const tBase = globalTicks;
  await sleep(2000);
  const framesVisiblePerSec = (line.frames - fBase) / 2;

  // 注入 document.hidden = true
  const g0 = globalTicks;
  const f0 = line.frames;
  const c0 = line.callbacks;
  forceHidden(true);
  await sleep(HIDDEN_SEC * 1000);
  const rafTicksDuringHidden = globalTicks - g0;
  const framesDuringHidden = line.frames - f0;
  const loopCallbacksDuringHidden = line.callbacks - c0;
  forceHidden(false);

  const f1 = line.frames;
  await sleep(2000);
  const framesAfterResume = line.frames - f1;

  const out = {
    label,
    hiddenMethod: 'document.hidden/visibilityState 强制注入 + visibilitychange',
    framesVisiblePerSec: r2(framesVisiblePerSec),
    rafTicksDuringHidden,
    framesDuringHidden,
    loopCallbacksDuringHidden,
    hiddenSkipsBelt: line.hiddenSkips,
    framesAfterResume,
    pass: framesDuringHidden === 0 && rafTicksDuringHidden > 0 && framesAfterResume > 20,
  };
  PROBE.disciplines.hiddenStop[key] = out;
  log(
    `[hidden] ${label}：隐藏期渲染帧 ${framesDuringHidden}（应为 0）、` +
      `同期 rAF 心跳 ${rafTicksDuringHidden}（应 >0）、恢复后 ${framesAfterResume} 帧 → ${out.pass ? '通过' : '不通过'}`
  );
  if (!line.running) line.start(); // 保持线可用（后续浸泡要用）
  line.stop();
  void tBase;
}

// ---------------- 步骤：WebGL 禁用降级（页内 stub 注入） ----------------
async function webglStubTest() {
  setPhase('WebGL 禁用降级（页内 stub：getContext("webgl*") → null）');
  const out = {
    method: 'HTMLCanvasElement.getContext 对 webgl/webgl2 返回 null 后重建 3D 线',
    three3dReportsUnavailable: null,
    twoDStillRuns: null,
    lightStillRuns: null,
    hostFallback: null,
    pass: null,
    error: null,
  };
  const orig = HTMLCanvasElement.prototype.getContext;
  HTMLCanvasElement.prototype.getContext = function (type, ...rest) {
    if (typeof type === 'string' && /webgl/i.test(type)) return null;
    return orig.call(this, type, ...rest);
  };
  try {
    const probe = await build3D(document.createElement('canvas'));
    out.three3dReportsUnavailable = probe.available === false;
    out.reason3d = probe.reason || 'available?!';
    if (probe.dispose) {
      try {
        probe.dispose();
      } catch (e) {
        /* 释放失败不影响判定 */
      }
    }
  } catch (e) {
    out.error = String(e).slice(0, 200);
    out.three3dReportsUnavailable = false;
  } finally {
    HTMLCanvasElement.prototype.getContext = orig;
  }
  // 宿主回落语义：3D 不可用时回落 2D 形象路线
  out.hostFallback = out.three3dReportsUnavailable ? 'canvas2d' : 'none';
  // 2D / 光态照常
  try {
    const l2 = build2D(document.createElement('canvas'));
    l2.start();
    await sleep(1500);
    out.twoDStillRuns = l2.frames > 30;
    l2.stop();
  } catch (e) {
    out.twoDStillRuns = false;
    out.error2d = String(e).slice(0, 200);
  }
  try {
    const ll = buildLight();
    ll.start();
    await sleep(1500);
    out.lightStillRuns = ll.frames > 30;
    ll.stop();
  } catch (e) {
    out.lightStillRuns = false;
    out.errorLight = String(e).slice(0, 200);
  }
  out.pass = !!(out.three3dReportsUnavailable && out.twoDStillRuns && out.lightStillRuns);
  PROBE.disciplines.webglFallback.inPageStub = out;
  log(
    `[WebGL 降级] 3D 线报不可用=${out.three3dReportsUnavailable}（${out.reason3d}）；` +
      `2D 照常=${out.twoDStillRuns}；光态照常=${out.lightStillRuns} → ${out.pass ? '通过' : '不通过'}`
  );
  return out;
}

// ---------------- 步骤：5 分钟浸泡（三线并发，最坏情况口径） ----------------
async function soakRun(lines) {
  setPhase(`浸泡 ${SOAK_SEC}s（三线并发，最坏情况口径）`);
  gcNow();
  await sleep(400);
  const heapStart = heapMB();
  const totalStart = await totalMemoryMB();
  const samples = [];
  const t0 = performance.now();
  for (const l of lines) l.start();
  while (performance.now() - t0 < SOAK_SEC * 1000) {
    await sleep(5000);
    const el = (performance.now() - t0) / 1000;
    const h = heapMB();
    samples.push({ tSec: Math.round(el), heapMB: h !== null ? r2(h) : null });
    setPhase(`浸泡 ${SOAK_SEC}s（${Math.round(el)}s / ${SOAK_SEC}s，JS 堆 ${h !== null ? r2(h) + 'MB' : 'n/a'}）`);
    if (Math.round(el) % 30 === 0) {
      const tm = await totalMemoryMB();
      if (tm !== null) samples[samples.length - 1].totalMemoryMB = tm;
    }
  }
  for (const l of lines) l.stop();
  gcNow();
  await sleep(600);
  const heapEnd = heapMB();
  const totalEnd = await totalMemoryMB();
  const first = samples.slice(0, 3).map((s) => s.heapMB).filter((x) => x !== null);
  const last = samples.slice(-3).map((s) => s.heapMB).filter((x) => x !== null);
  const driftMB = first.length && last.length ? r2(mean(last) - mean(first)) : null;
  const minutes = SOAK_SEC / 60;
  PROBE.soak = {
    durationSec: SOAK_SEC,
    loadsRunning: lines.map((l) => l.id),
    heapStartMB: heapStart !== null ? r2(heapStart) : null,
    heapEndMB: heapEnd !== null ? r2(heapEnd) : null,
    driftMB,
    slopeMBPerMin: driftMB !== null ? r2(driftMB / minutes) : null,
    totalMemoryStartMB: totalStart,
    totalMemoryEndMB: totalEnd,
    samples,
  };
  log(`[浸泡] 前后 JS 堆 ${PROBE.soak.heapStartMB}MB → ${PROBE.soak.heapEndMB}MB，漂移 ${driftMB}MB`);
}

// ---------------- 结果对象 + 面板 ----------------
const PROBE = {
  done: false,
  meta: {
    startedAt: new Date().toISOString(),
    finishedAt: null,
    mode: MODE,
    userAgent: navigator.userAgent,
    viewport: { w: innerWidth, h: innerHeight, dpr: devicePixelRatio },
    stageSize: STAGE,
    crossOriginIsolated: window.crossOriginIsolated === true,
    three: { installed: false, version: null },
    gl: { available: null },
    visibilityAtStart: document.visibilityState,
    hiddenAtStart: document.hidden,
    exposureGc: typeof window.gc === 'function',
    timing: { fpsSec: FPS_SEC, soakSec: SOAK_SEC, hiddenSec: HIDDEN_SEC },
  },
  lines: { three3d: null, canvas2d: null, lightCss: null },
  disciplines: { hiddenStop: {}, webglFallback: {} },
  soak: null,
  timeline: [],
};
window.__AVATAR_PERF_PROBE__ = PROBE;

function log(msg) {
  const t = `${new Date().toISOString().slice(11, 19)} ${msg}`;
  PROBE.timeline.push(t);
  const el = $('timeline');
  if (el) {
    el.textContent = PROBE.timeline.slice(-200).join('\n');
    el.scrollTop = el.scrollHeight;
  }
  console.log('[avatar-perf-probe]', msg);
}
function setPhase(text) {
  PROBE.meta.phase = text;
  const el = $('phase-text');
  if (el) el.textContent = text;
}
function badge(cls, text) {
  return `<span class="${cls}">${text}</span>`;
}
function renderPanel() {
  const L = PROBE.lines;
  const H = PROBE.disciplines.hiddenStop;
  const rows = [
    ['3D 卡通渲染路线', L.three3d, H.three3d],
    ['2D 形象路线', L.canvas2d, H.canvas2d],
    ['光态基线', L.lightCss, H.lightCss],
  ];
  let html = `<table><tr><th>负载线</th><th>FPS 均值</th><th>1% low</th><th>每帧 JS</th>
    <th>JS 堆稳态</th><th>浸泡漂移</th><th>hidden 停帧</th></tr>`;
  for (const [label, l, h] of rows) {
    if (!l || !l.available) {
      html += `<tr><td>${label}</td><td colspan="6">${badge('bad', '不可用')} ${l ? l.reason : ''}</td></tr>`;
      continue;
    }
    const drift = PROBE.soak ? PROBE.soak.driftMB : null;
    const cell = (v, suffix = '') => (v === undefined || v === null ? '—' : v + suffix);
    html += `<tr>
      <td>${label}</td>
      <td class="num">${cell(l.fps && l.fps.fpsMean)}</td>
      <td class="num">${cell(l.fps && l.fps.fps1Low)}</td>
      <td class="num">${cell(l.frameJsMs && l.frameJsMs.mean, 'ms')}</td>
      <td class="num">${cell(l.heap && l.heap.steadyMedianMB, 'MB')}</td>
      <td class="num">${drift !== null ? drift + 'MB（三线合计）' : 'n/a'}</td>
      <td>${h ? (h.pass ? badge('ok', 'PASS') : badge('bad', 'FAIL')) : 'n/a'}</td>
    </tr>`;
  }
  html += '</table>';
  const fb = PROBE.disciplines.webglFallback.inPageStub;
  html += `<div class="note">
    纪律一 document.hidden 停渲染：${
      Object.values(H).length && Object.values(H).every((x) => x.pass) ? badge('ok', '三线全部通过') : badge('bad', '存在不通过')
    }（隐藏期渲染帧=0，同期全局 rAF 心跳&gt;0，恢复后继续出帧）<br/>
    纪律二 WebGL 不可用降级：${
      fb ? (fb.pass ? badge('ok', '通过') : badge('bad', '不通过')) : badge('warn', '未测')
    }（3D 线报不可用由宿主回落 ${fb ? fb.hostFallback : '—'}，2D/光态照常）<br/>
    浸泡 ${PROBE.soak ? PROBE.soak.durationSec + 's' : '—'}（三线并发）JS 堆漂移：${
      PROBE.soak ? PROBE.soak.driftMB + 'MB（' + PROBE.soak.heapStartMB + ' → ' + PROBE.soak.heapEndMB + 'MB）' : '—'
    }<br/>
    WebGL 实体：${
      PROBE.meta.gl && PROBE.meta.gl.available
        ? `可用（${PROBE.meta.gl.renderer || 'renderer 未知'}）`
        : '不可用'
    }；three 运行库：${PROBE.meta.three.installed ? '已装载 r' + PROBE.meta.three.version : '未装载'}
  </div>`;
  $('summary').innerHTML = html;
}

// ---------------- 主流程 ----------------
async function runFull() {
  log(`启动：模式=${MODE}，视口 ${innerWidth}×${innerHeight}，目标舞台 ${STAGE.w}×${STAGE.h}`);
  if (document.hidden) {
    log('警告：页面启动即 document.hidden=true，FPS 口径可能失真（结果中 visibilityAtStart 留档）');
  }
  // 1) three 运行库
  try {
    const mod = await import('three');
    PROBE.meta.three.installed = true;
    PROBE.meta.three.version = mod.REVISION;
    log(`three 运行库已装载（r${mod.REVISION}）`);
  } catch (e) {
    PROBE.meta.three.loadError = String(e).slice(0, 200);
    log('three 运行库未装载：' + PROBE.meta.three.loadError + ' → 3D 线将报不可用（如实记录，不谎报）');
  }
  probeGL();
  log(`WebGL 探测：${PROBE.meta.gl.available ? '可用（' + (PROBE.meta.gl.renderer || '') + '）' : '不可用'}`);

  // 2) 构建三线
  let b3 = { available: false, reason: 'three-not-loaded' };
  if (PROBE.meta.three.installed) {
    setPhase('构建 3D 卡通渲染负载（程序化人形）');
    b3 = await build3D($('stage-3d'));
  }
  const line3d = b3.available ? b3.line : null;
  const line2d = build2D($('stage-2d'));
  const lineLight = buildLight();

  // 3) 逐线稳态 + hidden 纪律
  await measureLine('three3d', '3D 卡通渲染路线', b3, line3d, b3.available ? b3.extra : null);
  await measureLine('canvas2d', '2D 形象路线', null, line2d, null);
  await measureLine('lightCss', '光态基线', null, lineLight, null);
  if (line3d) await hiddenTest('three3d', '3D 卡通渲染路线', line3d);
  await hiddenTest('canvas2d', '2D 形象路线', line2d);
  await hiddenTest('lightCss', '光态基线', lineLight);

  // 4) WebGL 禁用降级（页内 stub）
  await webglStubTest();

  // 5) 5 分钟浸泡（三线并发 = 最坏情况）
  const soakLines = [line3d, line2d, lineLight].filter(Boolean);
  await soakRun(soakLines);

  // 6) 收尾
  PROBE.meta.finishedAt = new Date().toISOString();
  PROBE.done = true;
  setPhase('全部测项完成');
  renderPanel();
  log('完成：window.__AVATAR_PERF_PROBE__ 已就绪');
  postResult('/api/result');
}

async function runDegrade() {
  // 真实 WebGL 禁用环境（--disable-3d-apis）下的降级验证快跑
  log(`降级验证模式启动（mode=degrade），document.hidden=${document.hidden}`);
  probeGL();
  try {
    const mod = await import('three');
    PROBE.meta.three.installed = true;
    PROBE.meta.three.version = mod.REVISION;
  } catch (e) {
    PROBE.meta.three.loadError = String(e).slice(0, 200);
  }
  const b3 = PROBE.meta.three.installed
    ? await build3D($('stage-3d'))
    : { available: false, reason: 'three-not-loaded' };
  const line2d = build2D($('stage-2d'));
  const lineLight = buildLight();
  PROBE.lines.three3d = b3.available
    ? { id: 'three3d', label: '3D 卡通渲染路线', available: true, note: 'degrade 模式下仍可用？不应发生' }
    : { id: 'three3d', label: '3D 卡通渲染路线', available: false, reason: b3.reason };
  PROBE.disciplines.webglFallback.realRun = {
    env: '浏览器以 --disable-3d-apis 启动（真实 WebGL 禁用）',
    webglProbeAvailable: PROBE.meta.gl.available === true,
    three3dReportsUnavailable: b3.available === false,
    reason3d: b3.reason || null,
  };
  // 2D / 光态照常跑 5s 并计帧
  for (const [key, label, line] of [
    ['canvas2d', '2D 形象路线', line2d],
    ['lightCss', '光态基线', lineLight],
  ]) {
    line.start();
    await sleep(5000);
    line.stop();
    PROBE.lines[key] = {
      id: key,
      label,
      available: true,
      fps: { framesIn5s: line.frames },
      note: 'degrade 模式仅验"照常出帧"，不测稳态',
    };
    log(`[降级验证] ${label}：5s 内出帧 ${line.frames} → ${line.frames > 100 ? '照常' : '异常'}`);
  }
  await hiddenTest('canvas2d', '2D 形象路线', line2d);
  PROBE.disciplines.webglFallback.realRun.pass = !!(
    b3.available === false &&
    (PROBE.lines.canvas2d.fps.framesIn5s > 100) &&
    (PROBE.lines.lightCss.fps.framesIn5s > 100)
  );
  log(
    `[降级验证] 真实 WebGL 禁用：3D 报不可用=${b3.available === false}（${b3.reason}），` +
      `2D/光态照常=${PROBE.disciplines.webglFallback.realRun.pass} → ${
        PROBE.disciplines.webglFallback.realRun.pass ? '通过' : '不通过'
      }`
  );
  PROBE.meta.finishedAt = new Date().toISOString();
  PROBE.done = true;
  setPhase('降级验证完成');
  renderPanel();
  postResult('/api/result-degrade');
}

async function postResult(path) {
  try {
    await fetch(path, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(PROBE),
    });
  } catch (e) {
    log('结果回传失败（手动打开页面时可忽略，直接读 window.__AVATAR_PERF_PROBE__）：' + String(e).slice(0, 120));
  }
}

$('mode-badge').textContent = MODE;
if (MODE === 'degrade') {
  runDegrade().catch((e) => {
    PROBE.fatal = String(e);
    PROBE.done = true;
    setPhase('降级验证异常：' + String(e).slice(0, 200));
    postResult('/api/result-degrade');
  });
} else {
  runFull().catch((e) => {
    PROBE.fatal = String(e && e.stack ? e.stack : e);
    PROBE.done = true;
    setPhase('异常终止：' + String(e).slice(0, 200));
    renderPanel();
    postResult('/api/result');
    console.error(e);
  });
}
