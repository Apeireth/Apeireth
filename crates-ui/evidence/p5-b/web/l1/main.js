// L1 能力浏览器探针：先初始化 wasm，再跑 DOM 侧探针与「纹理上传」端到端探针。
import init, * as p5b from '/engine/pkg/p5b_harness.js';

const results = {};
try {
  await init();
  results.dom = p5b.l1_probe();
} catch (error) {
  results.dom = { ok: false, failures: [`DOM 探针抛错：${String(error)}`] };
}
try {
  // 传一个真实 canvas（引擎需要它建 surface）；宽高非 2 幂、且后续上传 3×2 图
  const canvas = document.createElement('canvas');
  canvas.width = 64;
  canvas.height = 64;
  canvas.id = 'l1-upload-canvas';
  canvas.style.display = 'none';
  document.body.appendChild(canvas);
  results.upload = await p5b.l1_upload_probe(canvas, 'webgl');
} catch (error) {
  results.upload = { ok: false, failures: [`上传探针抛错：${String(error)}`] };
}
window.__L1__ = results;
window.__L1_DONE__ = true;
