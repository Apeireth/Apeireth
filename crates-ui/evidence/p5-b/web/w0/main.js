// W0 第一屏示例：挂载引擎示例屏 + 驱动示例级 tick。
import init, * as p5b from '/engine/pkg/p5b_harness.js';

await init();

// ⭐ **A/B 开关**：`?defer=1` 走「延后一帧窗口化」，默认**立即**窗口化。
//    目的是让 `drive_w0.py` 能**同机同源**对比两条路径 ⇒ ⇒ 数字可比。
//
// ⚠️ 默认是**立即**（E-162 实测，三次复现）：
//    立即 = t_dom ≈ 18~20 ms；延后一帧 = t_dom ≈ 41~42 ms
//    ⇒ ⇒ ⇒ 两条路径的窗口**都正确**（7 行）⇒ ⇒ ⇒ 延后**没有任何收益**。
const DEFER = new URLSearchParams(location.search).has("defer");

// ⭐ 时间戳：`performance.now()`（毫秒，浮点）。
//   - `t_dom`    ：列表行**写进 DOM** 的时刻；
//   - `t_painted`：浏览器**画完**那一帧的时刻（再排一个 rAF ⇒ ⇒ 在 paint 之后触发）。
window.__W0_TIMES__ = { defer: DEFER, t_dom: null, t_painted: null };

// ⭐ 捕获**未捕获异常**（wasm trap / JS 异常）⇒ ⇒ 出问题时能看见原因，
//   而不是只看到「时间戳没设上」。
window.__W0_ERRORS__ = [];
window.addEventListener("error", (event) => {
  window.__W0_ERRORS__.push(String(event.message || event.error || "unknown"));
});
window.addEventListener("unhandledrejection", (event) => {
  window.__W0_ERRORS__.push("unhandledrejection: " + String(event.reason));
});

function markPainted() {
  requestAnimationFrame(() => {
    window.__W0_TIMES__.t_painted = performance.now();
    window.__W0_DONE__ = true;
  });
}

// ⭐⭐ **污染开关**（用于验证 E-160 的测量环境问题）：`?pollute=N`
//    ⇒ ⇒ 在窗口化**之前**先塞 N 个 DOM 节点（模拟「页面上已有大量节点」）。
//    ⇒ ⇒ 用来区分「首次 layout 成本**来自我们的列表**」与
//    ⇒ ⇒ 「首次 layout 成本**来自页面上已有的其它节点**」。
const POLLUTE = Number(new URLSearchParams(location.search).get("pollute") || 0);
if (POLLUTE > 0) {
  const box = document.createElement("div");
  box.id = "pollute-box";
  const frag = document.createDocumentFragment();
  for (let i = 0; i < POLLUTE; i++) {
    const node = document.createElement("div");
    node.className = "pollute-row";
    node.textContent = `填充 ${i}`;
    frag.appendChild(node);
  }
  box.appendChild(frag);
  document.body.appendChild(box);
}

// ① 首屏挂载（**只挂容器**：虚拟列表延后到 paint 之后才窗口化）。
// ⚠️⚠️ **只调一次**！此前诊断代码把它调了**两次**（try 块 + 赋值）
//   ⇒ ⇒ 挂载了**两个** `#w0-root` ⇒ ⇒ 选择器 `#w0-root #w0-list > *`
//   ⇒ ⇒ 把**两个**列表的子节点**加在一起** ⇒ ⇒ 读出「16 个」（假象）。
//   ⇒ ⇒ ⇒ 教训（E-163）：**每次「异常」都要先怀疑自己的测量**。
try {
  window.__W0_FIRST_SCREEN__ = p5b.w0_first_screen();
} catch (error) {
  // ⭐ 抓起来而不是让它**中断整个脚本**（否则后面什么都测不到）。
  window.__W0_ERRORS__.push("first_screen: " + String(error));
}

if (DEFER) {
  // ⭐ 延后窗口化（E-162）：**只等一帧**。
  //
  // ## 为什么必须延后（不是「优化」而是「正确性」）
  //
  // 同步窗口化时容器**还没被浏览器布局** ⇒ ⇒ `client_height = 0`
  // ⇒ ⇒ 窗口算出来**只有 1 行**（实测 `rows = 1`）⇒ ⇒ 列表**几乎没虚拟化**
  // ⇒ ⇒ ⇒ **这是功能错误，不是性能问题**。
  //
  // ## 为什么只等一帧
  //
  // 浏览器在 rAF 回调**之前**已经做过布局 ⇒ ⇒ 此刻读度量会**命中缓存**
  // ⇒ ⇒ 拿到的 `client_height` 是**真实值**（窗口算得对）⇒ ⇒ 而且**不触发额外回流**。
  //
  // ⇒ ⇒ 等两帧是多余的 ⇒ ⇒ 多等一帧 ≈ 多等 16 ms。
  requestAnimationFrame(() => {
    window.__W0_STEP__ = "rAF1";
    p5b.w0_windowize();
    window.__W0_STEP__ = "windowized";
    window.__W0_TIMES__.t_dom = performance.now();
    markPainted();
  });
} else {
  // ⭐ 默认路径：**立即**窗口化。
  // ⚠️ 首屏的窗口化**已经在 Rust 侧**做完（`w0_first_screen` 内部，E-162 实测最优）
  // ⇒ ⇒ 这里只取时间戳。
  window.__W0_STEP__ = "already-windowized";
  window.__W0_TIMES__.t_dom = performance.now();
  markPainted();
}

// 示例级调度器（生产请换成正式调度器）。
// ⚠️ `?notick=1` ⇒ 不启动 tick（**诊断用**：用来判别「DOM 翻倍是首屏造成的还是 tick 造成的」）。
if (!new URLSearchParams(location.search).has("notick")) {
  setInterval(() => {
    p5b.w0_tick();
  }, 50);
}

window.__W0__ = { ok: true };
