// `user-profile` 的**对照用替身**（**自动生成** —— 勿手改）。
//
// 生成器：`crates-ui/apeireth-ui-demo/harness/settings/scripts/gen-settings-mock.mjs`
// 来源：`frontend/companion-desktop/src/lib/user-profile.ts`（**只读**）
//
// ## 出口清单（由生产导出面**自动派生** ⇒ 不手抄）
//
// - 生产导出语句：**9** 条（去重后 **9** 个导出名）
// - 运行期导出（值）：**8** 个 ⇒ 其中 **3** 个已接线，**5** 个是显式报错桩；
// - 编译期擦除（type / interface）：**1** 个。
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
  downscaleAvatarImage,
  loadUserProfile,
  saveUserProfile,
} from "./real/user-profile.js";

// ── 未接线（显式报错桩） ───────────────────────────────────────────────
export const AVATAR_MAX_CHARS = notWiredValue("AVATAR_MAX_CHARS", "user-profile 替身：导出「AVATAR_MAX_CHARS」未接线（对照 harness 只喂固定 fixture）");

export const USER_PROFILE_STORAGE_KEY = notWiredValue("USER_PROFILE_STORAGE_KEY", "user-profile 替身：导出「USER_PROFILE_STORAGE_KEY」未接线（对照 harness 只喂固定 fixture）");

export const EMPTY_USER_PROFILE = notWiredValue("EMPTY_USER_PROFILE", "user-profile 替身：导出「EMPTY_USER_PROFILE」未接线（对照 harness 只喂固定 fixture）");

export function normalizeUserProfile() {,  throw new Error("user-profile 替身：导出「normalizeUserProfile」未接线（对照 harness 只喂固定 fixture）");,}

export function subscribeUserProfile() {,  throw new Error("user-profile 替身：导出「subscribeUserProfile」未接线（对照 harness 只喂固定 fixture）");,}
