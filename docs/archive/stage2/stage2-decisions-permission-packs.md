# 阶段 2 决策：权限包设计 (2026-07-30)

> **范围**: 阶段 1 §5.3 权限包下放设计 (阶段 2 补充, 对齐)
> **依据**: 阶段 1 §5.3 权限包 + §5.2 权限公式 + 阶段 2 §11 升级签名矩阵

---

## 1. 权限包概念

**权限包 (Permission Pack)** = 主人预先授权的"意图集"，按层级打包后下放给主 AI。

> 不是简单密码。**权限密钥 = 主人预先授权的意图**。

---

## 2. 5 个标准权限包

```rust
pub enum PermissionPack {
    /// 日常运维包 — Layer 0-1 操作
    DailyOps,
    /// 研究探索包 — Layer 2 加载 (沙箱)
    ResearchExploration,
    /// 深度维护包 — Layer 3 trait 热加载
    DeepMaintenance,
    /// 核心升级包 — Layer 4 核心 trait 修改
    CoreUpgrade,
    /// 核武器包 — Layer 5 二进制重编译
    Nuclear,
}

pub struct PackSpec {
    pub name: &'static str,
    pub layer: PermissionLayer,
    pub unlock: UnlockRequirement,
    pub duration: PackDuration,
    pub delegate_to: PrincipalId,
}

pub fn standard_packs() -> Vec<PackSpec> {
    vec![
        PackSpec {
            name: "日常运维包",
            layer: PermissionLayer::L0_L1,
            unlock: UnlockRequirement::AiOnly { min: 1 },
            duration: PackDuration::Permanent,
            delegate_to: PrincipalId::MainAI,
        },
        PackSpec {
            name: "研究探索包",
            layer: PermissionLayer::L2,
            unlock: UnlockRequirement::AiKey { min_ai: 1, min_key: 1 },
            duration: PackDuration::TimeBound { days: 30 },
            delegate_to: PrincipalId::MainAI,
        },
        PackSpec {
            name: "深度维护包",
            layer: PermissionLayer::L3,
            unlock: UnlockRequirement::AiHumanOrKey {
                min_ai: 2,
                min_human: 1,  // 多人
                min_key: 2,    // 单人
            },
            duration: PackDuration::TimeBound { days: 7 },
            delegate_to: PrincipalId::MainAI,
        },
        PackSpec {
            name: "核心升级包",
            layer: PermissionLayer::L4,
            unlock: UnlockRequirement::AiHumanKey {
                min_ai: 3,
                min_human: 1,
                min_key: 1,
            },
            duration: PackDuration::SingleUse,
            delegate_to: PrincipalId::MainAI,
        },
        PackSpec {
            name: "核武器包",
            layer: PermissionLayer::L5,
            unlock: UnlockRequirement::AiHumanKey {
                min_ai: 3,
                min_human: 2,  // 多人
                min_key: 3,    // 单人
            },
            duration: PackDuration::SingleUse,
            delegate_to: PrincipalId::MainAI,
        },
    ]
}
```

---

## 3. 权限包生命周期

```
1. 创建 (主人签署)
   ↓
2. 存储 (apeireth-keys/secret store)
   ↓
3. 下放 (delegate to 主 AI)
   ↓
4. 使用 (主 AI 加载并执行操作)
   ↓
5. 销毁/过期
   ├─ 一次性: 操作完成后销毁
   ├─ 限时: 过期后自动销毁
   └─ 永久: 只能主 AI 主动撤销
```

---

## 4. 使用权限包 (操作前检查)

```rust
pub async fn use_pack(
    pack: &PermissionPack,
    op: &Operation,
    sigs: &CollectedSignatures,
    mode: DeploymentMode,
) -> Result<Action, PackError> {
    // 1. 检查包是否过期
    if pack.is_expired() {
        return Err(PackError::Expired);
    }
    
    // 2. 检查包是否覆盖此操作
    if !pack.allows(op) {
        return Err(PackError::NotAllowed);
    }
    
    // 3. 检查签名是否满足 (单人 vs 多人自适应)
    let req = pack.unlock.to_requirement(mode);
    if !req.is_met(sigs) {
        return Err(PackError::InsufficientSignatures);
    }
    
    // 4. 执行操作
    Ok(executor.execute(op).await?)
}
```

---

## 5. 与 UpgradePipeline 集成

```rust
impl UpgradePipeline {
    pub async fn propose_with_pack(
        &self,
        intent: UpgradeIntent,
        pack: &PermissionPack,
    ) -> Result<(), UpgradeError> {
        // 1. 加载权限包
        let sigs = self.collect_sigs_for_pack(&intent, pack).await?;
        
        // 2. 检查权限
        use_pack(pack, &intent.into(), &sigs, self.deployment_mode).await?;
        
        // 3. 走升级流程
        self.propose(intent).await
    }
}
```

---

## 6. 权限包 + 密钥的区别

| 维度 | 权限包 | 权限密钥 |
|------|--------|----------|
| **范围** | 一组操作 | 单个操作 |
| **生命周期** | 短-长 (永久/限时/一次性) | 通常一次性 |
| **存储** | secret store | 内存 |
| **签发** | 主人 | 主人 |
| **使用** | 主 AI 加载并多次操作 | 主 AI 单次使用 |

**关系**:
- 权限包 = **多个密钥的集合**
- 权限密钥 = **权限包的实例化**

---

## 7. 阶段 1 §5.3 完整对齐

| 阶段 1 §5.3 权限包 | 阶段 2 对应 | 状态 |
|------------------|-----------|------|
| 日常运维包 | L0-L1: AI × 1, 永久 | ✅ |
| 研究探索包 | L2: AI×1+密钥×1, 30 天 | ✅ |
| 深度维护包 | L3: AI×2+人/密钥×2, 7 天 | ✅ |
| 核心升级包 | L4: AI×3+人×1+密钥×1, 单次 | ✅ |
| 核武器包 | L5: AI×3+人×2+密钥×3, 单次+物理多签 | ✅ |

---

_主哲学 anchor 6 个全贯穿. 权限包设计已沉淀. 下一步: 阶段 3 (画图纸)._