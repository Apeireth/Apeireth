import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { runtimeMockPlugin } from './scripts/runtime-mock-plugin.mjs';

// 生产前端在**仓库内**（只读引入）⇒ vite 必须放行该路径。
// 路径由 `APEIRETH_REPO` 覆盖（默认按相对位置推算）。
//
// ⚠️ 相对层数：`governance/` → `harness/` → `apeireth-ui-demo/` → `crates-ui/`
// → **仓库根** ⇒ 四层。
const repoRoot = process.env.APEIRETH_REPO
  ? new URL(process.env.APEIRETH_REPO, 'file:///').pathname
  : new URL('../../../../', import.meta.url).pathname.replace(/\/$/, '');
const frontendDir = `${repoRoot}/frontend/companion-desktop`;

// harness 根（`vite.config.mjs` 所在目录）。
const harnessDir = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  root: '.',
  base: './',
  plugins: [
    svelte(),
    // 生产 `runtime` 模块的**对照用替身**（只拦治理卷宗面四个 Tab 的导入
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
        engineApprovals: 'engine/approvals/index.html',
        referenceApprovals: 'reference/approvals/index.html',
        engineGrants: 'engine/grants/index.html',
        referenceGrants: 'reference/grants/index.html',
        engineAudit: 'engine/audit/index.html',
        referenceAudit: 'reference/audit/index.html',
        engineGuard: 'engine/guard/index.html',
        referenceGuard: 'reference/guard/index.html',
      },
      // wasm 胶水是**构建期产物**，放在 public/engine/pkg/ ⇒ 运行时按绝对
      // 路径装载（不进打包图 ⇒ wasm 二进制原样送达）。
      external: ['/engine/pkg/governance-harness.js'],
    },
  },
});
