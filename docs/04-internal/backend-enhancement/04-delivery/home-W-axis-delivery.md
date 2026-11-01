# W 轴交付册（W2-③ / W3-①②③，2026-10-12/13 批）

> **本册合订 4 篇交付实录**。读法：先看 §0 速览判断是不是你要找的那件；要看细节，点**归属**进原件。
> **现状以 [`docs/status/claims.yaml`](../../../status/claims.yaml) 与 [`test-baseline.md`](../../../03-reference/test-baseline.md) 为准**；
> 本册只记「当时交付了什么、证据在哪」。原件逐字归档在 `docs/archive/merged/2026-10-24/docs/04-internal/backend-enhancement/04-delivery`，每条引文标〔原篇摘录〕，**不改原值**。

## 0. 一句话

多户（星群）数据层：会话复用 → 数据隔离 → 身份角色 → 逐域伙伴实例，四步递进交付

| 篇 | 归属 | 当时状态 |
|---|---|---|
| W2-③ 多客户端会话复用 | `W2-3-multi-client-session-reuse.md` | 已交付（原篇自述，见归属） |
| W3-① 多户数据隔离 | `W3-1-multi-user-data-isolation.md` | 已交付（原篇自述，见归属） |
| W3-② 用户身份与角色 | `W3-2-user-identity-roles.md` | 已交付（原篇自述，见归属） |
| W3-③ 星群正身（逐域伙伴实例） | `W3-3-domain-partner-instances.md` | 已交付（原篇自述，见归属） |

## 1. W2-③ 多客户端会话复用

**一句话**〔原篇摘录〕：

> 两客户端同时对一个会话发消息时，第二个回合**排队**等第一个跑完 —— 不发 busy 帧、


**测试证据**〔原篇摘录，节选〕：

> | 验收项 | 测试（文件） | 例数 |
> |---|---|---|

**归属**：`docs/archive/merged/2026-10-24/docs/04-internal/backend-enhancement/04-delivery/W2-3-multi-client-session-reuse.md`


## 2. W3-① 多户数据隔离

**一句话**〔原篇摘录〕：

> <数据根>/                              ← 默认用户域的数据面（原位，见 §2 采用语义）


**测试证据**〔原篇摘录，节选〕：

> | 验收项 | 测试（文件） | 例数 |
> |---|---|---|

**归属**：`docs/archive/merged/2026-10-24/docs/04-internal/backend-enhancement/04-delivery/W3-1-multi-user-data-isolation.md`


## 3. W3-② 用户身份与角色

**一句话**〔原篇摘录〕：

> **角色词表（冻结）**：`owner` / `member` / `guest` —— 只有这三个字面值，未知角色（含大小写变体）= fail-closed 拒


**测试证据**〔原篇摘录，节选〕：

> | 验收项 | 测试（文件） | 例数 |
> |---|---|---|

**归属**：`docs/archive/merged/2026-10-24/docs/04-internal/backend-enhancement/04-delivery/W3-2-user-identity-roles.md`


## 4. W3-③ 星群正身（逐域伙伴实例）

**一句话**〔原篇摘录〕：

> HTTP 请求


**测试证据**〔原篇摘录，节选〕：

> | 验收项 | 测试（文件） | 例数 |
> |---|---|---|

**归属**：`docs/archive/merged/2026-10-24/docs/04-internal/backend-enhancement/04-delivery/W3-3-domain-partner-instances.md`

## 现状复核（2026-10-24 文档治理批）

| 项 | 现在 |
|---|---|
| 全套件数 | 以 [`docs/03-reference/test-baseline.md`](../../../03-reference/test-baseline.md) 为**单一真源**；本册只记各篇当时实测值 |
| 历史值 | 上文校验数字是各篇**当时实测值**，按「历史不追溯改写」保留原样 |

## 读这一册的边界

- 本册**不是**现状真源；要看某能力现在到哪一级，查 `claims.yaml` 与能力矩阵。
- 各篇的账（会话账 / 域账 / 角色矩阵 / 实例账）**各成一本**，互不替代。
- 要看设计取舍（为什么这么切），回原件；本册只做「按轴收拢 + 速览 + 归属」。

---

**变更历史**：2026-10-24 首版，4 篇合订（原件逐字归档 + 原路径留桩）。
