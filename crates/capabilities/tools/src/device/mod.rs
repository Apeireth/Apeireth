//! K 轴 · 设备受控执行骨架 (L3 受控执行 / L4 代理代办) —— 「它没有手, 但它有插头」。
//!
//! 本目录承载 K 轴 L3/L4 的**骨架件** (见
//! `docs/04-internal/backend-enhancement/K-life-presence.md` 四条安全底线 Q41-Q44):
//!
//! - **三拍执行** (底线④ 先模拟后执行): `plan` (设备+动作+参数 → 执行计划帧) →
//!   `preview` (mock 适配器投影"将发生什么") → `execute` (真执行 = 适配器调用 +
//!   状态账)。plan / preview **零副作用**;
//! - **动作三档分级** (底线②③): `reversible` (审批档, 可配自动档) /
//!   `irreversible` (永远逐次人批) / `money` (永远逐次人批 + IM 卡);
//! - **设备白名单** (底线①): 只对 `enabled` 设备动作, 未登记设备拒绝即帧,
//!   默认零设备;
//! - **mock 适配器**: 内置假设备 (灯 / 空调 / 微波炉 fixture) 真走三拍 + 状态账
//!   (`device-actions.jsonl` append-only: 何时 / 何设备 / 何动作 / 谁批);
//! - **开关**: `APEIRETH_ENABLE_DEVICE_CONTROL` 依赖 `APEIRETH_ENABLE_DEVICE_SENSE`
//!   双闸齐开 (默认关 fail-closed)。
//!
//! 模块布局 (K1 感知件与 L3/L4 件同目录并行落位, 各建各件、互不覆盖):
//!
//! - [`device_control`]: 受控执行骨架 (本批, K 轴 L3/L4)。
//! - [`approval`]: 设备审批卡→IM 推送链的两本账 (推送账 `device-approval-pushes.jsonl`
//!   + 按钮决定账 `device-approval-decisions.jsonl`: 推送幂等键 = 动作 plan_id /
//!   失败留账待重试 / 一计划一决定, 执行语义仍在 [`device_control`] 状态账)。
//! - [`registry`] / [`adapter`] / [`sense`] / [`audit`] / [`weather`]: K1 在场感知
//!   骨架件 (K 轴 L1/L2 批, 落位登记) —— 设备表 (`devices.json` StoredDoc 拒开,
//!   白名单默认零设备) + 适配器插槽图 (mock 已接固定 fixture, 真实协议族留插槽
//!   不接) + `tool.device_sense` (摄像头类敏感读面: 显式授权 + 审计) +
//!   `tool.weather` (mock 档如实标注数据源 + 提醒信号只交信号)。
//! - [`guard_watch`]: 守护骨架件 (K 轴 L5 批) —— 守护规则面 (`guard-rules.json`
//!   StoredDoc 拒开, 规则 = 设备状态阈值 / 时序条件) + 告警账
//!   (`guard-alerts.jsonl` append-only) + `tool.guard_watch` (**只看不动**:
//!   告警 = 关怀候选高优先, 零设备动作); 开关 `APEIRETH_ENABLE_GUARD_WATCH`
//!   依赖 [`sense::DEVICE_SENSE_ENABLE_ENV`] 双闸齐开, 默认关 fail-closed。
//!
//! (K1 落位登记: 白名单注册表在 [`registry`] 内自持, [`device_control`] 可只读
//! 消费; 感知族总开关 `APEIRETH_ENABLE_DEVICE_SENSE`
//! ([`sense::DEVICE_SENSE_ENABLE_ENV`]), 默认关 fail-closed。)
pub mod adapter;
pub mod approval;
pub mod audit;
pub mod device_control;
pub mod guard_watch;
pub mod registry;
pub mod sense;
pub mod weather;
