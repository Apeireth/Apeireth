import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';
import { dirname, join, resolve } from 'node:path';
import { settingsMockPlugin } from './mock/settings-mock-plugin.mjs';
import { PANELS } from './web/shared/panels.js';

// 生产前端在**仓库内**（只读引入）⇒ vite 必须放行该路径。
// 路径由 `APEIRETH_REPO` 覆盖（默认按相对位置推算）。
const repoRoot = process.env.APEIRETH_REPO
  ? new URL(process.env.APEIRETH_REPO, 'file:///').pathname
  : new URL('../../../../..', import.meta.url).pathname.replace(/\/$/, '');
const frontendDir = `${repoRoot}/frontend/companion-desktop`;

// 本目录（`harness/settings/`）根。
const settingsDir = dirname(fileURLToPath(import.meta.url));

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
