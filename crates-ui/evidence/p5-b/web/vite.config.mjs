import { defineConfig } from 'vite';

// 生产前端在**仓库内**（只读引入）⇒ vite 必须放行该路径。
// 路径按本文件位置推算仓库根（crates-ui/evidence/p5-b/web → 仓库根 = 4 级）。
const repoRoot = new URL('../../../../', import.meta.url).pathname.replace(/\/$/, '');

export default defineConfig({
  root: '.',
  base: '/',
  server: {
    fs: {
      // 生产前端（只读）必须能被 vite 读到。
      allow: [repoRoot],
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    target: 'es2022',
    rollupOptions: {
      input: {
        reference: 'reference/index.html',
        engine: 'engine/index.html',
        intro: 'intro/index.html',
      },
      // wasm 胶水是**构建期产物**（放在 public/engine/pkg/）⇒ 不进打包图，
      // 运行时按绝对路径装载（wasm 二进制原样送达）。
      external: ['/engine/pkg/p5b_harness.js'],
    },
  },
});
