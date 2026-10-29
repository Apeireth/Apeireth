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
// | `ready()` | 工具卡面是否就绪（数据已到 + 工具栏已渲染） |
// | `caseCount()` | 固定数据的用例数 |
// | `selectCase(i)` | 切到第 i 组用例（用例 = 一个搜索词） |
// | `state()` | **从 DOM 读出**当前状态（JSON 字符串） |
// | `clearSearch()` | 点「清除搜索」按钮（清空搜索词） |
//
// ## 搜索动作为什么两侧逐字相同
//
// 生产的搜索框是 `bind:value` ⇒ 改值要**写 value 属性 + 派发 `input` 事件**。
// 引擎侧的双向绑定**尚未接线**（`view!` 的 `bind:value` 显式报错；VDOM 层没有
// property 通道）⇒ 两侧都由本文件**写同一个 input 的 value 属性 + 派发同一个
// 事件**，引擎侧再由宿主把**同一个词**写进状态（`options.applyQuery`）。
// ⇒ DOM 动作逐字相同 ⇒ 画面可比；差异只可能来自渲染。
//
// ## 状态为什么从 DOM 读
//
// 参考侧把搜索词放在**生产组件内部**（外部没有写入口）⇒ 参考侧拿不到内部
// 状态；而两侧渲染出的 DOM 结构一致 ⇒ 「DOM 里有什么」就是两侧**同义**的
// 状态判据。引擎侧同理（读自己的 DOM）⇒ 两侧口径一致。

import fixture from "../fixtures/toolgrid-cases.json";

/** 搜索框（工具栏里的输入框）。 */
function searchInput() {
  return document.querySelector(".tools-toolbar input");
}

/**
 * 写搜索词（**两侧逐字相同**的 DOM 动作）。
 *
 * @param {string} query 搜索词。
 * @returns {boolean} 是否找到搜索框。
 */
function writeSearchInput(query) {
  const input = searchInput();
  if (!input) {
    return false;
  }
  input.value = query;
  input.dispatchEvent(new Event("input", { bubbles: true }));
  return true;
}

/** 第 index 组用例的搜索词。 */
function queryOf(index) {
  const item = fixture.cases[index];
  return item ? item.query : "";
}

/** 从 DOM 读出两侧同义的状态。 */
function collectState() {
  const cards = Array.from(document.querySelectorAll(".tool-card-box"));
  const summary = document.querySelector(".tools-summary-meta");
  const input = searchInput();
  const textOf = (root, selector) => {
    const node = root.querySelector(selector);
    return node ? node.textContent : null;
  };
  return {
    gridPresent: document.querySelector(".tools-grid") !== null,
    cardCount: cards.length,
    cardTitles: cards.map((card) => textOf(card, ".tool-title")),
    cardBadges: cards.map((card) => textOf(card, ".status-badge")),
    cardDescs: cards.map((card) => textOf(card, ".tool-desc")),
    cardSources: cards.map((card) => textOf(card, ".source-pill")),
    cardDetails: cards.map((card) => textOf(card, ".details-link")),
    summary: summary ? summary.textContent : null,
    inputValue: input ? input.value : null,
    clearButton: document.querySelector(".clear-search-btn") !== null,
  };
}

/**
 * 建一组控制 API。
 *
 * @param {{
 *   caseCount: () => number,
 *   applyQuery: (query: string) => void,
 * }} options
 *   两侧各自提供的「用例数」与「把搜索词写进状态」实现（其余方法两侧**完全
 *   相同**）。
 * @returns {object} 控制 API。
 */
export function createToolGridApi(options) {
  return {
    ready: () =>
      window.__TOOLGRID_READY__ === true &&
      document.querySelector(".tools-toolbar") !== null,
    caseCount: () => options.caseCount(),
    selectCase: (index) => {
      const query = queryOf(index);
      writeSearchInput(query);
      options.applyQuery(query);
      return true;
    },
    state: () => JSON.stringify(collectState()),
    clearSearch: () => {
      const button = document.querySelector(".clear-search-btn");
      if (button) {
        button.click();
      }
      writeSearchInput("");
      options.applyQuery("");
      return true;
    },
  };
}
