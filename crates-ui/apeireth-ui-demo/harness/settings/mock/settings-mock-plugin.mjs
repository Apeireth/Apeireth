// vite 插件：把生产**数据面模块**的导入拦到对照用替身。
//
// ## 为什么需要它
//
// 参考侧要**只读引入**生产的设置组件（不复制、不改）⇒ 这些组件在挂载时就会
// 碰数据面：
//
// | 生产模块 | 谁在挂载时碰它 |
// |---|---|
// | `runtime` | `listModels`（每次挂载都拉一次模型列表） |
// | `tauri-bridge` | `getWorkspaceDir`（每次挂载都读工作区） |
// | `desktop-bridge` | `isDesktop`（**渲染期**读 ⇒ 决定学习日志分支） |
// | `user-profile` | `loadUserProfile`（初始化时读本地存储） |
// | `user-impression` | `loadUserImpression`（初始化时读本地存储） |
// | `bg-store` | `getCustomBg`（`customBg` 为真时读 IndexedDB） |
//
// ⇒ 要喂**固定数据**，就得让这些 specifier 解析到替身 ⇒ 在 `resolveId` 阶段拦截。
//
// ## 拦截口径（**按 importer 路径判断**）
//
// | importer | 拦？ |
// |---|---|
// | 生产 `src/lib/**` 下的文件，且导入解析到**被拦模块**之一 | ✅ |
// | 其他任何 importer（harness 自己的文件等） | ❌ |
//
// ⇒ 只有生产组件对数据面的依赖被替换；harness 自身的模块图不受影响。
//
// ## 纪律
//
// - **不修改生产任何文件**（拦截发生在 vite 的解析阶段 ⇒ 磁盘上的生产文件不动）；
// - 拦不到 ⇒ **显式报错**（不静默回落到真实模块 ⇒ 那会去打真实后端）。

import path from "node:path";

/**
 * 被拦的生产模块（**basename**，不含扩展名）。
 *
 * ⚠️ 这张表是**显式列举**的（不是「凡是 runtime 都拦」）⇒ 漏一个就会在浏览器
 * 控制台里看到真实请求 ⇒ 对照脚本会因此判失败（**显式失败优于静默跑偏**）。
 */
export const MOCKED_MODULES = [
  "runtime",
  "tauri-bridge",
  "desktop-bridge",
  "user-profile",
  "user-impression",
  "bg-store",
];

/**
 * 建插件。
 *
 * @param {{frontendDir: string, mockDir: string}} options
 *   `frontendDir` = 生产前端根；`mockDir` = 替身模块目录（绝对路径）。
 * @returns {import('vite').Plugin} vite 插件。
 */
export function settingsMockPlugin({ frontendDir, mockDir }) {
  const libDir = path.resolve(frontendDir, "src", "lib");
  const bases = MOCKED_MODULES.map(
    (name) => path.join(libDir, name).replace(/\.(ts|js|mjs)$/, ""),
  );

  return {
    name: "apeireth-settings-mock",
    enforce: "pre",
    resolveId(source, importer) {
      if (!importer || !source.startsWith(".")) {
        return null;
      }
      const importerPath = path.resolve(importer.split("?")[0]);
      if (!importerPath.startsWith(`${libDir}${path.sep}`)) {
        return null;
      }
      const resolved = path
        .resolve(path.dirname(importerPath), source)
        .replace(/\.(ts|js|mjs)$/, "");
      const hit = bases.find((base) => base === resolved);
      if (!hit) {
        return null;
      }
      return path.join(mockDir, `${path.basename(resolved)}.js`);
    },
  };
}
