# 0005. R4 crate 拓扑手术——图像家族门禁隔离 + SDK/perception 裁决

- **Status**: Accepted（R4a 授权代裁 2026-10-20「按你的建议来吧」）；**R4b 待 owner 裁决**
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

## 3. R4b 待 owner 裁决（两项）

| 项 | 现状 | 选项 | 架构线建议 |
|---|---|---|---|
| **SDK 四个孤岛**（lark 6,741 / livekit 6,517 / voice 6,170 / sandbox 4,221 = 23,649 行） | gateway 无 `/v1/tools/*` 路由；`founding-design-v2 §10` 自述"契约断头待裁决"；`STUB_MODE=true` | A 归档（迁 `legacy/` 或 release-assets tar + 指针）；B 接线（约 3-4 周 + 逐子 SDK 治理） | **A 归档**：无生产消费者、契约断头、维护成本 > 收益；未来需要时按 ADR 重引入 |
| **perception 整包**（约 6,300 行） | 需取证是否被 production 消费（K 轴截图/窗口显著性口径待核） | A 保留；B 归档 | 先取证再定；若已被 K 轴生产路径消费则保留，否则归档 |

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
