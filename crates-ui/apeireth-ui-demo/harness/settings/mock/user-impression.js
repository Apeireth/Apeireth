// `user-impression` 的**对照用替身**（**自动生成** —— 勿手改）。
//
// 生成器：`crates-ui/apeireth-ui-demo/harness/settings/scripts/gen-settings-mock.mjs`
// 来源：`frontend/companion-desktop/src/lib/user-impression.ts`（**只读**）
//
// ## 出口清单（由生产导出面**自动派生** ⇒ 不手抄）
//
// - 生产导出语句：**21** 条（去重后 **21** 个导出名）
// - 运行期导出（值）：**19** 个 ⇒ 其中 **6** 个已接线，**13** 个是显式报错桩；
// - 编译期擦除（type / interface）：**2** 个。
//
// ## 纪律
//
// - 已接线的导出喂**固定 fixture**（两侧同源）⇒ 不碰 HTTP / SSE 契约；
// - 其余导出**调用即抛错**（const 是**读取即抛错**）⇒ 不静默给假数据。

/**
 * 未接线的常量值（**读取即抛错** ⇒ 不静默给假数据）。
 *
 * @param {string} name 导出名（报错文案里点名）。
 * @param {string} text 报错文案。
 * @returns {unknown} 读取即抛错的替身。
 */
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
  IMPRESSION_MAX_CHARS,
  clearUserImpression,
  loadUserImpression,
  saveUserImpression,
  summarizeUserImpression,
  transcriptFromConversations,
} from "./real/user-impression.js";

// ── 未接线（显式报错桩） ───────────────────────────────────────────────
export const USER_IMPRESSION_STORAGE_KEY = notWiredValue("USER_IMPRESSION_STORAGE_KEY", "user-impression 替身：导出「USER_IMPRESSION_STORAGE_KEY」未接线（对照 harness 只喂固定 fixture）");

export const TRANSCRIPT_MAX_CHARS = notWiredValue("TRANSCRIPT_MAX_CHARS", "user-impression 替身：导出「TRANSCRIPT_MAX_CHARS」未接线（对照 harness 只喂固定 fixture）");

export const AUTO_REFRESH_USER_MESSAGES = notWiredValue("AUTO_REFRESH_USER_MESSAGES", "user-impression 替身：导出「AUTO_REFRESH_USER_MESSAGES」未接线（对照 harness 只喂固定 fixture）");

export const MIN_USER_MESSAGES_FOR_FIRST_IMPRESSION = notWiredValue("MIN_USER_MESSAGES_FOR_FIRST_IMPRESSION", "user-impression 替身：导出「MIN_USER_MESSAGES_FOR_FIRST_IMPRESSION」未接线（对照 harness 只喂固定 fixture）");

export function emptyUserImpression() {,  throw new Error("user-impression 替身：导出「emptyUserImpression」未接线（对照 harness 只喂固定 fixture）");,}

export function normalizeUserImpression() {,  throw new Error("user-impression 替身：导出「normalizeUserImpression」未接线（对照 harness 只喂固定 fixture）");,}

export function subscribeUserImpression() {,  throw new Error("user-impression 替身：导出「subscribeUserImpression」未接线（对照 harness 只喂固定 fixture）");,}

export function loadUserImpressionLedger() {,  throw new Error("user-impression 替身：导出「loadUserImpressionLedger」未接线（对照 harness 只喂固定 fixture）");,}

export function transcriptForSummary() {,  throw new Error("user-impression 替身：导出「transcriptForSummary」未接线（对照 harness 只喂固定 fixture）");,}

export function buildImpressionPrompt() {,  throw new Error("user-impression 替身：导出「buildImpressionPrompt」未接线（对照 harness 只喂固定 fixture）");,}

export function parseImpressionReply() {,  throw new Error("user-impression 替身：导出「parseImpressionReply」未接线（对照 harness 只喂固定 fixture）");,}

export function impressionSystemBlock() {,  throw new Error("user-impression 替身：导出「impressionSystemBlock」未接线（对照 harness 只喂固定 fixture）");,}

export function shouldRefreshImpression() {,  throw new Error("user-impression 替身：导出「shouldRefreshImpression」未接线（对照 harness 只喂固定 fixture）");,}
