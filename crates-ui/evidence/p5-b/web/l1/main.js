// L1 能力浏览器探针：先初始化 wasm（`--target web` 胶水必须），再调用探针导出。
import init, * as p5b from '/engine/pkg/p5b_harness.js';
try {
  await init();
  window.__L1__ = p5b.l1_probe();
} catch (error) {
  window.__L1__ = { ok: false, failures: [`探针抛错：${String(error)}`] };
}
window.__L1_DONE__ = true;
