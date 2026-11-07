//! git_write 结果面的 M1 凭据形状绊线接线 (批次 2)。
//!
//! 存在理由: `src/git_write.rs` 在 size 棘轮基线上 (`arch-baseline.txt`, 0 容差)
//! ⇒ 任何净增都会让 `arch_guard` 变红 ⇒ 正路是**让本文件净减** ⇒ 接线的实现
//! 与它的口径说明都落在这里 (标准模块布局: `src/git_write.rs` +
//! `src/git_write/tripwire.rs` ⇒ `mod tripwire;` 自动解析, 不需要改 `lib.rs`)。
//!
//! # 口径
//!
//! * git 的 stdout/stderr 是**工作区内容面** —— 提交信息 / 分支名 / 远端地址里
//!   的裸密钥形状必须遮形后才回到模型 (统一助手
//!   `crate::guardrail::scan_result_face` 的形状层);
//! * 覆盖面: `output` (git 的 stdout/stderr) + `commands` (命令回显 —— 提交信息
//!   里的凭据形状同样会**原样回显**在这两面 ⇒ 只扫 `output` 会漏掉命令回显那一面);
//! * 本族**已有** M3 治理层正则 (执行面里的 `redact_secret_surfaces`) ⇒ ⇒ 那是
//!   **另一层**, 本层是**叠加层** (覆盖面不同: M3 认 `sk-` / `AKIA` / `ghp_` 等
//!   值形, 形状层另认私钥块 / JWT / 令牌族 / 明文口令赋值 —— 两层共存, 谁都不漏);
//! * **零命中 ⇒ 逐字节不变** (不挂任何标记 ⇒ 干净答案的形状与字节都不变, 归一
//!   合同冻结的四字段形状也不变);
//! * **命中可观测**: 成功面是**对象** ⇒ 侧车挂在对象上 (字段口径对齐 shell
//!   结果面); 失败面是**裸文本** (协议定的错误消息) ⇒ 侧车以末尾一行可机读的
//!   JSON 注解附在末尾 (同 filesystem 读面的做法)。

use super::GradedSegment;
use crate::guardrail::{
    append_credential_tripwire, attach_credential_tripwire, scan_result_face, LeakedCredentialKind,
};

/// 成功面: 归一合同冻结的四字段报告 (命中时挂对象侧车)。
///
/// 零命中 ⇒ 报告与接线前**逐字节相同** (四字段, 无侧车)。
pub(super) fn success_value(
    tier: &str,
    segments: &[GradedSegment],
    output: &str,
) -> serde_json::Value {
    let output_scan = scan_result_face(output);
    let mut leaked_kinds: Vec<LeakedCredentialKind> = output_scan.leaked_kinds.clone();
    let mut commands = Vec::with_capacity(segments.len());
    for segment in segments {
        let command_scan = scan_result_face(&segment.display);
        leaked_kinds.extend(command_scan.leaked_kinds.iter().copied());
        commands.push(command_scan.text);
    }
    let mut value = serde_json::json!({
        "status": "executed",
        "tier": tier,
        "commands": commands,
        "output": output_scan.text,
    });
    attach_credential_tripwire(&mut value, &leaked_kinds);
    value
}

/// 失败面: 遮形后的失败帧 (命中时末尾挂一行 JSON 注解)。
///
/// 零命中 ⇒ 帧**逐字节不变** (不挂任何标记)。
pub(super) fn scan_failure(failure: &str) -> String {
    let scan = scan_result_face(failure);
    append_credential_tripwire(&scan.text, &scan.leaked_kinds)
}
