//! crate 级旗标判据 —— CLI 参数层单一真源。
//!
//! 已收口的旗标扫描面 (`main.rs` 入口面 / `exposure.rs` / `lessons_command.rs` /
//! `egress_command.rs` / `trait_command.rs` / `backup.rs` / `crypto_keys.rs` /
//! `user_admin.rs` / `mcp_command.rs`) 的**判据**收口到这里, 各面只留自己的措辞与用法帧。**收口面增减时同步本清单** (清单是导航, 不是计数)。五条口径:
//!
//! 1. **旗标形态** —— 以 `-` 开头、长度 > 1、且**不是负数**
//!    (`--cooldown -1` ⇒ `-1` 是**值** ⇒ 报「值不合法」, 不报「缺值」);
//! 2. **未知旗标** —— 拒绝即帧: 点名旗标 + `--` 分隔提示 (+ 用法帧);
//! 3. **带值旗标取值** —— 值缺失 / 值本身像旗标 ⇒ 报「**缺值**」(不是「未知参数」);
//! 4. **布尔旗标** —— 不带值、**不吞**后面的词;
//! 5. **`--` 之后** —— 一律位置参数 (不再做旗标解析)。
//!
//! 措辞分两轴: 语言档 [`Style`] (中文 = 仓内多数派; 英文 = 用法文本是英文的
//! 面 —— 错误帧与同一条命令的用法帧同语言, 免得同一屏里两种语言), 未知项措辞
//! [`UnknownNoun`] (被拒的确实是旗标 ⇒「未知旗标」; 面把旗标与位置词类一并
//! 称「参数」 ⇒「未知参数」; 既有测试钉死的措辞照旧, 不在共享层强行统一)。

/// token 是不是**旗标形态**: 以 `-` 开头、长度 > 1、且不是负数形态。
///
/// 负数**不是**旗标 —— `--cooldown -1` 里的 `-1` 是值, 报「值不合法」;
/// 若把负数当旗标, 同一条命令会从「must be >= 0」退化成「缺值」。
pub fn is_flag_like(token: &str) -> bool {
    token.starts_with('-') && token.len() > 1 && !is_negative_number(token)
}

/// token 是不是**负数形态** (`-1` / `-1.5` / `-1e9`) —— 负数是值, 不是旗标。
pub fn is_negative_number(token: &str) -> bool {
    token
        .strip_prefix('-')
        .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
}

/// token 是不是**元旗标** (`--help` / `-h` / `--version` / `-V`)。
///
/// 元旗标只在首参数位生效; 出现在正文之后 = 正文 (不吞正文) —— 落在正文还是
/// 拒绝即帧, 由逐面的 [`FlagSpec::meta_as_positional`] 决定。
pub fn is_meta_flag(token: &str) -> bool {
    matches!(token, "--help" | "-h" | "--version" | "-V")
}

/// token 是不是**以 `-` 开头** (严格档: 负数与单个 `-` 也算)。
///
/// 比 [`is_flag_like`] 严一档: 给「值不得以 `-` 开头」的落档类闸用
/// (写盘路径若以 `-` 开头, 会被下一个命令读成旗标)。
pub fn starts_with_dash(token: &str) -> bool {
    token.starts_with('-')
}

/// 带值旗标取值: **缺失 / 值本身像旗标 ⇒ 报「缺值」**(不是「未知参数」)。
///
/// `index` 进到值的位置; 成功时指向**值**那一项 (调用方自行再 +1 跳过)。
pub fn take_flag_value<'a>(
    args: &'a [String],
    index: &mut usize,
    flag: &str,
) -> Result<&'a String, FlagError> {
    *index += 1;
    match args.get(*index) {
        Some(value) if !is_flag_like(value) => Ok(value),
        Some(value) => Err(FlagError::MissingValue {
            flag: flag.to_string(),
            next: Some(value.clone()),
        }),
        None => Err(FlagError::MissingValue {
            flag: flag.to_string(),
            next: None,
        }),
    }
}

/// 旗标白名单 (带值旗标 / 布尔旗标 **分开**声明)。
#[derive(Debug, Clone, Copy)]
pub struct FlagSpec {
    /// 带值旗标: `--name <VALUE>` (吃下一个词; 值缺失或像旗标 ⇒ 缺值)。
    pub value: &'static [&'static str],
    /// 布尔旗标: `--name` (不带值, **不吞**后面的词)。
    pub boolean: &'static [&'static str],
    /// 元旗标的处置: `true` = 当位置参数 (正文语义), `false` = 未知旗标。
    pub meta_as_positional: bool,
}

impl FlagSpec {
    /// 白名单里的全部旗标 (人读列表, 用于错误帧的「只认 …」提示)。
    pub fn known(&self) -> String {
        self.value
            .iter()
            .chain(self.boolean.iter())
            .copied()
            .collect::<Vec<_>>()
            .join(" / ")
    }
}

/// 扫描结果 (带值旗标取值 / 命中的布尔旗标 / 位置参数)。
#[derive(Debug, Default, Clone)]
pub struct Scan {
    /// 带值旗标取值 (同名时**最后一个**出现者生效 ⇒ 取值取末位)。
    pub values: Vec<(String, String)>,
    /// 命中的布尔旗标。
    pub booleans: Vec<String>,
    /// 位置参数 (`--` 之后的词也在内)。
    pub positionals: Vec<String>,
}

impl Scan {
    /// 取带值旗标的取值 (最后一个出现者生效)。
    pub fn value(&self, name: &str) -> Option<&str> {
        self.values
            .iter()
            .rev()
            .find(|(flag, _)| flag == name)
            .map(|(_, value)| value.as_str())
    }

    /// 布尔旗标是否命中。
    pub fn has(&self, name: &str) -> bool {
        self.booleans.iter().any(|flag| flag == name)
    }

    /// 带值旗标出现了几次 (重复即拒的面用)。
    pub fn count(&self, name: &str) -> usize {
        self.values.iter().filter(|(flag, _)| flag == name).count()
    }

    /// 面**不收**位置参数时: 第一个多余的位置参数 ⇒ 未知参数错误。
    ///
    /// 位置参数本身不是旗标层错误 (判据归位: 词类参数归「未知参数」),
    /// 但「本命令不收位置参数」这条纪律要由各面自己执行 ⇒ 共享层只给判据。
    pub fn extra_positional(&self) -> Option<FlagError> {
        self.positionals
            .first()
            .map(|token| FlagError::UnknownArgument {
                token: token.clone(),
            })
    }
}

/// 白名单式扫描 —— **拒绝即帧**。
///
/// 逐词判定: 认识的按类型取 (带值旗标吃下一个词, 布尔旗标什么都不吃),
/// 不认识的立刻报错并**指名旗标**; `--` 之后不做旗标解析 (一律位置参数)。
pub fn scan(args: &[String], spec: &FlagSpec) -> Result<Scan, FlagError> {
    let mut scanned = Scan::default();
    let mut index = 0;
    let mut literal_only = false;
    while index < args.len() {
        let token = args[index].as_str();
        if literal_only {
            scanned.positionals.push(token.to_string());
            index += 1;
            continue;
        }
        if token == "--" {
            literal_only = true;
            index += 1;
            continue;
        }
        if spec.boolean.contains(&token) {
            // 布尔旗标不带值 ⇒ **不吞**后面的词。
            scanned.booleans.push(token.to_string());
            index += 1;
            continue;
        }
        if spec.value.contains(&token) {
            // 值的位置上没有词 / 值本身像旗标 ⇒ 缺值 (不把 `--json` 当路径)。
            let value = take_flag_value(args, &mut index, token)?;
            scanned.values.push((token.to_string(), value.clone()));
            index += 1;
            continue;
        }
        if is_meta_flag(token) {
            // 元旗标落在正文 (逐面选) 还是拒绝即帧。
            if !spec.meta_as_positional {
                return Err(FlagError::UnknownFlag {
                    flag: token.to_string(),
                });
            }
            scanned.positionals.push(token.to_string());
            index += 1;
            continue;
        }
        if is_flag_like(token) {
            return Err(FlagError::UnknownFlag {
                flag: token.to_string(),
            });
        }
        scanned.positionals.push(token.to_string());
        index += 1;
    }
    Ok(scanned)
}

/// 旗标层的拒绝分类 (判据收口点; 各面只负责措辞与用法帧)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlagError {
    /// 未知旗标 (token 是旗标形态, 但不在白名单里)。
    UnknownFlag { flag: String },
    /// 未知参数 (token 不是旗标形态 —— 词类位置参数)。
    UnknownArgument { token: String },
    /// 带值旗标缺值 (值缺失, 或值本身像旗标)。
    MissingValue { flag: String, next: Option<String> },
}

/// 错误帧的语言档。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// 中文档 (用法文本是中文的面: 仓内多数派)。
    Zh,
    /// 英文档 (用法文本是英文的面: 错误帧与同一条命令的用法帧同语言)。
    En,
}

/// 未知项的措辞: 「未知旗标」(精确) 还是「未知参数」(泛指)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownNoun {
    /// 「未知旗标」/ "unknown flag" —— 被拒的确实是旗标。
    Flag,
    /// 「未知参数」/ "unknown argument" —— 泛指被拒的参数 (旗标与位置词类同称)。
    Argument,
}

/// 错误帧的渲染上下文 (命令路径 + 用法帧 + 白名单提示 + 措辞)。
#[derive(Debug, Clone, Copy)]
pub struct FrameContext<'a> {
    /// 命令路径 (点名用): 如 `chat` / `egress ledger` / `trait report`。
    pub command: &'a str,
    /// 用法帧 (可空: 追加在错误行之后)。
    pub usage: Option<&'a str>,
    /// 「本命令只认 …」的旗标白名单提示 (可空)。
    pub known: Option<&'a str>,
    /// 未知项的措辞。
    pub noun: UnknownNoun,
}

impl<'a> FrameContext<'a> {
    /// 构造渲染上下文 (命令路径 + 未知项措辞)。
    pub fn new(command: &'a str, noun: UnknownNoun) -> Self {
        Self {
            command,
            usage: None,
            known: None,
            noun,
        }
    }

    /// 带用法帧。
    pub fn with_usage(mut self, usage: &'a str) -> Self {
        self.usage = Some(usage);
        self
    }

    /// 带「只认 …」的白名单提示。
    pub fn with_known(mut self, known: &'a str) -> Self {
        self.known = Some(known);
        self
    }
}

impl FlagError {
    /// 渲染错误帧 (语言档 + 上下文)。
    ///
    /// 帧的**结构**逐面一致 (点名 + `--` 分隔提示 + 可选用法帧), 只换措辞词。
    pub fn frame(&self, style: Style, ctx: &FrameContext) -> String {
        let wording = match style {
            Style::Zh => &ZH,
            Style::En => &EN,
        };
        let command = ctx.command;
        let noun = match ctx.noun {
            UnknownNoun::Flag => 0,
            UnknownNoun::Argument => 1,
        };
        let mut frame = match self {
            FlagError::UnknownFlag { flag } => {
                let known = match ctx.known {
                    Some(known) if !known.is_empty() => match style {
                        Style::Zh => format!("（{command} 只认 {known}）"),
                        Style::En => format!(" ({command} accepts {known})"),
                    },
                    _ => String::new(),
                };
                format!(
                    "{command}: {noun}: {flag}{known}\n{command}: {hint}",
                    noun = wording.flag_noun[noun],
                    hint = (wording.hint)(command, flag),
                )
            }
            FlagError::UnknownArgument { token } => {
                format!("{command}: {noun}: {token}", noun = wording.argument_noun)
            }
            FlagError::MissingValue { flag, next } => match next {
                Some(next) => format!(
                    "{command} {flag} {text}",
                    text = wording.missing_next.replace("{next}", next),
                ),
                None => format!("{command} {flag} {text}", text = wording.missing_absent),
            },
        };
        if let Some(usage) = ctx.usage {
            frame.push('\n');
            frame.push_str(usage);
        }
        frame
    }
}

/// 一个语言档的措辞表 (帧的拼装口径统一, 只换词)。
struct Wording {
    /// 未知旗标时的措辞 (`[Flag, Argument]` 两档, 逐面选)。
    flag_noun: [&'static str; 2],
    /// 未知参数 (词类位置参数) 时的措辞。
    argument_noun: &'static str,
    /// 缺值 (值缺失) 的措辞。
    missing_absent: &'static str,
    /// 缺值 (值撞上旗标) 的措辞 (`{next}` = 撞上的那一项)。
    missing_next: &'static str,
    /// `--` 分隔提示 (填入命令路径与被拒的旗标)。
    hint: fn(&str, &str) -> String,
}

/// 中文档 (用法文本是中文的面用这一档)。
const ZH: Wording = Wording {
    flag_noun: ["未知旗标", "未知参数"],
    argument_noun: "未知参数",
    missing_absent: "缺值（旗标后必须跟一个取值）",
    missing_next: "缺值（下一项 `{next}` 是旗标，不是取值）",
    hint: |command, flag| {
        format!("提示: 以 '-' 开头的词要放在 `--` 之后（如 apeireth {command} -- {flag}）")
    },
};

/// 英文档 (用法文本是英文的面用这一档)。
const EN: Wording = Wording {
    flag_noun: ["unknown flag", "unknown argument"],
    argument_noun: "unknown argument",
    missing_absent: "requires a value",
    missing_next: "requires a value (next `{next}` looks like a flag, not a value — a '-' prefix would be read as another flag)",
    hint: |command, flag| {
        format!("hint: a word starting with '-' must follow `--` (e.g. apeireth {command} -- {flag})")
    },
};
