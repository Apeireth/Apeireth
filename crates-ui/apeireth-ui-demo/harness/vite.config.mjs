import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { runtimeMockPlugin } from './scripts/runtime-mock-plugin.mjs';

// 生产前端在**仓库内**（只读引入）⇒ vite 必须放行该路径。
// 路径由 `APEIRETH_REPO` 覆盖（默认按相对位置推算）。
//
// ⚠️ 相对层数：`harness/` → `apeireth-ui-demo/` → `crates-ui/` → **仓库根**
// ⇒ 三层（`../../..`）。层数算错 ⇒ 放行路径与替身拦截的根目录都会偏 ⇒
// 拦截不生效（实测踩过：四层 ⇒ 根落到仓库的上一级）。
const repoRoot = process.env.APEIRETH_REPO
  ? new URL(process.env.APEIRETH_REPO, 'file:///').pathname
  : new URL('../../..', import.meta.url).pathname.replace(/\/$/, '');
const frontendDir = `${repoRoot}/frontend/companion-desktop`;

// harness 根（`vite.config.mjs` 所在目录）。
const harnessDir = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  root: '.',
  base: './',
  plugins: [
    svelte(),
    // 生产 `runtime` 模块的**对照用替身**（只拦生产 `src/lib/**` 的导入
    // ⇒ 生产文件零改动；见 `scripts/runtime-mock-plugin.mjs`）。
    runtimeMockPlugin({
      frontendDir,
      mockFile: join(harnessDir, 'mock', 'runtime-mock.js'),
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
      input: {
        engine: 'engine/index.html',
        reference: 'reference/index.html',
        // 工具卡面对照（参考侧 / 引擎侧各一页）。
        engineTool: 'engine/tool/index.html',
        referenceTool: 'reference/tool/index.html',
        // 工具卡面批二（工具网格 + 工具栏）。
        engineToolgrid: 'engine/toolgrid/index.html',
        referenceToolgrid: 'reference/toolgrid/index.html',
      },
      // wasm 胶水是**构建期产物**，放在 public/engine/pkg/ ⇒ 运行时按绝对
      // 路径装载（不进打包图 ⇒ wasm 二进制原样送达）。
      external: ['/engine/pkg/apeireth-ui-demo.js'],
    },
  },
});
