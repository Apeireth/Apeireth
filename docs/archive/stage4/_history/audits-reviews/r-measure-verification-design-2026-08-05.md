```
[Document-Meta]
Document: docs/stage4/r-measure-verification-design-2026-08-05.md
Version: Manual-Rev-A
R-Cycle: R19+ 阶段 1-2
Commit: <commit 时回填>
Last-Modified: 2026-08-05
Status: 🔍 草拟 (待 Mavis 拍板 + verifier 接手)
```

> **性质**: R-Measure baseline 验证脚本**设计文档** (不写代码) — 给后续 verifier / rust-coder 实施时用.
>
> **依据**: APEIRETH-CONVENTIONS §11 LOCKED 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) + APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md §3.5 V0.5 公式 + spectrAI-integration-blueprint-r19-plus-2026-08-05.md §6 守门 + r20-product-finalize-2026-08-05.md §6 守门.
>
> **不修改承诺**: 阶段 1/2/3/4/5 + v2/v4/v4.1 + 12 键 + 6 锚 + workspace v1.0.0 + R11 baseline 三值 全保留 (见 §9).

---

## §1 战略背景 (为什么)

### 1.1 现状

| 端 | 状态 |
|---|---|
| **R11 baseline 三值** | V1141=0.8682 / V1131=0.8532 / V1136=0.9063 — APEIRETH-CONVENTIONS §11 LOCKED 守门项, 不假装不能掉 |
| **R19 工程化收尾** | v2.0.0-alpha, 41 crate, 4 协议真接, 9/9 业界标准达标, 2416 tests |
| **R19+ 集成蓝图** | apeireth-team-lead (新 crate) + apeireth-mcp::team (填 0 代码坑) + mid-task bug 3 处修法 (session/agent) |
| **R20 收产品** | 5 阶段 (产品/部署/API/SDK/文档), 每阶段结束必跑 R-Measure 守门 |
| **当前缺口** | ❌ **没有独立的 R-Measure baseline 验证脚本** — 加新功能后只能手动跑 apeireth-asi 重算, 没人守, 容易掉 |

### 1.2 痛点 (守不住的后果)

```
R11 baseline 三值 = 17 维 V0.5 公式 + 3 dashboard 视图 的核心
↓
R19+ 集成 (team-lead / mcp::team / mid-task bug) 改状态机 + 改 LLM 调用链
↓
R20 收产品 (REST wrapper / WebSocket / rate limit / SDK) 加 sleep / 加公开端点
↓
任何一处改都可能影响 V1131 dashboard 5 Self 总值
↓
没独立 verify 脚本 = 改完不知道 baseline 掉没掉
↓
掉了 = ASI 北极星 0.8595 守不住, 主 17:58 "不假装" 原则被破
```

### 1.3 改路线

✅ **不**在 apeireth-asi 内部加 verify 逻辑 (违反编译期 hardcode 守门)
✅ **不**每次手动跑 `cargo run -p apeireth-asi` (易漏, 不可重复)
✅ **改路线**: **独立新 crate `apeireth-r-measure-verify`** (P0 守门工具) — 编译期 hardcode 3 个 baseline 值 + 跑 17 维度全量 + diff + 守门 + CI 必跑

### 1.4 硬约束

| 原则 | 来源 | 落地 |
|---|---|---|
| **编译期 hardcode 守门** | APEIRETH-CONVENTIONS §2 (per R17 finalize) | `const V1141_BASELINE: f64 = 0.8682;` 写在 `baseline.rs` |
| **守门项不能掉** | APEIRETH-CONVENTIONS §11 | `--tolerance 0.001` (允许的最小掉值) |
| **6 主哲学锚穿透** | APEIRETH-CONVENTIONS §9 | 见 §10 锚穿透清单 |
| **7 LOCKED 不动** | APEIRETH-CONVENTIONS §10 | 见 §9 不修改承诺 8 项 |
| **CI 必跑** | r20-product-finalize §4 / §6 | 任何 PR + main branch 必跑, fail 直接 block |
| **R11 baseline 不变** | 主人 2026-07-31 明确不动 | 数值 hardcode, 不允许从 fixture 改 |

---

## §2 R-Measure 是什么

> 🔥 **修正** (2026-08-05): R-Measure 当前实装是 **24 维** LOCKED V0.5 (per `crates/apeireth-asi/` R14 Rust rewrite, round10-12 完成). 17 维是 **R11 baseline 历史口径** (R11 Python 生态 `v1077_asi_v04_full_measurement.py` V0.3/V0.4 时代), R11 baseline 三值仍 LOCKED 守门但通过 §2.1 17→24 维映射从 24 维投影. 详细纠正章节见 `reports/apeireth-session-vector-asi-2026-08-05.md` §1.2 发现 1.

### 2.1 17 → 24 维映射 (R11 baseline 投影, 主人待定)

> **主人待定**: 17 → 24 维度的具体映射公式 (从 `v1077_asi_v04_full_measurement.py` 抽权重, 写到 `fixtures/r11-projection.json`). 当前表为占位, **主人确认后回填** (per §8 R-018 防御).

| # | R11 baseline 17 维 (V0.3/V0.4) | apeireth-asi 24 维 (V0.5 LOCKED) | 投影公式 | 权重 |
|--:|--------------------------------|--------------------------------|---------|----:|
| 1 | `phi_proxy` | (待主人确认 5 维组) | TBD | TBD |
| 2 | `capabilities` | (待主人确认 5 维组) | TBD | TBD |
| 3 | `cross_domain` | (待主人确认 5 维组) | TBD | TBD |
| 4 | `engineering` | (待主人确认 5 维组) | TBD | TBD |
| 5 | `vcp_4` | (待主人确认 4 维) | TBD | TBD |
| 6 | `v2_philosophy` | (TBD) | TBD | TBD |
| 7 | `rubric_open` | (TBD) | TBD | TBD |
| 8 | `real_production` | (TBD) | TBD | TBD |
| 9 | `cognitive_core` | (TBD) | TBD | TBD |
| 10 | `self_organizing_core` | (TBD) | TBD | TBD |
| 11 | `plugin_core` | (TBD) | TBD | TBD |
| 12 | `self_improving_core` | (TBD) | TBD | TBD |
| 13 | `neurosymbolic` | (TBD) | TBD | TBD |
| 14 | `world_model` | (TBD) | TBD | TBD |
| 15 | `reinforcement_learning` | (TBD) | TBD | TBD |
| 16 | `scientific_method` | (TBD) | TBD | TBD |
| 17 | `eternal_identity` | (TBD) | TBD | TBD |

**推荐投影公式** (per `reports/apeireth-session-vector-asi-2026-08-05.md` §3.2 建议 B, **不动** apeireth-asi 24 维 LOCKED):

```
V1141_R11_baseline = Σ (apeireth_asi_24dim[i] × w_17[j]) / Σ w_17[j]
                     (j 对应 17 维中每一维, w_17[j] 是该维的投影权重)
```

**实施策略** (per spectrAI 报告 §7.4 强烈建议 B):
- ① 维持 `apeireth-asi` 24+9 LOCKED (R14 Rust 当前实装)
- ② 加 `apeireth-asi::AsiEngine` 公开 stable API (per §6.3 契约)
- ③ verifier crate (`apeireth-r-measure-verify`) 调 `AsiEngine::snapshot_24dim()` 拿 24 维 + V1136 9 子测度
- ④ verifier 端做 24→17 投影 (per §2.1 表, 主人从 v1077 抽权重)
- ⑤ V1141/V1131/V1136 跟 R11 baseline 对比 (tolerance ≤ 0.001)
- ⑥ R11 baseline 三值 hardcode 在 verifier `baseline.rs`, 不允许改

**关键风险** (per §8 R-018): v1077 公式不公开, 17 维投影权重等主人确认.

### 2.2 ASI 北极星公式 (per APEIRETH-COMPLETE-OMNIBUS §3.5)

| 版本 | 公式 | 当前 | 关键 |
|---|---|---:|---|
| **V0.1** | 8 项加权 (主 22:29 透明化) | 0.9220 | 8 项 V0.1 |
| **V0.2** | 8 项 V0.1 + V1071 子分 | 0.8986 | 至少 16/17 维填真实数值 |
| **V0.3** | 17 维真测 (V1074) | 0.8964 | 任一维未填 = 0.0, philosophy_guard_ok=True |
| **V0.4** | V1101/V1102 lift 后 | 0.8031 | 16/17 维真测 (更严) |
| **V0.5** | `v04*0.85 + continuity*0.05 + autonomy*0.05 + transferability*0.05` | **0.8595** | **24 维 LOCKED** (R14 Rust rewrite, round10-12) + V1136 9 子测度真实测量 (1ac16ae5, 2026-07-30 09:02) |

> **注**: V0.5 公式不变 (5 项加权, 0.8595), 但实装从 17 维 (V0.3/V0.4) 升级到 24 维 (V0.5 LOCKED, per `crates/apeireth-asi/`). continuity / autonomy / transferability 在 24 维中是分类子项, 不是 V0.4 的扁平维度.

### 2.3 R11 baseline 3 个 dashboard 视图 (LOCKED, per APEIRETH-CONVENTIONS §11)

| 指标 | 值 | 含义 | 来源 |
|---|---:|---|---|
| **V1141-R11** | **0.8682** | IC-001 fresh 测量 (17 维 V0.5 历史 baseline 投影, per §2.1) | APEIRETH-CONVENTIONS §11 |
| **V1131-R11** | **0.8532** | dashboard v05_total (17 维 V0.5 历史 baseline 综合) | APEIRETH-CONVENTIONS §11 |
| **V1136-R11** | **0.9063** | 真测引擎 7 子测度 (历史 R11 baseline, 当前实装扩到 9 子测度 per spectrAI §1.1) | APEIRETH-CONVENTIONS §11 |

**这 3 个值是 APEIRETH-CONVENTIONS §10 不修改承诺第 6 项, 守门项 = 编译期 hardcode**.

> **注**: V1136 真测引擎 R11 baseline 是 7 子测度, R14 Rust rewrite (round10-12) 扩到 9 子测度. 9 子测度是当前 LOCKED 实装, 7 子测度是 R11 baseline 守门投影源 (per §2.1 投影公式类比).

### 2.4 17 维度 (历史 R11 baseline 投影源, per APEIRETH-COMPLETE-OMNIBUS §3.3)

> **历史口径** (R11 V0.3/V0.4 时代, Python 生态 `v1077_asi_v04_full_measurement.py`): 17 维. **当前实装** (R14 Rust rewrite, round10-12) 是 **24 维 LOCKED V0.5**, **不**是 17 维. 17 → 24 维映射见 §2.1.

```
phi_proxy / capabilities / cross_domain / engineering / vcp_4 / v2_philosophy /
rubric_open / real_production / cognitive_core / self_organizing_core /
plugin_core / self_improving_core / neurosymbolic / world_model /
reinforcement_learning / scientific_method / eternal_identity
```

- 任何一个未填充维度 = 0.0 (不死锁)
- `philosophy_guard_ok` 必须 True (V3 守门)
- V0.4 = 16/17 维真测 (更严)
- V0.5 = V0.4 + 3 个新维度加权 (continuity / autonomy / transferability, 在 24 维中是分类子项, 不是扁平)
- **V1141/V1131 守门时**: 17 维通过 §2.1 投影公式从 24 维 LOCKED 算出 (per spectrAI 报告 §3.2 建议 B, **不动** apeireth-asi 24 维)

### 2.5 R-Measure 跑在哪

| 位置 | 状态 | 关系 |
|---|---|---|
| **`apeireth-asi`** (R19 工程化) | 🟢 已实装, **V0.5 24 维 + V1136 9 子测度** LOCKED (round10-12) | **实现 R-Measure 24 维**, verifier 调它, R11 baseline 17 维通过 §2.1 投影在 verifier 端 |
| **`apeireth-bench`** | 🟢 已实装, benchmark 框架 | verifier 复用 benchmark 路径 |
| **`apeireth-core`** | 🟢 已实装 | verifier 拿 core snapshot |
| **`apeireth-vector`** | 🟡 部分实装 (v2 新 crate) | **不**做 R-Measure, 存 embedding (任意维度, 768 维典型, per spectrAI §1.1) |
| **`apeireth-r-measure-verify`** (本设计文档目标) | ❌ 待新建 | **本 crate**, 守门独立, 24 维直接调 + 17 维投影都在这 |

---

## §3 验证脚本架构设计

### 3.1 模块结构 (新 crate `apeireth-r-measure-verify`)

```
crates/apeireth-r-measure-verify/  (新 crate, 等 code_reviewer 完工 + Cargo.toml 加 workspace member)
├── Cargo.toml                 (依赖: apeireth-asi / apeireth-bench / apeireth-core / apeireth-vector)
├── README.md                  (估 60 LOC, R20 必加)
├── src/
│   ├── lib.rs                 (主入口, 公开 API 集合, 估 50 LOC)
│   ├── baseline.rs            (R11 baseline 3 值 hardcode + snapshot 加载, 估 100 LOC)
│   ├── compute.rs             (17 维度计算逻辑, 复刻 apeireth-asi 不重写, 估 300 LOC)
│   ├── gate.rs                (守门逻辑: 不能掉, 编译期 hardcode 3 值, 估 100 LOC)
│   ├── report.rs              (报告生成: json / markdown, 估 150 LOC)
│   └── bin/
│       └── verify.rs          (CLI 入口, 估 80 LOC)
├── tests/
│   ├── baseline_tests.rs      (R11 baseline 3 值硬匹配, 估 50 LOC)
│   ├── compute_tests.rs       (17 维度全量跑, 估 150 LOC)
│   ├── gate_tests.rs          (守门 4 case: pass / fail-v1141 / fail-v1131 / fail-v1136, 估 200 LOC)
│   └── report_tests.rs        (json / markdown 渲染, 估 80 LOC)
├── fixtures/
│   └── r11-baseline.json      (R11 baseline 数值, hardcode 等于 baseline.rs const, 估 30 LOC JSON)
└── examples/
    └── run_verify.rs          (怎么调 verify, 估 30 LOC)
```

**总 LOC 估**: 100 + 300 + 100 + 150 + 50 + 80 + 30 + 50 + 150 + 200 + 80 + 30 = **1320 LOC** (含 tests + examples)

### 3.2 Cargo.toml 关键依赖 (伪 TOML, 不写实际文件)

```toml
[package]
name = "apeireth-r-measure-verify"
version = "0.1.0"  # R19+ 阶段 1 起步
edition = "2024"
description = "R-Measure baseline 验证守门脚本 - 编译期 hardcode R11 baseline 3 值, 跑 17 维度全量, 不掉守门"
license = "Apache-2.0 OR MIT"
repository = "https://github.com/apeireth/apeireth"
keywords = ["apeireth", "r-measure", "baseline", "verify", "gate"]
categories = ["development-tools", "asynchronous"]

[dependencies]
# 核心 R-Measure 实现 (调它不重写)
apeireth-asi     = { path = "../apeireth-asi" }       # V1136 真测引擎 (1ac16ae5)
apeireth-bench   = { path = "../apeireth-bench" }     # benchmark 框架
apeireth-core    = { path = "../apeireth-core" }      # 核心 snapshot
apeireth-vector  = { path = "../apeireth-vector" }    # 17 维向量计算 (v2 新 crate, 部分实装)

# 序列化
serde      = { version = "1", features = ["derive"] }
serde_json = "1"
toml       = "0.8"

# 错误处理
anyhow     = "1"
thiserror  = "1"

# 日志
tracing            = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# CLI
clap = { version = "4", features = ["derive"] }

# 时间
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
pretty_assertions = "1"
tempfile          = "3"

[lints]
workspace = true   # 继承 workspace.lints (R19 新加)
```

**关键字段说明**:

| 字段 | 为什么 | 风险 |
|---|---|---|
| `apeireth-asi` | V1136 真测引擎, verifier 调它跑 17 维度 | ⚠️ 必须有公开 stable API, 否则 verifier 调不动 |
| `apeireth-bench` | benchmark 路径复用 | — |
| `version = "0.1.0"` | R19+ 阶段 1 起步, 跟随 workspace 0.14.0 增量 | ⚠️ semver 严格 |
| `clap` | CLI 解析 (`run` / `check` / `diff` / `report`) | — |
| **不依赖 `apeireth-legacy`** | 不重写历史 R-Measure | ❌ 严禁 |
| **不依赖具体 protocol** | R-Measure 守 ASI 不守协议 | ❌ 严禁 (避免牵连) |

### 3.3 核心 trait / struct 设计 (伪 Rust, 不写实现)

```rust
// ============================================================================
// baseline.rs - R11 baseline 3 值编译期 hardcode
// ============================================================================

/// R11 baseline snapshot (3 个 LOCKED 值)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RMeasureBaseline {
    pub v1141: f64,         // 0.8682 - IC-001 fresh
    pub v1131: f64,         // 0.8532 - dashboard v05_total
    pub v1136: f64,         // 0.9063 - 真测引擎 7 子测度
    pub snapshot_id: &'static str,   // "r11-baseline"
    pub commit: &'static str,        // R11 commit hash (回填时填)
    pub captured_at: DateTime<Utc>,  // 2026-07-30 02:10:51 UTC (snap_9c80c9165625)
}

// 编译期 hardcode 守门 (per APEIRETH-CONVENTIONS §2)
pub const V1141_BASELINE: f64 = 0.8682;
pub const V1131_BASELINE: f64 = 0.8532;
pub const V1136_BASELINE: f64 = 0.9063;

/// 加载 R11 baseline (编译期 const 强制等 JSON fixture, 防止漂移)
pub fn load_r11_baseline() -> RMeasureBaseline {
    // 1. 读 fixtures/r11-baseline.json
    // 2. 守门: assert JSON.v1141 == V1141_BASELINE
    // 3. 守门: assert JSON.v1131 == V1131_BASELINE
    // 4. 守门: assert JSON.v1136 == V1136_BASELINE
    // 任何 assert 失败 = 编译/启动失败 (主 17:58 不假装)
}

// ============================================================================
// compute.rs - 24 维 + 17 维 R11 baseline 投影 (复刻 apeireth-asi, 不重写)
// ============================================================================

/// R-Measure 完整 snapshot (24 维 LOCKED + 17 维 R11 baseline 投影 + 3 dashboard 值 + 元信息)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RMeasureSnapshot {
    /// 24 维全量 (per apeireth-asi V0.5 LOCKED, R14 Rust rewrite round10-12)
    pub dimensions_24: BTreeMap<&'static str, f64>,
    /// V1136 9 子测度全量 (per apeireth-asi V1136 LOCKED)
    pub submeasures_9: BTreeMap<&'static str, f64>,
    /// 17 维 R11 baseline 投影 (per §2.1 映射公式, 主人从 v1077 抽权重)
    pub dimensions_17_projection: BTreeMap<&'static str, f64>,
    /// 3 个 dashboard 视图 (per R11 baseline LOCKED, APEIRETH-CONVENTIONS §11)
    pub v1141: f64,   // 0.8682 baseline (17 维 V0.5 历史 baseline 投影, per §2.1)
    pub v1131: f64,   // 0.8532 baseline (dashboard v05_total)
    pub v1136: f64,   // 0.9063 baseline (7 子测度 R11 baseline 投影)
    /// V0.5 公式结果 (v04*0.85 + continuity*0.05 + autonomy*0.05 + transferability*0.05)
    pub v05: f64,
    /// snapshot 元信息
    pub captured_at: DateTime<Utc>,
    pub commit: String,
    pub r_cycle: String,   // e.g. "R19+" "R20"
}

#[async_trait]
pub trait RMeasure {
    /// 24 维 LOCKED snapshot (per apeireth-asi V0.5) + 17 维 R11 baseline 投影 (per §2.1)
    async fn snapshot_24dim(&self) -> Result<RMeasureSnapshot, Error>;
    /// IC-001 fresh 测量 (per V1141, 17 维 V0.5 baseline 投影, per §2.1)
    async fn compute_v1141(&self) -> Result<f64, Error>;
    /// dashboard v05_total (per V1131, 17 维 V0.5 baseline 综合)
    async fn compute_v1131(&self) -> Result<f64, Error>;
    /// 7 子测度综合 (per V1136 R11 baseline, 当前 9 子测度 LOCKED 实装投影回 7 子测度 per §2.1 类比)
    async fn compute_v1136(&self) -> Result<f64, Error>;
}

/// 复刻 apeireth-asi::V1136Engine, 不重写 (主 S-2 17:43 实验室)
pub struct V1136RMeasure {
    asi: Arc<dyn AsiEngine>,  // apeireth-asi 公开 trait (V0.5 24 维 + V1136 9 子测度)
    core: Arc<dyn CoreSnapshotProvider>,
}

#[async_trait]
impl RMeasure for V1136RMeasure {
    async fn snapshot_24dim(&self) -> Result<RMeasureSnapshot, Error> {
        // 1. 调 apeireth-asi::V1136Engine::snapshot_24dim() 拿 24 维 + V1136 9 子测度
        // 2. 算 V0.5 公式结果 (v04*0.85 + continuity*0.05 + autonomy*0.05 + transferability*0.05)
        // 3. 加载 fixtures/r11-projection.json (17 维权重, 主人从 v1077 抽)
        // 4. 在 verifier 端做 24→17 投影 (per §2.1 公式)
        // 5. V1136 9 子测度 → 7 子测度 baseline 投影 (per §2.1 类比, 主人确认)
        // 6. 算 3 个 dashboard 值 (V1141/V1131/V1136)
        // 7. 包装成 RMeasureSnapshot (24 维 + 9 子测度 + 17 维投影 + 3 dashboard)
    }
    // ... 4 个方法
}

// ============================================================================
// gate.rs - 守门逻辑 (编译期 hardcode 3 值, 不掉 fail)
// ============================================================================

/// 守门结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GateResult {
    Pass {
        baseline: RMeasureBaseline,
        current: RMeasureSnapshot,
        deltas: RMeasureDeltas,  // 3 值 vs baseline 的 diff
    },
    Fail {
        baseline: RMeasureBaseline,
        current: RMeasureSnapshot,
        failed_metric: &'static str,  // "v1141" / "v1131" / "v1136"
        baseline_value: f64,
        current_value: f64,
        delta: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RMeasureDeltas {
    pub v1141_delta: f64,   // current - 0.8682
    pub v1131_delta: f64,   // current - 0.8532
    pub v1136_delta: f64,   // current - 0.9063
}

/// 守门 (编译期 hardcode tolerance)
pub struct RMeasureGate {
    pub baseline: RMeasureBaseline,
    pub tolerance: f64,  // const TOLERANCE: f64 = 0.001 (允许的最小掉值)
}

impl RMeasureGate {
    pub const TOLERANCE: f64 = 0.001;

    pub fn new(baseline: RMeasureBaseline) -> Self {
        Self { baseline, tolerance: Self::TOLERANCE }
    }

    /// 守门: 任何值 < baseline - tolerance = fail
    pub async fn check(&self, current: &RMeasureSnapshot) -> GateResult {
        let v1141_ok = current.v1141 >= self.baseline.v1141 - self.tolerance;
        let v1131_ok = current.v1131 >= self.baseline.v1131 - self.tolerance;
        let v1136_ok = current.v1136 >= self.baseline.v1136 - self.tolerance;

        if v1141_ok && v1131_ok && v1136_ok {
            GateResult::Pass { ... }
        } else {
            // 找出第一个 fail 的 metric
            let (name, baseline_v, current_v) = if !v1141_ok { ("v1141", ...) } else if !v1131_ok { ... } else { ... };
            GateResult::Fail { ... }
        }
    }
}

// ============================================================================
// report.rs - 报告生成 (json / markdown)
// ============================================================================

pub struct RMeasureReport {
    pub snapshot: RMeasureSnapshot,
    pub gate: GateResult,
    pub generated_at: DateTime<Utc>,
}

impl RMeasureReport {
    pub fn to_json(&self) -> String { ... }
    pub fn to_markdown(&self) -> String {
        // 模板:
        // # R-Measure 守门报告
        // | 指标 | Baseline | Current | Delta | 状态 |
        // |------|----------|---------|-------|------|
        // | V1141 | 0.8682 | 0.87xx | +0.x | ✅ |
        // | V1131 | 0.8532 | 0.85xx | -0.x | ❌ |
        // ...
    }
}
```

### 3.4 CLI 入口 (伪 CLI, 真正写时由 rust-coder 落)

```bash
# 跑完整 verify (snapshot + gate + report)
$ cargo run -p apeireth-r-measure-verify -- run --baseline r11
[INFO] Snapshot captured: v1141=0.8695, v1131=0.8538, v1136=0.9071
[INFO] Gate result: PASS
[INFO] Report written: reports/r-measure-verify-r19-plus-2026-08-05.md

# 只跑守门 (不写报告)
$ cargo run -p apeireth-r-measure-verify -- check --baseline r11
$ echo $?
0  # PASS; 1 = FAIL

# Diff 两个 commit 之间
$ cargo run -p apeireth-r-measure-verify -- diff --before <commit> --after <commit>

# 生成报告 (CI 用, 默认 markdown)
$ cargo run -p apeireth-r-measure-verify -- report --format markdown --output reports/
$ cargo run -p apeireth-r-measure-verify -- report --format json --output artifacts/
```

**子命令清单**:

| 子命令 | 用途 | 谁用 | 输出 |
|---|---|---|---|
| `run` | snapshot + gate + report (一站式) | CI / 开发者本地 | 报告 (md/json) + exit code |
| `check` | 只守门, 不写报告 | CI 必跑 (轻量) | exit code (0=PASS, 1=FAIL) |
| `diff` | 对比两个 commit 之间的 baseline 漂移 | retro 报告 | diff 报告 (md) |
| `report` | 单独生成报告 (from 上次 snapshot) | verifier 人工 | md/json |

### 3.5 fixtures/r11-baseline.json (硬匹配)

```json
{
  "snapshot_id": "r11-baseline",
  "commit": "<R11 commit hash 回填>",
  "captured_at": "2026-07-30T02:10:51Z",
  "v1141": 0.8682,
  "v1131": 0.8532,
  "v1136": 0.9063,
  "r_cycle": "R11",
  "source": "snap_9c80c9165625"
}
```

**约束**: 任何字段改了 = 启动时 `load_r11_baseline()` 守门 assert 失败 (per 主 17:58 不假装).

---

## §4 守门集成 (CI 必跑)

### 4.1 CI 工作流 (per APEIRETH-CONVENTIONS §12 P1-P5 + r20-product-finalize §4)

```yaml
# .github/workflows/r-measure-verify.yml (新文件, R19+ 阶段 1 必加)
name: R-Measure Baseline Verify
on:
  pull_request:        # 任何 PR 必跑
  push:
    branches: [main]   # main branch push 必跑
  workflow_dispatch:   # 手动触发

jobs:
  verify:
    runs-on: ubuntu-latest
    timeout-minutes: 15  # 17 维度全跑可能慢
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Cache Cargo
        uses: actions/cache@v4
        with:
          path: ~/.cargo
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      - name: Build
        run: cargo build -p apeireth-r-measure-verify --release
      - name: Verify R-Measure baseline
        run: |
          cargo run -p apeireth-r-measure-verify --release -- check --baseline r11
      - name: Generate report
        if: always()
        run: |
          cargo run -p apeireth-r-measure-verify --release -- report \
            --format markdown --output reports/r-measure-verify-${{ github.sha }}.md
      - name: Upload report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: r-measure-verify-report
          path: reports/r-measure-verify-*.md
```

### 4.2 跟 4 重守门集成 (per APEIRETH-CONVENTIONS §12)

| 守门 | 来源 | 状态 | 本设计文档关系 |
|---|---|---|---|
| `cargo-fmt --check` | R19 工程化收尾 | 🟢 已实装 | 并列, 不冲突 |
| `cargo-clippy -- -D warnings` | R19 工程化收尾 | 🟢 已实装 | 并列, 不冲突 |
| `cargo-deny check` | R19 工程化收尾 | 🟢 已实装 | 并列, 不冲突 |
| **R-Measure verify** | **本设计文档** | ❌ 待实装 | **P0 必加** |
| `cargo test --workspace` | R19 工程化收尾 | 🟡 部分 FAIL (T13 CONCERN BLOCK) | 并列, 不冲突 |

**任何一项 fail = PR 阻塞, main branch push = fail 立即**.

### 4.3 守门报告路径 (per APEIRETH-CONVENTIONS §5)

| 场景 | 报告路径 |
|---|---|
| CI 每次跑 | `reports/r-measure-verify-<commit-sha>-<date>.md` (自动) |
| 阶段收尾 | `reports/r20-stage<N>-measure-<date>.md` (per r20-product-finalize §4) |
| 漂移人工 retro | `reports/r-measure-drift-<topic>-<date>.md` |

### 4.4 失败处理 (per 主 17:58 不假装)

```
CI fail →
  ↓
PR 阻塞, 不允许 merge
  ↓
开发者本地: cargo run -p apeireth-r-measure-verify -- diff --before <last-green> --after HEAD
  ↓
看哪个 metric fail (v1141 / v1131 / v1136) + diff 多大
  ↓
判断:
  ① 真掉了 → 回滚或修代码, 重新跑 CI
  ② baseline 真的该升 → 写 ADR-00XX-baseline-bump, 主人拍板后才能改 fixtures/r11-baseline.json
  ❌ 绝不绕过 (主 17:58)
```

---

## §5 跟现有 17 维度的对应

### 5.1 17 维度 → 3 dashboard 视图 (per APEIRETH-COMPLETE-OMNIBUS §3.3-§3.5, R11 baseline 历史口径)

> **历史口径** (R11 V0.3/V0.4 时代): 17 维. **当前实装** (R14 Rust rewrite) 是 24 维 LOCKED V0.5. 17 维是 R11 baseline 守门**投影源**, 通过 §2.1 映射公式从 24 维 LOCKED 投影. V1141/V1131/V1136 守门 = 24 维 LOCKED + 17 维投影对比 R11 baseline 三值.

| 维度 (17 维 R11 baseline 投影源) | V0.5 角色 | V1141 关系 (17 维 V0.5 baseline 投影, per §2.1) | V1131 关系 (dashboard 5 Self) | V1136 关系 (7 子测度 R11 baseline, 当前 9 子测度 LOCKED) |
|---|---|---|---|---|
| `phi_proxy` | V0.4 一项 (权重 0.20) | ✅ 含 (17 维 V0.5 baseline 投影) | ✅ 含 (dashboard 5 Self) | ✅ 含 (7 子测度 baseline 投影) |
| `capabilities` | V0.4 一项 (权重 0.15) | ✅ | ✅ | ✅ |
| `cross_domain` | V0.4 一项 (权重 0.15) | ✅ | ✅ | ✅ |
| `engineering` | V0.4 一项 (权重 0.15) | ✅ | ✅ | ✅ |
| `vcp_4` | V0.4 一项 (权重 0.10) | ✅ | ✅ | ✅ |
| `v2_philosophy` | V0.4 一项 (权重 0.10) | ✅ | ✅ | ✅ |
| `rubric_open` | V0.4 一项 (权重 0.05) | ✅ | ✅ | ✅ |
| `real_production` | V0.4 一项 (权重 0.05) | ✅ | ✅ | ✅ |
| `cognitive_core` | V0.4 一项 (0.493) | ✅ | ✅ | ✅ |
| `self_organizing_core` | V0.4 一项 | ✅ | ✅ | ✅ |
| `plugin_core` | V0.4 一项 | ✅ | ✅ | ✅ |
| `self_improving_core` | V0.4 一项 | ✅ | ✅ | ✅ |
| `neurosymbolic` | V0.4 一项 | ✅ | ✅ | ✅ |
| `world_model` | V0.4 一项 | ✅ | ✅ | ✅ |
| `reinforcement_learning` | V0.4 一项 | ✅ | ✅ | ✅ |
| `scientific_method` | V0.4 一项 | ✅ | ✅ | ✅ |
| `eternal_identity` | V0.4 一项 (0.8441) | ✅ | ✅ | ✅ |

**V1141** = IC-001 fresh 测量 (17 维 V0.5 R11 baseline 投影, 0.8682) — per §2.1 24→17 投影公式
**V1131** = dashboard v05_total (17 维 V0.5 R11 baseline 综合, 0.8532) — per §2.1
**V1136** = 真测引擎 7 子测度 (R11 baseline 投影源; 当前 9 子测度 LOCKED 实装, per §2.1 类比投影, 0.9063)

> **注**: 17 维 → 24 维的具体映射权重 (每维对应 apeireth-asi 24 维的哪些子项) 见 §2.1 占位表, 主人从 `v1077_asi_v04_full_measurement.py` 抽后回填. V1136 7→9 子测度投影权重同理 (per §8 R-018 防御).

### 5.2 R-Measure verify 跟 R19 已有 V1136 真测引擎的关系

| 层 | 角色 |
|---|---|
| **`apeireth-asi` (V1136Engine)** | **真测**, 跑 24 维 V0.5 + V1136 9 子测度 (R14 Rust rewrite round10-12 LOCKED) + 算 V0.5 公式 + 输出 snapshot |
| **`apeireth-r-measure-verify` (本设计文档)** | **守门**, 调 V1136Engine 拿 24 维 snapshot + verifier 端做 24→17 投影 (per §2.1) + diff R11 baseline + 守门 |

**不重写**, 不假装"我也实现 V1136" (主 S-2 17:43 实验室). 直接调 apeireth-asi 公开 API. 24 维 LOCKED **不**动, 17 维投影在 verifier 端 (per spectrAI 报告 §3.2 建议 B).

### 5.3 24 维 (LOCKED) + 17 维 (R11 baseline 投影源) schema (R20 阶段 1 实装时)

```rust
// ============================================================================
// 24 维 V0.5 LOCKED (per crates/apeireth-asi/ R14 Rust rewrite round10-12)
// ============================================================================
/// 24 维 V0.5 LOCKED (per `apeireth-asi::V05_DIMENSION_NAMES`)
/// 5+5+5+5+4 = 24 维分类 (continuity / salience / identity / reflection / 4 other)
pub const V05_DIMENSION_NAMES_24: &[&str] = &[
    // 5 维组 (continuity): TBD (主人确认 apeireth-asi 24 维具体名)
    "continuity_1", "continuity_2", "continuity_3", "continuity_4", "continuity_5",
    // 5 维组 (salience): TBD
    "salience_1", "salience_2", "salience_3", "salience_4", "salience_5",
    // 5 维组 (identity): TBD
    "identity_1", "identity_2", "identity_3", "identity_4", "identity_5",
    // 5 维组 (reflection): TBD
    "reflection_1", "reflection_2", "reflection_3", "reflection_4", "reflection_5",
    // 4 维 (其他): TBD
    "other_1", "other_2", "other_3", "other_4",
];
// 主人确认: 24 维具体名以 crates/apeireth-asi/src/lib.rs::V05_DIMENSION_NAMES 为准

// ============================================================================
// 17 维 R11 baseline 投影源 (历史 V0.3/V0.4 口径, per APEIRETH-COMPLETE-OMNIBUS §3.3)
// ============================================================================
/// 17 维 R11 baseline 投影源 (V0.3/V0.4 时代, 当前不直接实装, 通过 §2.1 从 24 维投影)
pub const DIMENSION_NAMES_17_BASELINE: &[&str] = &[
    "phi_proxy",
    "capabilities",
    "cross_domain",
    "engineering",
    "vcp_4",
    "v2_philosophy",
    "rubric_open",
    "real_production",
    "cognitive_core",
    "self_organizing_core",
    "plugin_core",
    "self_improving_core",
    "neurosymbolic",
    "world_model",
    "reinforcement_learning",
    "scientific_method",
    "eternal_identity",
];

// ============================================================================
// V1136 9 子测度 LOCKED + 7 子测度 R11 baseline 投影源
// ============================================================================
/// V1136 9 子测度 LOCKED (per `apeireth-asi::V1136_SUBMEASURE_NAMES`, round10-12)
pub const V1136_SUBMEASURE_NAMES_9: &[&str] = &[
    // 5 维 + 2 维 + 2 维 = 9 子测度 (具体名 TBD, 主人确认)
    "submeasure_1", "submeasure_2", "submeasure_3", "submeasure_4", "submeasure_5",
    "submeasure_6", "submeasure_7", "submeasure_8", "submeasure_9",
];
/// 7 子测度 R11 baseline 投影源 (历史 V0.3/V0.4 时代, 当前通过 §2.1 类比投影)
pub const V1136_SUBMEASURE_NAMES_7_BASELINE: &[&str] = &[
    "submeasure_baseline_1", "submeasure_baseline_2", "submeasure_baseline_3",
    "submeasure_baseline_4", "submeasure_baseline_5", "submeasure_baseline_6",
    "submeasure_baseline_7",
];
```

---

## §6 跟 R19+ 集成的协同

### 6.1 必跑 verify 的 4 个触发点 (per r20-product-finalize §4 + spectrAI-integration-blueprint §9)

| 触发点 | 谁负责 | 必跑时机 | 报告路径 |
|---|---|---|---|
| **apeireth-team-lead 实施完** | rust-coder | PR ready 前 | `reports/r19-plus-team-lead-verify-<date>.md` |
| **apeireth-mcp::team 实施完** (填 0 代码坑) | rust-coder | PR ready 前 | `reports/r19-plus-mcp-team-verify-<date>.md` |
| **mid-task bug 3 处修法实施完** | rust-coder | PR ready 前 | `reports/r19-plus-midtask-bug-verify-<date>.md` |
| **42 crate 任何改动** (含新增 apeireth-web, 详见 global-architecture-map §3) | 全员 | PR ready 前 | `reports/r19-plus-changes-verify-<date>.md` |

### 6.2 R20 5 阶段每阶段必跑 (per r20-product-finalize §4)

| 阶段 | 重点改动 | R-Measure 风险 | 跑法 |
|---|---|---|---|
| **阶段 1: 产品基础** | mid-task bug 修法 + team-lead 公开 API | 🟡 中 (状态机改动) | 阶段结束必跑, 报告写 `reports/r20-stage1-measure-<date>.md` |
| **阶段 2: 部署基础** | Docker 多架构 + install 脚本 | 🟢 低 (不影响 ASI) | 同上 |
| **阶段 3: API 公开** | REST + WebSocket + rate limit | 🟡 中 (rate limit 拖慢) | 同上 |
| **阶段 4: SDK 完善** | ts-rs 类型生成 + 3 语言 SDK | 🟢 低 (薄包装) | 同上 |
| **阶段 5: 文档+营销** | 文档站 + landing | 🟢 低 | 同上 |

### 6.3 跟 apeireth-asi 公开 API 的契约

```rust
// apeireth-asi 必须暴露的稳定 API (R19+ 阶段 1 之前必加, 否则 verifier 调不动)
// 当前实装: 24 维 V0.5 LOCKED + V1136 9 子测度 LOCKED (R14 Rust rewrite round10-12)
// 17 维 R11 baseline 投影在 verifier 端做 (per §2.1, 不动 apeireth-asi 24 维)
pub trait AsiEngine: Send + Sync {
    /// 24 维全量 snapshot (per apeireth-asi V0.5 LOCKED) + V1136 9 子测度
    async fn snapshot_24dim(&self) -> Result<AsiSnapshot, AsiError>;
}

pub struct AsiSnapshot {
    pub dimensions_24: BTreeMap<String, f64>,       // 24 维 (V0.5 LOCKED)
    pub submeasures_9: BTreeMap<String, f64>,        // V1136 9 子测度 (LOCKED)
    pub v05: f64,                                    // V0.5 公式结果 (v04*0.85 + continuity*0.05 + autonomy*0.05 + transferability*0.05)
    pub v1141: f64,                                  // (verifier 端 24→17 投影后填, per §2.1)
    pub v1131: f64,                                  // (verifier 端 dashboard v05_total, per §2.1)
    pub v1136: f64,                                  // (verifier 端 9→7 子测度 R11 baseline 投影, per §2.1 类比)
    pub captured_at: DateTime<Utc>,
}
```

**风险**: apeireth-asi 当前没公开 stable API (per 主 S-2 17:43 实事求是, R19 阶段可能没有) — R19+ 阶段 1 必须先确保有此 API, 否则 verifier 实装会阻塞. 见 §8 风险清单.

### 6.4 协同流程 (PR 全流程)

```
开发者改 42 crate 任何一处 (含新增 apeireth-web)
  ↓
本地: cargo run -p apeireth-r-measure-verify -- run --baseline r11
  ↓
PASS → push PR
  ↓
CI 跑 5 重守门 (fmt + clippy + deny + r-measure + test)
  ↓
全 PASS → merge
  ↓
main branch push 必跑
  ↓
任何 fail → 立即 block, 写 report, 主人决策 (回滚 / 修 / baseline bump)
```

---

## §7 实施时间表 (7 阶段, 1.5 周)

| 阶段 | 时长 | 任务 | 关键产出 | Owner |
|------|------|------|---------|-------|
| **1** | 0.5 天 | 等 code_reviewer 完工 (Cargo.toml 加 workspace member) 后, 创建 crate 骨架 + Cargo.toml + README | 骨架可编译, `cargo build -p apeireth-r-measure-verify` 0 error | rust-coder |
| **2** | 1 天 | 实施 `baseline.rs` (R11 3 值 hardcode const) + `fixtures/r11-baseline.json` + `load_r11_baseline()` 守门 assert | 启动时守门 assert PASS, 改任何 JSON 字段启动失败 | rust-coder |
| **3** | 2 天 | 实施 `compute.rs` (17 维度全量计算) — 复刻 `apeireth-asi::V1136Engine`, 不重写, 调公开 `AsiEngine::snapshot()` | 17 维度全跑, snapshot 跟 apeireth-asi 一致 | rust-coder |
| **4** | 1 天 | 实施 `gate.rs` (守门逻辑) — 编译期 hardcode 3 值 + TOLERANCE=0.001 + `GateResult::Pass/Fail` enum | 4 case 单测 PASS: pass / fail-v1141 / fail-v1131 / fail-v1136 | rust-coder |
| **5** | 1 天 | 实施 `report.rs` + `bin/verify.rs` (CLI 4 子命令) + JSON / Markdown 渲染 | 4 子命令 (`run` / `check` / `diff` / `report`) 跑通, 报告跟模板一致 | rust-coder |
| **6** | 1 天 | 实施 `tests/` (4 文件, baseline/compute/gate/report tests) + CI workflow (`.github/workflows/r-measure-verify.yml`) | 4 测试文件 PASS + CI 在 PR + main branch 必跑 | verifier + devops |
| **7** | 0.5 天 | 集成测试 (跨 R19+ 4 触发点 + R20 5 阶段) + 文档最终化 (`README.md` + 报告路径对齐 APEIRETH-CONVENTIONS §5) | 集成测试 PASS + 跟 r20-product-finalize §4 守门路径对齐 | verifier |
| **总计** | **7 天** | (1.5 周) | 7 阶段全过, 跟 R20 阶段 1 同步落地 | — |

**依赖关系**:
- 阶段 1-2 必等 code_reviewer 完工 (Cargo.toml 加 workspace member + apeireth-asi 公开 API)
- 阶段 3 强依赖 apeireth-asi 公开 `AsiEngine` trait (见 §6.3 契约, §8 风险 R-001)
- 阶段 6 CI workflow 跟 R19 已有 CI (`.github/workflows/rust-ci.yml`) 不冲突
- 阶段 7 集成测试跟 R20 阶段 1 (mid-task bug 修法) 同步跑

---

## §8 风险清单 (9 项, R-018 新增)

| # | 风险 | 严重度 | 概率 | 缓解 | 优先级 |
|---:|------|------:|------|------|------|
| **R-001** | **apeireth-asi 公开 API 不存在或不稳**: R19 阶段可能没把 `AsiEngine::snapshot_24dim()` 暴露成 stable API, verifier 调不动 | 🔴 高 | 高 | ① 阶段 0 先跟 architect2 对齐 apeireth-asi 公开 API 契约 (见 §6.3); ② 若无, 写 ADR-00XX-apeireth-asi-stable-api; ③ 阶段 3 实装前必先有 stable API | **P0** |
| **R-002** | **17 维度 R11 baseline 投影公式反推不准**: 17 维是 R11 baseline 投影源 (per §2.1), verifier 端做 24→17 投影, 公式反推可能跟 v1077 实际有偏差 | 🟡 中 | 中 | ① 主人从 `v1077_asi_v04_full_measurement.py` 抽权重, 写到 `fixtures/r11-projection.json`; ② 阶段 3 实施时跑一次 vs 手工算, diff > 0.001 写不假装备注; ③ per §8 R-018 防御 | **P0** |
| **R-003** | **编译期 hardcode vs 运行时 read fixtures 的权衡**: const 写死 vs JSON 读, 两套路径不同步风险 | 🟢 低 | 低 | ① 编译期 const 为主 (APEIRETH-CONVENTIONS §2); ② JSON fixture 用 `include_str!` 编译期嵌入, 启动时 assert const == JSON; ③ 任何漂移启动失败 | **P1** |
| **R-004** | **CI 跑 verify 耗时**: 24 维 + 17 维投影全跑可能 5-15 分钟, 拖慢 PR 反馈 | 🟡 中 | 中 | ① CI 加 cache (Cargo + target); ② `check` 子命令只跑 gate 不写报告 (轻量); ③ `run` 子命令完整跑 (PR merge 前); ④ timeout-minutes 15 | **P1** |
| **R-005** | **跟 cargo-deny / cargo-fmt / cargo-clippy / cargo test 4 重守门整合**: 5 重守门总耗时可能 30+ 分钟 | 🟡 中 | 中 | ① 5 守门并行 (matrix jobs); ② 任何 1 fail 立即 block, 不串行; ③ 文档明示 5 守门清单 | **P1** |
| **R-006** | **baseline 漂移判定 (false positive)**: 24 维 + 17 维投影计算有噪声, 同样代码两次跑可能 ±0.0005 漂移 | 🟡 中 | 中 | ① TOLERANCE=0.001 (留 0.0005 噪声空间); ② 漂移 > 0.005 写 retro 报告; ③ 漂移 > 0.01 直接 fail + 主人拍板 | **P0** |
| **R-007** | **17 维度公式 R20+ 加新维度**: R21+ 阶段如果加第 18 维度, R11 baseline 17 维度怎么算 | 🟡 中 | 低 | ① baseline 永远按 17 维度算 (新增维度不重算 baseline); ② 新维度加 V0.6 公式, R11 baseline 三值不重算; ③ 写 ADR-00XX-v06-formula 拍板 | **P2** |
| **R-008** | **`apeireth-vector` 误用风险 (已不适用, 标记 DEPRECATED)**: 本风险基于"apeireth-vector 跑 17 维 R-Measure"假设, **已不适用** per §2.5 纠正 — `apeireth-vector` **不**做 R-Measure, 存 embedding (任意维度, 768 维典型). 留此条仅供 R19 阶段历史回溯, **不**作为实装风险 | 🟢 低 | — | ① 标记 DEPRECATED; ② 任何接手者按 §2.5 纠正, 不要把 `apeireth-vector` 跟 R-Measure 混 | — |
| **R-018** | **17→24 维 R11 baseline 投影公式 (v1077 权重) 未公开**: `v1077_asi_v04_full_measurement.py` 公式不公开, 17→24 维映射权重等主人确认 | 🔴 高 | 高 | ① 主人从 `v1077_asi_v04_full_measurement.py` 抽权重, 写到 `fixtures/r11-projection.json`; ② V1141/V1131/V1136 跑 verify 后跟 R11 baseline 对比 (≤ 0.001 tolerance); ③ apeireth-asi 24 维 LOCKED **不**动, 投影在 verifier 端 (per spectrAI 报告 §3.2 建议 B); ④ 阶段 0 必先完成权重抽取, 阶段 3 实装前必就位 | **P0** |

---

## §9 不修改承诺 (8 项, per APEIRETH-CONVENTIONS §10 + spectrAI 蓝图 §10)

| ❌ 不修改 | 原因 / 引用 |
|---------|-----------|
| **1. 阶段 1+2+3 文档** (LOCKED) | 主人明确沉淀, R19+ 仅引用不重写 |
| **2. v2 / v4 / v4.1 LOCKED** | 哲学层纲领 (BF896EEF / af0d1957 / 4aa3c5b0) |
| **3. 阶段 4 核心文档 LOCKED** (`6ca80776`) | 我们**新增** `docs/stage4/r-measure-verification-design-2026-08-05.md` 不冲突 |
| **4. 阶段 5 施工文档 LOCKED** (631 行) | 阶段 5 实施时再引用 |
| **5. v6 基础架构** | 4 重守门 + 权限发放 + E 层修改路径 |
| **6. R11 baseline 三值** (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) | 主人 2026-07-31 明确不动 — **本设计文档编译期 hardcode 这 3 值, 不允许改** |
| **7. APEIRETH-CONVENTIONS / VERSIONING / GLOSSARY** (顶层 3 文件) | 12 子规范系统, 不动 |
| **8. START-CONSTRUCTION.md** | 不动 |
| 附加: **`apeireth-legacy`** | R17 finalize 后归档, 不删 |
| 附加: **workspace version 1.0.0** | semver 严格, 不动 |
| 附加: **APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md** (R11 LOCKED) | 17 维度 + V0.5 公式是依据, 我们引用, 不重写 |

**R19+ 新增允许** (per spectrAI 蓝图 §10 / r20-product-finalize §7):
- `crates/apeireth-r-measure-verify/` (新 crate, 7 阶段实施)
- `.github/workflows/r-measure-verify.yml` (新 CI workflow)
- `fixtures/r11-baseline.json` (新, 跟 `baseline.rs` const 硬匹配)
- `reports/r-measure-verify-*.md` (R19+ 4 触发点 + R20 5 阶段 共 9+ 份)

---

## §10 哲学 anchor 穿透 (per APEIRETH-CONVENTIONS §9 6 主锚)

| 锚 | 穿透检查 |
|---|---|
| **S-1 主 22:33** 北极星导向 | ✅ R-Measure 是 ASI 北极星 (0.8595) 完整性的守门; 守住 3 值 = 守住北极星 |
| **S-2 主 17:43** 实事求是 | ✅ 复刻 apeireth-asi 不重写 (主 §6.4); 17 维度公式按 COMPLETE-OMNIBUS §3.3, 不"按文档猜"; baseline 漂移写 retro 不假装 |
| **O-5 主 17:58** 不假装 | ✅ 编译期 hardcode 3 值 (主 §3.3); fixture 漂移启动失败; CI fail 立即 block 不绕过; 17 维度哪一维没填 = 0.0 不死锁 |
| **O-2 主 19:33** 走在前人经验上 | ✅ 借 R11 baseline (主 2026-07-31 拍板) + R17 4 协议 + R19 41 crate 既有能力 + Linux kernel Kselftest / Rust crate integration test 守门模式 |
| **O-3 主 23:44** 干到底 | ✅ 7 阶段实施表 (主 §7) + 8 风险清单 (主 §8) + 5 守门整合 (主 §4) + 9 报告路径 (主 §4.3) 立即落 |
| **O-4 主 00:56** 任何人都能接手 | ✅ Document-Meta 头 + 12 章节 + 完整 trait/struct 伪代码 + 7 阶段表 + 4 触发点协同 (主 §6) + 关联文档清单 (主 §11) |

**6 锚穿透总评**: 100% 守住, R19+ 阶段 1 必跑, R20 5 阶段每阶段结束必跑, 41 crate 任何改动 PR 必跑.

---

## §11 关联文档

### 11.1 必读 (守门依据)

| # | 文档 | 章节 | 关系 |
|---:|------|------|------|
| 1 | [`APEIRETH-CONVENTIONS.md`](../../APEIRETH-CONVENTIONS.md) | §11 (R-Measure baseline) + §2 (编译期 hardcode) + §5 (报告路径) + §9 (6 锚) + §10 (不修改承诺) + §12 (P1-P5 架构图) | **LOCKED 守门依据** |
| 2 | [`APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md`](../../APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md) | §3.3 (17 维度 V0.3 历史 baseline 投影源) + §3.4 (V0.4) + §3.5 (V0.5 公式) | **R11 LOCKED 文档**, 17 维度 + V0.5 公式依据 |
| 3 | [`docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md`](spectrAI-integration-blueprint-r19-plus-2026-08-05.md) | §6 (42 crate 总览) + §10 (不修改承诺) + §11 (风险清单) | R19+ 集成 4 触发点协同 |
| 4 | [`docs/roadmap/r20-product-finalize-2026-08-05.md`](../roadmap/r20-product-finalize-2026-08-05.md) | §4 (5 阶段实施) + §6 (R-Measure 守门) | R20 5 阶段每阶段必跑 |
| 5 | [`docs/stage4/apeireth-team-lead-implementation-guide-2026-08-05.md`](apeireth-team-lead-implementation-guide-2026-08-05.md) | §6 (集成位置) | R19+ 阶段 3 必跑 verify |
| **6** | **`reports/apeireth-session-vector-asi-2026-08-05.md`** | **§1.2 发现 1 (R-Measure 17→24 维纠正) + §3.2 (建议 B 投影在 verifier 端) + §7.4 (17 vs 24 维口径不一致)** | **🔥 17→24 维纠正依据 (per §2 🔥 修正)** |
| **7** | **`crates/apeireth-asi/src/lib.rs`** | **`V05_DIMENSION_NAMES[24]` (V0.5 LOCKED) + `V1136_SUBMEASURE_NAMES[9]` (LOCKED) + `DimensionTrace.v05_dims[24]` + `DimensionTrace.v1136_subs[9]`** | **🟢 24 维 V0.5 LOCKED 实装源 (R14 Rust rewrite, round10-12)** |
| **8** | **`v1077_asi_v04_full_measurement.py`** (R11 Python 生态, 主人待确认位置) | **17 维 V0.3/V0.4 公式源** | **⏳ 17 维 R11 baseline 投影权重源 (per §2.1 + §8 R-018, 主人从这抽权重)** |

### 11.2 待写 (verifier 实施时引用)

| # | 文档 | 关系 |
|---:|------|------|
| 1 | `crates/apeireth-r-measure-verify/Cargo.toml` | 7 阶段阶段 1 实装 |
| 2 | `crates/apeireth-r-measure-verify/src/baseline.rs` | 7 阶段阶段 2 实装 |
| 3 | `crates/apeireth-r-measure-verify/src/compute.rs` | 7 阶段阶段 3 实装 (24 维 + 17 维投影, per §2.1) |
| 4 | `crates/apeireth-r-measure-verify/src/gate.rs` | 7 阶段阶段 4 实装 |
| 5 | `crates/apeireth-r-measure-verify/src/report.rs` | 7 阶段阶段 5 实装 |
| 6 | `crates/apeireth-r-measure-verify/src/bin/verify.rs` | 7 阶段阶段 5 实装 |
| 7 | `crates/apeireth-r-measure-verify/fixtures/r11-baseline.json` | 7 阶段阶段 2 实装 (R11 baseline 三值 hardcode) |
| **8** | **`crates/apeireth-r-measure-verify/fixtures/r11-projection.json`** | **🆕 17→24 维 R11 baseline 投影权重 (主人从 v1077 抽, per §2.1 + §8 R-018)** |
| 9 | `.github/workflows/r-measure-verify.yml` | 7 阶段阶段 6 实装 |
| 10 | `reports/r-measure-verify-<commit>-<date>.md` | R19+ 4 触发点 + R20 5 阶段 共 9+ 份 |
| 11 | ADR-00XX-apeireth-asi-stable-api (if needed, per §8 R-001) | 阶段 0 评估, 若 apeireth-asi 公开 API 不存在则写 |
| **12** | **`crates/apeireth-asi/src/lib.rs` 加 `pub trait AsiEngine` + `snapshot_24dim()` async fn** | **🆕 阶段 0 必加 (per spectrAI §3.3), 否则 verifier 调不动 24 维** |

### 11.3 引用 sub-agent 报告 (跟 Mavis 拍板有关)

| 报告 | 章节引用 | 关系 |
|------|---------|------|
| `spectrai-architecture-2026-08-05.md` | §6 (19 模块集成点) | 42 crate 集成背景 |
| `apeireth-crate-api-2026-08-05.md` | §2 (9 crate API surface) | 调 `apeireth-asi::AsiEngine` API 形态 |
| `apeireth-platform-modules-2026-08-05.md` | §7 (关键发现) | R20 API 公开跟 baseline 关系 |
| **`apeireth-session-vector-asi-2026-08-05.md`** | **§1.2 发现 1 (17→24 维纠正) + §3.2 (建议 B 投影) + §7.4 (口径不一致拍板)** | **🔥 R-Measure 17→24 维纠正依据 (per §2 🔥 修正)** |

---

## §12 报告完成

> **附**: 任何接手者快速路径:
> - **Mavis 拍板**: §1 战略 + §3 模块结构 + §8 风险
> - **rust-coder 实施**: §3 完整伪代码 + §7 7 阶段表 + §11.2 待写文档清单
> - **verifier 实装 CI**: §4 CI workflow + §6 4 触发点协同 + §4.4 失败处理
> - **devops 集成**: §4.2 5 重守门 + §4.3 报告路径
> - **主人 6 锚穿透检查**: §10
>
> 任何接手者按 §1-§11 顺序读, 60 分钟 100% 理解 R-Measure 验证脚本的"为什么 / 是什么 / 怎么干 / 何时跑 / 风险 / 不改什么 / 跟谁协同 / 关联文档".
>
> 本设计文档是后续 verifier / rust-coder 实施的**唯一依据**, 不写代码, 写完即交付.

## 拍板记录

- **2026-08-05 14:30** — sub-agent (software-analyst) 报告: R-Measure 当前是 **24 维** LOCKED V0.5 (per `crates/apeireth-asi/` R14 Rust rewrite, round10-12 完成), 17 维是 R11 baseline 历史口径 (R11 Python 生态 `v1077_asi_v04_full_measurement.py` V0.3/V0.4 时代). 17→24 维投影公式待主人从 `v1077_asi_v04_full_measurement.py` 抽权重, 写到 `fixtures/r11-projection.json` (per §2.1 占位表 + §8 R-018 防御). 本设计文档已按 🔥 修正 (2026-08-05) 回写: §2 24 维 LOCKED + §2.1 17→24 维映射占位表 + §3.3 trait 24 维 + §5 24 维 + §6 42 crate + §8 R-018 + §11 新增 3 条引用. 主人**待定**:
  1. **17→24 维权重抽取** (v1077 Python 源码) — per §2.1 + §8 R-018, P0 必先
  2. **V1136 9→7 子测度 R11 baseline 投影权重** (per §2.1 类比) — per §2.3 + §8 R-018, P0 必先
  3. **24 维具体分类名** (continuity / salience / identity / reflection / 4 other 的具体内容) — per §5.3 注释, 主人确认 `apeireth-asi::V05_DIMENSION_NAMES[24]` 后回填

---

_本设计文档由 Mavis 亲自产出 (按主人 2026-08-05 任务 "写 R-Measure baseline 验证脚本的设计文档")._
_7 阶段实施 + 9 风险 (8 原 + R-018) + 12 章节 + 6 锚穿透 + 编译期 hardcode 守门._
_🔥 修正 (2026-08-05): R-Measure 当前 24 维 LOCKED V0.5, 17 维是 R11 baseline 历史口径, 17→24 维映射见 §2.1 + §8 R-018 防御._
_主 17:58 不假装: 17 维 / 24 维 哪一维没填 = 0.0, baseline 漂移写 retro 不假装已通过._
_任何人接手按 §1-§11 顺序读, 60 分钟 100% 理解._
