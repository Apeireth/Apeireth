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

// ⓪ ⭐⭐⭐ **E-165 验证开关**：`?blank=1` ⇒ ⇒ 树里的列表**故意留空**
//   ⇒ ⇒ 模拟「调用方完全不知道窗口内容」⇒ ⇒ 验证**引擎级保证**。
if (new URLSearchParams(location.search).has("blank")) {
  p5b.w0_set_blank_list(true);
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
//
// ⭐⭐ **调度器也影响「输入延迟」**（E-164）：
// `setInterval(50ms)` ⇒ ⇒ 一次按键最多要等 **50 ms** 才反映到 DOM
// ⇒ ⇒ ⇒ 而 `w0_tick` 本身只要 **0.11 ms**（E-158）⇒ ⇒ ⇒ **延迟全在调度器上**。
// ⇒ ⇒ `?tick=raf` ⇒ 改用 `requestAnimationFrame` 驱动 ⇒ ⇒ 延迟 ≤ **一帧**（≈16 ms）。
// ⭐ 默认改用 **`requestAnimationFrame`** 驱动（E-164）。
//
// ## 为什么
//
// 端到端更新延迟 = **引擎耗时**（0.11 ms，E-158）+ **调度间隔**
// ⇒ ⇒ `setInterval(50ms)` ⇒ ⇒ 端到端最坏 **50 ms**
// ⇒ ⇒ `rAF` ⇒ ⇒ 端到端最坏 **一帧**（≈16 ms）
// ⇒ ⇒ ⇒ 且这与**生产做法一致**（`apeireth-ui-app` 的宿主就是 rAF 帧驱动）
// ⇒ ⇒ ⇒ ⇒ 参考实现不该用一个**生产不会用**的调度器。
//
// ⚠️ `?tick=interval` 保留为**对照**（可复现「调度间隔决定延迟上限」这件事）。
const TICK_MODE = new URLSearchParams(location.search).get("tick") || "raf";
if (!new URLSearchParams(location.search).has("notick")) {
  if (TICK_MODE === "raf") {
    const step = () => {
      const r = p5b.w0_tick();
      // ⭐ 记录最近一次 tick 的结果（scroll_seen / redrawn）⇒ ⇒ 诊断用。
      window.__W0_TICK__ = r;
      requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  } else {
    setInterval(() => {
      p5b.w0_tick();
    }, 50);
  }
}

// ⚠️⚠️ **这里曾放过一个「输入延迟自测」，已撤掉**（E-164 的教训）：
//
// 它给自己设了 `input.value = 目标值` ⇒ ⇒ 于是「DOM 值 == 目标值」这件事
// **一开始就是真的** ⇒ ⇒ 测到的「延迟」只是 rAF 的一帧（0.5 ms）
// ⇒ ⇒ ⇒ **完全测不到引擎的写回** ⇒ ⇒ **假测量**。
//
// ⭐ 为什么**这个屏测不出输入延迟**：「受控输入」的 DOM 值**本来就等于**
//   用户输入的值 ⇒ ⇒ 引擎写回的是**同一个值** ⇒ ⇒ DOM 上**看不出**何时更新。
// ⇒ ⇒ 要测端到端延迟，需要一个**派生 UI**（如「字符计数」标签）⇒ ⇒ **尚未做**。
//
// ⇒ ⇒ **可测的是结构性事实**：端到端延迟上限 = **调度间隔**
//   （`setInterval(50ms)` ⇒ ≤50 ms / `rAF` ⇒ ≤ 一帧 ≈16 ms）
//   ⇒ ⇒ 而**引擎侧**那部分只要 **0.11 ms**（E-158）⇒ ⇒ ⇒ 延迟几乎全在调度器。

// ⭐⭐ **滚动 → 窗口化**的自测（E-164）：滚动容器 ⇒ ⇒ 等 tick ⇒ ⇒
//   检查**窗口首行是否移动了**（这正是「按需窗口化」的唯一触发路径）。
//
// ⚠️ 判据要看**首行文本**而不是行数（行数不变是对的 ⇒ ⇒ 窗口只是**平移**了）。
window.__W0_SCROLL__ = null;
async function measureScrollWindow() {
  const list = document.querySelector("#w0-root #w0-list");
  if (!list) return null;
  const firstRow = () => {
    const row = list.querySelector(".row");
    return row ? row.textContent : null;
  };
  const before = firstRow();
  list.scrollTop = 4000;
  // 等若干帧，让 tick（rAF 驱动）跑到并完成窗口化。
  for (let i = 0; i < 30; i++) {
    await new Promise((resolve) => requestAnimationFrame(resolve));
    if (firstRow() !== before) {
      return { before, after: firstRow(), moved: true };
    }
  }
  return { before, after: firstRow(), moved: false };
}
measureScrollWindow().then((value) => {
  window.__W0_SCROLL__ = value;
});

window.__W0__ = { ok: true };
