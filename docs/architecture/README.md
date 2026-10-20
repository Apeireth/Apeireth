# Apeireth 架构契约体系（Architecture Contract System）

> `Status: ✅ v1.0 规则生效（2026-10-20，ADR-0001 裁决；target 条款逐项 TODO-OWNER；架构改动按 owner 指示暂缓）`
> 定位：**让以后新增的东西不打乱架构，并让架构只朝最优方向演进** 的机器可执行机制。
> 关系：本体系不替代 LOCKED 5 项、不替代 `docs/03-reference/test-baseline.md`、
> 不替代 `docs/status/claims.yaml`；它是"架构"这一层的真源与守卫。

## 1. 四件套

| 文件 | 性质 | 维护方式 |
|---|---|---|
| [`contract.toml`](contract.toml) | **唯一可执行真源**：层/边/扩展点/不变量/预算/例外参数 | 人工改 + owner 裁决；结构变更必须配 ADR |
| [`exceptions.toml`](exceptions.toml) | 例外登记簿（带权重/owner/到期/移除计划） | 人工登记；守卫校验；到期自动红 |
| [`scripts/arch_guard.py`](../../scripts/arch_guard.py) | 架构守卫（fitness functions） | 只读仓；`--report` / `--strict` / `--update-baseline` |
| [`scripts/arch-baseline.txt`](../../scripts/arch-baseline.txt) | 棘轮基线（现存违规快照，只减不增） | 仅 `--update-baseline` 生成；评审后缩表 |

配套：[功能准入清单](intake-checklist.md)、[三线协作契约](line-coordination.md)、[ADR 流程](../adr/README.md)、[集成计划](integration-plan.md)。

## 2. 三类"新增"只允许走三条通道

| 类型 | 默认 | 强制条件 |
|---|---|---|
| **A 新能力/机制**（工具、生物机制、认知模块…） | 允许，但必须挂既有扩展点 | ①用契约 `[extension_points]` 之一，不自造注册表/trait；②进 capability ledger（四级口径）；③默认关 + 关时逐字节零变化测试；④自述节有对应格；⑤有接线点（信号源/状态所有者/观测/审计）；⑥不新增裸 env 读取（须登记） |
| **B 新结构**（crate/层/trait/注册表/总线/开关族） | **默认不给** | ADR（O-6 三阶 + 契约影响 + 迁移/回滚 + 预算净额）+ owner 拍板 + 同批更新 contract/基线；**例外预算默认 0，且净零** |
| **C 新文档/口径** | 允许，但只能是生成物或历史卷宗 | 生成物从 contract/ledger 产出；历史卷宗头部标 `Status: 历史口径（日期）`；禁止"引用即现状" |

准入八问（每个新增必须逐条回答，见 [intake-checklist.md](intake-checklist.md)）：
**哪层？哪个既有扩展点？新开关（默认关？）？接线点/信号源/状态所有者？观测与审计落哪？四层口径标到哪级？关掉零变化怎么证？删掉它的路径是什么？**

## 3. 用法

```powershell
python scripts/arch_guard.py                 # report: 报告现状/违规/可缩表项, exit 0
python scripts/arch_guard.py --strict        # 拦截: 新增违规/例外到期/超预算 -> exit 1
python scripts/arch_guard.py --update-baseline  # 仅评审后缩表/初始化时使用
python scripts/arch_guard.py --wave 2        # 指定架构波次(默认读 contract [meta].current_wave)
```

判定口径：
- **新增违规** = 指纹不在 `arch-baseline.txt` 中（依赖边/开关名/概念定义/扩展点定义/尺寸增长）。
- **可缩表** = 基线中有、实际已消失的条目（只提示，不判红）——每波收口应把它删掉。
- **例外** = 在 `exceptions.toml` 登记且未到期、未超预算；到期/超预算一律按违规处理。
- **fatal** = contract/exceptions 结构不可解析、cargo metadata 失败（exit 2）。

## 4. 例外预算（已裁决 · ADR-0001）

### 4.0 裁决结果（owner 授权代裁，2026-10-20）

| 旋钮 | 裁决值 | 理由（全项目语境） |
|---|---|---|
| 计量单位 | **权重分**（size/doc=1，edge/duplicate/env=3，direction/structural=10） | 条数会让"3 个文档漂移"与"1 条方向违规"同权；权重分把预算留给真结构问题 |
| 每波预算 | **3 分** | 等价于"W3 最多 1 条 / W1 最多 3 条"；够迁移期周转，又不足以堆债 |
| 活跃上限 | **2 条** | 与"项目同时只有 2-3 条 AI 施工线"的实况匹配；超了说明在系统性绕契约 |
| 存活期 | **2 波**（按波，不按天） | 项目以波推进；按天会逼出"最后一天突击清理"的假合规 |
| 结构类预算 | **0** | W10（方向违规/新 crate/新层/新扩展 ABI）只能改契约，不许"例外"——否则契约可被例外蛀空 |
| owner 特批 | **允许，同时 ≤1 条，仅 P0 安全/修复，不适用结构类；不占每波权重，占活跃上限；必须 ADR 记名 + 到期 + 移除计划** | 给紧急修复留活口，又不让特批变成第二预算池 |
| 冻结窗口 | **预算 0**；已有例外须在 tag 前清零 | 发版是唯一"必须干净"的时刻 |
| 净零范围 | **依赖边 / 重复概念**；env 新增不要求净零（但必须登记+默认关+零变化测试） | 新开关是合法能力扩展；新边/新重复概念才是结构净增 |
| 到期延期 | **不允许自动续期**：延期=新开一条+删旧条、占新波预算 | 防"无限续命" |
| 指纹漂移 | 视为新例外重新登记 | 防"重构把违规洗白" |
| 基线更新 | **仅架构波收口评审由 lead 执行；CI 校验 baseline diff 只减不增** | 防"更新基线"成为绕过棘轮的后门 |
| 批准分工 | 结构类 owner；债务类（W3）工程主线 lead；W1 守卫自动 | AI 施工线不能自批自己的例外 |

### 4.1 候选方案（决策依据留档）

| 方案 | 规则 | 优点 | 缺点 |
|---|---|---|---|
| A 零预算 | 任何新违规都红，无例外 | 最纯 | 迁移期误伤合法中间态；逼出"假合规"（拆两步/瞒报） |
| B 固定配额 | 每波允许 N 条新违规 | 简单 | 与严重度无关，结构违规混进小额配额 |
| C 净零 | 加 1 必须删 1 | 总量守恒 | 迁移期"临时双写"无处安放 |
| D 分级+到期 | 结构类 0；债务类配额+到期+移除计划 | 精细 | 需要登记纪律 |

### 4.2 定稿：**分级 + 净零 + 到期 + 冻结窗口**（contract `[exceptions]` 已按裁决值定稿）

```text
结构性/方向性违规           预算 = 0     （只能改契约/走 ADR，不允许"例外"）
可验证债务类（边/重复/env） 每波权重 ≤ 3、活跃 ≤ 2 条、存活 ≤ 2 波、净零
尺寸/文档类（W1）           每波权重 ≤ 3（即最多 3 条 W1），随波清
发版冻结窗口                预算 = 0
owner 特批                  ADR 记名 + 不占配额 + 仍须到期/移除计划（限 P0 安全/修复）
```

权重表（contract `[exceptions].weights`）：`size/doc=1`、`edge/duplicate/env=3`、`direction/structural=10`。
**净零**只对"依赖边/重复概念"生效：加 1 条必须同批删 ≥1 条（删旧重复实现/删旧边）。
**到期**由 `current_wave` 判定：`current_wave > expires_wave` 即红；到期未清自动升级为 blocking。

### 4.3 一句话口径（可写进 AGENTS/宪章）

> **架构例外不是许可证，是带到期日的欠条：结构类零预算，债务类每波 ≤3 分、存活 ≤2 波、加一必须删一，发版冻结期一律零。**

## 5. 裁决与波次流程

1. **提出**：结构变更 → 写 ADR（`docs/adr/NNNN-*.md`）+ 改 `contract.toml` 草案；
2. **评审**：`arch_guard --report` 对照现状；owner 裁决（契约条款 target → accepted）；
3. **落地**：代码/文档同批；`--strict` 绿；基线缩表；
4. **收口**：每波结束时跑一次架构复盘（记分牌见 `integration-plan.md` §5），`current_wave += 1`。
5. **冻结**：发版窗口 `release_freeze = true` → 预算 0，只允许缩表与修红。

## 6. 0 装边界（v1.0 已覆盖/未覆盖）

**已自动校验**：层方向、生产依赖边新增、循环（生产/开发）、env 名新增、文件尺寸增长、同名概念定义数、扩展点定义数、例外到期/预算。

**尚未校验（v0.2 追加，如实标注）**：
- 依赖边"代码级使用"是否真实（只看 Cargo metadata）；
- env 读取位置（v1.0 只按名字卡，不按文件卡）；
- 接线账完整性（claims.yaml 与能力矩阵的逐项映射）；
- 生成物漂移（`ARCHITECTURE.md` 尚未改为生成物）；
- 测试/构建成本（门禁时长、target 体积）。

**接入现状**：`verify_changed.py`（report-only 常驻）、`commit.ps1 -ArchStrict`（默认 report-only）、`.github/workflows/arch-guard.yml`（report-only）已接入；灰度至 2026-11-03 后 CI/commit 转 blocking。详见 [integration-plan.md](integration-plan.md)。
