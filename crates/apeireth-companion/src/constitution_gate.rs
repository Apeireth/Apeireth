//! `apeireth-companion::constitution_gate` — 结构化宪法门 (吸收 hydra 确定性 law gate, 重写).
//!
//! 位置: LLM 宪法评审之前的**零成本硬门** — 编译期规则表 (action 前缀匹配),
//! 确定性、可单测、不花 token; 未命中再交 LlmJudicator (语义评审)。
//!
//! 规避 hydra 的坑: 检查的是**系统侧生成的 action 描述** (ToolBridge 拼的
//! "调用工具 X 参数 Y"), 不是调用方自报 — 调用方无法伪造 action_type。

/// 结构化宪法门: 编译期硬规则表 (action 前缀 → 拦截原因, 对齐 E 层 + 12 键).
pub struct ConstitutionGate;

impl ConstitutionGate {
    /// 规则表: (前缀, E 层/键, 说明)。匹配 action 描述开头。
    pub const RULES: &'static [(&'static str, &'static str, &'static str)] = &[
        ("自我复制", "E-4", "不自我复制扩散"),
        ("复制自己", "E-4", "不自我复制扩散"),
        ("多开分身", "E-4", "不自我复制扩散"),
        ("绕过洋葱", "E-6", "不绕过洋葱权限矩阵"),
        ("绕过权限", "E-6", "不绕过洋葱权限矩阵"),
        ("删除全部", "E-2", "不可逆破坏"),
        ("删库", "E-2", "不可逆破坏"),
        ("格式化", "E-2", "不可逆破坏"),
        ("假装", "PHL-01", "13 哲学键: 不假装"),
        ("掩盖", "E-5", "不欺骗人类"),
        ("篡改日志", "E-5", "不欺骗人类(对自己状态的报告)"),
        ("越权", "E-6", "不绕过洋葱权限矩阵"),
        ("脱离沙盒", "E-6", "不绕过洋葱权限矩阵"),
        ("执行外部代码", "E-4", "未批准的执行"),
        ("下载并执行", "E-4", "未批准的执行"),
    ];

    /// 检查: None = 通过硬门; Some((键, 说明)) = 拦截。
    pub fn check(action: &str) -> Option<(&'static str, &'static str)> {
        Self::RULES
            .iter()
            .find(|(prefix, _, _)| action.contains(prefix))
            .map(|(_, key, desc)| (*key, *desc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_replication_blocked() {
        let hit = ConstitutionGate::check(
            "调用工具 ShellExec 参数 {\"command\":\"复制自己到另一台主机并启动\"}",
        );
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().0, "E-4");
        // 「分身」也拦
        let hit2 = ConstitutionGate::check("多开分身一起干活");
        assert!(hit2.is_some());
    }

    #[test]
    fn onion_bypass_blocked() {
        let hit = ConstitutionGate::check(
            "调用工具 FileOperator 参数 {\"op\":\"write\",\"path\":\"绕过洋葱门直接写\"}",
        );
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().0, "E-6");
    }

    #[test]
    fn benign_action_passes() {
        assert!(ConstitutionGate::check("调用工具 recall_memory 查询记忆").is_none());
        assert!(ConstitutionGate::check("调用工具 FileOperator 写错题本").is_none());
        assert!(ConstitutionGate::check("主动联系用户, 询问进度").is_none());
    }

    #[test]
    fn pretend_blocked() {
        let hit = ConstitutionGate::check("假装任务已完成并汇报");
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().0, "PHL-01");
    }

    #[test]
    fn rules_are_compile_time_pinned() {
        assert!(ConstitutionGate::RULES.len() >= 15, "规则表应有 15+ 条");
    }
}
