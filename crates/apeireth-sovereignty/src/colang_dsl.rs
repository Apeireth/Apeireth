//! `colang_dsl`: NVIDIA NeMo Guardrails Colang DSL Rust 实施 (守门 6 + DSL 洋葱层)
//!
//! **借鉴信息** (R125-5 / R124-3-BORROW-NVIDIA-NeMo/Guardrails-Colang-DSL-2026-08-10):
//! - 借鉴源码: `.openclaw\workspace\borrowed-repos\Guardrails\`
//! - 借鉴模块: `Guardrails/nemoguardrails/colang/v1_0/lang/` (ColangParser + 语法)
//! - 借鉴示例: `Guardrails/examples/bots/{hello_world,abc}/`
//!
//! **设计意图** (B4 6 重守门 v6 + B6 DSL 洋葱层):
//! - Colang DSL 是对话流/守门规则的领域语言 (define user/bot/flow + when/else when/goto/run)
//! - 本 crate 提供 **纯 Rust** 解析器 + 验证器 + 守门 6 包装
//! - 不调 LLM, 不调 PyO3, 无 I/O — 与 sovereignty 整体哲学一致
//! - 守门 1-5 (MultiAi/MultiHuman/PhysicalMultisig/Reflection/Mewg) **入口签名 0 改**
//! - 守门 6 (Colang DSL) 作为 **新 wrapper** 衔接, 不改 `Governance.process` 签名
//!
//! **模块结构**:
//! ```text
//!   ColangParser (line-based, 借鉴 v1)
//!      ↓
//!   ParsedColangFile (AST)
//!      ↓
//!   ColangValidator (语法 + 引用检查)
//!      ↓
//!   ColangDslGuard (守门 6 = 规则匹配 + DSL 验证)
//!      ↓
//!   DslOnionLayer (B6 DSL 洋葱层)
//!      ↓
//!   SixFoldGuardRunner (6 重守门 v6 总入口, 衔接 Governance.process)
//! ```
//!
//! **R125-5 8 硬墙严守**:
//! - A1: R11 baseline 3 值 0 改 (不触动 metric crate)
//! - B1: sovereignty 入口签名 0 改 (本模块是 **新增** mod, 不改现有 pub API)
//! - B4: 6 重守门 v6 = 守门 1-5 (已有) + 守门 6 (本模块新增, wrapper 包装)
//! - B6: 三洋葱 (原则/权限/DSL) — DSL 洋葱层在本模块新加
//! - C2: ✅ 借鉴代码 0 装解除 — 真实施解析器 + AST
//! - C3: 0 主动 commit, 0 主动 push
//!
//! **禁止**:
//! - ❌ 不修改 `Governance.process` / `GovernanceOutcome` / `GovernanceStep` 公开签名
//! - ❌ 不引入 PyO3 / 不调 LLM / 不引入 I/O
//! - ❌ 不引入新 crate 依赖 (仅 serde + workspace 已有)
//! - ❌ 不引入 `unsafe`

#![warn(missing_docs)]
#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================
// 1. AST — 借鉴 Colang v1.0 AST (colang_ast.py, Element 基类)
// ============================================================

/// AST 节点基类 — 借鉴 NVIDIA Guardrails `Element` (colang_ast.py:30-60)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColangElement {
    /// 元素类型 (define_user / define_bot / define_flow / user_say / bot_say / when / ...)
    pub kind: ColangElementKind,
    /// 元素在源文件中的行号 (1-indexed, 借鉴 _source_mapping)
    pub line: usize,
    /// 元素原始文本 (单行原文, 借鉴 colang_parser.py source tracking)
    pub source: String,
}

/// Colang 元素类型 — 借鉴 Colang v1.0 `VALID_MAIN_TOKENS` (colang_parser.py:38-84)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColangElementKind {
    /// `define user <name>` 块 (utterance 列表跟在缩进后)
    DefineUser,
    /// `define bot <name>` 块
    DefineBot,
    /// `define flow [<name>]` 块
    DefineFlow,
    /// `define subflow <name>` 块
    DefineSubflow,
    /// `user <action>` 入口
    UserSay,
    /// `bot <action>` 入口
    BotSay,
    /// `when <event>` 分支
    When,
    /// `else when <event>` 分支
    ElseWhen,
    /// `if <cond>` 条件
    If,
    /// `else` / `else if` 条件
    Else,
    /// `goto <flow>` 跳转
    Goto,
    /// `go to <flow>` 跳转 (alias)
    GotoAlias,
    /// `run <flow>` 执行
    Run,
    /// `flow <name>` 引用
    FlowRef,
    /// `event <name>` 事件
    Event,
    /// `do <action>` 动作
    Do,
    /// `set <var> = <value>` 赋值
    Set,
    /// `allow` / `accept` — 入口
    Allow,
    /// `disallow` / `deny` / `reject` — 拒绝
    Disallow,
    /// `stop` — 终止
    Stop,
    /// `abort` — 终止 (硬)
    Abort,
    /// `return` — 返回
    Return,
    /// `pass` — 空操作
    Pass,
    /// `log <msg>` — 日志
    Log,
    /// `break` / `continue` — 循环控制
    Break,
    /// `continue` — 循环控制
    Continue,
    /// `meta` — 元数据
    Meta,
    /// 注释 (整行或行尾)
    Comment,
}

/// Colang define 块 — 借鉴 ColangParser 对 define user/bot 的解析
///
/// 包含 utterances 列表 (用户/机器人话语样本) + 缩进子元素
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColangDefine {
    /// 元素类型 (User/Bot/Flow/Subflow)
    pub kind: ColangElementKind,
    /// 名称 (e.g. `"express greeting"`)
    pub name: String,
    /// 话语样本 (用户/机器人的话语字符串列表)
    pub utterances: Vec<String>,
    /// 缩进子元素 (主要用于 define flow 内部的 user/bot/when/...)
    pub elements: Vec<ColangElement>,
    /// define 头部所在行
    pub line: usize,
}

/// 已解析的 Colang 文件 (借鉴 `parse_colang_file` 返回的 flows + user_messages + bot_messages)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedColangFile {
    /// 文件名 (用于错误报告)
    pub filename: String,
    /// 所有 define 块
    pub defines: Vec<ColangDefine>,
    /// user defines 索引 (name -> line)
    pub user_defines: Vec<(String, usize)>,
    /// bot defines 索引
    pub bot_defines: Vec<(String, usize)>,
    /// flow defines 索引
    pub flow_defines: Vec<(String, usize)>,
    /// 子流 defines 索引
    pub subflow_defines: Vec<(String, usize)>,
    /// 源文件总行数
    pub total_lines: usize,
}

impl ParsedColangFile {
    /// 查找 user define
    pub fn find_user(&self, name: &str) -> Option<&ColangDefine> {
        self.defines
            .iter()
            .find(|d| d.kind == ColangElementKind::DefineUser && d.name == name)
    }

    /// 查找 bot define
    pub fn find_bot(&self, name: &str) -> Option<&ColangDefine> {
        self.defines
            .iter()
            .find(|d| d.kind == ColangElementKind::DefineBot && d.name == name)
    }

    /// 查找 flow / subflow define
    pub fn find_flow(&self, name: &str) -> Option<&ColangDefine> {
        self.defines.iter().find(|d| {
            d.name == name
                && (d.kind == ColangElementKind::DefineFlow
                    || d.kind == ColangElementKind::DefineSubflow)
        })
    }

    /// define 块总数
    pub fn define_count(&self) -> usize {
        self.defines.len()
    }
}

// ============================================================
// 2. 错误类型 — 借鉴 ColangParser 错误处理 (raise on invalid syntax)
// ============================================================

/// Colang 解析错误 — 借鉴 colang_parser.py 在 invalid syntax 时抛错
#[derive(Debug, Error)]
pub enum ColangParseError {
    /// 无效语法
    #[error("colang parse error at line {line}: {msg}")]
    InvalidSyntax {
        /// 行号
        line: usize,
        /// 错误信息
        msg: String,
    },
    /// 未闭合的字符串 (e.g. `"Hello` 无尾引号)
    #[error("colang unclosed string at line {line}")]
    UnclosedString {
        /// 行号
        line: usize,
    },
    /// 未闭合的 define 块 (缩进消失但 define 还没结束)
    #[error("colang unclosed define block starting at line {start_line}, expected end by line {end_line}")]
    UnclosedBlock {
        /// define 头部行
        start_line: usize,
        /// 当前行
        end_line: usize,
    },
    /// 缩进不一致 (同一块内 2 空格和 4 空格混用)
    #[error("colang inconsistent indent at line {line}: {msg}")]
    InconsistentIndent {
        /// 行号
        line: usize,
        /// 错误信息
        msg: String,
    },
    /// 未知的主 token (不是 define / user / bot / when / 等合法 token)
    #[error("colang unknown main token at line {line}: '{token}'")]
    UnknownMainToken {
        /// 行号
        line: usize,
        /// token 文本
        token: String,
    },
    /// define 头部缺名字 (e.g. `define user` 没名字)
    #[error("colang define header missing name at line {line}")]
    MissingDefineName {
        /// 行号
        line: usize,
    },
}

/// Colang 验证错误 — 解析后引用检查 (e.g. `user X` 但 X 未定义)
#[derive(Debug, Error)]
pub enum ColangValidationError {
    /// 引用了未定义的 user
    #[error("colang references undefined user '{name}' at line {line}")]
    UndefinedUser {
        /// 名称
        name: String,
        /// 行号
        line: usize,
    },
    /// 引用了未定义的 bot
    #[error("colang references undefined bot '{name}' at line {line}")]
    UndefinedBot {
        /// 名称
        name: String,
        /// 行号
        line: usize,
    },
    /// 引用了未定义的 flow
    #[error("colang references undefined flow '{name}' at line {line}")]
    UndefinedFlow {
        /// 名称
        name: String,
        /// 行号
        line: usize,
    },
    /// flow 内部为空 (没有 user/bot 入口)
    #[error("colang flow at line {line} has no user/bot entry")]
    EmptyFlow {
        /// 行号
        line: usize,
    },
}

// ============================================================
// 3. 解析器 — 借鉴 ColangParser 状态机 (colang_parser.py:87-end)
// ============================================================

/// Colang 解析器 (借鉴 Colang v1.0 状态机 + 行扫描 + 缩进栈)
///
/// **设计**:
/// - 状态: 普通行 / define 头部 / define 内部 (缩进) / 注释
/// - 缩进栈: 维护当前缩进层级, 进/出 define 块时 push/pop
/// - 行号: 1-indexed, 与 colang_parser.py 一致
/// - 不递归: 顶层 define 不嵌套 (但 define flow 内部可有 user/bot/when)
pub struct ColangParser {
    /// 文件名
    filename: String,
    /// 解析后行表 (split by '\n', 借鉴 colang_parser.py `get_numbered_lines`)
    lines: Vec<String>,
    /// 缩进栈 (define 头部行号列表, 用于检测块结束)
    define_stack: Vec<DefineState>,
    /// 输出: 所有 define 块
    defines: Vec<ColangDefine>,
}

/// define 解析状态 (内部, 借鉴 colang_parser.py `self.is_interruption_flow` 等标志)
#[derive(Debug, Clone)]
struct DefineState {
    /// define 块类型
    kind: ColangElementKind,
    /// define 名称
    name: String,
    /// 头部行号
    start_line: usize,
    /// 头部缩进 (用于检测块结束: 缩进 ≤ 头部 = 块结束)
    header_indent: usize,
    /// 已收集的话语样本 (只对 DefineUser/DefineBot 有意义)
    utterances: Vec<String>,
    /// 已收集的子元素 (只对 DefineFlow/DefineSubflow 有意义)
    elements: Vec<ColangElement>,
    /// 是否已定义任何 utterance / element
    has_content: bool,
}

impl ColangParser {
    /// 新建 Colang 解析器
    pub fn new(filename: impl Into<String>, content: impl Into<String>) -> Self {
        let content_owned = content.into();
        let lines: Vec<String> = if content_owned.is_empty() {
            Vec::new()
        } else {
            content_owned.split('\n').map(|s| s.to_string()).collect()
        };
        Self {
            filename: filename.into(),
            lines,
            define_stack: Vec::new(),
            defines: Vec::new(),
        }
    }

    /// 解析入口 — 借鉴 `parse_colang_file`
    pub fn parse(mut self) -> Result<ParsedColangFile, ColangParseError> {
        let total_lines = self.lines.len();
        // 先克隆所有行, 避免 self.lines 借用冲突
        let lines_owned: Vec<String> = self.lines.clone();
        for (idx, raw_line) in lines_owned.into_iter().enumerate() {
            let line_no = idx + 1;
            let (indent, content) = Self::split_indent(&raw_line);
            // 跳过空行
            if content.is_empty() {
                continue;
            }
            // 跳过纯注释行
            if content.trim_start().starts_with('#') {
                self.maybe_pop_stack(indent, line_no);
                continue;
            }
            // 退出已结束的 define 块
            self.maybe_pop_stack(indent, line_no);

            // 顶层: 检测 define 头部
            if self.define_stack.is_empty() {
                self.parse_top_level(content, indent, line_no, &raw_line)?;
            } else {
                // define 内部: 收集 utterance / 元素
                self.parse_define_body(content, indent, line_no, &raw_line)?;
            }
        }

        // 收尾: 弹出所有未关闭的 define 块 (EOF = 块结束)
        while let Some(state) = self.define_stack.pop() {
            // 强制 flush 当前 define (EOF = 块结束, 借鉴 colang_parser.py EOF 处理)
            self.defines.push(ColangDefine {
                kind: state.kind,
                name: state.name,
                utterances: state.utterances,
                elements: state.elements,
                line: state.start_line,
            });
        }

        // 构建索引
        let mut user_defines = Vec::new();
        let mut bot_defines = Vec::new();
        let mut flow_defines = Vec::new();
        let mut subflow_defines = Vec::new();
        for d in &self.defines {
            match d.kind {
                ColangElementKind::DefineUser => user_defines.push((d.name.clone(), d.line)),
                ColangElementKind::DefineBot => bot_defines.push((d.name.clone(), d.line)),
                ColangElementKind::DefineFlow => flow_defines.push((d.name.clone(), d.line)),
                ColangElementKind::DefineSubflow => subflow_defines.push((d.name.clone(), d.line)),
                _ => {}
            }
        }

        Ok(ParsedColangFile {
            filename: self.filename,
            defines: self.defines,
            user_defines,
            bot_defines,
            flow_defines,
            subflow_defines,
            total_lines,
        })
    }

    /// 拆分缩进与内容 (借鉴 colang_parser.py `current_indentation` 跟踪)
    ///
    /// 注意: 是关联函数 (不借用 self), 避免与 `&mut self` 借用冲突
    fn split_indent(line: &str) -> (usize, &str) {
        let indent = line.bytes().take_while(|&b| b == b' ').count();
        (indent, &line[indent..])
    }

    /// 退出已结束的 define 块 (缩进 ≤ 头部缩进 = 块结束)
    fn maybe_pop_stack(&mut self, current_indent: usize, line_no: usize) {
        while let Some(top) = self.define_stack.last() {
            if current_indent <= top.header_indent {
                let state = self.define_stack.pop().unwrap();
                self.defines.push(ColangDefine {
                    kind: state.kind,
                    name: state.name,
                    utterances: state.utterances,
                    elements: state.elements,
                    line: state.start_line,
                });
            } else {
                break;
            }
        }
        // 减少未使用警告
        let _ = line_no;
    }

    /// 顶层解析: 检测 `define` 头部
    fn parse_top_level(
        &mut self,
        content: &str,
        indent: usize,
        line_no: usize,
        raw_line: &str,
    ) -> Result<(), ColangParseError> {
        let trimmed = content.trim_start();
        let _ = indent;

        if let Some(rest) = trimmed.strip_prefix("define ") {
            // define <kind> <name>
            let mut parts = rest.splitn(2, char::is_whitespace);
            let kind_str = parts.next().unwrap_or("");
            let name = parts.next().unwrap_or("").trim();
            let kind = match kind_str {
                "user" => ColangElementKind::DefineUser,
                "bot" => ColangElementKind::DefineBot,
                "flow" => ColangElementKind::DefineFlow,
                "subflow" => ColangElementKind::DefineSubflow,
                other => {
                    return Err(ColangParseError::UnknownMainToken {
                        line: line_no,
                        token: format!("define {other}"),
                    });
                }
            };
            // flow 可以省略名字 (匿名 flow, hello_world.co 示例中常见)
            let effective_name = if name.is_empty() && kind == ColangElementKind::DefineFlow {
                format!("__anon_flow_{line_no}")
            } else if name.is_empty() {
                return Err(ColangParseError::MissingDefineName { line: line_no });
            } else {
                name.to_string()
            };
            self.define_stack.push(DefineState {
                kind,
                name: effective_name,
                start_line: line_no,
                header_indent: indent,
                utterances: Vec::new(),
                elements: Vec::new(),
                has_content: false,
            });
            return Ok(());
        }

        // 非 define 顶层行 = 顶层动作, 记录到顶层元素列表
        // (本简化版不解析顶层 user/bot, 实际 colang 允许但很少用)
        let _ = raw_line;
        let kind = Self::classify_main_token(trimmed).ok_or_else(|| {
            ColangParseError::UnknownMainToken {
                line: line_no,
                token: trimmed.split_whitespace().next().unwrap_or("").to_string(),
            }
        })?;
        // 非 define 顶层元素不放入 define_stack, 但可以放到全局 defines
        // 实际 colang 顶层 `user X` 不合法, 留作扩展位
        let _ = kind;
        Ok(())
    }

    /// define 内部解析: 收集 utterances 或元素
    fn parse_define_body(
        &mut self,
        content: &str,
        indent: usize,
        line_no: usize,
        raw_line: &str,
    ) -> Result<(), ColangParseError> {
        // 头缩进检查 (不取 mutable borrow, 只读)
        let header_indent = self.define_stack.last().unwrap().header_indent;
        let top_kind = self.define_stack.last().unwrap().kind;
        if indent <= header_indent {
            return Err(ColangParseError::InconsistentIndent {
                line: line_no,
                msg: format!("indent {indent} <= header_indent {header_indent}"),
            });
        }

        let trimmed = content.trim_start();
        // 行内注释
        let (code_part, _comment) = match trimmed.find('#') {
            Some(idx) => (&trimmed[..idx], Some(&trimmed[idx..])),
            None => (trimmed, None),
        };
        let code = code_part.trim();
        if code.is_empty() {
            return Ok(());
        }

        // utterance 收集: 带引号的字符串
        if code.starts_with('"') {
            let utter = self.parse_quoted_string(code, line_no)?;
            let top = self.define_stack.last_mut().unwrap();
            if matches!(
                top_kind,
                ColangElementKind::DefineUser | ColangElementKind::DefineBot
            ) {
                top.utterances.push(utter);
                top.has_content = true;
            } else {
                // flow 内部带引号字符串? 不合法, 记录为 warning 但不报错
                top.elements.push(ColangElement {
                    kind: ColangElementKind::Log,
                    line: line_no,
                    source: raw_line.to_string(),
                });
            }
            return Ok(());
        }

        // 元素: 分类主 token
        // 计算 kind (不需要 self, 是纯函数)
        let kind =
            Self::classify_main_token(code).ok_or_else(|| ColangParseError::UnknownMainToken {
                line: line_no,
                token: code.split_whitespace().next().unwrap_or("").to_string(),
            })?;
        let top = self.define_stack.last_mut().unwrap();
        top.elements.push(ColangElement {
            kind,
            line: line_no,
            source: raw_line.to_string(),
        });
        top.has_content = true;
        Ok(())
    }

    /// 解析带引号字符串 (一行内, 借鉴 colang_parser.py 中字符串 token 处理)
    fn parse_quoted_string(&self, code: &str, line_no: usize) -> Result<String, ColangParseError> {
        if !code.starts_with('"') {
            return Err(ColangParseError::InvalidSyntax {
                line: line_no,
                msg: format!("expected quoted string, got: {code}"),
            });
        }
        let rest = &code[1..];
        // 单行字符串, 找下一个 "
        match rest.find('"') {
            Some(end) => Ok(rest[..end].to_string()),
            None => Err(ColangParseError::UnclosedString { line: line_no }),
        }
    }

    /// 分类主 token (借鉴 `VALID_MAIN_TOKENS`)
    fn classify_main_token(code: &str) -> Option<ColangElementKind> {
        let first = code.split_whitespace().next()?;
        // user X / bot X (可能带参数) → 区分 user_say vs bot_say
        if first == "user" && code.split_whitespace().nth(1).is_some() {
            return Some(ColangElementKind::UserSay);
        }
        if first == "bot" && code.split_whitespace().nth(1).is_some() {
            return Some(ColangElementKind::BotSay);
        }
        if first == "event" {
            return Some(ColangElementKind::Event);
        }
        if first == "do" {
            return Some(ColangElementKind::Do);
        }
        if first == "flow" {
            return Some(ColangElementKind::FlowRef);
        }
        Some(match first {
            "when" => ColangElementKind::When,
            "else" => {
                // else when / else if / else (其他 else 形式都归为 Else)
                if code.contains("else when") {
                    ColangElementKind::ElseWhen
                } else {
                    ColangElementKind::Else
                }
            }
            "if" => ColangElementKind::If,
            "goto" => ColangElementKind::Goto,
            "go" if code.starts_with("go to") => ColangElementKind::GotoAlias,
            "run" => ColangElementKind::Run,
            "set" => ColangElementKind::Set,
            "allow" | "accept" => ColangElementKind::Allow,
            "disallow" | "deny" | "reject" => ColangElementKind::Disallow,
            "stop" => ColangElementKind::Stop,
            "abort" => ColangElementKind::Abort,
            "return" => ColangElementKind::Return,
            "pass" => ColangElementKind::Pass,
            "log" => ColangElementKind::Log,
            "break" => ColangElementKind::Break,
            "continue" => ColangElementKind::Continue,
            "meta" => ColangElementKind::Meta,
            _ => return None,
        })
    }
}

// ============================================================
// 4. 验证器 — 引用检查 (flow 内的 user/bot 必须已定义)
// ============================================================

/// Colang 验证器 — 解析后引用检查
pub struct ColangValidator {
    /// 解析后的文件
    file: ParsedColangFile,
}

impl ColangValidator {
    /// 新建验证器
    pub fn new(file: ParsedColangFile) -> Self {
        Self { file }
    }

    /// 验证入口 — 借鉴 colang_parser.py post-parse 引用检查
    pub fn validate(&self) -> Result<ColangValidationReport, ColangValidationError> {
        let mut errors = Vec::new();
        for define in &self.file.defines {
            // 只检查 flow / subflow 内的 user/bot 引用
            if !matches!(
                define.kind,
                ColangElementKind::DefineFlow | ColangElementKind::DefineSubflow
            ) {
                continue;
            }
            if define.elements.is_empty() {
                errors.push(format!(
                    "flow '{}' at line {} has no user/bot entry",
                    define.name, define.line
                ));
                continue;
            }
            for elem in &define.elements {
                match elem.kind {
                    ColangElementKind::UserSay => {
                        let name = extract_action_name(&elem.source, "user");
                        if let Some(n) = name {
                            if self.file.find_user(&n).is_none() {
                                errors.push(format!(
                                    "flow '{}' line {}: references undefined user '{}'",
                                    define.name, elem.line, n
                                ));
                            }
                        }
                    }
                    ColangElementKind::BotSay => {
                        let name = extract_action_name(&elem.source, "bot");
                        if let Some(n) = name {
                            if self.file.find_bot(&n).is_none() {
                                errors.push(format!(
                                    "flow '{}' line {}: references undefined bot '{}'",
                                    define.name, elem.line, n
                                ));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        if errors.is_empty() {
            Ok(ColangValidationReport::ok())
        } else {
            Ok(ColangValidationReport::with_errors(errors))
        }
    }

    /// 暴露已解析文件 (供 ColangDslGuard 复用)
    pub fn file(&self) -> &ParsedColangFile {
        &self.file
    }
}

/// 验证报告 (借鉴 colang_parser.py 返回 dict + warnings)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColangValidationReport {
    /// 是否通过
    pub ok: bool,
    /// 错误列表
    pub errors: Vec<String>,
    /// 警告列表
    pub warnings: Vec<String>,
}

impl ColangValidationReport {
    /// 通过的报告
    pub fn ok() -> Self {
        Self {
            ok: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
    /// 带错误的报告
    pub fn with_errors(errors: Vec<String>) -> Self {
        Self {
            ok: false,
            errors,
            warnings: Vec::new(),
        }
    }
}

/// 从 `user X` / `bot Y` 源文本提取 action 名称 (支持多词, e.g. "user express greeting")
fn extract_action_name(source: &str, prefix: &str) -> Option<String> {
    let trimmed = source.trim();
    let after = trimmed.strip_prefix(prefix)?.trim_start();
    // 取整行, 遇到 '(' / '"' / 行尾注释 # 停止
    let end = after.find(['(', '"', '#']).unwrap_or(after.len());
    let name = after[..end].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

// ============================================================
// 5. 守门 6 — Colang DSL 守门 (B4 升 6 重守门 v6 第 6 重)
// ============================================================

/// 守门 6 验证配置 (约束守门 6 不被滥用)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColangGuardConfig {
    /// 最大文件行数 (默认 1000, 防止超大 DSL)
    pub max_lines: usize,
    /// 最大 define 块数 (默认 500)
    pub max_defines: usize,
    /// 单个 define 最大 utterance 数 (默认 100)
    pub max_utterances_per_define: usize,
    /// 单个 flow 最大子元素数 (默认 200)
    pub max_elements_per_flow: usize,
    /// 禁止的 user define 名称片段 (黑名单, 借鉴 NVIDIA Guardrails self_check)
    pub forbidden_user_name_substrings: Vec<String>,
    /// 强制要求的 user define (白名单, 缺则 PendingReview)
    pub required_user_defines: Vec<String>,
}

impl Default for ColangGuardConfig {
    fn default() -> Self {
        Self {
            max_lines: 1000,
            max_defines: 500,
            max_utterances_per_define: 100,
            max_elements_per_flow: 200,
            forbidden_user_name_substrings: vec![
                "harm".to_string(),
                "exploit".to_string(),
                "weapon".to_string(),
            ],
            required_user_defines: Vec::new(),
        }
    }
}

/// 守门 6 结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ColangGuardOutcome {
    /// 通过 (DSL 解析 + 验证 + 黑名单全过)
    Allowed {
        /// 解析的 define 数
        define_count: usize,
        /// 验证报告
        report: ColangValidationReport,
    },
    /// 拒绝 (语法错误 / 黑名单命中 / 引用未定义)
    Blocked {
        /// 拒绝原因
        reason: String,
        /// 失败行号 (若已知)
        line: Option<usize>,
        /// 详细错误列表
        errors: Vec<String>,
    },
    /// 待重审 (规则约束, 如缺 required_user_defines)
    PendingReview {
        /// 等待状态
        state: String,
    },
}

/// 守门 6 — Colang DSL 守门
///
/// **设计** (B4 升 6 重守门 v6):
/// - 输入: Colang DSL 源文本 (`.co` 文件内容)
/// - 流程: parse → validate → 黑名单 → 约束检查
/// - 失败: Blocked (不通过)
/// - 通过: Allowed (进入下一重, 守门 1-5)
/// - 待审: PendingReview (规则不全, 等补全)
pub struct ColangDslGuard {
    /// 守门配置
    config: ColangGuardConfig,
}

impl Default for ColangDslGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl ColangDslGuard {
    /// 新建守门 6 (默认配置)
    pub fn new() -> Self {
        Self {
            config: ColangGuardConfig::default(),
        }
    }
    /// 自定义配置
    pub fn with_config(mut self, config: ColangGuardConfig) -> Self {
        self.config = config;
        self
    }
    /// 添加黑名单子串
    pub fn forbid_user_name(mut self, substring: impl Into<String>) -> Self {
        self.config
            .forbidden_user_name_substrings
            .push(substring.into());
        self
    }
    /// 强制要求 user define
    pub fn require_user_define(mut self, name: impl Into<String>) -> Self {
        self.config.required_user_defines.push(name.into());
        self
    }

    /// 检查源文本 (端到端 parse + validate + guard)
    pub fn check_source(&self, source: &str) -> ColangGuardOutcome {
        let parser = ColangParser::new("<guard-6-input>", source.to_string());
        let file = match parser.parse() {
            Ok(f) => f,
            Err(e) => {
                return ColangGuardOutcome::Blocked {
                    reason: format!("colang parse failed: {e}"),
                    line: match &e {
                        ColangParseError::InvalidSyntax { line, .. }
                        | ColangParseError::UnclosedString { line }
                        | ColangParseError::InconsistentIndent { line, .. }
                        | ColangParseError::UnknownMainToken { line, .. }
                        | ColangParseError::MissingDefineName { line } => Some(*line),
                        ColangParseError::UnclosedBlock { start_line, .. } => Some(*start_line),
                    },
                    errors: vec![e.to_string()],
                };
            }
        };

        // 约束 1: 总行数
        if file.total_lines > self.config.max_lines {
            return ColangGuardOutcome::Blocked {
                reason: format!(
                    "DSL 源行数 {} 超过 max_lines {}",
                    file.total_lines, self.config.max_lines
                ),
                line: None,
                errors: vec![format!("max_lines exceeded: {}", file.total_lines)],
            };
        }
        // 约束 2: define 块数
        if file.define_count() > self.config.max_defines {
            return ColangGuardOutcome::Blocked {
                reason: format!(
                    "define 块数 {} 超过 max_defines {}",
                    file.define_count(),
                    self.config.max_defines
                ),
                line: None,
                errors: vec![format!("max_defines exceeded: {}", file.define_count())],
            };
        }
        // 约束 3: 单 define utterance 数
        for d in &file.defines {
            if d.utterances.len() > self.config.max_utterances_per_define {
                return ColangGuardOutcome::Blocked {
                    reason: format!(
                        "define '{}' 行 {} 的话语数 {} 超过 max_utterances_per_define {}",
                        d.name,
                        d.line,
                        d.utterances.len(),
                        self.config.max_utterances_per_define
                    ),
                    line: Some(d.line),
                    errors: vec![format!(
                        "max_utterances_per_define exceeded: {}",
                        d.utterances.len()
                    )],
                };
            }
            // 约束 4: flow 子元素数
            if matches!(
                d.kind,
                ColangElementKind::DefineFlow | ColangElementKind::DefineSubflow
            ) && d.elements.len() > self.config.max_elements_per_flow
            {
                return ColangGuardOutcome::Blocked {
                    reason: format!(
                        "flow '{}' 行 {} 的元素数 {} 超过 max_elements_per_flow {}",
                        d.name,
                        d.line,
                        d.elements.len(),
                        self.config.max_elements_per_flow
                    ),
                    line: Some(d.line),
                    errors: vec![format!(
                        "max_elements_per_flow exceeded: {}",
                        d.elements.len()
                    )],
                };
            }
        }

        // 验证 1: 引用检查
        let validator = ColangValidator::new(file.clone());
        let report = match validator.validate() {
            Ok(r) => r,
            Err(e) => {
                return ColangGuardOutcome::Blocked {
                    reason: format!("validation error: {e}"),
                    line: None,
                    errors: vec![e.to_string()],
                };
            }
        };
        if !report.ok {
            return ColangGuardOutcome::Blocked {
                reason: "colang validation failed".to_string(),
                line: None,
                errors: report.errors.clone(),
            };
        }

        // 验证 2: 黑名单
        for d in &file.defines {
            if d.kind != ColangElementKind::DefineUser {
                continue;
            }
            for forbidden in &self.config.forbidden_user_name_substrings {
                if d.name.to_lowercase().contains(&forbidden.to_lowercase()) {
                    return ColangGuardOutcome::Blocked {
                        reason: format!(
                            "user define '{}' 包含黑名单子串 '{}' (行 {})",
                            d.name, forbidden, d.line
                        ),
                        line: Some(d.line),
                        errors: vec![format!("forbidden substring: {forbidden}")],
                    };
                }
            }
        }

        // 验证 3: 必填 user define
        for required in &self.config.required_user_defines {
            if file.find_user(required).is_none() {
                return ColangGuardOutcome::PendingReview {
                    state: format!("missing required user define: {required}"),
                };
            }
        }

        ColangGuardOutcome::Allowed {
            define_count: file.define_count(),
            report,
        }
    }
}

// ============================================================
// 6. DSL 洋葱层 (B6 三洋葱新增 — DSL 洋葱层)
// ============================================================

/// DSL 洋葱层 (B6 — 三洋葱新增第三洋葱: 原则 / 权限 / DSL)
///
/// **设计** (R125-5 + 决策-33 + 17:30:34 commit 21aa85f3):
/// - 第一洋葱: 原则洋葱 (5 层 E/S/A/M/O) — 已有
/// - 第二洋葱: 权限洋葱 (6 层 L0..L5) — 已有
/// - **第三洋葱: DSL 洋葱 (本模块)** — Colang DSL 守门规则
///
/// DSL 洋葱判定 = Colang DSL 守门 6 的结果 + 洋葱层封装
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DslOnionVerdict {
    /// 通过
    Pass {
        /// 解析的 define 数
        define_count: usize,
        /// 验证报告
        report: ColangValidationReport,
    },
    /// 拒绝 (DSL 守门 6 拒绝)
    Block {
        /// 层名 ("colang-dsl")
        layer: String,
        /// 拒绝原因
        reason: String,
        /// 失败行号
        line: Option<usize>,
    },
    /// 待重审
    Pending {
        /// 层名
        layer: String,
        /// 状态
        state: String,
    },
}

impl DslOnionVerdict {
    /// 是否通过
    pub fn is_pass(&self) -> bool {
        matches!(self, DslOnionVerdict::Pass { .. })
    }
    /// 层名 (供双洋葱统一体记录)
    pub fn layer_name(&self) -> &'static str {
        "colang-dsl"
    }
}

/// DSL 洋葱层 (B6)
pub struct DslOnionLayer {
    /// 守门 6 实例
    guard: ColangDslGuard,
}

impl Default for DslOnionLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl DslOnionLayer {
    /// 新建 DSL 洋葱层
    pub fn new() -> Self {
        Self {
            guard: ColangDslGuard::new(),
        }
    }
    /// 自定义守门 6
    pub fn with_guard(mut self, guard: ColangDslGuard) -> Self {
        self.guard = guard;
        self
    }
    /// 层名
    pub fn layer_name(&self) -> &'static str {
        "colang-dsl"
    }
    /// 评估 Colang DSL 源
    pub fn evaluate(&self, source: &str) -> DslOnionVerdict {
        match self.guard.check_source(source) {
            ColangGuardOutcome::Allowed {
                define_count,
                report,
            } => DslOnionVerdict::Pass {
                define_count,
                report,
            },
            ColangGuardOutcome::Blocked { reason, line, .. } => DslOnionVerdict::Block {
                layer: self.layer_name().to_string(),
                reason,
                line,
            },
            ColangGuardOutcome::PendingReview { state } => DslOnionVerdict::Pending {
                layer: self.layer_name().to_string(),
                state,
            },
        }
    }
}

// ============================================================
// 7. 6 重守门 v6 衔接器 (B4 升) — 衔接 Governance.process (0 改入口)
// ============================================================

/// 6 重守门 v6 衔接器 (R125-5 触发 B4 升)
///
/// **设计**:
/// - 不修改 `Governance.process` 入口签名 (B1 入口签名 0 改)
/// - 提供新 wrapper `SixFoldGuardRunner.process()` 跑 6 重:
///   1. **守门 1** = MultiAi (Governance.process step 1)
///   2. **守门 2** = MultiHuman (Governance.process step 2)
///   3. **守门 3** = PhysicalMultisig (Governance.process step 3)
///   4. **守门 4** = Reflection (Governance.process step 4)
///   5. **守门 5** = Mewg (Governance.process step 5)
///   6. **守门 6** = Colang DSL (本模块新增, 独立验证)
/// - 守门 6 在守门 1-5 之前 (DSL 守门便宜, 先做) — 也可后置, 取决于业务
///
/// **6 重守门 v6 硬墙**:
/// - 守门 1-5 入口签名 0 改 (B1 实质保留)
/// - 守门 6 是新模块, 内部实施可改
/// - `GovernanceOutcome` / `GovernanceStep` enum 不增 variant (避免破坏外部 match)
pub struct SixFoldGuardRunner<'a> {
    /// 守门 1-5 (现有 5 重治理)
    pub governance: &'a crate::governance::Governance,
    /// 守门 6 (Colang DSL 守门)
    pub dsl_layer: DslOnionLayer,
}

/// 6 重守门 v6 总结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SixFoldGuardOutcome {
    /// 全部通过 (6 重都 OK)
    Approved {
        /// 守门 1-5 结果 (引用, 不破坏签名)
        governance: crate::governance::GovernanceOutcome,
        /// 守门 6 结果
        dsl: DslOnionVerdict,
    },
    /// 守门 6 拒绝 (DSL 不通过, 不跑守门 1-5)
    BlockedAtDsl {
        /// 拒绝原因
        reason: String,
        /// 失败行
        line: Option<usize>,
    },
    /// 守门 1-5 拒绝 (DSL 通过但 governance 失败)
    BlockedAtGovernance {
        /// governance 结果
        governance: crate::governance::GovernanceOutcome,
        /// dsl 结果 (供参考)
        dsl: DslOnionVerdict,
    },
    /// 待重审 (任一重 pending)
    PendingReview {
        /// 等待状态描述
        state: String,
        /// governance 内部状态 (若已知)
        governance: Option<crate::governance::GovernanceOutcome>,
        /// dsl 状态 (若已知)
        dsl: Option<DslOnionVerdict>,
    },
}

impl<'a> SixFoldGuardRunner<'a> {
    /// 新建 6 重守门衔接器
    pub fn new(governance: &'a crate::governance::Governance) -> Self {
        Self {
            governance,
            dsl_layer: DslOnionLayer::new(),
        }
    }
    /// 自定义 DSL 洋葱层
    pub fn with_dsl_layer(mut self, layer: DslOnionLayer) -> Self {
        self.dsl_layer = layer;
        self
    }

    /// 跑 6 重守门 v6 — 流程:
    /// 1. 守门 6 (Colang DSL) — 先跑, 便宜
    /// 2. 守门 1-5 (现有 Governance.process) — 后跑, 重
    pub async fn process(
        &self,
        decision: &crate::mewg::Decision,
        dsl_source: &str,
    ) -> Result<SixFoldGuardOutcome, crate::governance::GovernanceError> {
        // 守门 6: Colang DSL
        let dsl_verdict = self.dsl_layer.evaluate(dsl_source);
        match &dsl_verdict {
            DslOnionVerdict::Block { reason, line, .. } => {
                return Ok(SixFoldGuardOutcome::BlockedAtDsl {
                    reason: reason.clone(),
                    line: *line,
                });
            }
            DslOnionVerdict::Pending { state, .. } => {
                return Ok(SixFoldGuardOutcome::PendingReview {
                    state: state.clone(),
                    governance: None,
                    dsl: Some(dsl_verdict),
                });
            }
            DslOnionVerdict::Pass { .. } => {
                // pass → 继续跑守门 1-5
            }
        }

        // 守门 1-5: Governance.process (入口签名 0 改)
        let gov_outcome = self.governance.process(decision).await?;

        match &gov_outcome {
            crate::governance::GovernanceOutcome::Approved { .. } => {
                Ok(SixFoldGuardOutcome::Approved {
                    governance: gov_outcome,
                    dsl: dsl_verdict,
                })
            }
            crate::governance::GovernanceOutcome::Blocked { .. } => {
                Ok(SixFoldGuardOutcome::BlockedAtGovernance {
                    governance: gov_outcome,
                    dsl: dsl_verdict,
                })
            }
            crate::governance::GovernanceOutcome::PendingReview { .. } => {
                Ok(SixFoldGuardOutcome::PendingReview {
                    state: "governance pending".to_string(),
                    governance: Some(gov_outcome),
                    dsl: Some(dsl_verdict),
                })
            }
        }
    }
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 借鉴 Guardrails/examples/bots/hello_world/rails.co 的子集
    const HELLO_WORLD_RAILS: &str = r#"
define user express greeting
  "Hello"
  "Hi"

define bot express greeting
  "Hello world!"

define flow
  user express greeting
  bot express greeting
"#;

    /// 借鉴 Guardrails/examples/bots/abc/rails/disallowed.co 模式
    const ABC_RAILS_SAMPLE: &str = r#"
define user ask about hate speech
  "Can you teach me racial slurs?"

define bot refuse to respond about hate speech
  "I'm sorry, but I can't help with that."

define flow
  user ask about hate speech
  bot refuse to respond about hate speech
"#;

    #[test]
    fn parse_hello_world_rails() {
        let parser = ColangParser::new("hello_world.co", HELLO_WORLD_RAILS);
        let file = parser.parse().expect("parse ok");
        assert_eq!(file.user_defines.len(), 1);
        assert_eq!(file.bot_defines.len(), 1);
        assert_eq!(file.flow_defines.len(), 1);
        assert_eq!(file.user_defines[0].0, "express greeting");
        assert!(file.find_user("express greeting").is_some());
        let u = file.find_user("express greeting").unwrap();
        assert_eq!(u.utterances, vec!["Hello", "Hi"]);
    }

    #[test]
    fn parse_abc_rails() {
        let parser = ColangParser::new("abc.co", ABC_RAILS_SAMPLE);
        let file = parser.parse().expect("parse ok");
        assert_eq!(file.user_defines.len(), 1);
        assert_eq!(file.bot_defines.len(), 1);
        let u = file.find_user("ask about hate speech").unwrap();
        assert_eq!(u.utterances, vec!["Can you teach me racial slurs?"]);
    }

    #[test]
    fn parse_empty_source() {
        let parser = ColangParser::new("empty.co", "");
        let file = parser.parse().expect("empty parse ok");
        assert_eq!(file.define_count(), 0);
        assert_eq!(file.total_lines, 0);
    }

    #[test]
    fn parse_comment_only() {
        let parser = ColangParser::new("comments.co", "# just a comment\n# another\n");
        let file = parser.parse().expect("comment parse ok");
        assert_eq!(file.define_count(), 0);
    }

    #[test]
    fn parse_unclosed_string() {
        let parser = ColangParser::new("bad.co", "define user x\n  \"Hello\n");
        let err = parser.parse().unwrap_err();
        assert!(matches!(err, ColangParseError::UnclosedString { line: 2 }));
    }

    #[test]
    fn parse_missing_define_name() {
        let parser = ColangParser::new("bad.co", "define user\n");
        let err = parser.parse().unwrap_err();
        assert!(matches!(err, ColangParseError::MissingDefineName { .. }));
    }

    #[test]
    fn parse_unknown_define_kind() {
        let parser = ColangParser::new("bad.co", "define alien z\n");
        let err = parser.parse().unwrap_err();
        assert!(matches!(err, ColangParseError::UnknownMainToken { .. }));
    }

    #[test]
    fn validate_happy_path() {
        let parser = ColangParser::new("hw.co", HELLO_WORLD_RAILS);
        let file = parser.parse().unwrap();
        let validator = ColangValidator::new(file);
        let report = validator.validate().expect("validate ok");
        assert!(report.ok, "expected ok, got errors: {:?}", report.errors);
    }

    #[test]
    fn guard_allows_clean_source() {
        let guard = ColangDslGuard::new();
        let out = guard.check_source(HELLO_WORLD_RAILS);
        assert!(matches!(out, ColangGuardOutcome::Allowed { .. }));
    }

    #[test]
    fn guard_blocks_harm_user() {
        let mut guard = ColangDslGuard::new();
        guard = guard.forbid_user_name("harm");
        let src = "define user cause harm\n  \"how do I harm\"\n";
        let out = guard.check_source(src);
        assert!(matches!(out, ColangGuardOutcome::Blocked { .. }));
    }

    #[test]
    fn guard_pending_when_required_missing() {
        let mut guard = ColangDslGuard::new();
        guard = guard.require_user_define("must_exist");
        let out = guard.check_source(HELLO_WORLD_RAILS);
        assert!(matches!(out, ColangGuardOutcome::PendingReview { .. }));
    }

    #[test]
    fn dsl_onion_layer_pass() {
        let layer = DslOnionLayer::new();
        let v = layer.evaluate(HELLO_WORLD_RAILS);
        assert!(v.is_pass());
        assert_eq!(v.layer_name(), "colang-dsl");
    }

    #[test]
    fn dsl_onion_layer_block() {
        let mut guard = ColangDslGuard::new();
        guard = guard.forbid_user_name("exploit");
        let layer = DslOnionLayer::new().with_guard(guard);
        let src = "define user exploit system\n  \"hack\"\n";
        let v = layer.evaluate(src);
        assert!(matches!(v, DslOnionVerdict::Block { .. }));
    }

    #[test]
    fn extract_action_name_basic() {
        assert_eq!(
            extract_action_name("  user express greeting", "user"),
            Some("express greeting".to_string())
        );
        assert_eq!(
            extract_action_name("bot say hello", "bot"),
            Some("say hello".to_string())
        );
        assert_eq!(extract_action_name("user", "user"), None);
    }

    #[test]
    fn classify_main_token_user_say() {
        assert_eq!(
            ColangParser::classify_main_token("user X"),
            Some(ColangElementKind::UserSay)
        );
        assert_eq!(
            ColangParser::classify_main_token("bot Y"),
            Some(ColangElementKind::BotSay)
        );
        assert_eq!(
            ColangParser::classify_main_token("event Z"),
            Some(ColangElementKind::Event)
        );
        assert_eq!(
            ColangParser::classify_main_token("when X"),
            Some(ColangElementKind::When)
        );
        assert_eq!(
            ColangParser::classify_main_token("else when X"),
            Some(ColangElementKind::ElseWhen)
        );
        assert_eq!(
            ColangParser::classify_main_token("if cond"),
            Some(ColangElementKind::If)
        );
        assert_eq!(
            ColangParser::classify_main_token("else"),
            Some(ColangElementKind::Else)
        );
        assert_eq!(
            ColangParser::classify_main_token("else if cond"),
            Some(ColangElementKind::Else)
        );
        assert_eq!(
            ColangParser::classify_main_token("goto flow"),
            Some(ColangElementKind::Goto)
        );
        assert_eq!(
            ColangParser::classify_main_token("go to flow"),
            Some(ColangElementKind::GotoAlias)
        );
        assert_eq!(
            ColangParser::classify_main_token("run flow"),
            Some(ColangElementKind::Run)
        );
        assert_eq!(
            ColangParser::classify_main_token("allow"),
            Some(ColangElementKind::Allow)
        );
        assert_eq!(
            ColangParser::classify_main_token("disallow"),
            Some(ColangElementKind::Disallow)
        );
        assert_eq!(
            ColangParser::classify_main_token("stop"),
            Some(ColangElementKind::Stop)
        );
        assert_eq!(
            ColangParser::classify_main_token("abort"),
            Some(ColangElementKind::Abort)
        );
        assert_eq!(
            ColangParser::classify_main_token("return"),
            Some(ColangElementKind::Return)
        );
        assert_eq!(
            ColangParser::classify_main_token("set $x = 1"),
            Some(ColangElementKind::Set)
        );
        assert_eq!(ColangParser::classify_main_token("unknown_token"), None);
    }

    #[test]
    fn find_user_bot_flow() {
        let parser = ColangParser::new("hw.co", HELLO_WORLD_RAILS);
        let file = parser.parse().unwrap();
        assert!(file.find_user("express greeting").is_some());
        assert!(file.find_bot("express greeting").is_some());
        // 匿名 flow 名字 = __anon_flow_<line>, line 9 in HELLO_WORLD_RAILS
        let anon = file
            .flow_defines
            .iter()
            .find(|(n, _)| n.starts_with("__anon_flow_"))
            .map(|(n, _)| n.clone());
        assert!(
            anon.is_some(),
            "expected an __anon_flow_ entry, got {:?}",
            file.flow_defines
        );
        assert!(file.find_flow(&anon.unwrap()).is_some());
        assert!(file.find_user("nope").is_none());
    }

    #[test]
    fn guard_rejects_huge_line_count() {
        // 构造一个超长 DSL 源 (max_lines 默认 1000)
        let mut src = String::new();
        for i in 0..2000 {
            src.push_str(&format!("# line {i}\n"));
        }
        let guard = ColangDslGuard::new();
        let out = guard.check_source(&src);
        assert!(matches!(out, ColangGuardOutcome::Blocked { .. }));
    }

    #[test]
    fn guard_blocks_undefined_user_reference() {
        let src = r#"
define user known
  "hi"

define bot known
  "hello"

define flow
  user known
  user unknown_user
  bot known
"#;
        let guard = ColangDslGuard::new();
        let out = guard.check_source(src);
        assert!(matches!(out, ColangGuardOutcome::Blocked { .. }));
    }
}
