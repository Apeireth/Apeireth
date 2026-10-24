// `tauri-bridge` 的**对照用替身**（**自动生成** —— 勿手改）。
//
// 生成器：`crates-ui/apeireth-ui-demo/harness/settings/scripts/gen-settings-mock.mjs`
// 来源：`frontend/companion-desktop/src/lib/tauri-bridge.ts`（**只读**）
//
// ## 出口清单（由生产导出面**自动派生** ⇒ 不手抄）
//
// - 生产导出语句：**16** 条（去重后 **16** 个导出名）
// - 运行期导出（值）：**14** 个 ⇒ 其中 **7** 个已接线，**7** 个是显式报错桩；
// - 编译期擦除（type / interface）：**2** 个。
//
// ## 纪律
//
// - 已接线的导出喂**固定 fixture**（两侧同源）⇒ 不碰 HTTP / SSE 契约；
// - 其余导出**调用即抛错**（const 是**读取即抛错**）⇒ 不静默给假数据。

/**
 * 未接线的常量值（**读取即抛错** ⇒ 不静默给假数据）。
 *
 * @param {string} name 导出名（报错文案里点名）。
 * @param {string} text 报错文案。
 * @returns {unknown} 读取即抛错的替身。
 */
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
} from "./real/tauri-bridge.js";

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
