// vite 插件：把生产 `runtime` 模块的导入**拦到对照用替身**。
//
// ## 为什么需要它
//
// 参考侧要**只读引入**生产的 `ApprovalsTab.svelte` / `GrantsTab.svelte` /
// `AuditTab.svelte` / `GuardTab.svelte`（不复制、不改）⇒ 这四个组件在
// `onMount` 里 `import { fetchGrants, … } from '../../runtime'`。要喂固定数据，
// 就得让这个 specifier 解析到替身 ⇒ 在 `resolveId` 阶段拦截。
//
// ## 拦截口径（**按 importer 路径判断**）
//
// | importer | 拦？ |
// |---|---|
// | 生产 `src/lib/views/governance/**` 下的文件，且导入解析到 `src/lib/runtime` | ✅ |
// | 其他任何 importer（harness 自己的文件、生产别的目录等） | ❌ |
//
// ⇒ 只有**治理卷宗面四个 Tab** 对 `runtime` 的依赖被替换；生产别的视图不受
// 影响（本 harness 只引入这四个 Tab ⇒ 实际拦截面与声明一致）。
//
// ## 纪律
//
// - **不修改生产任何文件**（拦截发生在 vite 的解析阶段，磁盘上的生产文件不动）；
// - 拦不到 ⇒ **显式报错**（不静默回落到真实 runtime ⇒ 那会去打真实后端）。

import path from "node:path";

/**
 * 建插件。
 *
 * @param {{frontendDir: string, mockFile: string}} options
 *   `frontendDir` = 生产前端根；`mockFile` = 替身模块的绝对路径。
 * @returns {import('vite').Plugin} vite 插件。
 */
export function runtimeMockPlugin({ frontendDir, mockFile }) {
  const libDir = path.resolve(frontendDir, "src", "lib");
  const governanceDir = path.join(libDir, "views", "governance");
  const runtimeBase = path.join(libDir, "runtime").replace(/\.(ts|js|mjs)$/, "");

  return {
    name: "apeireth-governance-runtime-mock",
    enforce: "pre",
    resolveId(source, importer) {
      if (!importer || !source.startsWith(".")) {
        return null;
      }
      const importerPath = path.resolve(importer.split("?")[0]);
      if (!importerPath.startsWith(`${governanceDir}${path.sep}`)) {
        return null;
      }
      const resolved = path
        .resolve(path.dirname(importerPath), source)
        .replace(/\.(ts|js|mjs)$/, "");
      if (resolved !== runtimeBase) {
        return null;
      }
      return mockFile;
    },
  };
}
