//! 入口面帮助文本（用户可见静态产物；入口面 = `apeireth` 二进制）。
//!
//! **为什么独立成模块（2026-11-06 拆分批）**：帮助文本是**纯静态字符串**，
//! 与入口面的解析 / 分发逻辑零耦合 ⇒ 天然是独立关注点 ⇒ 抽走后
//! `src/main.rs` 只留调用点（`print_help`），帮助文本以后增删**不再推高**
//! 入口文件的尺寸棘轮。
//!
//! **口径不变**：文本自 `src/main.rs` 逐字搬移（帮助是**用户可见产物**
//! ⇒ 输出必须逐字一致，搬移只改归属，不改一个字）。

/// 帮助文本（单一真源；`print_help` 与防漂移测试共用）。
///
/// 纪律：**分发里有的命令，帮助里必须出现**（`help_covers_every_dispatched_command`
/// 机检）。此前这里是手写的一小撮命令，实测漏掉了 `dream` / `nightwatch` /
/// `lessons` / `egress` / `bundle` / `bio` / `council` / `subagent` —— 用户从
/// `--help` 根本发现不了这些能力（帮助低估产品，同样是 0 装问题）。
pub(crate) const HELP_TEXT: &str = "\
apeireth — 本地优先的个人 AI 运行时（单二进制，多入口）

Usage:
  [会话]
    apeireth session      就绪自检：构建运行时、打印就绪与 provider 后退出（非交互）
    apeireth chat <PROMPT> [--model MODEL] [--session SESSION]
    apeireth onboard [--tier casual|pro] [--show | --reset]
  [审批]
    apeireth approve --session SESSION --approval APPROVAL
    apeireth reject  --session SESSION --approval APPROVAL [--reason REASON]
    apeireth cancel  --session SESSION --approval APPROVAL [--reason REASON]
  [服务]
    apeireth gateway serve [--bind ADDR] [--port PORT] [--pid-file PATH]
  [身份与访问]
    apeireth user add --user-id ID --role owner|member|guest [--clients NAME[,NAME...]] [--domain DOMAIN] [--token TOKEN]
    apeireth user list [--token TOKEN]
    apeireth user role --user-id ID --role owner|member|guest [--token TOKEN]
    apeireth user remove --user-id ID [--token TOKEN]
  [记忆与认知]
    apeireth dream
    apeireth nightwatch [--session SESSION] [--limit N] [--watch] [--interval S [--cooldown S [--idle S]]]
    apeireth lessons [--db DB] [--scope problem|file|interaction|tool] [--status candidate|verified|retracted] [--limit N]
    apeireth lessons add --statement 一句话教训 --scope SCOPE --episode 经历ID [--episode ...] --evidence 证据摘要
    apeireth council <主题...>
    apeireth subagent <标题...> [--payload JSON]
    apeireth memory verify [--db DB] [--subject SUBJECT]
    apeireth memory export --out FILE [--db DB] [--subject SUBJECT] [--limit N]
    apeireth memory handover --out FILE [--db DB] [--issue TEXT]... [--next TEXT]
    apeireth memory migrate-subject --from ANCHOR --to ANCHOR [--db DB]
  [治理与安全]
    apeireth trait report
    apeireth trait lock <SLIDER>
    apeireth trait unlock <SLIDER>
    apeireth bio list
    apeireth bio enable <mechanism>
    apeireth bio disable <mechanism>
    apeireth crypto wrap --partition <ID> --keys <FILE> --recovery-out <FILE>
    apeireth crypto recovery-export --keys <FILE> --out <FILE>
    apeireth crypto unlock --keys <FILE> [--recovery <FILE>] [--out <KEYFILE>]
    apeireth egress ledger [--path LEDGER_JSONL] [--limit N] [--json]
    apeireth egress impact [--path IMPACT_JSONL] [--limit N] [--json]
    apeireth egress outcome <ACTION_ID> <confirmed|failed>
    apeireth egress bind <ACTION_ID> <TOOL_CALL_ID>
    apeireth egress notify <FACT_ID> <wording|partial|refuted>
    apeireth exposure report [--json] [--out PATH] [--bundle BUNDLE]
  [外部工具（协议桥）]
    apeireth mcp list [--live]
    apeireth mcp deny <server>.<tool>
    apeireth mcp undeny <server>.<tool>
    apeireth mcp authorize <server>.<tool>
    apeireth mcp unauthorize <server>.<tool>
  [数据与迁移]
    apeireth backup export <DIR>
    apeireth backup restore <BUNDLE> [--to ROOT]
    apeireth backup verify <BUNDLE>
    apeireth bundle --out DIR [--now EPOCH_S]

其它二进制（同一 workspace 的其它入口，不属 `apeireth` 子命令）:
  apeireth-tui [--endpoint URL] [--token TOKEN]   终端驾驶舱（全屏 TUI 客户端）
  apeireth-gateway 形态由 `apeireth gateway serve` 提供；SDK 见 crates/adapters/sdk

Options:
  -h, --help       Show this help (leading position only: apeireth --help / apeireth chat --help)
  -V, --version    Show the version (leading position only: apeireth --version)
  --               End flag parsing: every following word is a positional argument

Notes:
  Unknown flags are refused (never silently treated as text); to pass a word that
  starts with '-', put it after `--` (e.g. apeireth chat -- --modle gpt-4 hello).

Environment:
  APEIRETH_LISTEN  gateway serve 绑定地址 (缺省 127.0.0.1; --bind 显式旗标优先)
  APEIRETH_GATEWAY_TOKEN  全 API 面令牌门 (缺省回环; 非回环绑定必须设令牌)
  APEIRETH_DATA_DIR  数据根 (缺省 ~/.apeireth)
";
