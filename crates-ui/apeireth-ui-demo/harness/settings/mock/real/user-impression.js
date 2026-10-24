// `user-impression` 替身（**已接线**）。
//
// ## 边界
//
// 印象账本落**本地存储**（宿主）⇒ 替身只返回固定值。

import character from "../../fixtures/character.json";

/** 印象正文的字数上限（与生产同口径）。 */
export const IMPRESSION_MAX_CHARS = 4000;

/** 读印象。 */
export function loadUserImpression() {
  return {...character.impression};
}

/** 存印象（替身只回报结果）。 */
export function saveUserImpression(entry) {
  return {...entry};
}

/** 清空印象正文（替身只回报结果）。 */
export function clearUserImpression(entry) {
  return {...entry, summary: ""};
}

/** 从会话账本拼转录（替身给空 ⇒ 「立即总结」走「还没有可总结的对话」分支）。 */
export function transcriptFromConversations() {
  return "";
}

/** 立即总结（替身不发请求 ⇒ 返回固定回执）。 */
export async function summarizeUserImpression() {
  return {...character.impression};
}

