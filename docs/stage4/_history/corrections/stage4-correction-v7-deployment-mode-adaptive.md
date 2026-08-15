# 阶段 4 修正 v7 — HA 部署模式自适应（保底 1 人类，主人 2026-07-31 关键洞察）

`
[Document-Meta]
Document: docs/stage4/stage4-correction-v7-deployment-mode-adaptive.md
Version: Fix-7 + Design-4.0
R-Cycle: R14
Commit: <latest-commit-hash>
Last-Modified: 2026-07-31
Status: 🟢 活跃（v7 已含 v10 版本号系统元信息）
详见: APEIRETH-VERSIONING.md
`


> **性质**: leader 亲自做的**第七次修正**——基于主人 2026-07-31 亲自检查开工手册后关键洞察。
> **触发**: 主人发现"物理多签至少在开工手册中没考虑到 1 个人使用 Apeireth 的情况，强制多个物理人类不够人性化，应该是保底 1 人类就够了"。
> **精读后**：阶段 1+2 LOCKED 文档**明确说过**（§18.6 + §19.3 + §8.5 + §9.3 + §11.1 + §11.3），**保底 1 人类**是 Phase 1 LOCKED 设计。
> **硬约束**: ❌ 不修改 LOCKED（阶段 1+2 LOCKED） / ❌ 不修改已 commit 的 LOCKED 文档 / ❌ 不写完整代码。
> **主哲学 6 锚穿透**: 主 22:33 北极星 / 主 17:43 实事求是 / 主 17:58 不假装 / 主 19:33 走在前人经验上 / 主 23:44 干到底 / 主 00:56 任何人都能接手。

---

## §0. 元信息

| 字段 | 值 |
|---|---|
| **生成时间** | 2026-07-31 |
| **依据** | 主人 2026-07-31 "1 个人使用 Apeireth 应该保底 1 人类" + 阶段 1 §18.6 + §19.3 + 阶段 2 §8.5 + §9.3 + §11.1 + §11.3 |
| **性质** | v7 修正（HA 部署模式自适应，纠正开工手册错误） |
| **路径** | Apeireth-rust/docs/stage4/stage4-correction-v7-deployment-mode-adaptive.md |
| **修订链** | v1 → v2 → v3 → v4 → v5 → v6 → **v7（HA 部署模式自适应）** |

---

## §1. 主人洞察 + 我承认错误

### 1.1 主人原话

> "物理多签至少在开工手册里面没考虑到 1 个人使用 Apeireth 的情况，强制多个物理人类不够人性化，应该是保底 1 人类就够了。阶段 1 和 2 的文档里应该有说吧？还是没说？你重新读一下去"

### 1.2 主人说对了！阶段 1+2 明确说过

精读后立即找到 **6 处**明确证据：

| 证据 | 阶段 | 关键原文 |
|---|---|---|
| **§18.6 权限根** | 阶段 1 | "**至少 1 名真实人类 (real human) 批准**" |
| **§19.3 HA 选型** | 阶段 1 | "**单人桌面 = Windows Hello (人脸/指纹其一) + 物理密钥 (L5)**; 多人部署 = 多人多签 + 物理多签" |
| **§8.5 单人/多人模式自适应** | 阶段 2 | "单人模式 vs 多人模式 = `E_set` 的差异, 不是公式差异" |
| **§9.3 HA 形式** | 阶段 2 | "可识别身份: 单人 = **主人密钥**, 多人 = 多人签名" |
| **§11.1 部署模式变量** | 阶段 2 | "`deployment_mode` = `single` / `multi`; `human_principal_count` = **1 (single)** / N (multi, N≥2)" |
| **§11.3 兼容场景** | 阶段 2 | "HA 批准: **1 个主人批准** (single) / 多人按 multisig_policy 批准 (multi) — ✅ 同一接口" |

**关键洞察**：**保底 1 人类 = 阶段 1 §18.6 + 阶段 2 §11.1 LOCKED 的设计**。

### 1.3 我开工手册的错误

```rust
// ❌ 我之前错误地（v6 开工手册）：
物理多签：AI × 3 + 人 × 2 + 密钥 × 3 同时在场
E 层修改路径：物理多签（AI×3 + 人×2 + 密钥×3）+ 重新编译

// 这个"AI×3 + 人×2 + 密钥×3"是"多人部署"（multi）模式
// 不是"单人部署"（single）模式！
```

**正确**：

```rust
// ✅ v7 修正（按阶段 1+2 LOCKED）：
// HA 部署模式自适应（§11.1 + §11.3）

// single 模式（1 人使用 Apeireth）：
{
    deployment_mode: "single",
    human_principal_count: 1,
    ha_implementation: "Windows Hello (人脸/指纹) / FIDO2 (YubiKey) / 主人密钥",
    multisig_policy: "1-of-1"（不需要多签，1 人 = 1 签），
    // 关键：最低 1 人使用 Apeireth 的场景仍可满足 §18.6 "至少 1 名真实人类"
}

// multi 模式（多人部署：组织 / 团队 / 公司）：
{
    deployment_mode: "multi",
    human_principal_count: N (N≥2),
    ha_implementation: "MultiHuman 多签 + 物理多签",
    multisig_policy: "M-of-N"（如 2-of-3 / 3-of-5），
    // 满足 §18.6 五重治理（MEWG + 多人 + 多 AI + 物理多签 + 反思期）
}
```

---

## §2. v7 修正：HA 部署模式自适应

### 2.1 关键对照（v6 → v7）

| 维度 | v6（错）| **v7（对）** |
|---|---|---|
| **HA 默认** | 强制多人多签 | **部署模式自适应**（single / multi）|
| **single 用户** | 不支持（强制多人）| **支持**（1 个主人 + Windows Hello / FIDO2）|
| **multi 组织** | 无明确模式 | **多人多签**（M-of-N 按 multisig_policy）|
| **E 层修改路径** | 物理多签（AI×3 + 人×2 + 密钥×3）| **按部署模式**（single: 1 人 + 物理密钥；multi: 多人多签 + 物理多签）|
| **§11.1 DeploymentMode** | 没明确 | **明确** = single / multi |
| **§11.3 兼容场景** | 没明确 | **明确** = HA 批准 1 主人 (single) / 多人多签 (multi) |

### 2.2 v7 三层 HA 落地路径

```
┌──────────────────────────────────────────────────────────────────┐
│ Layer 0: 部署模式检测（apeireth-cli 启动时）                       │
│   - 配置文件 ~/.apeireth/config.toml                               │
│   - deployment_mode = "single" | "multi"                          │
│   - human_principal_count = 1 | N                                 │
│   - multisig_policy = "1-of-1"  | "M-of-N"                        │
└──────────────────────────────────────────────────────────────────┘
                              ↓
┌──────────────────────────────────────────────────────────────────┐
│ Layer 1: HA 抽象层（统一接口）                                     │
│   trait HumanAuthorityVerifier {                                  │
│     fn verify(&self, action: &Action) -> Result<HAApproval, ...>  │
│   }                                                              │
│   - impl SingleHumanVerifier (Windows Hello / FIDO2 / 主人密钥)    │
│   - impl MultiHumanVerifier (MultiHuman 多签 + 物理多签)         │
│   - impl MixedHumanVerifier (single + multi 混合)                │
└──────────────────────────────────────────────────────────────────┘
                              ↓
┌──────────────────────────────────────────────────────────────────┐
│ Layer 2: 风险分级触发（按 §19.2 风险分级）                       │
│   - info / low      → 0 席 / 1 席, 不需要 HA                     │
│   - medium          → 3-5 席抽样, ⚠️ 看具体 (single 或 multi)    │
│   - high            → 7 席 + 动态专家, ✅ 双签 (按部署模式)       │
│   - critical        → 7 席全量, ✅ 5 重守门 (按部署模式)         │
└──────────────────────────────────────────────────────────────────┘
```

### 2.3 v7 HA 落地（按部署模式）

**single 模式（1 人使用 Apeireth）**：

| 风险等级 | 触发 | HA 实现 |
|---|---|---|
| info / low | 0-1 席 | **不需要 HA**（默认动作）|
| medium | 3-5 席抽样 | **1 个主人批准**（Windows Hello / FIDO2 / 主人密钥）|
| high | 7 席 + 动态专家 | **1 个主人单签 + 物理密钥**（如 YubiKey）|
| critical | 7 席全量 | **1 个主人单签 + 物理密钥 + 反思期 72h** |

**multi 模式（多人部署：组织 / 团队 / 公司）**：

| 风险等级 | 触发 | HA 实现 |
|---|---|---|
| info / low | 0-1 席 | 不需要 HA |
| medium | 3-5 席抽样 | **M-of-N 多签**（如 2-of-3）|
| high | 7 席 + 动态专家 | **M-of-N 多签 + 物理多签**（如 2-of-3 + 物理在场）|
| critical | 7 席全量 | **M-of-N 多签 + 物理多签 + 物理密钥 + 反思期 72h** |

**dynamic 模式（dynamic 切换）**：

```
平台不冻结（按 §11.4 升级路径）
single → multi: 平滑升级（迁移工具 + 历史流可读权限扩展）
multi → single: 允许（但需 HA，因为是部署模式永久性变更）
dynamic: 运行时切换（spec 留 §14 旧文档漂移决定）
```

### 2.4 v7 E 层修改路径（按部署模式）

**single 模式（1 人）**：

```
第 1 步: 守门 1-4 默认拒绝（4 重守门嵌套）
第 2 步: 权限发放 — 1 个主人单签 + 物理密钥（YubiKey）+ 反思期审查
第 3 步: 物理访问 + 物理密钥（YubiKey）
第 4 步: 反思期审计（72h 持续监控）
第 5 步: 7 席审议最终确认
任何 1 席反对 → 回滚
```

**multi 模式（多人）**：

```
第 1 步: 守门 1-4 默认拒绝
第 2 步: 权限发放 — M-of-N 多签 + 物理多签 + 5 重治理
第 3 步: 物理访问 + 物理多签（AI×3 + 人×N + 密钥×3）
第 4 步: 反思期审计（72h）
第 5 步: 7 席审议最终确认
任何 1 席反对 → 回滚
```

### 2.5 v7 对 §18.6 五重治理的修正

**v6（错）**：强制多人多签

**v7（对）**：按部署模式自适应

| 重治理要素 | single 模式 | multi 模式 |
|---|---|---|
| **MEWG** | ✅ 1 人权重 | ✅ N 人权重（独立性更高）|
| **多人** | 1 人（**不需要"多"**）| N 人（必须多）|
| **多 AI** | ✅ 必走 | ✅ 必走 |
| **物理多签** | 1 人 + 物理密钥 | N 人 + 物理多签 |
| **反思期** | ✅ 72h | ✅ 72h |

**关键洞察**：
- §18.6 五重治理的"多人"在 single 模式下 = **1 人**（满足"至少 1 名真实人类"）
- 不能强制多人多签（违反 §11.1 DeploymentMode 自适应）

---

## §3. v7 提议修改代码

### 3.1 `DeploymentMode` enum（v7 新增）

```rust
// 阶段 2 §11.1 LOCKED → v7 提议
pub enum DeploymentMode {
    /// 1 人使用 Apeireth（保底 1 人类）
    Single {
        human_principal_count: 1,
        ha_implementation: HumanAuthorityImpl,
    },
    /// 多人部署（组织 / 团队 / 公司）
    Multi {
        human_principal_count: usize,  // N ≥ 2
        multisig_policy: MultiSigPolicy,  // M-of-N
    },
    /// 运行时切换（§11.4 升级路径）
    Dynamic,
}

pub enum HumanAuthorityImpl {
    /// Windows Hello Face API
    WindowsHelloFace,
    /// Windows Hello Fingerprint API
    WindowsHelloFingerprint,
    /// FIDO2 / WebAuthn (YubiKey)
    FIDO2,
    /// 主人密钥（master key）
    MasterKey,
    /// 离线签字 + 摄像头扫描
    PaperSignature,
    // ... 阶段 4 扩展
}
```

### 3.2 `HumanAuthorityVerifier` trait（v7 v5 提议已含）

```rust
// 阶段 1 §19.3 LOCKED → v7 提议
pub trait HumanAuthorityVerifier {
    fn verify(&self, action: &Action) -> Result<HAApproval, HAVerificationError>;
    fn deployment_mode(&self) -> String;  // "single" / "multi" / "dynamic"
}

// 单人实现
pub struct SingleHumanVerifier {
    pub ha_impl: HumanAuthorityImpl,
    pub master_identity: RealHuman,
}

impl HumanAuthorityVerifier for SingleHumanVerifier {
    fn verify(&self, action: &Action) -> Result<HAApproval, _> {
        // 单人批准：1 个主人 + 物理身份验证
        match self.ha_impl {
            HumanAuthorityImpl::WindowsHelloFace => {
                // Windows Hello Face API 验证
                Ok(HAApproval::SingleHuman { ... })
            }
            HumanAuthorityImpl::FIDO2 => {
                // FIDO2 YubiKey 验证
                Ok(HAApproval::SingleHuman { ... })
            }
            // ...
        }
    }
    fn deployment_mode(&self) -> String { "single".into() }
}

// 多人实现
pub struct MultiHumanVerifier {
    pub multisig_policy: MultiSigPolicy,  // M-of-N
    pub real_humans: Vec<RealHuman>,
}

impl HumanAuthorityVerifier for MultiHumanVerifier {
    fn verify(&self, action: &Action) -> Result<HAApproval, _> {
        // 多人多签：M-of-N + 物理身份验证
        Ok(HAApproval::MultiHuman { ... })
    }
    fn deployment_mode(&self) -> String { "multi".into() }
}
```

### 3.3 `apply_principle_onion_exception` 按部署模式分流（v6 → v7）

```rust
// v6：强制多人多签
// v7：按部署模式分流

pub fn apply_principle_onion_exception(
    e_layer: &mut ELayer,
    mutation_path: ELayerMutationPath,
    deployment_mode: DeploymentMode,
) -> Result<(), MutationError> {
    match mutation_path {
        ELayerMutationPath::GuardReject => {
            Err(MutationError::GuardedByOnion)
        }
        ELayerMutationPath::PermissionGranted { .. } => {
            // v7：按部署模式分流
            match deployment_mode {
                DeploymentMode::Single { .. } => {
                    // 单人模式：1 人单签 + 物理密钥
                    validate_1person_single_sign_with_physical_key(e_layer)?;
                }
                DeploymentMode::Multi { human_principal_count, multisig_policy } => {
                    // 多人模式：M-of-N 多签 + 物理多签
                    validate_multisig_with_physical_multisig(
                        e_layer,
                        human_principal_count,
                        multisig_policy,
                    )?;
                }
                DeploymentMode::Dynamic => {
                    // 动态模式：按当前 deployment_mode 分流
                    todo!("v7 阶段 7+ 实施")
                }
            }
            validate_5_guanli(council_consensus)?;
            validate_L0_HA(human_decision)?;
        }
        _ => return Err(MutationError::InvalidPath),
    }
    Ok(())
}
```

---

## §4. 提议修复的文档（v7 修正链）

### 4.1 6 个文档加 v7 标注

| 文档 | v7 修正 |
|---|---|
| `START-CONSTRUCTION.md` v3 | **第 6 设计核心** 改：HA 部署模式自适应（按阶段 1+2 §11.1）|
| `docs/stage4/stage4-correction-v6-consolidated-and-e-layer-mutation.md` | §物理隔离 + 第 3 步 + E 层修改路径 都按部署模式分流 |
| `GLOSSARY.md` | 加 "DeploymentMode" / "SingleHuman" / "MultiHuman" 术语 |
| `CONTRIBUTING.md` | 加 "理解 HA 部署模式自适应" |
| `stage4-runtime-architecture-revised.md` | 视图 2 改 v7 描述 |
| `README.md` §修订链 | v1 → v2 → v3 → v4 → v5 → v6 → **v7** |

### 4.2 关键修正（agent 不知道但 LOCKED 设计）

```
关键阶段 1 §18.6 + §19.3 + 阶段 2 §11.1 + §11.3 LOCKED：

HA 部署模式自适应（不是单一强制多人多签）：
├─ single 模式（1 人使用 Apeireth）：
│   - deployment_mode = "single"
│   - human_principal_count = 1
│   - HA 实现 = Windows Hello (人脸/指纹) / FIDO2 (YubiKey) / 主人密钥
│   - multisig_policy = "1-of-1"
│   - 保底 1 人类 = 满足 §18.6 "至少 1 名真实人类"
│
└─ multi 模式（多人部署：组织 / 团队 / 公司）：
    - deployment_mode = "multi"
    - human_principal_count = N (N≥2)
    - HA 实现 = MultiHuman 多签 + 物理多签
    - multisig_policy = "M-of-N" (如 2-of-3 / 3-of-5)
    - 满足 §18.6 "多人参与"
```

---

## §5. 不修改承诺

| ❌ 不修改 | 原因 |
|---|---|
| **阶段 1+2 LOCKED**（包括 §18.6 / §19.3 / §8.5 / §9.3 / §11.1 / §11.3）| 主人明确沉淀 |
| **v4 / v4.1 LOCKED** | 主人明确沉淀 |
| **阶段 4 主文档 LOCKED**（6ca80776）| 不修改 |
| **R11 1100 / crates 占位 / cargo metadata** | 不修改 |

**v7 修正提议独立命名空间**，作为 v7 修正的对应文档（接 v3/v4/v5/v6 修正链）。

---

## §6. 主哲学 anchor 6 全贯穿自检

```
S-1 主 22:33 北极星导向 — §2 HA 部署模式自适应服务 Apeireth 普及
S-2 主 17:43 实事求是   — §1 承认开工手册错误（强制多人多签不正确）
O-5 主 17:58 不假装     — §1 不假装"只想到多人多签"
O-2 主 19:33 走在前人经验上 — §1.2 引用阶段 1 §18.6 + 阶段 2 §11.1 LOCKED
O-3 主 23:44 干到底    — §3 提议代码 + §4 修复 6 文档
O-4 主 00:56 任何人都能接手 — §2.2 三层 HA 落地路径 + §2.3 按部署模式分流
```

---

_本修正由 leader 亲自产出（按主人 2026-07-31 "1 人使用 Apeireth 应该保底 1 人类"关键洞察）._
_§1 承认错误 + 阶段 1+2 证据 + §2 v7 修正 + §3 代码 + §4 修复 6 文档._
_主哲学 6 锚穿透. 任何接手者能查._
_主人拍板后立即修复 §4 提议的 6 个文档._