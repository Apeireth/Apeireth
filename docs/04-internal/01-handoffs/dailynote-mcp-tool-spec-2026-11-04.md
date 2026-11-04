[Document-Meta]
Document: docs/04-internal/01-handoffs/dailynote-mcp-tool-spec-2026-11-04.md
Class: DECISION
Status: 🟡 待裁决
Last-Modified: 2026-11-04
Author: deputy

# `DailyNote` 4 个 MCP 工具的接入规格 ⇒ **要不要进工具面、怎么进**

> **性质**：接入**规格 + 选项**，**未实施**。需要 owner / 总指挥拍板。
> **背景**：`dailynote::DailyNote` 原被台账记为「A 类真缺口（零消费）」⇒ ⇒
> **11-04 实测发现它「零消费」的原因是它本来就是桩** ⇒ ⇒ **库侧已做真**（MCP 真执行 · 无 tag 也能搜 · 共用一份库）
> ⇒ ⇒ **剩下的唯一问题 = 「要不要把它的 4 个工具暴露给模型」**。

## 一句话结论

**这 4 个工具的库侧已经可用（11-04 实测），缺的只是「暴露面」** ⇒ ⇒
**不是功能债，是产品决策**：日记要不要成为**模型可调用的工具**？

---

## 一、⭐ 一个**必须先说清的技术事实**：它进不了「现有 MCP 那条路」

生产里的 MCP 路径是 **`apeireth_tools_canonical::mcp_bridge`** ⇒ ⇒ 它的语义是
**「客户端」**：连接**外部** MCP 服务器、发现它们的工具（`DiscoveredTool`）、
放进 `McpToolCatalog`、再映射成 `CapabilityId`（`capability_id_for(server, tool)`）。

而 `DailyNoteMcp` 是**服务器端实现**（它自己实现 `initialize` / `tools/list` / `tools/call`）。

⇒ ⇒ **两者角色相反** ⇒ ⇒ **不能直接把 `DailyNoteMcp` 塞进 `mcp_bridge`**（那会把「本地功能」伪装成「外部服务器」）。

⇒ ⇒ **正确接法有两个**（见 §四选项）。

## 二、4 个工具的现状（11-04 改完后的**真实行为**，实测过）

| 工具 | 名字 | 入参 | 真行为 | 出参 |
|---|---|---|---|---|
| 创建 | `note_create` | `title`（必填）· `content`（必填）· `tags`（选填，字符串数组） | **真落库**（`store.insert`）⇒ 回显 `created note <id>` | `created note <id>` |
| 读 | `note_get` | `id`（必填） | **真读回**（`store.get`）⇒ 回显 `note <id> title="…" content="…"` | 标题 + 内容 |
| 搜 | `note_search` | `query`（必填）· `tag`（选填） | **真搜**：有 tag ⇒ 走 tag 索引；**无 tag ⇒ 走全量**（`list_all`）⇒ 命中走 `search_notes`（标题 ×2 / 正文 ×1 / tag ×1 加权） | `search query="…" hits=N` |
| 导出 | `note_export` | `id`（必填）· `format`（`markdown` 默认 / `json`） | **真导出**（`export_markdown` / `export_json`） | Markdown 或 JSON 全文 |

**存储面**：`DailyNoteStore`（SQLite）⇒ `insert` / `get` / `delete` / `count` / `list_all` / `list_by_tag`
⇒ ⚠️ **没有 update**（更新走 `insert` 的 `INSERT OR REPLACE` 语义）。

**实体**：`DailyNote { id, date, title, content, tags[], created_at, updated_at }`

## 三、⚠️ 接进工具面前**必须补的 4 件事**（现状不具备）

| # | 缺什么 | 为什么必须有 | 缺了会怎样 |
|---|---|---|---|
| 1 | **`ToolCapability` 实现** | 工具面的契约是 `id` / `declaration` / `module_contract`（`foundation/plugin/src/tool.rs:44`）⇒ ⇒ 现在的 MCP 响应**不是**这个形状 | 模型看不到工具 / 调度器不认 |
| 2 | **能力 id 命名空间** | `id` 必须是**命名空间**化的稳定身份（如 `tool.dailynote.create`）⇒ ⇒ 4 个工具 = 4 个 id | 与现有工具撞名 / 注册表拒收 |
| 3 | **参数 JSON Schema** | `declaration()` 要给出**模型侧**的参数 schema ⇒ ⇒ 现在的 `args` 是裸 `serde_json::Value` | 模型不知道该传什么 |
| 4 | **权限与写闸** | 其它写类工具（`doc_ops` / `sheet_ops`）都有**写闸 + live 策略槽** | **无写闸的写工具 = 裸奔** ⇒ ⇒ 必须先补 |

⇒ ⇒ **第 4 条是硬门槛**：日记是**写操作**，不能像只读工具那样直接暴露。

## 四、📌 给 owner / 总指挥的**选项**

### 选项 A：**不接**（**保守，我倾向这个**）
⇒ ⇒ 日记保持**库内能力**，不进模型工具面。
⇒ ⇒ 理由：① 日记的**用户价值尚不明确**（模型主动写日记 = 替用户做主）；
② ⇒ 要接就得补**写闸 + 权限**（§三第 4 条）⇒ ⇒ 那是**安全面**（他的区）；
③ ⇒ 库侧已经做真 ⇒ ⇒ **零消费只是「没暴露」，不是「没实现」** ⇒ ⇒ 台账分类该改**而不是**急着接线。
⇒ ⇒ 代价：日记功能**暂时对用户不可见**。

### 选项 B：**接，但**默认关（**推荐**）
⇒ ⇒ 注册 4 个 `ToolCapability` ⇒ ⇒ 由 **env 开关**控制（沿用生物族那套 `APEIRETH_ENABLE_*=1` 才开的 **fail-closed** 惯例）
⇒ ⇒ **默认关 ⇒ 零风险上线** ⇒ ⇒ 想要的人自己开。
⇒ ⇒ 必须补：§三的 4 条（尤其**写闸**）。
⇒ ⇒ 归属：`crates/capabilities/tools`（**工具面 = 产品契约 ⇒ 他的区**）⇒ ⇒ 我只出规格，不动手。

### 选项 C：**接，且默认开**
⇒ ⇒ 直接进生产工具面 ⇒ ⇒ **不建议**：写工具无写闸 ⇒ ⇒ 而且日记的**产品定位**（模型该主动写日记吗）没答案。

### 选项 D：**只接只读的两个**（`note_get` / `note_search` / `note_export`）
⇒ ⇒ 读工具不需要写闸 ⇒ ⇒ 风险低 ⇒ ⇒ 模型可以「查我的日记」但不能写。
⇒ ⇒ **折中方案**：价值可见（模型能引用日记内容）· 风险可控（无写面）。
⇒ ⇒ 代价：只读工具的**权限**也要走治理链（读也要有可见范围）⇒ ⇒ 仍需补 §三的 1/2/3。

## 五、我的建议

**选项 B（默认关）**，但**分两步**：
1. **第一步（本文件）**：**先把台账分类改对** ⇒ ⇒ `DailyNote` 从「A 类真缺口」改成
   「**库侧已做真 · 暴露面待产品决策**」⇒ ⇒ **零代码改动** ⇒ ⇒ 停止「假装它是缺口」；
2. **第二步（等你拍板 B 或 D）**：再补 `ToolCapability` 实现 + 写闸 + schema ⇒ ⇒ **跨区协作**。

⇒ ⇒ **理由**：现在的「零消费」**不是缺陷**，是**缺一个产品决定** ⇒ ⇒
**先把账做实，再决定要不要接线** ⇒ ⇒ 别用「清零 A 类缺口」当目标去倒逼一个没定的产品决策。

## 六、我这次**没有做**什么

| 没做 | 原因 |
|---|---|
| **没写 `ToolCapability` 实现** | 那是 `crates/capabilities/tools` 的区（工具面 = 产品契约）⇒ ⇒ 且**产品定位未拍板** ⇒ ⇒ 写了也可能白写 |
| **没改台账分类** | ⚠️ **他正在改那个文件**（11-04 他提交过）⇒ ⇒ 本轮**只改我自己的那一条**（`AdaptiveBaseline`/`DriftDetector`）⇒ ⇒ `DailyNote` 那条**等本文件被采纳后再改** |
| **没接 MCP bridge** | 它是**客户端**路径，而 `DailyNoteMcp` 是**服务器端** ⇒ ⇒ 角色相反 ⇒ ⇒ 接了是**伪装** |
| **没接记忆召回路径** | 日记是**用户可见产物**，不是召回候选 ⇒ ⇒ 混进召回会污染检索质量 |

## 七、复现命令

```sh
cd /home/bai/Projects/apeireth-debt94-wt
cargo test -p apeireth-memory --lib dailynote    # 11-04 修完后的真执行测试
```

## 相关文件

- 台账（`DailyNote` 条目）：[`docs/status/unwired-ledger.md`](../../status/unwired-ledger.md)
- D2 处置建议书：[`unwired-disposition-proposal-2026-11-04.md`](./unwired-disposition-proposal-2026-11-04.md)
- 正名候选表：[`unwired-canonical-name-candidates-2026-11-04.md`](./unwired-canonical-name-candidates-2026-11-04.md)
- 记忆线状态：[`memory-engine-status-2026-11-03.md`](./memory-engine-status-2026-11-03.md)
