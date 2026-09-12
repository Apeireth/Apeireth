# 功能旋钮四方对账审计 (Capability-Knob Audit)

> **日期**: 2026-09-12。**口径**: 后端代码为权威 (composition root
> `crates/adapters/cli/src/lib.rs` + governance + tools plugin); 桌面注入层与
> 设置页 UI 为第二方; docs 现行文档 (非 archive, 163 个) 由三路子代理精读
> (01-architecture 47 个 / 03+02 21 个 / 04-internal+其余 72 个) + 主代理精读
> HANDOFF-NOTES 等关键文件。本文只记事实与矛盾; 建议方向见 §4 (均为提案,
> 未实施, 待主人拍板)。

## 1. 四方对账总表

| 能力/旋钮 | 后端实际语义 (代码) | 后端默认 | 桌面旋钮 | 文档口径 | 对账判定 |
|---|---|---|---|---|---|
| `tool.repo` | 恒 grant (build_production_governance_parts) | ✅ 可用 | 无需 | "只读默认开" (4 份文档) | ✅ 一致 |
| `tool.filesystem` / `tool.search` | 默认注册 (BuiltinToolsPlugin) 但治理层**默认 Deny**, 需 `APEIRETH_ENABLE_LOCAL_READ_TOOLS=1` 才 grant | ❌ 拒绝 | ❌ **无旋钮** | "默认注册 3 个只读工具" (user-manual §4, maintenance-guide, HANDOFF-NOTES L89, plugin-authoring-guide) | ❌ **代码违文档意图; 桌面永远用不了** |
| `tool.shell` | 旋钮开 → 注册+grant+每次调用 `require_approval_for` | 关 | ✅ enable_shell | 6 旋钮全集之一, 默认关, 每次审批 (HANDOFF-NOTES L7 等) | ✅ 一致 |
| `tool.fetch` | 旋钮开 → 注册+grant+每次审批; 受控 egress 默认 PublicInternetOnly | 关 | ✅ enable_fetch | 同上 | ✅ 一致 |
| `cognitive.organs` | `APEIRETH_ENABLE_ORGANS=1` → OrganModule (fail-open) | 关 | ✅ enable_organs | 默认关 | ✅ 一致 |
| `cognitive.judge` | `APEIRETH_COGNITIVE_JUDGE=1` | 关 | ✅ cognitive_judge | WIRED OFF by default; judge 默认 ON 是远期路线 (2027-Q1, v2-architecture-reflection:368) | ✅ 一致 (远期方向已注) |
| `cognitive.council` | `APEIRETH_COGNITIVE_COUNCIL=1` (≤7 advisor, 10s/60s) | 关 | ✅ cognitive_council | 同上 | ✅ 一致 |
| `cognitive.preference_learning` | `APEIRETH_ENABLE_PREFERENCE_LEARNING=1` | 关 | ✅ preferenceLearning | 默认关 (2026-09-08 已接线) | ✅ 一致 |
| memory_recall / memory_writeback / self_assessment / Experience extraction | 恒开 (backends 恒 Some, 无旋钮) | ✅ 开 | 无需 | "默认无额外模型成本…走注入 backend" (HANDOFF-NOTES §7.1) | ✅ 一致 |
| 3 治理 hook (Permission/CredentialDisclosure/PromptInjection) | 生产恒挂 | ✅ | 无需 | 生产恒挂 | ✅ 一致 |
| 敏感路径保护 (.env/.ssh/.aws/.gnupg/.secret 屏蔽) | 恒开 (sensitive_path.rs) | ✅ | 无需 | 恒开 | ✅ 一致 |
| 会话级权限预设 read_only/standard/full | 已实现 (PermissionPresetGovernanceHook 接生产管线, commit e7da5809) | standard | ✅ 会话内选择器 | 已实现 | ✅ 一致 |
| 设置页"全局默认预设" | **未接会话创建** (仅 localStorage, settings 代理自报偏差) | — | ✅ 下拉 (无实效) | 无文档 | ⚠️ UI 有旋钮但后端不生效 |
| trust tier 信任分级 | **缺失** (v1 有, v2 PermissionPolicy 未实现, RC-13 排期) | — | ❌ | v2-unabsorbed-features L198-206; system-capabilities:103-107 (Low/Standard/High/Trusted 限流档设计已写) | ⚠️ 文档设计了但未实现 |
| 审批频率限制 / 免审批黑名单 | **缺失** (同上 RC-13) | — | ❌ | v2-unabsorbed-features | ⚠️ 同上 |
| 审批策略档 (每次审批/会话内记住/完全放行) | **无** — shell/fetch 写死每次审批 | 每次审批 | ❌ | DSH 参考 P1 "permission-presets 单一选择器" (frontend-reference-deepseek-harness L104-105) | ⚠️ 参考文档建议了但未实现 |
| 会话创建时快照冻结旋钮 | **无** — 旋钮全局即时生效 | — | ❌ | DSH 参考 P1 "预设于会话创建时冻结" (同 L138) | ⚠️ 同上 |

## 2. 后端到底实现了什么 (组合根事实)

1. **工具层** (BuiltinToolsPlugin): filesystem/search/repo 恒注册; shell/fetch 需显式 options 才注册。
2. **治理层** (PermissionPolicy, build_production_governance_parts):
   - 恒 grant: `tool.repo`
   - `APEIRETH_ENABLE_LOCAL_READ_TOOLS=1` 才 grant: filesystem/search
   - `ENABLE_SHELL/FETCH=1`: grant + require_approval_for (每次调用审批)
   - 未知 capability: default-deny
3. **认知模块** (CognitiveModuleConfig): judge/council/organs/preference_learning 四旋钮默认关; 记忆/自评/经验提炼恒开且"默认无额外模型成本"。
4. **Council 消耗独立 LLM factory** (build_council_from_env): openai-compatible → MiniMax → Noop 回退。
5. **审批生命周期**: TTL 5 分钟, at-most-once, Deny≠RequireApproval (三态)。

## 3. 关键文档矛盾 (最影响判断的)

1. **只读工具口径漂移**: 文档说"默认注册 3 个只读工具"(注册层为真), 但执行许可默认拒绝且桌面无开关 — 用户读文档会以为开箱可用, 实际 filesystem/search 永远 Deny。
2. **MiniMax key 两个变量名**: `APEIRETH_API_KEY` (user-manual §6/api.md/deployment) vs `APEIRETH_MINIMAX_API_KEY` (migration-v1-to-v2/user-manual FAQ §9)。
3. **crate 数 13/16/17 并存** (maintenance-guide 同文件内两值)。
4. **canonical 网关路由口径相反**: core-capabilities (8/30) "仅 5 路由" vs gateway-api-contract (9/4) "全部已实现"。
5. **`cognitive.self_assessment` 默认态**: 一处 "OFF by default" vs 多处 "WIRED Judge-backed"。
6. **L0 是否已硬实现**: v2-architecture-reflection:178 "runtime 还没硬实现" vs cognitive-9:102 "已就位物理隔离"。
7. 其余 20+ 处 (见三路子代理报告全文, 多为 v1 遗留/时间演进, 已在各文件头注部分自标)。

## 4. 建议方向 (提案, 未实施, 待主人拍板)

1. **只读工具默认可用**: filesystem/search 与 repo 同待遇 (默认 grant),
   `APEIRETH_ENABLE_LOCAL_READ_TOOLS` 语义改为"显式关闭只读工具" (CLI 隐私逃生门)。
   依据: 4 份文档口径 + 只读 + 敏感路径保护恒开 + 风险 low/medium。
2. **全局权限预设接入会话创建**: 新会话用设置页选定预设初始化 (补 settings 代理留下的尾巴)。
3. **审批策略预设档**: 每次审批 / 会话内记住 / 完全放行 (完全放行仅在 full 预设下可选),
   落点 SessionSettings + PermissionPresetGovernanceHook (基础设施已就位)。
4. **认知深度预设** (可选, 远期): 轻量(全关)/平衡(judge)/深度(judge+council) 替代两个裸旋钮;
   文档已注 judge 默认 ON 是 2027-Q1 远期路线, 预设档是过渡形态。
5. **文档对账修复**: 上述 §3 矛盾 + 旋钮口径统一 (一次 doc-fix 批次)。
