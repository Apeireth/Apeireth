// `user-profile` 替身（**已接线**）。
//
// ## 边界
//
// 本地存储属**宿主**（设计文档 §1.3 第 7 条）⇒ 替身只返回固定值。
// ⚠️ 头像压缩（`downscaleAvatarImage`）在宿主侧 ⇒ 替身给**固定**结果。

import data from "../../fixtures/data.json";

/** 读用户资料。 */
export function loadUserProfile() {
  return {...data.userProfile};
}

/** 存用户资料（替身只回报结果）。 */
export function saveUserProfile(profile) {
  return {...profile};
}

/** 头像压缩（替身不做压缩 ⇒ 原样返回）。 */
export async function downscaleAvatarImage(file) {
  return typeof file === "string" ? file : "";
}

