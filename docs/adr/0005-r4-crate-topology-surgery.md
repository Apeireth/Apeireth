# 0005. R4 crate 拓扑手术——图像家族门禁隔离 + SDK/perception 裁决

- **Status**: Accepted（R4a 与 **R4b 均** owner 2026-10-20「拍板的话按你建议来吧」授权；src 移动按协作契约派实施线执行，架构线出迁移清单/验收）
- **Date**: 2026-10-20
- **Owners**: 架构线（主导/验收）· 各实施线（CI/打包配合）
- **Supersedes**: —
- **契约影响**: `contract.toml` 增加 `[vendored]` 图像家族 11 包；`arch_guard` 对 vendored 包按"非 workspace 节点"处理；依赖基线缩表
- **例外关联**: 迁移期 `tools→image-core` 边标注（从 workspace 边转 vendored 边）
- **预算净额**: 0 源码语义变化；门禁命令与口径更新

## 1. 背景

图像家族 11 crate = 353 文件 / 126,103 行 = 全仓 .rs 的 22.3%；`tools-canonical` 直接依赖 `image-core`。主门禁 `--workspace` 每次都要编译+测试它们，是"全量 >18 分钟"的最大单项来源。它们本质是 vendored 内核（许可搬运线），与"数字伙伴"的业务演进节奏不同步。

## 2. R4a 决策（图像家族）

### R4a-1（Wave 2，立即，可逆）：门禁隔离

- 新增 `scripts/gate_full.ps1`：全量门禁 = 主 workspace（`--workspace` + 显式 `--exclude` 11 个图像包）+ 图像家族独立门禁（`cargo test --manifest-path image/...` 或独立 workspace 命令）；
- 新增 `.github/workflows/image-family.yml`：图像家族独立 CI（仅图像路径变更/夜间触发）；
- `test-baseline.md` 增两行口径：**主 workspace 口径（排除图像家族）** + **图像家族独立口径**；历史行保留不追溯；
- 主 workspace `--exclude` 清单由 `contract.toml [vendored].image_family` 唯一维护，脚本从契约读取（禁止手抄第二份）。

### R4a-2（条件触发，不晚于 Wave 4）：独立 workspace 抽取

- 前置条件：R4a-1 运行 ≥1 波无回归；打包/许可/NOTICE 路径核查通过；
- 动作：图像家族迁入独立 workspace（与 `frontend/` 并列），根 workspace 31→20 members；`tools` 以 path 依赖继续使用 `image-core`；
- 由架构线主笔 ADR 修订 + 执行，迁移清单与许可原件核对同步。

### R4a 影响的契约/守卫改动

- `contract.toml [vendored]` 登记 11 包；`arch_guard` 规则：vendored 包不作为 workspace 节点参与层方向判定（允许 `capabilities→vendored`；其它层引用需 ADR）；
- 依赖基线：图像内部 28 条 workspace 边从主基线移除（缩表），`tools→image-core` 由 disputed edge 转为 vendored edge 登记。

## 3. R4b 裁决（owner 2026-10-20「拍板的话按你建议来吧」）

**取证（2026-10-20 实测）**：
- SDK 四孤岛（lark 6,741 / livekit 6,517 / voice 6,170 / sandbox 4,221 = 23,649 行）：`crates/` 内 **sdk 之外零引用**；仅 sdk 自身 examples/tests 有 7 处 lark；gateway 无 `/v1/tools/*` 路由（契约断头，founding-design-v2 §10 自述）。
- perception（约 6,300 行）：全仓 Cargo.toml **无任何 crate 依赖它**（仅自身声明）——无生产消费者，属孤岛。

**裁定**：
| 项 | 裁定 | 依据 | 执行 |
|---|---|---|---|
| SDK 四孤岛 | **A 档（本批）**：`livekit`/`voice`/`sandbox` 三岛归档（16,908 行）；**lark 暂缓**（见下表） | 三岛 sdk 外零引用；lark 被保留面 `sdk::im` 内部依赖（`im/message.rs:81,87,135`、`im/channel.rs:59`、`im/card.rs:197` test、`lib.rs:288` feature 门），gateway 经 `apeireth_sdk::im` 依赖 | 实施线按 §3.1（本批只 3 岛） |
| **lark**（6,741 行） | **暂缓归档**（另立解耦任务，Wave 3 内排期） | 取证修正：sdk 外零引用属实，但 **sdk 内部 `im → lark` 真实边**（wire 原语 WebhookEvent/EventType/TextContent/CardContent/MAX_MESSAGE_TEXT_BYTES/错误映射）；直接搬走会弄断 gateway 的 `apeireth_sdk::im` | 先解耦（抽 wire 原语到 `im` 自有模块 ~250-350 行、行为保持）再归档；解耦属设计归属变更，需单独 commit + 验收 |
| perception 整包 | **归档**（迁 `legacy/archived/perception/`；主 workspace 移除该成员；恢复 = revert/从 legacy 迁回） | 零依赖者 = 不可达代码 | 同上 |

### 3.1 迁移清单（实施线执行，架构线验收）

1. SDK（**本批 A 档 = 3 岛**）：`git mv crates/adapters/sdk/src/{livekit,voice,sandbox}` → `legacy/archived/sdk-islands/`；删 `lib.rs` 对应 3 个 mod 声明与失效自检；保留 `pub mod lark`、`lark` feature 与 `im` 的 cfg 门原样；清三岛专属 feature/optional deps；example 只移除三岛 demo（lark demo 保留）；补 `legacy/archived/sdk-islands/MANIFEST.md`（原路径/行数/恢复方式/许可 + **lark 暂缓原因与证据**）。**lark 归档另立任务**：先抽 wire 原语到 `im` 自有模块（行为保持），再按同一清单归档；解耦为独立 commit + 独立验收。
2. perception：`git mv crates/engine/perception legacy/archived/perception`；根 `Cargo.toml` members 移除；`Cargo.lock` 随 `cargo check` 更新；能力矩阵/claims 中 perception 相关行按四级口径标注"已归档（2026-10-20，ADR-0005 R4b）"。
3. 文案：`founding-design-v2 §10` 决策项标"已裁决：归档"；能力矩阵/README honest gaps 如涉及同步。
4. 验收：`cargo metadata` members 31→30（图像家族仍在，R4a-2 另计）；`cargo check -p apeireth-sdk -p apeireth-gateway -p apeireth-cli` 通过；`arch_guard --strict` 无新增（归档会删边/包，基线缩表走 Wave 收口）；许可/NOTICE 迁移核对。
5. 回滚：`git revert` 或从 legacy 迁回 + 恢复 members/mod 声明（单批可逆）。

### 3.2 执行记录（2026-10-20，R2：接受现状 + 事故登记）

- **B（perception）**: `e8e7db33` 干净单任务；24 paths（20 rename + MANIFEST + 根 Cargo.toml + Cargo.lock + founding §10），members 31→30，`cargo check -p gateway -p cli` exit 0，Cargo.lock −108 包。
- **A（SDK 三岛）**: 内容已全部落地但被**共享 index 抢占**拆成两半——`df78542f`（env-registry 改动 + 32 rename 混合）与 `6dccb1d2`（mod/Cargo/自检/MANIFEST 编辑，干净）；实际功能态 = 两者叠加，子代理已实测 `cargo check -p apeireth-sdk --all-targets` exit 0、`cargo test -p apeireth-sdk --lib` 220 passed/0 failed、`cargo fmt --check` exit 0、members=30、三岛引用 0。
- **裁决 R2（不重写历史）**: 并行线在 17:10:09/17:10:14/随后数分钟内 5 秒级连提，此刻 `git reset` 重放会**二次事故**；故接受现状，明确缺陷边界——`df78542f` 单点 checkout 不可编译（rename 已移走、lib.rs 仍声明），**禁止以该点做 bisect/单点 CI**，以 `6dccb1d2` 及之后为可用树。未 push（无 upstream），影响限于本地历史整洁度，不触碰内容。
- **流程修复（待落地）**: ①`commit.ps1` 只提交显式路径白名单（禁止扫全部 staged），或提交前检测"staged 属于其他线"即中止；②多线共享工作树时，提交前 5 秒互斥窗口/口头让行；③历史事故登记：见 handbook 变更记录。
- **B 遗留（另立清理，不伪称已对齐）**: 3 条注释引用（tools/content_feed、organ/lib、plugin/perception.rs）；`crates.md`/`architecture.md`/`perception-setup.md` 未对齐；arch_guard STALE 5 条 perception 边/env（基线缩表在 Wave 3 收口由架构线做）；`capabilities-matrix.md` 属效率线在途文件，0 触碰。
- **lark**: 解耦任务另立（抽 wire 原语 ~250-350 行到 im 自有模块，行为保持）→ 再按本清单归档；本批不动 lark。

## 4. 迁移 / 回滚 / 验收

- R4a-1 回滚：删脚本/CI + 恢复门禁命令 + 口径行（5 分钟）；不动源码目录，0 风险。
- R4a 验收：主门禁耗时前后对比（基线 >18 min，目标 ≤12 min）；图像 CI 绿；`arch_guard --strict` 0 新增且依赖基线缩表 ≥28 条；文档口径（`test-baseline`/能力矩阵/`AGENTS` crate 口径若 R4a-2 则同步）。
- R4b 执行后：SDK 归档留 manifest + 恢复指引；perception 裁决写回能力矩阵。

## 5. O-6 三阶审查

- **总体最优**：先门禁隔离（当天可逆）再谈独立 workspace——变量最少地拿到"门禁时间下降"这一最大收益。拒的替代：①直接抽独立 workspace（11 个 Cargo.toml + 许可路径 + CI 一次性搬，风险高于收益）②不做（22% 代码永远拖慢每批验证）。
- **系统最优**：`tools→image-core` 保留 path 依赖（功能路径不动），只在构建/门禁层隔离；vendored 登记放契约，脚本从契约读，避免第二份清单。拒的替代：①把 image_ops 迁出 tools（功能回归面大）②把图像家族打进 tools crate（混合体）。
- **架构最优**：crate 口径与门禁口径解耦——先"不算进主门禁"，再"不算进主 workspace"；每步单独可回滚。拒的替代：①改 AGENTS crate 口径先行（文档先于事实）②把图像家族复制两份（双源）。

## 6. 后续

R4b 裁决后执行；R4a-2 完成后更新 `AGENTS.md` crate 口径（31→20 + 图像 11 独立）与 README workspace 徽章；届时由 owner 拍板是否在发布文案说明。
