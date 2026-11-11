// `runtime` 替身的**已接线部分**（真实现）。
//
// ## 边界
//
// - 只有**本批真正调用到**的导出在这里给真实现；其余导出是生成脚本产出的
//   **显式报错桩**（见 `runtime-mock.js` 的生成头）。
// - 真实现**只喂固定 fixture** ⇒ 不碰 HTTP / SSE 契约（设计文档 §1.3 第 6 条）。
// - 授权 / 审批两条数据源本批**不喂**（属治理卷宗面）⇒ 返回空数组 ⇒ 生产组件
//   的对应区块不渲染 ⇒ 参考侧画面只剩工具卡面。
//
// ## 能力门控口径
//
// `findCapability` / `capabilitySupported` / `capabilityAvailable` 三个函数
// 按生产 `runtime.ts` 的口径实现（静态 `supported` + 动态 `available` + 别名
// 回落）⇒ 参考侧的门控行为与生产一致。

import fixture from "../fixtures/toolgrid-cases.json";

/** 能力别名表（与生产同口径）。 */
const CAPABILITY_ALIASES = {
  "approvals.read": "permissions.approval.read",
  "approvals.resolve": "permissions.approval.resolve",
};

/** 取规范 id。 */
function canonicalCapabilityId(id) {
  return CAPABILITY_ALIASES[id] ?? id;
}

/** 跨组查找能力声明。 */
export function findCapability(manifest, id) {
  if (!manifest) return null;
  const want = canonicalCapabilityId(id);
  let aliasMatch = null;
  for (const group of manifest.capabilities) {
    for (const cap of group.capabilities) {
      if (cap.id === want || cap.id === id) return cap;
      if (cap.alias_of === want || cap.alias_of === id) aliasMatch = cap;
    }
  }
  return aliasMatch;
}

/** 运行时是否**实现**了该能力（静态语义）。 */
export function capabilitySupported(manifest, id) {
  const cap = findCapability(manifest, id);
  return cap?.supported === true;
}

/** 该能力**此时此刻**是否可调用（动态语义；旧清单无 `available` ⇒ 回落 `supported`）。 */
export function capabilityAvailable(manifest, id) {
  if (!manifest) return false;
  const cap = findCapability(manifest, id);
  if (!cap) return false;
  return cap.supported === true && (cap.available === undefined || cap.available === true);
}

/** 工具清单（固定 fixture ⇒ 每次返回新副本）。 */
export async function fetchTools() {
  if (typeof window !== "undefined") {
    // 数据已到 ⇒ 对照脚本可以开始抓画面（两侧同名同义的 `ready()` 判据）。
    window.__TOOLGRID_READY__ = true;
  }
  return fixture.tools.map((tool) => ({ ...tool }));
}

/** 活跃授权（本批不喂 ⇒ 空）。 */
export async function fetchGrants() {
  return [];
}

/** 待审批请求（本批不喂 ⇒ 空）。 */
export async function fetchApprovalRequests() {
  return [];
}
