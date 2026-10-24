// `data` 子面的**面内 vite 配置**（**只服务于本面** ⇒ 与其它面零撞车）。
//
// ## 为什么需要「面内配置」
//
// 共享的 `vite.config.mjs` 有两处让**本面的两态对照**跑不起来：
//
// 1. **替身目录是坏的**：`mock/<模块>.js` 是 `scripts/gen-settings-mock.mjs` 的
//    **生成物**，而生成器把「桩」的**数组**直接 push 进行列表 ⇒ join 出
//    `export function x() {,  throw new Error(…);,}` ⇒ **语法错误** ⇒ 一旦
//    替身真的被解析，vite 构建直接失败；
// 2. **仓库根推算少一级**：`new URL('../../../../..', import.meta.url)` 从
//    `harness/settings/` 往上数五级 ⇒ 落在**仓库根的上一级** ⇒ 替身的
//    `frontendDir` 指到不存在的目录 ⇒ 替身**永远不被解析** ⇒ 参考侧一直跑
//    **生产真模块**（浏览器里 ⇒ 读数恒为「读不到」）。
//
// ⇒ 本配置：**面内替身**（`web/engine/data/mock/`，同一份导出面）+ **按目录
// 内容推算仓库根** ⇒ 参考侧能真正喂上固定数据 ⇒ 「有读数 / 读不到」两态
// 都能对照。
//
// ## 纪律
//
// - **不改任何共享文件**（本配置是**新增**文件 ⇒ 在本面独占目录里）；
// - **生产文件零改动**（拦截发生在 vite 的解析阶段）；
// - 产物落在 `out/data-dist/`（**不入库**）。

import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// 本目录（`harness/settings/web/engine/data/`）。
const HERE = dirname(fileURLToPath(import.meta.url));
// 设置页 harness 根（`harness/settings/`）。
const SETTINGS = resolve(HERE, "../../..");

/**
 * 按**目录内容**推算仓库根（不靠「数几级」⇒ 换目录深度也不会错）。
 *
 * @param {string} start 起点目录。
 * @returns {string} 仓库根（含 `frontend/companion-desktop/`）。
 */
function findRepoRoot(start) {
  let dir = start;
  for (let depth = 0; depth < 10; depth += 1) {
    if (existsSync(join(dir, "frontend/companion-desktop/package.json"))) {
      return dir;
    }
    const parent = dirname(dir);
    if (parent === dir) {
      break;
    }
    dir = parent;
  }
  throw new Error(`找不到仓库根（含 frontend/companion-desktop/）：起点 ${start}`);
}

const REPO = process.env.APEIRETH_REPO
  ? new URL(process.env.APEIRETH_REPO, "file:///").pathname.replace(/\/$/, "")
  : findRepoRoot(SETTINGS);
const FRONTEND = join(REPO, "frontend/companion-desktop");
const MOCK_DIR = join(HERE, "mock");

/** 被替身拦下的生产数据面模块（basename，不含扩展名）。 */
const MOCKED_MODULES = [
  "runtime",
  "tauri-bridge",
  "desktop-bridge",
  "user-profile",
  "user-impression",
  "bg-store",
];

/**
 * 面内替身插件（**与共享插件同构**，只是指向**面内替身**）。
 *
 * @param {{frontendDir: string, mockDir: string}} options 目录。
 * @returns {import('vite').Plugin} vite 插件。
 */
function dataMockPlugin({ frontendDir, mockDir }) {
  const libDir = resolve(frontendDir, "src", "lib");
  const bases = MOCKED_MODULES.map((name) => join(libDir, name).replace(/\.(ts|js|mjs)$/, ""));

  return {
    name: "apeireth-data-face-mock",
    enforce: "pre",
    resolveId(source, importer) {
      if (!importer || !source.startsWith(".")) {
        return null;
      }
      const importerPath = resolve(importer.split("?")[0]);
      if (!importerPath.startsWith(`${libDir}/`)) {
        return null;
      }
      const resolved = resolve(dirname(importerPath), source).replace(/\.(ts|js|mjs)$/, "");
      const hit = bases.find((base) => base === resolved);
      if (!hit) {
        return null;
      }
      return join(mockDir, `${resolved.split("/").pop()}.js`);
    },
  };
}

/**
 * 给**引擎侧**页面链入**面内补丁样式**（`settings-data-face.css`）。
 *
 * ## 为什么用插件注入（而不是手改入口页）
 *
 * 入口页是**共享生成器**（`scripts/gen-entry-pages.mjs`）的产物 ⇒ 手改会
 * 让 `pages:check` 报漂移 ⇒ 共享门禁变红 ⇒ **不许**。
 *
 * ⇒ 用插件在**构建期**注入 `<link>`（生成物在 `out/` 里 ⇒ 不入库 ⇒ 共享
 * 生成物**零改动**）。
 *
 * @returns {import('vite').Plugin} vite 插件。
 */
function faceCssPlugin() {
  return {
    name: "apeireth-data-face-css",
    transformIndexHtml(html, ctx) {
      // ⚠️ **只**给引擎侧页面链（参考侧自带生产样式 ⇒ 重复链入反而多一份）。
      const page = ctx.path ?? "";
      if (!page.includes("/web/engine/data/")) {
        return html;
      }
      // ⚠️ **内联**注入（不落 `public/` ⇒ 共享目录**零改动**）。
      const css = readFileSync(join(HERE, "settings-data-face.css"), "utf8");
      return {
        html,
        tags: [
          {
            tag: "style",
            children: css,
            injectTo: "head",
          },
        ],
      };
    },
  };
}

export default defineConfig({
  // ⚠️ `root` = 设置页 harness 根 ⇒ 入口路径带 `web/` 前缀。
  root: SETTINGS,
  publicDir: resolve(SETTINGS, "public"),
  base: "./",
  plugins: [
    svelte(),
    dataMockPlugin({ frontendDir: FRONTEND, mockDir: MOCK_DIR }),
    faceCssPlugin(),
  ],
  server: {
    fs: {
      allow: [REPO, FRONTEND],
    },
  },
  build: {
    // 产物在**不入库**的目录里（`out/` 已被 harness 的 .gitignore 忽略）。
    outDir: resolve(SETTINGS, "out/data-dist"),
    emptyOutDir: true,
    target: "es2022",
    rollupOptions: {
      input: {
        // 参考侧：**共享**的入口页（只读引用生产组件）⇒ 面内替身生效。
        referenceData: "web/reference/data/index.html",
        // 引擎侧：**共享生成器**产出的入口页（本面独占目录里）。
        engineData: "web/engine/data/index.html",
      },
      // wasm 胶水是**构建期产物**（`public/engine/pkg/`）⇒ 不进打包图。
      external: ["/engine/pkg/apeireth-ui-demo.js"],
    },
  },
});
