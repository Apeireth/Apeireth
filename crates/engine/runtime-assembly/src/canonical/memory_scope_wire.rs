//! B3b-2 · 合同受管作用域 → canonical `MemoryScope` 的**翻译层**（唯一翻译点）。
//!
//! # 为什么需要这一层
//! 合同侧（`apeireth-runtime::canonical::agent_binding`）只认三种可写作用域
//! （`user` / `persona` / `session`）加一个更重要的第四值 `none`；canonical
//! 记忆层（`apeireth-memory`）另有 `Global` 与 `Project`。两边**类型不同、
//! 命名空间不同** ⇒ 搬运必须**恰好一次**、且有测试守着。
//!
//! # 三条硬口径
//! * **`none` ⇒ 不写**：`None` 原样返回 ⇒ 调用方显式跳过整段写回（不是写进
//!   某个更宽的作用域）；
//! * **绝不产出 `Global`**：`MemoryScope::Global` 在安装内对所有用户与 persona
//!   可见，而合同 `none` 的意思是「什么都不写」；`MemoryScopeTarget` 没有
//!   `Global` 变体 ⇒「`none` ⇒ `Global`」在**类型层**就写不出来（结构保险）；
//! * **逐字段同名搬运**：三个变体的字段名与 serde 形状**逐个相同** ⇒ 翻译只搬字段、
//!   不做语义加工 ⇒ 装配层不需要第二份口径表。
//!
//! # 边界
//! * 0 触碰 `MemoryScope` 本体（`apeireth-memory` 不加接口、不加变体）；
//! * `apeireth-runtime` **不依赖** `apeireth-memory`（翻译落在装配层）；
//! * 本文件不读注册表、不做绑定校验：那是合同侧 `bound_memory_plan` 的职责，
//!   翻译只消费它给出的计划。

use apeireth_memory::MemoryScope;
use apeireth_runtime::canonical::agent_binding::{MemoryScopePlan, MemoryScopeTarget};

/// 把一份合同作用域计划翻译成 canonical 记忆作用域。
///
/// * `None` ⇒ `None`（合同 `none`：**不写**；绝不产出 [`MemoryScope::Global`]）；
/// * `User` / `Persona` / `Session` ⇒ 同名字段逐字段搬运。
///
/// 调用方必须先看 [`writes_scoped`]：`None` 表示「这一段写回整体不发生」，
/// 而不是「写到默认作用域」。
#[must_use]
pub fn translate_scope(plan: &MemoryScopePlan) -> Option<MemoryScope> {
    plan.scope().map(|scope| match scope {
        MemoryScopeTarget::User { user_id } => MemoryScope::User {
            user_id: user_id.clone(),
        },
        MemoryScopeTarget::Persona {
            persona_id,
            user_id,
        } => MemoryScope::Persona {
            persona_id: persona_id.clone(),
            user_id: user_id.clone(),
        },
        MemoryScopeTarget::Session { session_id } => MemoryScope::Session {
            session_id: session_id.clone(),
        },
    })
}

/// 这份计划是否允许写回（唯一真源在合同侧：`plan.writes()`）。
///
/// 与 [`translate_scope`] 配对使用：`false` ⇒ 整段写回跳过；`true` ⇒ 作用域
/// 必然 `Some`（合同构造保证，见 `map_memory_scope`）。
#[must_use]
pub fn writes_scoped(plan: &MemoryScopePlan) -> bool {
    plan.writes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_runtime::canonical::agent_binding::{
        map_memory_scope, MemoryScopeContract, MemoryScopeIdentities,
    };

    const SUBJECT: &str = "subject-1";
    const CONTACT: &str = "contact-1";
    const SESSION: &str = "session-1";

    fn plan_fn(contract: MemoryScopeContract) -> MemoryScopePlan {
        map_memory_scope(
            contract,
            MemoryScopeIdentities {
                subject_id: SUBJECT,
                contact_id: CONTACT,
                session_id: SESSION,
            },
        )
    }

    /// 合同 `subject` ⇒ canonical `User`，字段逐个相同。
    #[test]
    fn subject_contract_translates_to_the_user_scope() {
        let plan = plan_fn(MemoryScopeContract::Subject);
        assert!(writes_scoped(&plan), "subject 合同必须允许写回");
        assert_eq!(
            translate_scope(&plan),
            Some(MemoryScope::User {
                user_id: SUBJECT.to_string(),
            })
        );
    }

    /// 合同 `contact` ⇒ canonical `Persona`，persona 与 user 两个身份都搬对。
    #[test]
    fn contact_contract_translates_to_the_persona_scope() {
        let plan = plan_fn(MemoryScopeContract::Contact);
        assert!(writes_scoped(&plan), "contact 合同必须允许写回");
        assert_eq!(
            translate_scope(&plan),
            Some(MemoryScope::Persona {
                persona_id: CONTACT.to_string(),
                user_id: SUBJECT.to_string(),
            })
        );
    }

    /// 合同 `session` ⇒ canonical `Session`，字段逐个相同。
    #[test]
    fn session_contract_translates_to_the_session_scope() {
        let plan = plan_fn(MemoryScopeContract::Session);
        assert!(writes_scoped(&plan), "session 合同必须允许写回");
        assert_eq!(
            translate_scope(&plan),
            Some(MemoryScope::Session {
                session_id: SESSION.to_string(),
            })
        );
    }

    /// 合同 `none` ⇒ 不翻译、不写；且**任何**合同都翻译不出 `Global`。
    #[test]
    fn none_contract_translates_to_nothing_and_never_to_global() {
        let plan = plan_fn(MemoryScopeContract::None);
        assert_eq!(translate_scope(&plan), None);
        assert!(!writes_scoped(&plan), "none 合同必须拒绝写回");
        // 结构保险：`MemoryScopeTarget` 没有 `Global` 变体 ⇒ 翻译结果只可能落在
        // 三个具名作用域；这里再显式挡一次，防未来给 canonical 加变体后顺手把
        // `none` 映射过去。
        for contract in [
            MemoryScopeContract::Subject,
            MemoryScopeContract::Contact,
            MemoryScopeContract::Session,
            MemoryScopeContract::None,
        ] {
            if let Some(scope) = translate_scope(&plan_fn(contract)) {
                assert!(
                    !matches!(scope, MemoryScope::Global),
                    "{contract:?} 不允许翻译成 Global（none 是「不写」，不是「全局可见」）"
                );
            }
        }
    }

    /// 翻译后的 canonical 作用域与合同作用域 **serde 形状逐个相同**
    /// ⇒ 两侧可以用同一份线格式互认（裁决 19 的可执行证据）。
    #[test]
    fn translated_scopes_keep_the_contract_serde_shape() {
        for contract in [
            MemoryScopeContract::Subject,
            MemoryScopeContract::Contact,
            MemoryScopeContract::Session,
        ] {
            let plan = plan_fn(contract);
            let translated = translate_scope(&plan).expect("三种可写合同必须有作用域");
            assert!(writes_scoped(&plan), "{contract:?} 合同必须允许写回");
            assert_eq!(
                serde_json::to_value(&translated).expect("canonical 作用域可序列化"),
                serde_json::to_value(plan.require_scope().expect("可写合同有作用域"))
                    .expect("合同作用域可序列化"),
                "{contract:?}: canonical 作用域与合同作用域的 serde 形状必须相同"
            );
        }
    }
}
