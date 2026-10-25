import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';
import { dirname, join, resolve } from 'node:path';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { settingsMockPlugin } from './mock/settings-mock-plugin.mjs';
import { PANELS } from './web/shared/panels.js';

// 生产前端在**仓库内**（只读引入）⇒ vite 必须放行该路径。
// 路径由 `APEIRETH_REPO` 覆盖（默认按相对位置推算）。
const repoRoot = process.env.APEIRETH_REPO
  ? new URL(process.env.APEIRETH_REPO, 'file:///').pathname
  // ⚠️ **层数是 4 级，不是 5**（并行面 M5 实测发现，2026-10-24）：
  // `harness/settings/` ⇒ `..`=harness/ ⇒ `../..`=apeireth-ui-demo/ ⇒ `../../..`=crates-ui/
  // ⇒ `../../../..`=**仓库根**。写成 5 级会落到**仓库根的上一级**
  // ⇒ `frontendDir` 指向**不存在的目录** ⇒ 替身**永远不被解析**
  // ⇒ 参考侧一直跑**生产真模块**（浏览器里 ⇒ 读数恒为「读不到」）
  // ⇒ 「有读数」态在共享构建里**不可达**。
  : new URL('../../../..', import.meta.url).pathname.replace(/\/$/, '');
const frontendDir = `${repoRoot}/frontend/companion-desktop`;

// 本目录（`harness/settings/`）根。
const settingsDir = dirname(fileURLToPath(import.meta.url));

/**
 * 给**引擎侧**页面内联**面内补丁样式**（`web/engine/<面>/<面>-face.css`）。
 *
 * ## 为什么需要它
 *
 * 共享的抽取器（`scripts/extract-settings-assets.mjs`）按「**规则的全部 class
 * 选择器**都在本面的 class 线索集里」来归属规则 ⇒ **一条规则里有多个选择器
 * 分支、而各分支属于不同面时，整条规则会被漏掉**
 * ⇒ 引擎侧落到**浏览器默认样式** ⇒ 与参考侧（生产 CSS）不一致。
 *
 * ⚠️ **2026-10-25 实测**：`data` 面就撞上了这一条
 * （生产 `SettingsView.svelte:4138-4142` 的 `.form-group label`）
 * ⇒ 引擎侧 `<label>` 落到默认 14px ⇒ **两个表单行各高 3.1px**
 * ⇒ 内容块**高 6px** ⇒ 裁剪框**尺寸不等** ⇒ 逐像素对照**无法进行**。
 *
 * ## 为什么用插件注入（而不是手改入口页）
 *
 * 入口页是**共享生成器**（`scripts/gen-entry-pages.mjs`）的产物
 * ⇒ 手改会让 `pages:check` 报漂移 ⇒ 共享门禁变红 ⇒ **不许**。
 *
 * ⇒ 插件在**构建期**注入 `<style>`（生成物在 `out/` 里 ⇒ 不入库
 * ⇒ 共享生成物**零改动**）。
 *
 * ## 约定（**通用**，不针对某个面）
 *
 * 面内补丁文件 = `web/engine/<面 id>/<面 id>-face.css`（**一个面最多一个**）
 * ⇒ 例：`web/engine/data/data-face.css`
 * ⇒ 插件按面 id 自动发现 ⇒ 新增面的补丁**不用改本插件**。
 *
 * ⚠️ **只**给引擎侧页面链（参考侧自带生产样式 ⇒ 重复链入会多一份）。
 */
function faceCssPlugin() {
  return {
    name: 'apeireth-settings-face-css',
    transformIndexHtml(html, ctx) {
      const page = ctx.path ?? '';
      if (!page.includes('/web/engine/')) {
        return html;
      }
      // 从路径里取面 id：`/web/engine/<面 id>/…`。
      const match = page.match(/\/web\/engine\/([^/]+)\//);
      if (!match) {
        return html;
      }
      const faceCss = join(settingsDir, 'web', 'engine', match[1], `${match[1]}-face.css`);
      if (!existsSync(faceCss)) {
        return html;
      }
      return {
        html,
        tags: [
          {
            tag: 'style',
            children: readFileSync(faceCss, 'utf8'),
            injectTo: 'head',
          },
        ],
      };
    },
  };
}

/** 每个面两条入口（参考侧 / 引擎侧）。 */
const inputs = {};
for (const panel of PANELS) {
  inputs[`reference${panel.id[0].toUpperCase()}${panel.id.slice(1)}`] =
    `web/reference/${panel.id}/index.html`;
  inputs[`engine${panel.id[0].toUpperCase()}${panel.id.slice(1)}`] =
    `web/engine/${panel.id}/index.html`;
}
inputs.referenceOverview = 'web/reference/index.html';
inputs.engineOverview = 'web/engine/index.html';

export default defineConfig({
  // ⚠️ `root` = 本目录 ⇒ 入口路径带 `web/` 前缀（`web/reference/<面>/index.html`）
  // ⇒ 对照脚本按同一前缀构造 URL（见 `tools/compare_settings.py`）。
  root: '.',
  // 静态资源（设计层 CSS / 各子面 scoped CSS / wasm 胶水）在 `settings/public/`
  // ⇒ 复制到 `dist/` 根 ⇒ 页面用绝对路径 `/design/...` 引用。
  publicDir: resolve(settingsDir, 'public'),
  base: './',
  plugins: [
    svelte(),
    // 生产**数据面模块**的对照用替身（只拦生产 `src/lib/**` 的导入
    // ⇒ 生产文件零改动；见 `mock/settings-mock-plugin.mjs`）。
    settingsMockPlugin({
      frontendDir,
      mockDir: join(settingsDir, 'mock'),
    }),
    // 面内补丁样式（**通用**：按面 id 自动发现 `<面 id>-face.css`）。
    faceCssPlugin(),
  ],
  server: {
    fs: {
      // 生产前端（只读）与仓库内的 wasm 产物都要能被 vite 读到。
      allow: [repoRoot, frontendDir],
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    target: 'es2022',
    rollupOptions: {
      input: inputs,
      // wasm 胶水是**构建期产物**，放在 public/engine/pkg/ ⇒ 运行时按绝对
      // 路径装载（不进打包图 ⇒ wasm 二进制原样送达）。
      external: ['/engine/pkg/apeireth-ui-demo.js'],
    },
  },
});
