// `desktop-bridge` 替身（**已接线**）。
//
// ## 边界
//
// 侧车状态 / 常驻探针 / 允许服务器 / 学习日志都属**宿主读面**
// （设计文档 §1.3 第 7 条）⇒ 替身只返回**固定读数**。
//
// ## ⚠️ `isDesktop()` 的取值
//
// 生产在浏览器里 `isDesktop()` 恒为 `false`（没有 `__TAURI_INTERNALS__`）⇒
// 「学习日志仅桌面版可读」那条分支**永远**出现 ⇒ 对照不到「有日志」的分支。
//
// ⇒ 这里返回 **true**，让参考侧能渲染出学习日志列表。
// ⇒ 要对照「无读数」那一支 ⇒ 把下面 `isDesktop` 改成 `false`（或让
// `readTuningLog` 返回 `null`）。

import data from "../../fixtures/data.json";
import character from "../../fixtures/character.json";

/** 是否桌面宿主（替身返回 true ⇒ 学习日志分支可达）。 */
export function isDesktop() {
  return true;
}

/** 侧车状态读数。 */
export async function getBackendStatus() {
  return data.daemonStatus ? {...data.daemonStatus} : null;
}

/** 常驻服务探针读数。 */
export async function getResidentProbe() {
  return data.residentProbe ? {...data.residentProbe} : null;
}

/** 允许服务器列表。 */
export async function listAllowedServers() {
  return [...(capability.allowedServers ?? [])];
}

/** 加允许服务器（替身只回报结果）。 */
export async function addAllowedServer(base) {
  return [...(capability.allowedServers ?? []), base];
}

/** 移除允许服务器（替身只回报结果）。 */
export async function removeAllowedServer(base) {
  return (capability.allowedServers ?? []).filter((item) => item !== base);
}

/** 学习日志读数（固定 fixture）。 */
export async function readTuningLog() {
  return character.tuningLog.map((entry) => ({...entry}));
}

