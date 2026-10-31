// W0 第一屏示例：挂载引擎示例屏 + 驱动示例级 tick。
import init, * as p5b from '/engine/pkg/p5b_harness.js';

await init();
p5b.w0_first_screen();

// 示例级调度器（生产请换成正式调度器）。
setInterval(() => {
  p5b.w0_tick();
}, 50);

window.__W0__ = { ok: true };
window.__W0_DONE__ = true;
