import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';
import { dirname, join, resolve } from 'node:path';

// 状态条面对照的**独立** vite 配置（共享的 `harness/vite.config.mjs` 是
// **共享点** ⇒ 本面不改它 ⇒ 本面自带一份）。
//
// 生产前端在**仓库内**（只读引入）⇒ vite 必须放行该路径。
// 路径由 `APEIRETH_REPO` 覆盖（默认按相对位置推算）。
//
// ⚠️ 相对层数：`harness/statusbar/` → `apeireth-ui-demo/` → `crates-ui/` →
// **仓库根** ⇒ 三层（`../../..`）。
// ⚠️ Windows：`new URL(...).pathname` 给的是 `/C:/...` 这种 POSIX 形态（空格还会
// 被编码成 %20）⇒ 仓库根一律走 `fileURLToPath`（Windows 下是 `C:\...`）。
// 判错的后果：仓库根算偏 ⇒ 生产 `src/lib` 目录算不对 ⇒ 替身**永远不被解析**
// ⇒ 参考侧跑生产真模块 ⇒ 真去请求后端 ⇒ 对照基准失真。
const repoRoot = process.env.APEIRETH_REPO
  ? (process.env.APEIRETH_REPO.startsWith('file:')
      ? fileURLToPath(new URL(process.env.APEIRETH_REPO))
      : resolve(process.env.APEIRETH_REPO))
  : fileURLToPath(new URL('../../../..', import.meta.url)).replace(/[\\/]+$/, '');
const frontendDir = join(repoRoot, 'frontend', 'companion-desktop');

// harness 根（`vite.config.mjs` 所在目录）。
const harnessDir = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  root: '.',
  base: './',
  plugins: [svelte()],
  server: {
    fs: {
      // 生产前端（只读）要被 vite 读到。
      allow: [repoRoot, frontendDir],
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    target: 'es2022',
    rollupOptions: {
      input: {
        engineStatusbar: 'engine/statusbar/index.html',
        referenceStatusbar: 'reference/statusbar/index.html',
      },
      // wasm 胶水是**构建期产物**，放在 public/engine/pkg/ ⇒ 运行时按绝对
      // 路径装载（不进打包图 ⇒ wasm 二进制原样送达）。
      external: ['/engine/pkg/statusbar_harness.js'],
    },
  },
});
