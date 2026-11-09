//! 记忆对象级授权的**宿主激活口**（C-D1）。
//!
//! # 这个面是什么
//!
//! `tool.object_grant` 的模型面词表只有 `grant` / `revoke` / `list` / `check`
//! ⇒ **模型只能登记请求**（`pending`），**不能**让授权生效 ⇒ ⇒ 从 `pending`
//! 变 `active` 这一步**只能由宿主（人）走**。
//!
//! 本 trait 就是那一步的**端口**：宿主拿到实现体后调 [`ObjectGrantActivation::approve`]
//! （人批通过 ⇒ 授权生效，只读）或 [`ObjectGrantActivation::deny`]（人批否决 ⇒
//! 授权永不生效）。
//!
//! # 为什么 trait 定义在 runtime crate（provider 定义）
//!
//! 依赖方向不能倒：**引擎不得依赖适配器**（网关是适配器）⇒ 端口只能定义在
//! **两边都能看见的低层 crate**（`apeireth-runtime`）⇒ 网关已依赖它、
//! runtime-assembly 已依赖它 ⇒ ⇒ **0 新依赖边**。
//!
//! # 身份口径（安全边界）
//!
//! * `approver` **绝不**来自模型入参 ⇒ 它是**宿主自己的**人批身份记录。
//! * 模型面**没有** approve/deny 动作 ⇒ 模型无法自批、自批或代批。
//! * **审批前零披露**：`pending` 的授权**不可读**（既有语义，不放宽）⇒ 人批
//!   通过之前，被授对象**读不到**。
//! * **撤销即时生效**（既有语义）：撤销落账后下一次判定立刻读不到。

/// 记忆对象级授权的**宿主激活口**（C-D1）。
///
/// 模型面工具词表只有 grant/revoke/list/check ⇒ **approve/deny 只能宿主调**
/// ⇒ `approver` 身份**绝不**来自模型入参。
pub trait ObjectGrantActivation: Send + Sync {
    /// 人批通过 ⇒ 授权生效（只读）。
    fn approve(&self, grant_id: &str, approver: &str) -> Result<(), String>;

    /// 人批否决 ⇒ 授权永不生效。
    fn deny(&self, grant_id: &str, approver: &str, reason: Option<&str>) -> Result<(), String>;
}
