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
// ⚠️ 本文件用 `capability.allowedServers`（允许服务器增删查）⇒ **必须** import，
// 否则 `listAllowedServers` / `addAllowedServer` / `removeAllowedServer` 全部
// `ReferenceError: capability is not defined` ⇒ 参考侧「允许服务器」路径必然失败、
// 该列表分支**无法对照**（并行面 M7 实测发现，2026-10-24）。
import capability from "../../fixtures/capabilities.json";

/** 是否桌面宿主（替身返回 true ⇒ 学习日志分支可达）。 */
export function isDesktop() {
  return true;
}

/** 运维读数是否可读（默认 **true**）。
 *
 * ⚠️ **页面全局开关**（2026-10-25 主代理加）：对照脚本在**装载页面之前**
 * 用 CDP 注入 `globalThis.__DATA_READINGS__ = false` ⇒ 替身返回空读数
 * ⇒ 生产落「仅桌面版可读（web 构建无监督面）」那一支 ⇒ **「读不到」态可对照**。
 *
 * ⇒ 读数**取值**仍来自共享固定数据（`fixtures/data.json`）⇒ **不另造数据**
 * ⇒ 参考侧与引擎侧**读的是同一份**（只是「能不能读到」不同）。
 *
 * ⚠️ 此前**只有面内替身**（`web/engine/data/mock/`）有这个开关 ⇒ 参考侧
 * 用的是**共享替身** ⇒ 开关不生效 ⇒ 「读不到」那一态**对照不了**
 * （2026-10-25 主代理重建 `data` 面驱动时实测发现）。
 */
function readingsAvailable() {
  return globalThis.__DATA_READINGS__ !== false;
}

/** 侧车状态读数。 */
export async function getBackendStatus() {
  if (!readingsAvailable()) return null;
  return data.daemonStatus ? {...data.daemonStatus} : null;
}

/** 常驻服务探针读数。 */
export async function getResidentProbe() {
  if (!readingsAvailable()) return null;
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

