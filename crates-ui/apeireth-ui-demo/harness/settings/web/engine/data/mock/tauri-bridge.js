// `tauri-bridge` 替身（**面内派生** —— 由共享的生成物派生 ⇒ 见文件头的说明）。
//
// ## 为什么有「面内替身」
//
// 共享的 `mock/tauri-bridge.js` 是**生成物**（`scripts/gen-settings-mock.mjs`），
// 而**当前那份是坏的**：生成器把「桩」的**数组**直接 push 进行列表 ⇒ join 出
// `export function x() {,  throw new Error(…);,}` ⇒ **语法错误** ⇒ vite 构建
// 直接失败 ⇒ **替身路径从未真正跑起来**（共享构建里 `APEIRETH_REPO` 未设 ⇒
// 替身根本没被解析 ⇒ 参考侧一直跑的是**生产真模块**）。
//
// ⇒ 本文件按**同一份导出面**派生一份**语法正确**的替身（**面内写域**）⇒
// 参考侧能真正喂上固定数据 ⇒ 「有读数 / 无读数」两态可对照。
//
// ## 纪律
//
// - **已接线的导出**：直接**转出**共享的 `mock/real/tauri-bridge.js`（同一份实现，
//   同一份固定数据）⇒ 不重复实现；
// - **未接线的导出**：**显式报错桩**（调用即抛错）⇒ 不静默给假数据；
// - 生产文件**零改动**（拦截发生在 vite 的解析阶段）。

function notWiredValue(name, text) {
  const target = function stub() {};
  return new Proxy(target, {
    get(_target, prop) {
      if (typeof prop === 'symbol' || prop === 'then' || prop === 'toJSON') {
        return undefined;
      }
      throw new Error(text);
    },
    apply() {
      throw new Error(text);
    },
    construct() {
      throw new Error(text);
    },
  });
}

// ── 已接线（真实现；喂固定 fixture） ─────────────────────────────────────
export {
  deleteProviderKey,
  gatewayAuthHeaders,
  getProviderKey,
  getWorkspaceDir,
  listWorkspaceSuggestions,
  setProviderKey,
  setWorkspaceDir,
} from "../../../../mock/real/tauri-bridge.js";

// ── 未接线（显式报错桩） ───────────────────────────────────────────────
export async function hasProviderKey() {
  throw new Error("tauri-bridge 替身：导出「hasProviderKey」未接线（对照 harness 只喂固定 fixture）");
}

export function getGatewayEffectiveConfig() {
  throw new Error("tauri-bridge 替身：导出「getGatewayEffectiveConfig」未接线（对照 harness 只喂固定 fixture）");
}

export function getGatewayToken() {
  throw new Error("tauri-bridge 替身：导出「getGatewayToken」未接线（对照 harness 只喂固定 fixture）");
}

export const UNPROTECTED_TOKEN_SENTINEL = notWiredValue("UNPROTECTED_TOKEN_SENTINEL", "tauri-bridge 替身：导出「UNPROTECTED_TOKEN_SENTINEL」未接线（对照 harness 只喂固定 fixture）");

export function pickGatewayTokenFace() {
  throw new Error("tauri-bridge 替身：导出「pickGatewayTokenFace」未接线（对照 harness 只喂固定 fixture）");
}

export const defaultGatewayTokenSources = notWiredValue("defaultGatewayTokenSources", "tauri-bridge 替身：导出「defaultGatewayTokenSources」未接线（对照 harness 只喂固定 fixture）");

export async function resolveGatewayToken() {
  throw new Error("tauri-bridge 替身：导出「resolveGatewayToken」未接线（对照 harness 只喂固定 fixture）");
}
