// 引擎侧（设置页三页）的 wasm 装载 + 控制 API。
//
// ## 装配形态
//
// 胶水放在 `public/engine/pkg/`（**构建期产物**）⇒ 运行时按绝对路径动态装载
// （不进 vite 的打包图 ⇒ wasm 二进制原样送达）。
//
// ## 应用形态
//
// 同一个 wasm 产物挂多种应用 ⇒ 由 `window.__APP_MODE__` 决定（**必须在 wasm
// 初始化之前**设定 ⇒ 初始化即挂载）。
//
// ⚠️ **`__APP_MODE__` 的分发在共享 crate 里**（`crates-ui/apeireth-ui-demo/src/main.rs`）
// ⇒ **本批不改共享文件** ⇒ 本文件里的取值是**给迁移线的契约**，接线由主线做
// （见 `README.md` §8-3）。
//
// 取值格式：`settings:<面 id>`（面 id 见 `web/shared/panels.js`）。

import { PANELS } from '../shared/panels.js';

// 当前面（由 URL 路径决定：`/engine/<面>/index.html`）。
const panelId = (() => {
  const parts = location.pathname.split('/').filter(Boolean);
  return parts.length >= 2 ? parts[parts.length - 2] : PANELS[0].id;
})();

const isOverview = location.pathname.endsWith("/engine/") || location.pathname.endsWith("/engine");

if (!isOverview) {
  const panel = PANELS.find((item) => item.id === panelId) ?? PANELS[0];
  const mode = `settings:${panel.id}`;
  if (typeof window !== 'undefined') {
    window.__APP_MODE__ = mode;
    window.__SETTINGS_PANEL__ = panel.id;
  }

  const module = await import(/* @vite-ignore */ '/engine/pkg/apeireth-ui-demo.js');
  const { createSettingsApi } = await import('../shared/settings-api.js');

  // 供对照 harness 调用（与参考侧的 `window.__SETTINGS__` 同名同义）。
  // ⚠️ 引擎侧的 `selectSection` 只切**本面内**的分区（跨面切换由脚本换页面）。
  window.__SETTINGS__ = createSettingsApi({
    panel: panel.id,
    sections: panel.sections,
    fixture: panel.fixture,
    // 引擎侧自己的裁剪选择器（按当前分区取）。
    rootSelector: () => {
      const current = window.__SETTINGS_SECTION__ ?? panel.sections[0].id;
      const row = panel.sections.find((item) => item.id === current);
      return (row ?? panel.sections[0]).engine;
    },
    currentSection: () => window.__SETTINGS_SECTION__ ?? panel.sections[0],
    selectSection: (id) => {
      window.__SETTINGS_SECTION__ = id;
      if (typeof module.set_settings_section === 'function') {
        module.set_settings_section(id);
      }
    },
    ready: () => window.__SETTINGS_READY__ === true,
  });

  await module.default();
} else {
  const list = document.getElementById('app');
  if (list) {
    list.innerHTML = PANELS.map(
      (panel) =>
        `<li><a href="./${panel.id}/index.html">${panel.title}</a>` +
        `<small>${panel.id} · ${panel.engine.mode} · ${panel.engine.module}</small></li>`,
    ).join('');
  }
}
