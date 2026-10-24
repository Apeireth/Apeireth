/**
 * P5-b 引擎页 —— Rust 引擎（wasm）用**生产侧实测抓到的 uniform 取值**画两帧。
 *
 * - `window.__P5B_UNIFORMS__` 由 CDP 在页面脚本之前注入（生产侧抓到的原样数值）；
 * - 两个后端各画一块画布（一个 canvas 只能绑一种 context 类型 ⇒ 必须两个 canvas）；
 * - 画布尺寸 = 960×600 设备像素（与参考页完全一致 ⇒ uRes / viewport 一致）。
 */

const reports = {};
window.__P5B_REPORTS__ = reports;

(async () => {
  try {
    const wasm = await import(/* @vite-ignore */ '/engine/pkg/p5b_harness.js');
    await wasm.default();
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
