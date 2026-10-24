// `bg-store` 替身（**已接线**）。
//
// ## 边界
//
// 背景图本体存**浏览器 IndexedDB**（宿主）⇒ 替身只返回固定值。
// ⚠️ `getCustomBg` 返回 `null` ⇒ 生产走「没有自定义背景」分支（不渲染预览）。

/** 读自定义背景（替身给 null）。 */
export async function getCustomBg() {
  return null;
}

/** 写自定义背景（替身只回报结果）。 */
export async function putCustomBg() {
  return true;
}

/** 清自定义背景（替身只回报结果）。 */
export async function clearCustomBg() {
  return true;
}

/** 校验背景图文件（替身放行）。 */
export function validateCustomBgFile() {
  return {ok: true, reason: ""};
}

