// 设置页对照用替身：**真实现**（喂固定 fixture）。
//
// ## 边界
//
// - **只喂固定 fixture** ⇒ 不碰 HTTP / SSE 契约（设计文档 §1.3 第 6 条）、
//   不碰宿主能力（第 7 条：钥匙串 / 工作区 / 允许服务器 / 运维读数）；
// - 未在下面列出的导出 ⇒ **显式报错桩**（见 `settings-mock-stub.js`）⇒
//   **不静默给假数据**；
// - 本文件**只读**生产文件（拿导出面），**不改**生产任何文件。
//
// ## 口径提醒
//
// - `isDesktop()` 在替身里返回 **true** ⇒ 生产代码里
//   「学习日志仅桌面版可读」那条分支**不会**出现 ⇒ 能对照到「有日志」的分支。
//   ⚠️ ���对照「无读数」那一支时，把 `desktop-bridge.js` 里的
//   `isDesktop` 改成 false 即可（见该文件里的说明）。
// - 密钥相关导出**只返回状态位与打码串**，绝不返回明文。

import provider from "../../fixtures/provider.json";
import budget from "../../fixtures/budget.json";
import data from "../../fixtures/data.json";
import character from "../../fixtures/character.json";
import capability from "../../fixtures/capabilities.json";

/** 归一化的端点（与生产 `normalizeBaseUrl` 同口径：去尾斜杠）。 */
export function normalizeBaseUrl(raw) {
  return String(raw ?? "").trim().replace(/\/+$/, "");
}

/** 错误描述（生产 `describeError` 的最小替身）。 */
export function describeError(err) {
  const message = err instanceof Error ? err.message : String(err);
  return {message, solution: "检查配置与网络后重试。"};
}

/** 捕获到的错误（生产 `describeCaught` 的最小替身）。 */
export function describeCaught(err) {
  return err instanceof Error ? err.message : String(err);
}

/** HTTP 错误类型（生产 `HttpError` 的最小替身）。 */
export class HttpError extends Error {
  constructor(message, code, status) {
    super(message);
    this.name = "HttpError";
    this.code = code;
    this.status = status;
  }
}

/** 默认人设（生产 `DEFAULT_PERSONAS` 的替身 ⇒ 喂 fixture 的人设列表）。 */
export const DEFAULT_PERSONAS = character.personas.map((persona) => ({...persona}));

/** 拉模型列表（固定 fixture ⇒ 每次返回新副本）。 */
export async function listModels() {
  return provider.discoveredModels.map((model) => ({...model}));
}

/** 测连接（固定结果 ⇒ 不发请求）。 */
export async function testProviderConnection() {
  return {...provider.testResult, models: [...(provider.testResult.models ?? [])]};
}

/** 网关热应用（固定回执）。 */
export async function applyAdminConfig() {
  return {...provider.applyResult, warnings: [...(provider.applyResult.warnings ?? [])]};
}

/** 网关生效配置回显（固定回执）。 */
export async function getAdminConfig() {
  return {...provider.effectiveConfig};
}

/** 深度诊断（固定读数）。 */
export async function checkHealthDetailed() {
  return {
    overall: budget.runtimeReport.overall,
    latencyMs: budget.runtimeReport.latencyMs,
    subsystems: budget.runtimeReport.subsystems.map((item) => ({...item})),
  };
}

/** 单次对话（固定回执 ⇒ 不发请求）。 */
export async function chatOnce() {
  return "固定替身回执：对照 harness 不发真实请求。";
}

/** 读会话账本（固定为空 ⇒ 「立即总结」走「还没有可总结的对话」分支）。 */
export async function loadConversations() {
  return [];
}
