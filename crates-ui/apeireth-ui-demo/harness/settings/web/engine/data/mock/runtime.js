// `runtime` 替身（**面内派生** —— 由共享的生成物派生 ⇒ 见文件头的说明）。
//
// ## 为什么有「面内替身」
//
// 共享的 `mock/runtime.js` 是**生成物**（`scripts/gen-settings-mock.mjs`），
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
// - **已接线的导出**：直接**转出**共享的 `mock/real/runtime.js`（同一份实现，
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
  DEFAULT_PERSONAS,
  HttpError,
  applyAdminConfig,
  chatOnce,
  checkHealthDetailed,
  describeCaught,
  describeError,
  getAdminConfig,
  listModels,
  loadConversations,
  normalizeBaseUrl,
  testProviderConnection,
} from "../../../../mock/real/runtime.js";

// ── 未接线（显式报错桩） ───────────────────────────────────────────────
export const DEFAULT_PERSONA_TEXT = notWiredValue("DEFAULT_PERSONA_TEXT", "runtime 替身：导出「DEFAULT_PERSONA_TEXT」未接线（对照 harness 只喂固定 fixture）");

export function activePersonaOf() {
  throw new Error("runtime 替身：导出「activePersonaOf」未接线（对照 harness 只喂固定 fixture）");
}

export class ApprovalRequiredError extends Error {
  constructor() {
    super("runtime 替身：导出「ApprovalRequiredError」未接线（对照 harness 只喂固定 fixture）");
    this.name = "ApprovalRequiredError";
  }
}

export function classifyHttpError() {
  throw new Error("runtime 替身：导出「classifyHttpError」未接线（对照 harness 只喂固定 fixture）");
}

export const ERROR_SOLUTIONS = notWiredValue("ERROR_SOLUTIONS", "runtime 替身：导出「ERROR_SOLUTIONS」未接线（对照 harness 只喂固定 fixture）");

export function friendlyErrorMessage() {
  throw new Error("runtime 替身：导出「friendlyErrorMessage」未接线（对照 harness 只喂固定 fixture）");
}

export function toRuntimeError() {
  throw new Error("runtime 替身：导出「toRuntimeError」未接线（对照 harness 只喂固定 fixture）");
}

export function loadConfig() {
  throw new Error("runtime 替身：导出「loadConfig」未接线（对照 harness 只喂固定 fixture）");
}

export function saveConfig() {
  throw new Error("runtime 替身：导出「saveConfig」未接线（对照 harness 只喂固定 fixture）");
}

export async function checkHealth() {
  throw new Error("runtime 替身：导出「checkHealth」未接线（对照 harness 只喂固定 fixture）");
}

export async function getSessionSettings() {
  throw new Error("runtime 替身：导出「getSessionSettings」未接线（对照 harness 只喂固定 fixture）");
}

export async function patchSessionSettings() {
  throw new Error("runtime 替身：导出「patchSessionSettings」未接线（对照 harness 只喂固定 fixture）");
}

export function applyCanonicalEvents() {
  throw new Error("runtime 替身：导出「applyCanonicalEvents」未接线（对照 harness 只喂固定 fixture）");
}

export async function streamChat() {
  throw new Error("runtime 替身：导出「streamChat」未接线（对照 harness 只喂固定 fixture）");
}

export function runtimeStatus() {
  throw new Error("runtime 替身：导出「runtimeStatus」未接线（对照 harness 只喂固定 fixture）");
}

export function createAgentRuntime() {
  throw new Error("runtime 替身：导出「createAgentRuntime」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchCapabilities() {
  throw new Error("runtime 替身：导出「fetchCapabilities」未接线（对照 harness 只喂固定 fixture）");
}

export function capabilitySupported() {
  throw new Error("runtime 替身：导出「capabilitySupported」未接线（对照 harness 只喂固定 fixture）");
}

export function requireCapability() {
  throw new Error("runtime 替身：导出「requireCapability」未接线（对照 harness 只喂固定 fixture）");
}

export function capabilityAvailable() {
  throw new Error("runtime 替身：导出「capabilityAvailable」未接线（对照 harness 只喂固定 fixture）");
}

export function capabilityUnavailableReason() {
  throw new Error("runtime 替身：导出「capabilityUnavailableReason」未接线（对照 harness 只喂固定 fixture）");
}

export function findCapability() {
  throw new Error("runtime 替身：导出「findCapability」未接线（对照 harness 只喂固定 fixture）");
}

export function releaseContractManifest() {
  throw new Error("runtime 替身：导出「releaseContractManifest」未接线（对照 harness 只喂固定 fixture）");
}

export const legacyCapabilityManifest = notWiredValue("legacyCapabilityManifest", "runtime 替身：导出「legacyCapabilityManifest」未接线（对照 harness 只喂固定 fixture）");

export async function fetchBackendSessions() {
  throw new Error("runtime 替身：导出「fetchBackendSessions」未接线（对照 harness 只喂固定 fixture）");
}

export async function deleteBackendSession() {
  throw new Error("runtime 替身：导出「deleteBackendSession」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchMemoryEpisodes() {
  throw new Error("runtime 替身：导出「fetchMemoryEpisodes」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchGraphData() {
  throw new Error("runtime 替身：导出「fetchGraphData」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchRuntimeSnapshot() {
  throw new Error("runtime 替身：导出「fetchRuntimeSnapshot」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchAuditLogs() {
  throw new Error("runtime 替身：导出「fetchAuditLogs」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchTools() {
  throw new Error("runtime 替身：导出「fetchTools」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchCanonicalApprovals() {
  throw new Error("runtime 替身：导出「fetchCanonicalApprovals」未接线（对照 harness 只喂固定 fixture）");
}

export async function resolveCanonicalApproval() {
  throw new Error("runtime 替身：导出「resolveCanonicalApproval」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchApprovalRequests() {
  throw new Error("runtime 替身：导出「fetchApprovalRequests」未接线（对照 harness 只喂固定 fixture）");
}

export async function grantToolPermission() {
  throw new Error("runtime 替身：导出「grantToolPermission」未接线（对照 harness 只喂固定 fixture）");
}

export async function appendMemoryEpisode() {
  throw new Error("runtime 替身：导出「appendMemoryEpisode」未接线（对照 harness 只喂固定 fixture）");
}

export function saveConversations() {
  throw new Error("runtime 替身：导出「saveConversations」未接线（对照 harness 只喂固定 fixture）");
}

export function subscribeCompanionEvents() {
  throw new Error("runtime 替身：导出「subscribeCompanionEvents」未接线（对照 harness 只喂固定 fixture）");
}

export async function forgetMemoryEpisode() {
  throw new Error("runtime 替身：导出「forgetMemoryEpisode」未接线（对照 harness 只喂固定 fixture）");
}

export async function protectMemoryEpisode() {
  throw new Error("runtime 替身：导出「protectMemoryEpisode」未接线（对照 harness 只喂固定 fixture）");
}

export async function unprotectMemoryEpisode() {
  throw new Error("runtime 替身：导出「unprotectMemoryEpisode」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchGrants() {
  throw new Error("runtime 替身：导出「fetchGrants」未接线（对照 harness 只喂固定 fixture）");
}

export async function revokeGrant() {
  throw new Error("runtime 替身：导出「revokeGrant」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchTraceList() {
  throw new Error("runtime 替身：导出「fetchTraceList」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchTraceDetail() {
  throw new Error("runtime 替身：导出「fetchTraceDetail」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchGuardStatus() {
  throw new Error("runtime 替身：导出「fetchGuardStatus」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchGuardEvents() {
  throw new Error("runtime 替身：导出「fetchGuardEvents」未接线（对照 harness 只喂固定 fixture）");
}

export async function evaluateGuard() {
  throw new Error("runtime 替身：导出「evaluateGuard」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchWorkbenchTurn() {
  throw new Error("runtime 替身：导出「fetchWorkbenchTurn」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchUsers() {
  throw new Error("runtime 替身：导出「fetchUsers」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchDomains() {
  throw new Error("runtime 替身：导出「fetchDomains」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchSkills() {
  throw new Error("runtime 替身：导出「fetchSkills」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchProposalLedger() {
  throw new Error("runtime 替身：导出「fetchProposalLedger」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchCare() {
  throw new Error("runtime 替身：导出「fetchCare」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchTraits() {
  throw new Error("runtime 替身：导出「fetchTraits」未接线（对照 harness 只喂固定 fixture）");
}
