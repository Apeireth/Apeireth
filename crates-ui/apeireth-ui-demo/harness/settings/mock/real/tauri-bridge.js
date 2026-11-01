// `tauri-bridge` 替身（**已接线**）。
//
// ## 边界
//
// 钥匙串与工作区属**宿主能力**（设计文档 §1.3 第 7 条）⇒ 替身只返回
// **状态位 / 打码串 / 固定读数**，绝不返回明文密钥，也绝不落盘。
//
// ⚠️ 未列出的导出由 `settings-mock-stub.js` 补齐（**显式报错桩**）。

import provider from "../../fixtures/provider.json";
import data from "../../fixtures/data.json";

/** 读钥匙串（**只回打码值**）。 */
export async function getProviderKey() {
  return provider.keychain.storedKeyExists ? provider.keychain.storedKeyMasked : null;
}

/** 写钥匙串（替身只回报结果，不落盘）。 */
export async function setProviderKey() {
  return true;
}

/** 删钥匙串（替身只回报结果）。 */
export async function deleteProviderKey() {
  return true;
}

/** 读工作区目录。 */
export async function getWorkspaceDir() {
  return data.workspace.dir;
}

/** 写工作区目录。 */
export async function setWorkspaceDir(dir) {
  return dir ?? null;
}

/** 工作区候选项。 */
export async function listWorkspaceSuggestions() {
  return [...data.workspace.suggestions];
}

/** 网关鉴权头（替身给固定值）。 */
export function gatewayAuthHeaders() {
  return {"x-apeireth-auth": "fixed-mock-token"};
}

