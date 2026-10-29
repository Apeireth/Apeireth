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
// | `ready()` | 页面与卡片是否就绪 |
// | `caseCount()` | 固定数据的用例数 |
// | `selectCase(i)` | 切到第 i 组用例 |
// | `state()` | **从 DOM 读出**当前展开态（JSON 字符串） |
// | `expandTool(i)` | 点第 i 张工具卡的标题（展开 / 收起） |
// | `toggleRaw(i)` | 点第 i 张工具卡的「原始 JSON」按钮 |
// | `expandLifecycle(i)` | 点第 i 张生命周期卡的标题（展开 / 收起） |
//
// ## 状态为什么从 DOM 读
//
// 生产组件把展开态放在**组件内部**（外部没有写入口）⇒ 参考侧拿不到内部状态；
// 而两侧渲染出的 DOM 结构一致 ⇒ 「DOM 里有没有 `.tool-body`」就是两侧**同义**
// 的状态判据。引擎侧同理（读自己的 DOM）⇒ 两侧口径一致。

/** 工具卡的标题元素（第 index 张）。 */
function toolHeader(index) {
  const cards = document.querySelectorAll('.tool-card');
  const card = cards[index];
  return card ? card.querySelector('.tool-header') : null;
}

/** 工具卡的「原始 JSON」按钮（第 index 张）。 */
function rawButton(index) {
  const cards = document.querySelectorAll('.tool-card');
  const card = cards[index];
  return card ? card.querySelector('.raw-toggle-btn') : null;
}

/** 生命周期卡的标题元素（第 index 张）。 */
function lifecycleHeader(index) {
  const cards = document.querySelectorAll('.tool-lifecycle-card');
  const card = cards[index];
  return card ? card.querySelector('.tool-header') : null;
}

/** 从 DOM 读出两侧同义的展开态。 */
function collectState() {
  const tools = Array.from(document.querySelectorAll('.tool-card'));
  const lifecycles = Array.from(document.querySelectorAll('.tool-lifecycle-card'));
  return {
    toolExpanded: tools.map((card) => card.querySelector('.tool-body') !== null),
    toolRaw: tools.map((card) => card.querySelector('.json-detail') !== null),
    lifecycleExpanded: lifecycles.map((card) => card.querySelector('.tool-body') !== null),
    toolStatus: tools.map((card) => {
      const badge = card.querySelector('.tool-status');
      return badge ? badge.textContent.trim() : null;
    }),
    toolAria: tools.map((card) => {
      const button = card.querySelector('.expand-btn');
      return button ? button.getAttribute('aria-label') : null;
    }),
  };
}

/**
 * 建一组控制 API。
 *
 * @param {{caseCount: () => number, selectCase: (index: number) => void}} options
 *   两侧各自提供的「用例数」与「切用例」实现（其余方法两侧**完全相同**）。
 */
export function createToolCardApi(options) {
  return {
    ready: () => document.querySelector('.tool-card') !== null,
    caseCount: () => options.caseCount(),
    selectCase: (index) => options.selectCase(index),
    state: () => JSON.stringify(collectState()),
    expandTool: (index) => {
      const target = toolHeader(index);
      if (target) target.click();
      return Boolean(target);
    },
    toggleRaw: (index) => {
      const target = rawButton(index);
      if (target) target.click();
      return Boolean(target);
    },
    expandLifecycle: (index) => {
      const target = lifecycleHeader(index);
      if (target) target.click();
      return Boolean(target);
    },
  };
}
