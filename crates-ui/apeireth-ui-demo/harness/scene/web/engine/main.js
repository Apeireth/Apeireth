/**
 * 引擎页 —— **场景层语义 crate**（wasm）算出的 16 个取值喂给引擎画布。
 *
 * - 用例参数走 URL 查询串（与参考页**同一组**参数）；
 * - 动画路径的帧时长序列 `window.__SCENE_DTS__` 由 CDP 在页面脚本之前注入
 *   （生产侧实测抓到的 rAF 时间戳序列）⇒ 两侧的状态推进逐步一致；
 * - 引擎页回报「算出来的取值」（f64 口径 + f32 口径）供驱动脚本逐值比较。
 */

const reports = {};
window.__SCENE_REPORTS__ = reports;

const params = new URLSearchParams(location.search);
const number = (key, fallback) => {
  const raw = params.get(key);
  if (raw === null || raw === '') return fallback;
  const value = Number(raw);
  return Number.isFinite(value) ? value : fallback;
};
const optionalNumber = (key) => {
  const raw = params.get(key);
  if (raw === null || raw === '') return undefined;
  const value = Number(raw);
  return Number.isFinite(value) ? value : undefined;
};

const caseSpec = {
  mode: params.get('mode') ?? 'quiet',
  hour: optionalNumber('hour'),
  camera: number('camera', 0),
  path: params.get('path') === 'animated' ? 'animated' : 'static',
  frozenTime: optionalNumber('t0'),
  padP: number('padP', 0),
  padA: number('padA', 0),
  padD: number('padD', 0),
  intensity: optionalNumber('intensity'),
};

(async () => {
  try {
    const wasm = await import(/* @vite-ignore */ '/engine/pkg/scene_harness.js');
    await wasm.default();
    const dts = window.__SCENE_DTS__ ?? [];
    const canvas = document.getElementById('webgl');
    const rect = canvas.getBoundingClientRect();
    for (const backend of ['webgl', 'webgpu']) {
      const target = document.getElementById(backend);
      try {
        const raw = await wasm.run_case(
          target,
          backend,
          caseSpec.mode,
          caseSpec.hour,
          caseSpec.camera,
          caseSpec.path,
          caseSpec.frozenTime,
          rect.width,
          rect.height,
          window.devicePixelRatio,
          dts,
          caseSpec.padP,
          caseSpec.padA,
          caseSpec.padD,
          caseSpec.intensity,
          3,
        );
        try {
          reports[backend] = JSON.parse(raw);
        } catch (e) {
          reports[backend] = { error: String(e), raw: raw };
        }
        console.log('[scene] ' + backend + ' | ' + raw);
      } catch (e) {
        reports[backend] = { error: String((e && e.message) ? e.message : e) };
        console.log('[scene] ' + backend + ' | ERROR ' + reports[backend].error);
      }
    }
  } catch (e) {
    reports.error = String((e && e.message) ? e.message : e);
    console.log('[scene] harness load FAILED: ' + reports.error);
  }
  window.__SCENE_DONE__ = true;
})();
