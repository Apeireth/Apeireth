// 设置页对照 · **两侧同名同义的 API 骨架**。
//
// ## 为什么需要它
//
// 对照脚本要**用同一套调用**驱动两侧（换分区 / 切步骤 / 触发动作 / 读 DOM）⇒
// 两侧各暴露一个 `window.__SETTINGS__` 对象，**方法名与语义逐条对齐**。
//
// 批一已有同构装置（`harness/shared/toolcard-api.js`）⇒ 这里沿用同一形态。
//
// ## 契约
//
// | 方法 | 语义 | 两侧都必须有 |
// |---|---|:--:|
// | `ready()` | 画面已渲染完毕（可截图） | ✅ |
// | `panel()` | 当前面 id | ✅ |
// | `section()` | 当前生产分区 id（`SettingsView` 的 15 个之一；独立组件面用面板自身的步骤 id） | ✅ |
// | `sections()` | 本面覆盖的分区列表 | ✅ |
// | `selectSection(id)` | 切到某个分区 | ✅ |
// | `fixture()` | 本面喂的固定数据（对象） | ✅ |
// | `dom()` | 本面根节点的 HTML（归一化前的原文） | ✅ |
//
// ## 纪律
//
// - **不引入业务名词到引擎侧的实现**：本文件是**对照装置的契约**，不是引擎 API。
// - 引擎侧的 `selectSection` 只切**本面内**的分区；跨面切换由脚本换页面。

/**
 * 建一份 API（两侧共用同一份实现 ⇒ 语义不可能漂移）。
 *
 * @param {{
 *   panel: string,
 *   sections: string[],
 *   fixture: unknown,
 *   selectSection: (id: string) => void,
 *   currentSection: () => string,
 *   rootSelector: () => string,
 *   ready: () => boolean,
 * }} options
 *   `options` = 两侧各自的实现。
 * @returns {object} 挂到 `window.__SETTINGS__` 的 API。
 */
export function createSettingsApi(options) {
  return {
    /** 画面已渲染完毕。 */
    ready: () => options.ready(),
    /** 当前面 id。 */
    panel: () => options.panel,
    /** 当前分区 id。 */
    section: () => options.currentSection(),
    /** 本面覆盖的分区列表。 */
    sections: () => options.sections.slice(),
    /** 切到某个分区。 */
    selectSection: (id) => options.selectSection(id),
    /** 本面喂的固定数据。 */
    fixture: () => options.fixture,
    /** 本面根节点的 HTML 原文（按**本侧自己的**裁剪选择器）。 */
    dom: () => {
      const el = document.querySelector(options.rootSelector());
      return el ? el.outerHTML : null;
    },
  };
}
