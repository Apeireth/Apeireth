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
// | `ready()` | 卷宗面根节点（`.gov-tab`）是否已就位 |
// | `caseCount()` | 固定数据的用例数 |
// | `applyCase(i)` | 按用例的**动作序列**点 DOM（两侧同义） |
// | `state()` | **从 DOM 读出**当前状态（JSON 字符串） |
//
// ## 动作为什么用「DOM 点击」
//
// 用例的动作序列（切数据源 / 选轨迹 / 展开事件 / 切过滤档）**两边都是点击
// 真实按钮** ⇒ 参考侧触发的是生产组件自己的 handler，引擎侧触发的是 Rust
// 组件的 handler ⇒ 两侧走的是**同一条交互路径** ⇒ 状态可比。
//
// ## 状态为什么从 DOM 读
//
// 生产组件把过滤档 / 选中项放在**组件内部**（外部没有写入口）⇒ 参考侧拿不到
// 内部状态；而两侧渲染出的 DOM 结构一致 ⇒ 「DOM 里有没有 `.md-row.on`」就是
// 两侧**同义**的状态判据。引擎侧同理（读自己的 DOM）⇒ 两侧口径一致。

/** 点第 `index` 个匹配 `selector` 的元素。 */
function clickAt(selector, index) {
  const nodes = document.querySelectorAll(selector);
  const node = nodes[index];
  if (!node) return false;
  node.click();
  return true;
}

/** 从 DOM 读出两侧同义的状态快照。 */
function collectState() {
  const text = (selector) => {
    const node = document.querySelector(selector);
    return node ? node.textContent.trim() : null;
  };
  const count = (selector) => document.querySelectorAll(selector).length;
  return {
    cards: count('.gov-card'),
    rows: count('.md-row'),
    rowsSelected: count('.md-row.on'),
    chips: count('.gov-chip'),
    chipsOn: count('.gov-chip.on'),
    chipsSub: count('.gov-chip.sm'),
    badges: count('.status-badge'),
    guides: count('.guide-card'),
    confirmBlocks: count('.revoke-confirm'),
    details: count('.ev-detail'),
    emptyMarks: count('.empty-mark'),
    unsupported: count('.gov-unsupported'),
    manifestPending: count('.manifest-pending'),
    loadingStates: count('.loading-state-wrap'),
    errorStates: count('.error-state-wrap'),
    placeholders: count('.md-placeholder'),
    spanRows: count('.span-row'),
    auditDetails: count('.audit-detail'),
    tally: count('.tally-cell'),
    dryCard: count('.dry-card'),
    position: text('.gov-position'),
    title: text('.empty-title'),
  };
}

/**
 * 建一组控制 API。
 *
 * @param {{cases: Array<{actions?: Array<{click: string, index?: number}>}>}} options
 *   当前面的用例清单（两侧喂**同一份** fixture ⇒ 同一份清单）。
 * @returns {object} 控制 API。
 */
export function createGovernanceApi(options) {
  return {
    // 「就绪」= 卷宗面根节点已就位 **且** 数据请求已发出（参考侧由 mock 置位、
    // 引擎侧由 wasm 装载完成置位）⇒ 抓到的画面是**数据到达后**的状态。
    ready: () =>
      document.querySelector('.gov-tab') !== null &&
      window.__GOV_DATA_READY__ === true,
    caseCount: () => options.cases.length,
    applyCase: (index) => {
      const item = options.cases[index];
      if (!item) return false;
      for (const action of item.actions || []) {
        clickAt(action.click, action.index || 0);
      }
      return true;
    },
    state: () => JSON.stringify(collectState()),
  };
}
