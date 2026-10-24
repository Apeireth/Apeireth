// `desktop-bridge` 替身（**面内派生** —— 由共享的生成物派生 ⇒ 见文件头的说明）。
//
// ## 为什么有「面内替身」
//
// 共享的 `mock/desktop-bridge.js` 是**生成物**（`scripts/gen-settings-mock.mjs`），
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
// - **已接线的导出**：直接**转出**共享的 `mock/real/desktop-bridge.js`（同一份实现，
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
  addAllowedServer,
  isDesktop,
  listAllowedServers,
  readTuningLog,
  removeAllowedServer,
} from "../../../../mock/real/desktop-bridge.js";

// ── 运维读数（**面内加一个开关** ⇒ 对照「有读数 / 读不到」两态） ─────────
//
// ## 为什么面内要加这个开关
//
// 生产的读数映射是：**读数为空 ⇒ 4 行占位 + 一条说明**（「读不到」分支）。
// 而共享固定数据给的是**有读数**的一份 ⇒ 要对照「读不到」那一支，**必须**能
// 让替身返回空读数。
//
// ⇒ 这里读**页面全局** `globalThis.__DATA_READINGS__`（默认 **true** ⇒ 有读数）：
// 对照脚本在**装载页面之前**用 CDP 注入该全局 ⇒ 两种取值都能对照 ⇒
// **两侧读的是同一份固定数据**（不是两份不同的数据）。
//
// ⚠️ 读数**取值**仍来自共享固定数据（`fixtures/data.json`）⇒ 不另造数据。
import data from "../../../../fixtures/data.json";

/** 运维读数是否可读（默认 **true**）。 */
function readingsAvailable() {
  return globalThis.__DATA_READINGS__ !== false;
}

/** 侧车状态读数（读不到 ⇒ `null` ⇒ 生产落「仅桌面版可读」分支）。 */
export async function getBackendStatus() {
  if (!readingsAvailable()) return null;
  return data.daemonStatus ? { ...data.daemonStatus } : null;
}

/** 常驻服务探针读数（读不到 ⇒ `null`）。 */
export async function getResidentProbe() {
  if (!readingsAvailable()) return null;
  return data.residentProbe ? { ...data.residentProbe } : null;
}

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
