//! 入口面（`apeireth` 二进制）的**旗标白名单与措辞档**。
//!
//! **为什么独立成模块（2026-11-06 拆分批）**：判据与共享实现住在 crate 级共享模块
//! `apeireth_cli::flags`（别的线在管，本批 0 触碰）；本模块只放**入口面自己**的
//! 两样东西 —— ① 各命令的旗标白名单（静态数据）② 入口面的措辞档（本面用法文本是
//! 英文的 ⇒ 错误帧走英文档）。两者都是**声明式**内容 ⇒ 抽走后入口文件
//! `src/main.rs` 只留「解析 + 分发 + 运行」⇒ **不改变任何运行时行为**
//! （判据仍在共享模块里，本模块只是白名单与措辞的搬运工）。
//!
//! **口径不变**：白名单与措辞逐字搬移（判据仍在 `apeireth_cli::flags`）。

use apeireth_cli::flags;

// ---------------------------------------------------------------------------
// 旗标扫描口径 (crate 级单一真源 = `apeireth_cli::flags`; 本面只保留自己的
// **措辞档**: 本面用法文本是英文的 ⇒ 错误帧走英文档, 与用法帧同语言):
// - `--` 之后一律位置参数 (不解析成旗标);
// - 未知旗标 → 拒绝即帧 (指出旗标名 + `--` 分隔提示), 不静默吞成正文;
// - 悬空旗标 / 旗标值本身像旗标 → 报「缺值」, 不报「未知参数」;
// - 负数是**值**不是旗标 (`--cooldown -1` ⇒ 报「值不合法」, 不报「缺值」);
// - `--help` / `-h` / `--version` / `-V` 只在首参数位生效 (首参数位之外当正文,
//   所以 `apeireth chat how --help` 的正文照发模型, 不被帮助吞掉)。
// ---------------------------------------------------------------------------

/// `apeireth chat` 的旗标白名单 (元旗标当正文: 正文不被帮助吞掉)。
pub(crate) const CHAT_FLAGS: flags::FlagSpec = flags::FlagSpec {
    value: &["--model", "--session"],
    boolean: &[],
    meta_as_positional: true,
};

/// `apeireth council` 的旗标白名单 (本命令不收旗标; 元旗标当议题正文)。
pub(crate) const COUNCIL_FLAGS: flags::FlagSpec = flags::FlagSpec {
    value: &[],
    boolean: &[],
    meta_as_positional: true,
};

/// `apeireth subagent` 的旗标白名单 (元旗标当标题正文)。
pub(crate) const SUBAGENT_FLAGS: flags::FlagSpec = flags::FlagSpec {
    value: &["--payload"],
    boolean: &[],
    meta_as_positional: true,
};

/// `apeireth approve|reject|cancel` 的旗标白名单 (本组命令不收位置参数)。
pub(crate) const APPROVAL_FLAGS: flags::FlagSpec = flags::FlagSpec {
    value: &["--session", "--approval", "--reason"],
    boolean: &[],
    meta_as_positional: false,
};

/// `apeireth dream` 的旗标白名单。
pub(crate) const DREAM_FLAGS: flags::FlagSpec = flags::FlagSpec {
    value: &["--session", "--limit", "--date"],
    boolean: &[],
    meta_as_positional: false,
};

/// `apeireth nightwatch` 的旗标白名单 (`--watch` 是布尔旗标: 不带值、不吞词)。
pub(crate) const NIGHTWATCH_FLAGS: flags::FlagSpec = flags::FlagSpec {
    value: &["--session", "--limit", "--idle", "--cooldown", "--interval"],
    boolean: &["--watch"],
    meta_as_positional: false,
};

/// `apeireth onboard` 的旗标白名单 (`--show` / `--reset` 是布尔旗标)。
pub(crate) const ONBOARD_FLAGS: flags::FlagSpec = flags::FlagSpec {
    value: &["--tier"],
    boolean: &["--show", "--reset"],
    meta_as_positional: false,
};

/// `apeireth gateway serve` 的旗标白名单。
pub(crate) const GATEWAY_FLAGS: flags::FlagSpec = flags::FlagSpec {
    value: &["--bind", "--port", "--pid-file"],
    boolean: &[],
    meta_as_positional: false,
};

/// 入口面的错误帧上下文 (英文档 + 「未知旗标」措辞)。
pub(crate) fn frame(command: &str) -> flags::FrameContext<'_> {
    flags::FrameContext::new(command, flags::UnknownNoun::Flag)
}

/// 按白名单扫描本命令的参数 (判据与措辞都在 crate 级共享模块里)。
pub(crate) fn scan(
    args: &[String],
    spec: &flags::FlagSpec,
    command: &str,
) -> Result<flags::Scan, String> {
    flags::scan(args, spec).map_err(|error| error.frame(flags::Style::En, &frame(command)))
}
