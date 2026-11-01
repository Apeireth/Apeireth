// 两侧**同名同义**的控制 API（对照 harness 用同一套触发序列驱动两侧）。
//
// ## 为什么放在共享文件里
//
// 参考侧与引擎侧的 API 必须是**逐字同一份实现** ⇒ 放在共享模块里，两侧各
// import 一份 ⇒ 「同名同义」不是靠约定，而是靠**同一份源码**。
//
// ## 语义
//
// | 方法 | 语义 |
// |---|---|
// | `ready()` | 页面与状态条是否就绪 |
// | `caseCount()` | 固定数据的用例数 |
// | `selectCase(i)` | 切到第 i 组用例 |
// | `state()` | **从 DOM 读出**当前状态（JSON 字符串） |
// | `clickCell(i)` | 点第 i 格（可点的格 ⇒ 触发其回调） |
// | `clickCount()` | 回调被调用过几次（**证据**：回调真的接上了） |
//
// ## 状态为什么从 DOM 读
//
// 状态条的状态**全部**落在 DOM 上（class / title / 文本）⇒ 两侧从各自的 DOM
// 读出**同一形状**的描述 ⇒ 可逐字对照。`clickCount()` 例外：它读的是**各自
// 实现的计数器**（参考侧 = Svelte 状态，引擎侧 = Rust 计数器）⇒ 证明「回调
// 真的被调用」。

/** 状态条上的格（第 index 格）。 */
function cell(index) {
  const cells = document.querySelectorAll('.sb-item');
  return cells[index] ?? null;
}

/** 从 DOM 读出两侧同义的状态。 */
function collectState() {
  const cells = Array.from(document.querySelectorAll('.sb-item'));
  return {
    barLabel: (() => {
      const bar = document.querySelector('.status-bar');
      return bar ? bar.getAttribute('aria-label') : null;
    })(),
    cells: cells.map((item) => ({
      tag: item.tagName.toLowerCase(),
      className: item.getAttribute('class'),
      title: item.getAttribute('title'),
      text: item.textContent,
      sim: item.querySelector('.sim-badge') !== null,
    })),
    badge: (() => {
      const badge = document.querySelector('.status-badge');
      if (!badge) {
        return null;
      }
      return {
        className: badge.getAttribute('class'),
        text: badge.textContent,
      };
    })(),
  };
}

/**
 * 建一组控制 API。
 *
 * @param {{
 *   caseCount: () => number,
 *   selectCase: (index: number) => void,
 *   clickCount: () => number,
 * }} options 两侧各自提供的实现（其余方法两侧**完全相同**）。
 */
export function createStatusBarApi(options) {
  return {
    ready: () => document.querySelector('.status-bar') !== null,
    caseCount: () => options.caseCount(),
    selectCase: (index) => options.selectCase(index),
    state: () => JSON.stringify(collectState()),
    clickCell: (index) => {
      const target = cell(index);
      if (target) {
        target.click();
      }
      return Boolean(target);
    },
    clickCount: () => options.clickCount(),
  };
}
