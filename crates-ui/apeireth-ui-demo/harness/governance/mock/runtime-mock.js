// `runtime` 模块的**对照用替身**（**自动生成** —— 勿手改）。
//
// 生成器：`crates-ui/apeireth-ui-demo/harness/governance/scripts/gen-runtime-mock.mjs`
// 来源：`frontend/companion-desktop/src/lib/runtime.ts`（**只读**）
//
// ## 这个文件是什么
//
// 参考侧要**只读引入**生产的治理卷宗面四个 Tab ⇒ 它们在 `onMount` 里拉数据、
// 做能力门控 ⇒ 必须把 `../../runtime` 换成喂固定数据的替身（由 vite 插件在
// `resolveId` 阶段拦截 ⇒ **生产文件零改动**）。
//
// ## 出口清单（由生产导出面**自动派生** ⇒ 不手抄）
//
// - 生产导出语句：**91** 条（去重后 **89** 个导出名）
// - 运行期导出（值）：**67** 个 ⇒ 其中 **13** 个已接线（真实现），
//   **54** 个是显式报错桩；
// - 编译期擦除（type / interface）：**22** 个 ⇒ 运行期不存在。
// - 重载同名导出：listModels（按名字去重）。
//
// ## 纪律
//
// - 已接线的导出喂**固定 fixture**（两侧同源）⇒ 不碰 HTTP / SSE 契约；
// - **变更调用不接线**（`resolveCanonicalApproval` / `revokeGrant` 是桩）
//   ⇒ 对照不覆盖「点签批 / 点撤销」之后的服务端状态；
// - 其余导出**调用即抛错**（const 是**读取即抛错**）⇒ 不静默给假数据。

/**
 * 未接线的常量值（**读取即抛错** ⇒ 不静默给假数据）。
 *
 * @param {string} name 导出名（报错文案里点名）。
 * @returns {unknown} 读取即抛错的替身。
 */
function notWiredValue(name) {
  const text = `runtime mock：导出「${name}」未接线（对照 harness 只喂固定 fixture）`;
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
  capabilityAvailable,
  capabilityUnavailableReason,
  findCapability,
  friendlyErrorMessage,
  fetchBackendSessions,
  fetchCanonicalApprovals,
  fetchGrants,
  fetchTraceList,
  fetchTraceDetail,
  fetchAuditLogs,
  fetchGuardStatus,
  fetchGuardEvents,
  evaluateGuard,
} from "./runtime-mock-real.js";

// ── 未接线（显式报错桩） ───────────────────────────────────────────────
export const DEFAULT_PERSONA_TEXT = notWiredValue("DEFAULT_PERSONA_TEXT");

export const DEFAULT_PERSONAS = notWiredValue("DEFAULT_PERSONAS");

export function activePersonaOf() {
  throw new Error("runtime mock：导出「activePersonaOf」未接线（对照 harness 只喂固定 fixture）");
}

export class ApprovalRequiredError extends Error {
  constructor(message) {
    super("runtime mock：导出「ApprovalRequiredError」未接线（对照 harness 只喂固定 fixture）");
    this.name = "ApprovalRequiredError";
  }
}

export function classifyHttpError() {
  throw new Error("runtime mock：导出「classifyHttpError」未接线（对照 harness 只喂固定 fixture）");
}

export class HttpError extends Error {
  constructor(message) {
    super("runtime mock：导出「HttpError」未接线（对照 harness 只喂固定 fixture）");
    this.name = "HttpError";
  }
}

export const ERROR_SOLUTIONS = notWiredValue("ERROR_SOLUTIONS");

export function describeError() {
  throw new Error("runtime mock：导出「describeError」未接线（对照 harness 只喂固定 fixture）");
}

export function describeCaught() {
  throw new Error("runtime mock：导出「describeCaught」未接线（对照 harness 只喂固定 fixture）");
}

export function toRuntimeError() {
  throw new Error("runtime mock：导出「toRuntimeError」未接线（对照 harness 只喂固定 fixture）");
}

export function loadConfig() {
  throw new Error("runtime mock：导出「loadConfig」未接线（对照 harness 只喂固定 fixture）");
}

export function saveConfig() {
  throw new Error("runtime mock：导出「saveConfig」未接线（对照 harness 只喂固定 fixture）");
}

export async function testProviderConnection() {
  throw new Error("runtime mock：导出「testProviderConnection」未接线（对照 harness 只喂固定 fixture）");
}

export function normalizeBaseUrl() {
  throw new Error("runtime mock：导出「normalizeBaseUrl」未接线（对照 harness 只喂固定 fixture）");
}

export async function checkHealth() {
  throw new Error("runtime mock：导出「checkHealth」未接线（对照 harness 只喂固定 fixture）");
}

export async function checkHealthDetailed() {
  throw new Error("runtime mock：导出「checkHealthDetailed」未接线（对照 harness 只喂固定 fixture）");
}

export async function listModels() {
  throw new Error("runtime mock：导出「listModels」未接线（对照 harness 只喂固定 fixture）");
}

export async function applyAdminConfig() {
  throw new Error("runtime mock：导出「applyAdminConfig」未接线（对照 harness 只喂固定 fixture）");
}

export async function getAdminConfig() {
  throw new Error("runtime mock：导出「getAdminConfig」未接线（对照 harness 只喂固定 fixture）");
}

export async function getSessionSettings() {
  throw new Error("runtime mock：导出「getSessionSettings」未接线（对照 harness 只喂固定 fixture）");
}

export async function patchSessionSettings() {
  throw new Error("runtime mock：导出「patchSessionSettings」未接线（对照 harness 只喂固定 fixture）");
}

export function applyCanonicalEvents() {
  throw new Error("runtime mock：导出「applyCanonicalEvents」未接线（对照 harness 只喂固定 fixture）");
}

export async function streamChat() {
  throw new Error("runtime mock：导出「streamChat」未接线（对照 harness 只喂固定 fixture）");
}

export async function chatOnce() {
  throw new Error("runtime mock：导出「chatOnce」未接线（对照 harness 只喂固定 fixture）");
}

export function runtimeStatus() {
  throw new Error("runtime mock：导出「runtimeStatus」未接线（对照 harness 只喂固定 fixture）");
}

export function createAgentRuntime() {
  throw new Error("runtime mock：导出「createAgentRuntime」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchCapabilities() {
  throw new Error("runtime mock：导出「fetchCapabilities」未接线（对照 harness 只喂固定 fixture）");
}

export function capabilitySupported() {
  throw new Error("runtime mock：导出「capabilitySupported」未接线（对照 harness 只喂固定 fixture）");
}

export function requireCapability() {
  throw new Error("runtime mock：导出「requireCapability」未接线（对照 harness 只喂固定 fixture）");
}

export function releaseContractManifest() {
  throw new Error("runtime mock：导出「releaseContractManifest」未接线（对照 harness 只喂固定 fixture）");
}

export const legacyCapabilityManifest = notWiredValue("legacyCapabilityManifest");

export async function deleteBackendSession() {
  throw new Error("runtime mock：导出「deleteBackendSession」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchMemoryEpisodes() {
  throw new Error("runtime mock：导出「fetchMemoryEpisodes」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchGraphData() {
  throw new Error("runtime mock：导出「fetchGraphData」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchRuntimeSnapshot() {
  throw new Error("runtime mock：导出「fetchRuntimeSnapshot」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchTools() {
  throw new Error("runtime mock：导出「fetchTools」未接线（对照 harness 只喂固定 fixture）");
}

export async function resolveCanonicalApproval() {
  throw new Error("runtime mock：导出「resolveCanonicalApproval」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchApprovalRequests() {
  throw new Error("runtime mock：导出「fetchApprovalRequests」未接线（对照 harness 只喂固定 fixture）");
}

export async function grantToolPermission() {
  throw new Error("runtime mock：导出「grantToolPermission」未接线（对照 harness 只喂固定 fixture）");
}

export async function appendMemoryEpisode() {
  throw new Error("runtime mock：导出「appendMemoryEpisode」未接线（对照 harness 只喂固定 fixture）");
}

export function loadConversations() {
  throw new Error("runtime mock：导出「loadConversations」未接线（对照 harness 只喂固定 fixture）");
}

export function saveConversations() {
  throw new Error("runtime mock：导出「saveConversations」未接线（对照 harness 只喂固定 fixture）");
}

export function subscribeCompanionEvents() {
  throw new Error("runtime mock：导出「subscribeCompanionEvents」未接线（对照 harness 只喂固定 fixture）");
}

export async function forgetMemoryEpisode() {
  throw new Error("runtime mock：导出「forgetMemoryEpisode」未接线（对照 harness 只喂固定 fixture）");
}

export async function protectMemoryEpisode() {
  throw new Error("runtime mock：导出「protectMemoryEpisode」未接线（对照 harness 只喂固定 fixture）");
}

export async function unprotectMemoryEpisode() {
  throw new Error("runtime mock：导出「unprotectMemoryEpisode」未接线（对照 harness 只喂固定 fixture）");
}

export async function revokeGrant() {
  throw new Error("runtime mock：导出「revokeGrant」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchWorkbenchTurn() {
  throw new Error("runtime mock：导出「fetchWorkbenchTurn」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchUsers() {
  throw new Error("runtime mock：导出「fetchUsers」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchDomains() {
  throw new Error("runtime mock：导出「fetchDomains」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchSkills() {
  throw new Error("runtime mock：导出「fetchSkills」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchProposalLedger() {
  throw new Error("runtime mock：导出「fetchProposalLedger」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchCare() {
  throw new Error("runtime mock：导出「fetchCare」未接线（对照 harness 只喂固定 fixture）");
}

export async function fetchTraits() {
  throw new Error("runtime mock：导出「fetchTraits」未接线（对照 harness 只喂固定 fixture）");
}
