import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// 生产前端在**仓库内**（只读引入）⇒ vite 必须放行该路径。
// 路径由 `APEIRETH_REPO` 覆盖（默认按相对位置推算）。
const repoRoot = process.env.APEIRETH_REPO
  ? new URL(process.env.APEIRETH_REPO, 'file:///').pathname
  : new URL('../../../..', import.meta.url).pathname.replace(/\/$/, '');
const frontendDir = `${repoRoot}/frontend/companion-desktop`;

export default defineConfig({
  root: '.',
  base: './',
  plugins: [svelte()],
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
      },
      // wasm 胶水是**构建期产物**，放在 public/engine/pkg/ ⇒ 运行时按绝对
      // 路径装载（不进打包图 ⇒ wasm 二进制原样送达）。
      external: ['/engine/pkg/apeireth-ui-demo.js'],
    },
  },
});
