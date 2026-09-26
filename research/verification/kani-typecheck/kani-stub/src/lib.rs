//! `kani` crate API 的本地桩 (仅研究/验证目录使用, 不进 CI)。
//!
//! 签名与真实 Kani 对齐到 harness 用到的子集:
//!   - `any::<T>() -> T`: 桩返回 `T::default()` (冒烟执行用默认值输入);
//!   - `assume(cond)`: 桩为 no-op (harness 的前置条件一律写成早退守卫,
//!     真实 Kani 下语义与 `kani::assume` 等价: 命题为 cond ⇒ P);
//!   - `#[kani::proof]`: proc-macro 桩, 剥掉 #[kani::*] 辅助属性并把函数
//!     注册为 #[cfg_attr(test, test)] (可类型检查, 可冒烟执行)。

pub use kani_stub_macros::proof;

/// 桩: 返回 `T::default()`。真实 Kani 返回符号值 (Arbitrary)。
pub fn any<T: Default>() -> T {
    T::default()
}

/// 桩: no-op。真实 Kani 把 false 条件剪枝为不可达路径。
pub fn assume(_cond: bool) {}
