// `desktop-bridge` 的**对照用替身**（**自动生成** —— 勿手改）。
//
// 生成器：`crates-ui/apeireth-ui-demo/harness/settings/scripts/gen-settings-mock.mjs`
// 来源：`frontend/companion-desktop/src/lib/desktop-bridge.ts`（**只读**）
//
// ## 出口清单（由生产导出面**自动派生** ⇒ 不手抄）
//
// - 生产导出语句：**23** 条（去重后 **23** 个导出名）
// - 运行期导出（值）：**16** 个 ⇒ 其中 **7** 个已接线，**9** 个是显式报错桩；
// - 编译期擦除（type / interface）：**7** 个。
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
  addAllowedServer,
  getBackendStatus,
  getResidentProbe,
  isDesktop,
  listAllowedServers,
  readTuningLog,
  removeAllowedServer,
} from "./real/desktop-bridge.js";

// ── 未接线（显式报错桩） ───────────────────────────────────────────────
export function restartBackend() {
  throw new Error("desktop-bridge 替身：导出「restartBackend」未接线（对照 harness 只喂固定 fixture）");
}

export function backendProviderEnvFromConfig() {
  throw new Error("desktop-bridge 替身：导出「backendProviderEnvFromConfig」未接线（对照 harness 只喂固定 fixture）");
}

export async function applyBackendProviderEnv() {
  throw new Error("desktop-bridge 替身：导出「applyBackendProviderEnv」未接线（对照 harness 只喂固定 fixture）");
}

export function capabilityEnvFromConfig() {
  throw new Error("desktop-bridge 替身：导出「capabilityEnvFromConfig」未接线（对照 harness 只喂固定 fixture）");
}

export async function applyBackendConfig() {
  throw new Error("desktop-bridge 替身：导出「applyBackendConfig」未接线（对照 harness 只喂固定 fixture）");
}

export async function applyBackendConfigOrThrow() {
  throw new Error("desktop-bridge 替身：导出「applyBackendConfigOrThrow」未接线（对照 harness 只喂固定 fixture）");
}

export function getLogDirectory() {
  throw new Error("desktop-bridge 替身：导出「getLogDirectory」未接线（对照 harness 只喂固定 fixture）");
}

export async function openLogDirectory() {
  throw new Error("desktop-bridge 替身：导出「openLogDirectory」未接线（对照 harness 只喂固定 fixture）");
}

export async function resolveBackendEndpoint() {
  throw new Error("desktop-bridge 替身：导出「resolveBackendEndpoint」未接线（对照 harness 只喂固定 fixture）");
}
