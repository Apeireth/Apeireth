/**
 * P5-b 引擎页 —— Rust 引擎（wasm）用**生产侧实测抓到的 uniform 取值**画两块画布。
 *
 * - `window.__P5B_UNIFORMS__` 由 CDP 在页面脚本之前注入（生产侧抓到的原样数值）；
 * - 两个后端各画一块画布（一个 canvas 只能绑一种 context 类型 ⇒ 必须两个 canvas）；
 * - 画布尺寸 = 960×600 设备像素（与参考页完全一致 ⇒ uRes / viewport 一致）。
 *
 * ## ⭐ 为什么**持续重绘**（而不是画 3 帧就停）
 *
 * ### 现象（2026-10-25 CI 实测，run 40155703764 / 40170312024）
 *
 * runner 上引擎侧**画完 3 帧**（控制台逐条可见：pipeline created / frame 0..2 drawn /
 * 适配器 = SwiftShader）⇒ **引擎没画错**；但截图拿到的是**全白画布**
 * ⇒ 逐像素差 **575,629 / 576,000 = 99.94%** ⇒ **被误判为「引擎画错」**。
 *
 * ### 根因
 *
 * 浏览器的 WebGL **drawing buffer 默认不保留**（`preserveDrawingBuffer: false`）：
 * **每次合成之后** buffer 被清空 ⇒ 下一次截图触发的合成拿到的就是**空**。
 * ⇒ 「画完 3 帧就停」⇒ 停后浏览器发生一次合成 ⇒ buffer 被清 ⇒ 之后截图必白。
 * ⇒ **生产侧为什么没事**：它**每帧都在画**（动画循环）⇒ 每次合成时 buffer 里都有内容
 * ⇒ ⇒ **引擎侧也必须持续画** ⇒ 两侧口径才一致。
 *
 * ### 为什么「画面恒定」
 *
 * `uTime` 是**注入的固定值**（1.5）⇒ 持续重绘**画的是同一帧** ⇒
 * ⇒ 画面**逐像素恒定** ⇒ ⇒ 截图拿到什么，与「画了几帧」无关 ⇒ **对时序不敏感**。
 * ⇒ 驱动不需要知道引擎画了多少帧。
 *
 * ### 停止
 *
 * 驱动在取完证据后**关闭页面** ⇒ 循环随之停止（没有额外的停止协议）。
 */

const reports = {};
window.__P5B_REPORTS__ = reports;

//: 每个后端的重绘句柄（`stop` 时用来取消 rAF）。
const redrawHandles = {};

/** ⭐ 持续重绘：每帧用**同一批** uniforms 重画一次（画面恒定）。
 *
 * ⚠️ **由驱动按需开关**（`window.__P5B_REDRAW__.start/stop`）⇒ 理由：
 * 两个后端**各自**占一个 context ⇒ 同时持续重绘 ⇒ **两个 context 持续竞争**
 * ⇒ runner 上资源紧 ⇒ 可能反而更糟。
 * ⇒ ⇒ 只在**要截图的那个后端**上开重绘 ⇒ 截完立即停 ⇒ 无竞争。
 */
function startRedrawLoop(wasm, backend, uniforms) {
  if (redrawHandles[backend] !== undefined) return;
  const canvas = document.getElementById(backend);
  const tick = () => {
    // ⚠️ 用 `redraw`（**复用**引擎与管线）⇒ 不是 `run`（那会**重建** GPU 资源
    // ⇒ 实测 `Page.captureScreenshot` **超时**）⇒ ⇒ 每帧只是「再画一帧」。
    // 失败不致命：首帧的失败已经记进 reports ⇒ 循环里只静默重试。
    wasm.redraw(backend, 1).catch(() => {});
    redrawHandles[backend] = requestAnimationFrame(tick);
  };
  redrawHandles[backend] = requestAnimationFrame(tick);
}

function stopRedrawLoop(backend) {
  const handle = redrawHandles[backend];
  if (handle === undefined) return;
  cancelAnimationFrame(handle);
  delete redrawHandles[backend];
}

// 暴露给驱动（截图前 start、截完 stop）。
window.__P5B_REDRAW__ = {
  start: (backend) => {
    const wasm = window.__P5B_WASM__;
    const uniforms = window.__P5B_UNIFORMS__;
    if (wasm && uniforms) startRedrawLoop(wasm, backend, uniforms);
  },
  stop: (backend) => stopRedrawLoop(backend),
};

(async () => {
  try {
    const wasm = await import(/* @vite-ignore */ '/engine/pkg/p5b_harness.js');
    await wasm.default();
    window.__P5B_WASM__ = wasm;
    const uniforms = window.__P5B_UNIFORMS__;
    if (!uniforms) {
      reports.error = 'uniforms were not injected';
      window.__P5B_DONE__ = true;
      return;
    }
    for (const backend of ['webgl', 'webgpu']) {
      const canvas = document.getElementById(backend);
      try {
        const raw = await wasm.run(canvas, backend, 3, uniforms);
        try {
          reports[backend] = JSON.parse(raw);
        } catch (e) {
          reports[backend] = { error: String(e), raw: raw };
        }
        console.log('[p5b] ' + backend + ' | ' + raw);
        // ⚠️ **不**在这里自动开重绘 ⇒ 由驱动在**截图前**开、**截完**停
        // ⇒ ⇒ 截图对时序不敏感，且**两个 context 不会同时持续渲染**。
        // ⇒ 驱动侧的开关见 `window.__P5B_REDRAW__`。
      } catch (e) {
        reports[backend] = { error: String((e && e.message) ? e.message : e) };
        console.log('[p5b] ' + backend + ' | ERROR ' + reports[backend].error);
      }
    }
  } catch (e) {
    reports.error = String((e && e.message) ? e.message : e);
    console.log('[p5b] harness load FAILED: ' + reports.error);
  }
  window.__P5B_DONE__ = true;
})();
