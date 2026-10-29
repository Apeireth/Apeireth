//! 设置页三页 · `data` 子面的 **wasm 宿主挂载**模块。
//!
//! ## 职责
//!
//! 1. **装载固定数据**（`harness/settings/fixtures/data.json` ⇒ **编译期内嵌**
//!    ⇒ 引擎侧与参考侧**同一份文件**）；
//! 2. **算运维读数行**（宿主侧算 ⇒ 引擎只渲染；生产把这段映射放在
//!    `lib/daemon-ops.ts`，本模块是它的**同构实现**）；
//! 3. **组装 props**（宿主注入 ⇒ 组件保持纯函数）；
//! 4. **挂载**（`WebDom` + `Renderer` + `Runtime::effect` + 微任务调度），
//!    并把**输入框的值**回写成 DOM **property**（生产用 `bind:value` ⇒ 值是
//!    property ⇒ DOM 上**没有** `value` 属性）。
//!
//! ## 诚实边界
//!
//! | 事项 | 口径 |
//! |---|---|
//! | 运维读数 | **宿主算**（固定数据喂进来）⇒ 引擎不碰数据面 |
//! | 「读不到」分支 | 宿主**判定**（读数为空 ⇒ 4 行占位 + 说明）⇒ 引擎只渲染 |
//! | **危险动作的执行**（删数据） | **不迁** ⇒ 引擎侧**只发意图**（`getIntents()` 可读） |
//! | 二次确认弹层 | **共享覆盖层**（不属本面）⇒ 引擎侧不渲染 |
//! | 工作区目录的读写 | **宿主能力** ⇒ 引擎只发意图 |
//! | 头像压缩 / 本地存取 | **宿主能力** ⇒ 引擎只渲染已有的 data URL |
//! | 剪贴板 | **宿主能力** ⇒ 引擎只发意图 |
//!
//! ## ⚠️ 启停指引的取值：**逐字取自生产**（不是固定数据）
//!
//! 生产把启停指引写在 `lib/daemon-ops.ts` 的常量表里 ⇒ **参考侧渲染的是生产
//! 原文**。而共享固定数据里的 `daemonGuidance` **与生产原文不一致**（标签与说明
//! 是草案）⇒ 若引擎侧喂固定数据 ⇒ 两侧 DOM **必然不等**。
//!
//! ⇒ 本模块按**生产原文**给出指引行（[`DAEMON_GUIDANCE`]）⇒ 两侧一致；
//! ⚠️ 共享固定数据需要主线对齐（见交付报告的「需要主线改的共享文件」清单）。

use apeireth_ui_core::{Callback, ReactiveScope, Signal};
use apeireth_ui_kit::settings_data::{
    DangerAction, DataPanelsProps, DataSection, GuidanceRow, OpsRow, UserProfile, DANGER_ACTIONS,
};

/// 固定数据的原文（**编译期内嵌** ⇒ 与参考侧同一份文件）。
pub const FIXTURE_JSON: &str = include_str!("../harness/settings/fixtures/data.json");

/// 「读不到」时的占位串（**逐字**取自生产的占位常量）。
pub const DASH: &str = "—";

/// 「读不到」时的说明行（**逐字**取自生产的读数映射常量）。
pub const UNAVAILABLE_NOTE: &str = "仅桌面版可读（web 构建无监督面）";

/// 零装边界的注记（**逐字**取自生产的零装说明常量）。
///
/// ⚠️ **不是**共享固定数据里的 `daemonZeroInstallNote`（那份是草案 ⇒ 与生产
/// 不一致）⇒ 见模块头的说明。
pub const DAEMON_ZERO_INSTALL_NOTE: &str = "本卡只读展示 + 指引：不代跑安装/启停、不加安装按钮。安装/卸载与参数面（-Port / -Bind / -RotateToken）见运维手册 docs/02-guides/server-deployment.md 与 packaging/service/README.md。";

/// 启停指引行（**逐字**取自生产的运维指引常量表）。
///
/// ⚠️ **不是**共享固定数据里的 `daemonGuidance`（那份是草案 ⇒ 与生产不一致）
/// ⇒ 见模块头的说明。
pub const DAEMON_GUIDANCE: [(&str, &str, &str, &str); 3] = [
    (
        "status",
        "看状态（只读）",
        "pwsh -NoProfile -File packaging/service/apeireth-service.ps1 status",
        "任务安装态/常驻占位/健康检查/令牌在否（只报 present/absent，永不回显值）。",
    ),
    (
        "start",
        "启动常驻服务",
        "pwsh -NoProfile -File packaging/service/apeireth-service.ps1 start",
        "已在跑则报 already running；桌面壳探测到常驻即直连，不重复拉起侧车。",
    ),
    (
        "stop",
        "优雅停机",
        "pwsh -NoProfile -File packaging/service/apeireth-service.ps1 stop",
        "三级递降：令牌门停机端点 → 停计划任务 → 兜底强杀（非优雅路径收尾不保证）。",
    ),
];

// ── 极小 JSON 解析器（**零外部依赖**） ─────────────────────────────────────

/// 解析出来的 JSON 值（本面只需要对象 / 数组 / 字符串 / 布尔 / 数值）。
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    /// 空值。
    Null,
    /// 布尔。
    Bool(bool),
    /// 数值。
    Number(f64),
    /// 字符串。
    String(String),
    /// 数组。
    Array(Vec<Json>),
    /// 对象（**保序**）。
    Object(Vec<(String, Json)>),
}

impl Json {
    /// 取对象的某个键。
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(entries) => entries
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    /// 取字符串。
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(text) => Some(text),
            _ => None,
        }
    }

    /// 取布尔。
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(flag) => Some(*flag),
            _ => None,
        }
    }

    /// 取数值。
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Json::Number(value) => Some(*value),
            _ => None,
        }
    }

    /// 取数组。
    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(items) => Some(items),
            _ => None,
        }
    }
}

/// JSON 解析器（递归下降；**报错带位置** ⇒ 固定数据坏了要**显式失败**）。
struct JsonParser<'a> {
    chars: &'a [char],
    pos: usize,
}

/// 解析一段 JSON 文本。
pub fn parse_json(text: &str) -> Result<Json, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut parser = JsonParser {
        chars: &chars,
        pos: 0,
    };
    parser.skip_ws();
    let value = parser.parse_value()?;
    parser.skip_ws();
    if parser.pos != parser.chars.len() {
        return Err(format!("JSON 尾部有多余内容（位置 {}）", parser.pos));
    }
    Ok(value)
}

impl JsonParser<'_> {
    /// 跳过空白。
    fn skip_ws(&mut self) {
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    /// 解析一个值。
    fn parse_value(&mut self) -> Result<Json, String> {
        self.skip_ws();
        let Some(ch) = self.chars.get(self.pos).copied() else {
            return Err("JSON 意外结束".to_owned());
        };
        match ch {
            '{' => self.parse_object(),
            '[' => self.parse_array(),
            '"' => Ok(Json::String(self.parse_string()?)),
            't' => self.parse_literal("true", Json::Bool(true)),
            'f' => self.parse_literal("false", Json::Bool(false)),
            'n' => self.parse_literal("null", Json::Null),
            _ => self.parse_number(),
        }
    }

    /// 解析对象。
    fn parse_object(&mut self) -> Result<Json, String> {
        self.pos += 1;
        let mut entries: Vec<(String, Json)> = Vec::new();
        self.skip_ws();
        if self.chars.get(self.pos) == Some(&'}') {
            self.pos += 1;
            return Ok(Json::Object(entries));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            if self.chars.get(self.pos) != Some(&':') {
                return Err(format!("JSON 对象缺 `:`（位置 {}）", self.pos));
            }
            self.pos += 1;
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_ws();
            match self.chars.get(self.pos) {
                Some(',') => self.pos += 1,
                Some('}') => {
                    self.pos += 1;
                    return Ok(Json::Object(entries));
                }
                _ => return Err(format!("JSON 对象缺 `}}`（位置 {}）", self.pos)),
            }
        }
    }

    /// 解析数组。
    fn parse_array(&mut self) -> Result<Json, String> {
        self.pos += 1;
        let mut items: Vec<Json> = Vec::new();
        self.skip_ws();
        if self.chars.get(self.pos) == Some(&']') {
            self.pos += 1;
            return Ok(Json::Array(items));
        }
        loop {
            let value = self.parse_value()?;
            items.push(value);
            self.skip_ws();
            match self.chars.get(self.pos) {
                Some(',') => self.pos += 1,
                Some(']') => {
                    self.pos += 1;
                    return Ok(Json::Array(items));
                }
                _ => return Err(format!("JSON 数组缺 `]`（位置 {}）", self.pos)),
            }
        }
    }

    /// 解析字符串。
    fn parse_string(&mut self) -> Result<String, String> {
        if self.chars.get(self.pos) != Some(&'"') {
            return Err(format!("JSON 期望字符串（位置 {}）", self.pos));
        }
        self.pos += 1;
        let mut out = String::new();
        while let Some(ch) = self.chars.get(self.pos).copied() {
            self.pos += 1;
            match ch {
                '"' => return Ok(out),
                '\\' => {
                    let Some(escape) = self.chars.get(self.pos).copied() else {
                        return Err("JSON 字符串转义未结束".to_owned());
                    };
                    self.pos += 1;
                    match escape {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{0008}'),
                        'f' => out.push('\u{000c}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            let code = self.parse_hex4()?;
                            if (0xD800..0xDC00).contains(&code) {
                                // 高代理项 ⇒ 需要低代理项。
                                if self.chars.get(self.pos) == Some(&'\\')
                                    && self.chars.get(self.pos + 1) == Some(&'u')
                                {
                                    self.pos += 2;
                                    let low = self.parse_hex4()?;
                                    let combined =
                                        0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                                    match char::from_u32(combined) {
                                        Some(ch) => out.push(ch),
                                        None => return Err("JSON 代理项对无效".to_owned()),
                                    }
                                    continue;
                                }
                                return Err("JSON 代理项对不完整".to_owned());
                            }
                            match char::from_u32(code) {
                                Some(ch) => out.push(ch),
                                None => return Err("JSON 转义不是合法字符".to_owned()),
                            }
                        }
                        other => return Err(format!("JSON 转义 `\\{other}` 未知")),
                    }
                }
                other => out.push(other),
            }
        }
        Err("JSON 字符串未结束".to_owned())
    }

    /// 解析四位十六进制。
    fn parse_hex4(&mut self) -> Result<u32, String> {
        let mut value = 0u32;
        for _ in 0..4 {
            let Some(ch) = self.chars.get(self.pos).copied() else {
                return Err("JSON 转义未结束".to_owned());
            };
            self.pos += 1;
            let digit = ch
                .to_digit(16)
                .ok_or_else(|| format!("JSON 转义 `\\u{ch}` 不是十六进制"))?;
            value = value * 16 + digit;
        }
        Ok(value)
    }

    /// 解析字面量（`true` / `false` / `null`）。
    fn parse_literal(&mut self, word: &str, value: Json) -> Result<Json, String> {
        for expected in word.chars() {
            if self.chars.get(self.pos) != Some(&expected) {
                return Err(format!("JSON 期望 `{word}`（位置 {}）", self.pos));
            }
            self.pos += 1;
        }
        Ok(value)
    }

    /// 解析数值。
    fn parse_number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        if self.chars.get(self.pos) == Some(&'-') {
            self.pos += 1;
        }
        while let Some(ch) = self.chars.get(self.pos).copied() {
            if ch.is_ascii_digit() || ch == '.' || ch == 'e' || ch == 'E' || ch == '+' || ch == '-'
            {
                self.pos += 1;
            } else {
                break;
            }
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        text.parse::<f64>()
            .map(Json::Number)
            .map_err(|_| format!("JSON 数值 `{text}` 非法"))
    }
}

// ── 固定数据（**宿主注入**） ────────────────────────────────────────────────

/// 侧车状态读数（**宿主读面**；`None` ⇒ 读不到）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DaemonStatus {
    /// 生命周期状态（`Stopped` / `Starting` / `Ready` / `Failed` / `Stopping`）。
    pub state: String,
    /// 归属（`OwnedByDesktop` / `External`）。
    pub ownership: String,
    /// 端点（`None` ⇒ 未上报）。
    pub endpoint: Option<String>,
    /// 版本（`None` ⇒ 未上报）。
    pub backend_version: Option<String>,
    /// 进程号（`None` ⇒ 未上报）。
    pub pid: Option<i64>,
    /// 重启次数。
    pub restart_count: i64,
}

/// 常驻服务探测报告（**宿主读面**；`None` ⇒ 读不到）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResidentProbe {
    /// 健康判定（`live` / `stale` / `absent`）。
    pub health: String,
    /// 占位是否存在。
    pub descriptor_present: bool,
    /// 占位端点（`None` ⇒ 没有）。
    pub endpoint: Option<String>,
    /// 进程号（`None` ⇒ 未上报）。
    pub pid: Option<i64>,
}

/// 数据子面的固定数据。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DataFixture {
    /// 侧车状态读数（`None` ⇒ 读不到）。
    pub daemon_status: Option<DaemonStatus>,
    /// 常驻探测读数（`None` ⇒ 读不到）。
    pub resident_probe: Option<ResidentProbe>,
    /// 工作区目录。
    pub workspace_dir: String,
    /// 工作区错误。
    pub workspace_error: String,
    /// 工作区读写进行中。
    pub workspace_loading: bool,
    /// 用户资料。
    pub user_profile: UserProfile,
    /// 用户资料错误。
    pub user_profile_error: String,
    /// 已复制的指引行键。
    pub copied_key: String,
    /// 复制失败说明。
    pub copy_error: String,
    /// 危险动作的待确认态（**键**；`None` ⇒ 没有待确认）。
    pub danger_pending: Option<String>,
}

/// 读固定数据（**编译期内嵌的那一份** ⇒ 与参考侧同一份文件）。
///
/// 固定数据坏了 ⇒ **显式报错**（不静默给空数据）。
pub fn data_fixture() -> Result<DataFixture, String> {
    let root = parse_json(FIXTURE_JSON)?;
    let daemon_status = match root.get("daemonStatus") {
        Some(Json::Null) | None => None,
        Some(value) => Some(DaemonStatus {
            state: string_field(value, "state")?,
            ownership: string_field(value, "ownership")?,
            endpoint: optional_string_field(value, "endpoint")?,
            backend_version: optional_string_field(value, "backend_version")?,
            pid: optional_number_field(value, "pid")?,
            restart_count: optional_number_field(value, "restart_count")?.unwrap_or(0),
        }),
    };
    let resident_probe = match root.get("residentProbe") {
        Some(Json::Null) | None => None,
        Some(value) => Some(ResidentProbe {
            health: string_field(value, "health")?,
            descriptor_present: value
                .get("descriptor_present")
                .and_then(Json::as_bool)
                .unwrap_or(false),
            endpoint: optional_string_field(value, "endpoint")?,
            pid: optional_number_field(value, "pid")?,
        }),
    };
    let workspace = root
        .get("workspace")
        .ok_or("固定数据缺少 `workspace` 对象")?;
    let profile = root
        .get("userProfile")
        .ok_or("固定数据缺少 `userProfile` 对象")?;
    let danger = root.get("danger").ok_or("固定数据缺少 `danger` 对象")?;
    Ok(DataFixture {
        daemon_status,
        resident_probe,
        workspace_dir: string_field(workspace, "dir")?,
        workspace_error: string_field(workspace, "error")?,
        workspace_loading: workspace
            .get("loading")
            .and_then(Json::as_bool)
            .unwrap_or(false),
        user_profile: UserProfile {
            nickname: string_field(profile, "nickname")?,
            bio: string_field(profile, "bio")?,
            avatar: optional_string_field(profile, "avatar")?,
        },
        user_profile_error: string_field(&root, "userProfileError")?,
        copied_key: string_field(&root, "opsCopiedKey")?,
        copy_error: string_field(&root, "opsCopyError")?,
        danger_pending: match danger.get("pending") {
            Some(Json::Null) | None => None,
            Some(value) => value.as_str().map(str::to_owned),
        },
    })
}

/// 取一个字符串字段。
fn string_field(value: &Json, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Json::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("固定数据缺字段 `{key}`（或类型不对）"))
}

/// 取一个可选字符串字段（**缺失或 `null` ⇒ `None`**）。
fn optional_string_field(value: &Json, key: &str) -> Result<Option<String>, String> {
    match value.get(key) {
        Some(Json::String(text)) => Ok(Some(text.clone())),
        Some(Json::Null) | None => Ok(None),
        Some(_) => Err(format!("固定数据字段 `{key}` 类型不对")),
    }
}

/// 取一个可选整数字段（**缺失或 `null` ⇒ `None`**）。
fn optional_number_field(value: &Json, key: &str) -> Result<Option<i64>, String> {
    match value.get(key) {
        Some(Json::Number(number)) => Ok(Some(*number as i64)),
        Some(Json::Null) | None => Ok(None),
        Some(_) => Err(format!("固定数据字段 `{key}` 类型不对")),
    }
}

// ── 运维读数的**展示映射**（宿主侧算；引擎只渲染） ──────────────────────────

/// 侧车状态标签（**逐字**取自生产的标签表；未知值**原样透出**）。
pub fn daemon_state_label(state: &str) -> String {
    match state {
        "Stopped" => "未运行",
        "Starting" => "启动中",
        "Ready" => "运行中",
        "Failed" => "异常",
        "Stopping" => "停止中",
        other => other,
    }
    .to_owned()
}

/// 侧车归属标签（**逐字**取自生产的标签表；未知值**原样透出**）。
pub fn daemon_ownership_label(ownership: &str) -> String {
    match ownership {
        "OwnedByDesktop" => "桌面拉起的侧车（生命周期归桌面壳）",
        "External" => "外部常驻服务（直连，生命周期归服务管理面）",
        other => other,
    }
    .to_owned()
}

/// 常驻探测标签（**逐字**取自生产的标签表；未知值**原样透出**）。
pub fn resident_probe_label(health: &str) -> String {
    match health {
        "live" => "在跑（/health 应答）",
        "stale" => "占位陈旧（端口不应答）",
        "absent" => "无占位（未探测到常驻）",
        other => other,
    }
    .to_owned()
}

/// 侧车状态行（**宿主算好的展示行**）。
pub fn status_rows(status: Option<&DaemonStatus>) -> Vec<OpsRow> {
    let Some(status) = status else {
        return vec![
            OpsRow::new("state", "侧车连接态", DASH, UNAVAILABLE_NOTE),
            OpsRow::new("ownership", "归属", DASH, UNAVAILABLE_NOTE),
            OpsRow::new("endpoint", "端点", DASH, UNAVAILABLE_NOTE),
            OpsRow::new("version", "版本", DASH, UNAVAILABLE_NOTE),
        ];
    };
    let state_note = if status.pid.is_none() && status.restart_count == 0 {
        "未见进程号（状态未上报 pid）".to_owned()
    } else {
        format!(
            "pid {} · 重启 {} 次",
            status
                .pid
                .map(|pid| pid.to_string())
                .unwrap_or_else(|| DASH.to_owned()),
            status.restart_count
        )
    };
    vec![
        OpsRow::new(
            "state",
            "侧车连接态",
            daemon_state_label(&status.state),
            state_note,
        ),
        OpsRow::new(
            "ownership",
            "归属",
            daemon_ownership_label(&status.ownership),
            "",
        ),
        OpsRow::new(
            "endpoint",
            "端点",
            status.endpoint.clone().unwrap_or_else(|| DASH.to_owned()),
            if status.endpoint.is_none() {
                "未上报端点（后端未就绪或读取失败）"
            } else {
                ""
            },
        ),
        OpsRow::new(
            "version",
            "版本",
            status
                .backend_version
                .clone()
                .unwrap_or_else(|| DASH.to_owned()),
            if status.backend_version.is_none() {
                "侧车未上报版本"
            } else {
                ""
            },
        ),
    ]
}

/// 常驻探测行（**宿主算好的展示行**）。
pub fn probe_rows(probe: Option<&ResidentProbe>) -> Vec<OpsRow> {
    let Some(probe) = probe else {
        return vec![OpsRow::new(
            "resident",
            "常驻服务探测",
            DASH,
            UNAVAILABLE_NOTE,
        )];
    };
    let note = if probe.descriptor_present && probe.endpoint.is_some() {
        let pid_part = match probe.pid {
            Some(pid) => format!(" · pid {pid}"),
            None => String::new(),
        };
        format!(
            "占位 {}{pid_part}",
            probe.endpoint.clone().unwrap_or_default()
        )
    } else {
        "占位读面：读不懂即无常驻（不猜）".to_owned()
    };
    vec![OpsRow::new(
        "resident",
        "常驻服务探测",
        resident_probe_label(&probe.health),
        note,
    )]
}

// ── 宿主状态（**信号由调用方持有** ⇒ 组件是纯函数） ────────────────────────

/// 数据子面的宿主状态。
#[derive(Clone)]
pub struct DataHostState {
    /// 响应式作用域（**使用者形态** ⇒ 本状态**挂不了 effect**）。
    scope: ReactiveScope,
    /// 当前分区。
    section: Signal<DataSection>,
    /// 读数**是否可读**（`false` ⇒ 走「读不到」分支）。
    readings_available: Signal<bool>,
    /// 已复制的指引行键。
    copied_key: Signal<String>,
    /// 复制失败说明。
    copy_error: Signal<String>,
    /// 读数进行中。
    daemon_loading: Signal<bool>,
    /// 工作区目录。
    workspace_dir: Signal<String>,
    /// 工作区错误。
    workspace_error: Signal<String>,
    /// 工作区读写进行中。
    workspace_loading: Signal<bool>,
    /// 用户资料。
    user_profile: Signal<UserProfile>,
    /// 用户资料错误。
    user_profile_error: Signal<String>,
    /// 危险动作的待确认键。
    danger_pending: Signal<String>,
    /// **已发出的意图**（**只发意图，不执行** ⇒ 这里就是全部的副作用面）。
    intents: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}

impl DataHostState {
    /// 建一份状态（读固定数据 ⇒ 数据坏了**显式报错**）。
    pub fn new(fixture: &DataFixture, scope: ReactiveScope) -> Self {
        Self {
            section: scope.signal(DataSection::Data),
            readings_available: scope.signal(true),
            copied_key: scope.signal(fixture.copied_key.clone()),
            copy_error: scope.signal(fixture.copy_error.clone()),
            daemon_loading: scope.signal(false),
            workspace_dir: scope.signal(fixture.workspace_dir.clone()),
            workspace_error: scope.signal(fixture.workspace_error.clone()),
            workspace_loading: scope.signal(fixture.workspace_loading),
            user_profile: scope.signal(fixture.user_profile.clone()),
            user_profile_error: scope.signal(fixture.user_profile_error.clone()),
            danger_pending: scope.signal(fixture.danger_pending.clone().unwrap_or_default()),
            intents: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
            scope,
        }
    }

    /// 响应式作用域（**使用者形态**；供宿主复核「同一个运行时」）。
    pub fn scope(&self) -> &ReactiveScope {
        &self.scope
    }

    /// 当前分区。
    pub fn section(&self) -> Signal<DataSection> {
        self.section.clone()
    }

    /// 读数是否可读。
    pub fn readings_available(&self) -> Signal<bool> {
        self.readings_available.clone()
    }

    /// 切分区（**未知 id ⇒ 落回 `data`**）。
    pub fn select_section(&self, id: &str) {
        self.section.set(DataSection::from_id(id));
    }

    /// 设「读数是否可读」（**对照用**：两态切换）。
    pub fn set_readings_available(&self, available: bool) {
        self.readings_available.set(available);
    }

    /// 记一条**意图**（**不执行**）。
    pub fn record_intent(&self, intent: impl Into<String>) {
        self.intents.borrow_mut().push(intent.into());
    }

    /// 已发出的意图（**只读**）。
    pub fn emitted_intents(&self) -> Vec<String> {
        self.intents.borrow().clone()
    }

    /// 清空意图记录。
    pub fn clear_intents(&self) {
        self.intents.borrow_mut().clear();
    }
}

/// 组装组件 props（**宿主注入**：数据 → 组件）。
pub fn data_panel_props(fixture: &DataFixture, state: &DataHostState) -> DataPanelsProps {
    let available = state.readings_available.get();
    let status = if available {
        fixture.daemon_status.as_ref()
    } else {
        None
    };
    let probe = if available {
        fixture.resident_probe.as_ref()
    } else {
        None
    };

    let guidance: Vec<GuidanceRow> = DAEMON_GUIDANCE
        .iter()
        .map(|(key, label, command, desc)| {
            let key_text = (*key).to_owned();
            let sink = state.clone();
            GuidanceRow::new(key_text.clone(), *label, *command, *desc).on_copy(Callback::new(
                move || {
                    sink.record_intent(format!("copy-command:{key_text}"));
                },
            ))
        })
        .collect();

    let danger_actions: Vec<DangerAction> = DANGER_ACTIONS
        .iter()
        .map(|(key, label)| {
            let key_text = (*key).to_owned();
            let sink = state.clone();
            DangerAction::new(key_text.clone(), *label).on_request(Callback::new(move || {
                // ⚠️ **只发意图**：宿主把「待确认」记下来 ⇒ 执行在宿主侧。
                sink.danger_pending.set(key_text.clone());
                sink.record_intent(format!("danger:{key_text}"));
            }))
        })
        .collect();

    let refresh_sink = state.clone();
    let pick_sink = state.clone();
    let avatar_sink = state.clone();
    let avatar_file_sink = state.clone();
    let remove_sink = state.clone();
    let nickname_sink = state.clone();
    let bio_sink = state.clone();
    let workspace_sink = state.clone();

    DataPanelsProps {
        section: state.section.get(),
        status_rows: status_rows(status),
        probe_rows: probe_rows(probe),
        guidance,
        copied_key: state.copied_key.get(),
        copy_error: state.copy_error.get(),
        zero_install_note: DAEMON_ZERO_INSTALL_NOTE.to_owned(),
        daemon_loading: state.daemon_loading.get(),
        workspace_dir: state.workspace_dir.get(),
        workspace_loading: state.workspace_loading.get(),
        workspace_error: state.workspace_error.get(),
        danger_actions,
        user_profile: state.user_profile.get(),
        user_profile_error: state.user_profile_error.get(),
        on_refresh: Some(Callback::new(move || {
            refresh_sink.record_intent("refresh-readings");
        })),
        on_pick_workspace: Some(Callback::new(move || {
            pick_sink.record_intent("open-workspace-picker");
        })),
        on_workspace_input: Some(Callback::new(move || {
            workspace_sink.record_intent("workspace-dir-input");
        })),
        on_request_avatar: Some(Callback::new(move || {
            avatar_sink.record_intent("request-avatar-upload");
        })),
        on_remove_avatar: Some(Callback::new(move || {
            remove_sink.record_intent("remove-avatar");
        })),
        on_avatar_file_changed: Some(Callback::new(move || {
            avatar_file_sink.record_intent("avatar-file-changed");
        })),
        on_nickname_input: Some(Callback::new(move || {
            nickname_sink.record_intent("user-nickname-input");
        })),
        on_bio_input: Some(Callback::new(move || {
            bio_sink.record_intent("user-bio-input");
        })),
    }
}

/// wasm32 侧的装配（**native 编译不参与** ⇒ 行为由浏览器内对照钉住）。
#[cfg(target_arch = "wasm32")]
pub mod wasm {
    //! ## 诚实边界
    //!
    //! - 本模块整体被 `#[cfg(target_arch = "wasm32")]` 隔离 ⇒ **native 编译不参与**
    //!   ⇒ 它的行为**未经 native 侧验证**（浏览器内的验证由设置页 harness 承担：
    //!   像素对照 + DOM 全等 + 宽度同源断言 + 意图断言）。
    //! - **真后端未接线**：数据面是**固定数据**（编译期内嵌的那一份）。
    //! - **危险动作不执行**：引擎侧只发意图（`window.__DATA__.getIntents()` 可读）。

    use std::cell::RefCell;
    use std::rc::Rc;

    use apeireth_ui_core::{HostScheduler, Runtime, UiComponent};
    use apeireth_ui_dom::{Renderer, VNode, WebDom};
    use apeireth_ui_kit::settings_data::DataPanels;
    use apeireth_ui_macro::view;
    use js_sys::Reflect;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::Element;

    use crate::settings_data::{data_fixture, data_panel_props, DataFixture, DataHostState};

    /// 微任务调度器：把 flush 排进浏览器的微任务队列。
    ///
    /// ⚠️ **不用** `#[wasm_bindgen] extern "C"` 声明 `queueMicrotask`：用
    /// `js_sys::Reflect` 直接调全局函数（**零导出** ⇒ 零符号冲突）。
    struct MicrotaskScheduler;

    impl HostScheduler for MicrotaskScheduler {
        fn schedule_flush(&self, flush: Box<dyn FnOnce()>) {
            let queue = Reflect::get(&js_sys::global(), &JsValue::from_str("queueMicrotask"))
                .ok()
                .and_then(|value| value.dyn_into::<js_sys::Function>().ok());
            match queue {
                Some(queue) => {
                    let callback = Closure::once_into_js(move || {
                        flush();
                        JsValue::UNDEFINED
                    });
                    let _ = queue.call1(&JsValue::UNDEFINED, &callback);
                }
                None => {
                    // 没有微任务队列 ⇒ **同步**跑（不静默丢刷新）。
                    flush();
                }
            }
        }
    }

    /// wasm 侧的单例应用。
    struct DataApp {
        /// 宿主状态。
        state: DataHostState,
    }

    thread_local! {
        /// wasm 侧的单例应用（数据子面形态）。
        static DATA_APP: RefCell<Option<DataApp>> = const { RefCell::new(None) };
        /// 控制 API 的闭包（**必须持有** ⇒ 否则 JS 侧调用会被回收）。
        static CONTROL_CLOSURES: RefCell<Vec<Closure<dyn FnMut(JsValue) -> JsValue>>> =
            const { RefCell::new(Vec::new()) };
    }

    /// 渲染整棵视图树。
    ///
    /// ## 舞台与作用域锚点
    ///
    /// - `.settings-stage`：**对照专用样式表**里的 828px 舞台（参考侧靠生产
    ///   自己的布局得到同样的宽度 ⇒ 两侧裁剪框宽度相等）。
    /// - `.sp-data`：**子面 scoped CSS 的作用域前缀**（抽取器按它前缀化）。
    fn render_tree(state: &DataHostState, fixture: &DataFixture) -> VNode {
        let props = data_panel_props(fixture, state);
        let panel = DataPanels::render(&props);
        view! {
            div(class = "settings-stage") {
                div(class = "sp-data") {
                    { panel }
                }
            }
        }
    }

    /// 挂载数据子面。
    ///
    /// ## 失败口径
    ///
    /// 失败 ⇒ **既返回错误、又发布到页面**（`window.__SETTINGS_ERROR__`）⇒
    /// 调用方忽略返回值时**仍然看得见**（不静默）。
    pub fn mount(container: &Element) -> Result<(), String> {
        match mount_inner(container) {
            Ok(()) => Ok(()),
            Err(error) => {
                report_error(&error);
                Err(error)
            }
        }
    }

    /// 挂载数据子面（内部实现）。
    fn mount_inner(container: &Element) -> Result<(), String> {
        let dom = WebDom::new().map_err(|error| error.to_string())?;
        let root: web_sys::Node = container.clone().into();
        let mut renderer = Renderer::new(dom, root);

        let fixture = data_fixture()?;
        // 应用根（**runtime 的所有者**）：建运行时 ⇒ ⇒ 交给状态持有者一个作用域。
        let runtime = Runtime::new();
        runtime.set_host_scheduler(Rc::new(MicrotaskScheduler));
        let state = DataHostState::new(&fixture, runtime.reactive_scope());

        renderer
            .mount(render_tree(&state, &fixture))
            .map_err(|error| error.to_string())?;

        let renderer = Rc::new(RefCell::new(renderer));
        let effect_renderer = renderer.clone();
        let effect_state = state.clone();
        let effect_fixture = fixture.clone();
        let effect_container = container.clone();
        runtime.effect(move || {
            let tree = render_tree(&effect_state, &effect_fixture);
            let mut guard = effect_renderer.borrow_mut();
            let _ = guard.patch(tree);
            // 补丁会把属性写回**原文** ⇒ property 回写必须**跟着补丁**跑。
            publish_state();
        });

        DATA_APP.with(|slot| {
            *slot.borrow_mut() = Some(DataApp { state });
        });
        install_control_api();
        patch_settings_api();
        publish_state();
        set_ready();
        Ok(())
    }

    /// 标记「画面已渲染完毕」（对照脚本的 `ready()` 判据；两侧同名同义）。
    fn set_ready() {
        let Some(window) = web_sys::window() else {
            return;
        };
        let _ = Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__SETTINGS_READY__"),
            &JsValue::TRUE,
        );
    }

    /// 记录一条**显式错误**（对照脚本可读；不静默）。
    fn report_error(message: &str) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let _ = Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__SETTINGS_ERROR__"),
            &JsValue::from_str(message),
        );
    }

    /// 把宿主状态**发布**到页面的全局（对照脚本可读）。
    fn publish_state() {
        let Some(window) = web_sys::window() else {
            return;
        };
        let (section, readings, intents) = DATA_APP.with(|slot| {
            let borrowed = slot.borrow();
            let Some(app) = borrowed.as_ref() else {
                return (String::new(), false, Vec::new());
            };
            (
                app.state.section().get().id().to_owned(),
                app.state.readings_available().get(),
                app.state.emitted_intents(),
            )
        });
        let _ = Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__SETTINGS_SECTION__"),
            &JsValue::from_str(&section),
        );
        let _ = Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__DATA_READINGS__"),
            &JsValue::from_bool(readings),
        );
        let array = js_sys::Array::new();
        for intent in intents {
            array.push(&JsValue::from_str(&intent));
        }
        let _ = Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__DATA_INTENTS__"),
            array.as_ref(),
        );
    }

    /// 装控制 API（`window.__DATA__`）—— 对照脚本用它驱动本面。
    fn install_control_api() {
        let Some(window) = web_sys::window() else {
            return;
        };

        let select_section = Closure::wrap(Box::new(|args: JsValue| -> JsValue {
            let id = args.as_string().unwrap_or_default();
            DATA_APP.with(|slot| {
                let borrowed = slot.borrow();
                if let Some(app) = borrowed.as_ref() {
                    app.state.select_section(&id);
                }
            });
            publish_state();
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let set_readings = Closure::wrap(Box::new(|args: JsValue| -> JsValue {
            let available = args.as_bool().unwrap_or(true);
            DATA_APP.with(|slot| {
                let borrowed = slot.borrow();
                if let Some(app) = borrowed.as_ref() {
                    app.state.set_readings_available(available);
                }
            });
            publish_state();
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let get_intents = Closure::wrap(Box::new(|_args: JsValue| -> JsValue {
            let intents = DATA_APP.with(|slot| {
                slot.borrow()
                    .as_ref()
                    .map(|app| app.state.emitted_intents())
                    .unwrap_or_default()
            });
            let array = js_sys::Array::new();
            for intent in intents {
                array.push(&JsValue::from_str(&intent));
            }
            array.into()
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let clear_intents = Closure::wrap(Box::new(|_args: JsValue| -> JsValue {
            DATA_APP.with(|slot| {
                let borrowed = slot.borrow();
                if let Some(app) = borrowed.as_ref() {
                    app.state.clear_intents();
                }
            });
            publish_state();
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let api = js_sys::Object::new();
        let bind = |key: &str, closure: &Closure<dyn FnMut(JsValue) -> JsValue>| {
            let function: &js_sys::Function = closure.as_ref().unchecked_ref();
            let _ = Reflect::set(api.as_ref(), &JsValue::from_str(key), function.as_ref());
        };
        bind("selectSection", &select_section);
        bind("setReadings", &set_readings);
        bind("getIntents", &get_intents);
        bind("clearIntents", &clear_intents);
        let _ = Reflect::set(
            api.as_ref(),
            &JsValue::from_str("ready"),
            &JsValue::from_bool(true),
        );
        let _ = Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__DATA__"),
            api.as_ref(),
        );
        CONTROL_CLOSURES.with(|slot| {
            let mut slot = slot.borrow_mut();
            slot.push(select_section);
            slot.push(set_readings);
            slot.push(get_intents);
            slot.push(clear_intents);
        });
    }

    /// 包裹共享 API 的 `selectSection`（**兜底**：让共享契约对本面**也**生效）。
    ///
    /// ## 为什么**不**导出 wasm 函数
    ///
    /// 共享的 `web/engine/main.js` 在 `selectSection(id)` 里会调
    /// `module.set_settings_section(id)`（若存在）⇒ 但**那个导出名已被向导面
    /// 占用** ⇒ 本面再导出同名符号 ⇒ **wasm 链接失败**（实测：duplicate symbol）。
    ///
    /// ⇒ 本面**零导出** ⇒ 改为**包裹共享 API 里的 `selectSection`**（共享契约
    /// ⇒ 本面状态这条链**不依赖导出是否存在**）⇒ 与向导面同口径，且**幂等**
    /// （与导出重复设置也无副作用）。
    fn patch_settings_api() {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Ok(api) = Reflect::get(window.as_ref(), &JsValue::from_str("__SETTINGS__")) else {
            return;
        };
        if api.is_null() || api.is_undefined() {
            return;
        }
        let Ok(original) = Reflect::get(&api, &JsValue::from_str("selectSection")) else {
            return;
        };
        if original.is_null() || original.is_undefined() {
            return;
        }
        let Ok(original) = original.dyn_into::<js_sys::Function>() else {
            return;
        };
        let wrapped = Closure::wrap(Box::new(move |args: JsValue| -> JsValue {
            // 先按**共享契约**更新 `__SETTINGS_SECTION__`，再驱动本面状态。
            let _ = original.call1(&JsValue::NULL, &args);
            let id = args.as_string().unwrap_or_default();
            DATA_APP.with(|slot| {
                let borrowed = slot.borrow();
                if let Some(app) = borrowed.as_ref() {
                    app.state.select_section(&id);
                }
            });
            publish_state();
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);
        let function: &js_sys::Function = wrapped.as_ref().unchecked_ref();
        let _ = Reflect::set(&api, &JsValue::from_str("selectSection"), function.as_ref());
        CONTROL_CLOSURES.with(|slot| slot.borrow_mut().push(wrapped));
    }
}

#[cfg(test)]
mod tests {
    //! 宿主侧的**行为**断言（固定数据 ⇒ 读数行 / 意图 ⇒ 状态）。

    use super::*;
    use apeireth_ui_core::Runtime;

    #[test]
    fn the_json_parser_reads_the_fixture_shape() {
        let value =
            parse_json(r#"{"a": [1, 2.5, true, null], "b": {"c": "x"}}"#).expect("解析成功");
        assert_eq!(
            value
                .get("a")
                .and_then(Json::as_array)
                .map(|items| items.len()),
            Some(4)
        );
        assert_eq!(
            value
                .get("b")
                .and_then(|item| item.get("c"))
                .and_then(Json::as_str),
            Some("x")
        );
        assert_eq!(
            value
                .get("a")
                .and_then(Json::as_array)
                .and_then(|items| items.first())
                .and_then(Json::as_number),
            Some(1.0)
        );
    }

    #[test]
    fn the_json_parser_reports_broken_input() {
        assert!(parse_json("{\"a\": }").is_err(), "坏输入必须显式报错");
        assert!(parse_json("").is_err(), "空输入必须显式报错");
        assert!(parse_json("{} {}").is_err(), "尾部多余内容必须显式报错");
    }

    #[test]
    fn the_fixture_carries_the_readings_and_the_profile() {
        let fixture = data_fixture().expect("固定数据可读");
        assert!(fixture.daemon_status.is_some(), "固定数据给了侧车读数");
        assert!(fixture.resident_probe.is_some(), "固定数据给了常驻探测读数");
        assert_eq!(fixture.workspace_dir, "/home/user/projects/demo");
        assert_eq!(fixture.user_profile.nickname, "小林");
        assert!(fixture.danger_pending.is_none(), "默认没有待确认的危险动作");
    }

    #[test]
    fn the_readable_state_renders_the_values() {
        let fixture = data_fixture().expect("固定数据可读");
        let rows = status_rows(fixture.daemon_status.as_ref());
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].value, "运行中");
        assert_eq!(rows[0].note, "pid 4821 · 重启 0 次");
        assert_eq!(rows[1].value, "桌面拉起的侧车（生命周期归桌面壳）");
        assert_eq!(rows[2].value, "http://127.0.0.1:8080");
        assert_eq!(rows[3].value, "2.0.0-rc.4");
        assert_eq!(rows[2].note, "", "有端点 ⇒ 无注记");

        let probes = probe_rows(fixture.resident_probe.as_ref());
        assert_eq!(probes.len(), 1);
        assert_eq!(probes[0].value, "在跑（/health 应答）");
        assert_eq!(probes[0].note, "占位 http://127.0.0.1:8081 · pid 4822");
    }

    #[test]
    fn the_unreadable_state_renders_the_placeholder_and_the_note() {
        let rows = status_rows(None);
        assert_eq!(rows.len(), 4);
        for row in &rows {
            assert_eq!(row.value, DASH, "读不到 ⇒ 占位");
            assert_eq!(row.note, UNAVAILABLE_NOTE, "读不到 ⇒ 说明");
        }
        let probes = probe_rows(None);
        assert_eq!(probes.len(), 1);
        assert_eq!(probes[0].value, DASH);
        assert_eq!(probes[0].note, UNAVAILABLE_NOTE);
    }

    #[test]
    fn the_unknown_labels_are_passed_through() {
        assert_eq!(daemon_state_label("Ready"), "运行中");
        assert_eq!(daemon_state_label("Whatever"), "Whatever");
        assert_eq!(
            daemon_ownership_label("External"),
            "外部常驻服务（直连，生命周期归服务管理面）"
        );
        assert_eq!(resident_probe_label("absent"), "无占位（未探测到常驻）");
    }

    #[test]
    fn the_props_follow_the_readings_switch() {
        let fixture = data_fixture().expect("固定数据可读");
        let state = DataHostState::new(&fixture, Runtime::new().reactive_scope());
        let props = data_panel_props(&fixture, &state);
        assert_eq!(props.status_rows.len(), 4);
        assert_eq!(props.status_rows[0].value, "运行中");

        state.set_readings_available(false);
        let props = data_panel_props(&fixture, &state);
        assert_eq!(props.status_rows[0].value, DASH);
        assert_eq!(props.status_rows[0].note, UNAVAILABLE_NOTE);
        assert_eq!(props.probe_rows[0].value, DASH);
    }

    #[test]
    fn the_danger_action_only_records_an_intent() {
        let fixture = data_fixture().expect("固定数据可读");
        let state = DataHostState::new(&fixture, Runtime::new().reactive_scope());
        let props = data_panel_props(&fixture, &state);
        let action = props.danger_actions.first().expect("有危险动作");
        let callback = action.on_request.clone().expect("有意图出口");
        callback.call();
        assert_eq!(
            state.emitted_intents(),
            vec![String::from("danger:clearRecentConversations")]
        );
        // ⚠️ **没有执行**：待确认态只是**被记下来**（执行在宿主侧）。
        assert_eq!(state.section.get(), DataSection::Data);
    }

    #[test]
    fn the_copy_intent_carries_the_row_key() {
        let fixture = data_fixture().expect("固定数据可读");
        let state = DataHostState::new(&fixture, Runtime::new().reactive_scope());
        let props = data_panel_props(&fixture, &state);
        let row = props.guidance.first().expect("有指引行");
        let callback = row.on_copy.clone().expect("有复制意图");
        callback.call();
        assert_eq!(
            state.emitted_intents(),
            vec![String::from("copy-command:status")]
        );
    }

    #[test]
    fn the_shared_fixture_drafts_diverge_from_the_production_rows() {
        // ⚠️ 登记在案：共享固定数据里的两处取值与生产原文**不一致**
        // （`daemonGuidance` 的标签 / 说明、`daemonZeroInstallNote` 的整句）
        // ⇒ 引擎侧按**生产原文**渲染 ⇒ 需要主线把固定数据对齐。
        let root = parse_json(FIXTURE_JSON).expect("固定数据可读");
        let draft = root
            .get("daemonGuidance")
            .and_then(Json::as_array)
            .and_then(|rows| rows.first())
            .and_then(|row| row.get("label"))
            .and_then(Json::as_str)
            .unwrap_or_default();
        assert_ne!(
            draft, DAEMON_GUIDANCE[0].1,
            "固定数据的指引标签已经与生产对齐 ⇒ 本模块可改回读固定数据"
        );
        let note = root
            .get("daemonZeroInstallNote")
            .and_then(Json::as_str)
            .unwrap_or_default();
        assert_ne!(
            note, DAEMON_ZERO_INSTALL_NOTE,
            "固定数据的零装注记已经与生产对齐 ⇒ 本模块可改回读固定数据"
        );
    }

    #[test]
    fn the_guidance_rows_are_the_production_ones() {
        let fixture = data_fixture().expect("固定数据可读");
        let state = DataHostState::new(&fixture, Runtime::new().reactive_scope());
        let props = data_panel_props(&fixture, &state);
        assert_eq!(props.guidance.len(), 3);
        assert_eq!(props.guidance[0].label, "看状态（只读）");
        assert_eq!(
            props.guidance[0].command,
            "pwsh -NoProfile -File packaging/service/apeireth-service.ps1 status"
        );
        assert_eq!(
            props.guidance[0].desc,
            "任务安装态/常驻占位/健康检查/令牌在否（只报 present/absent，永不回显值）。"
        );
    }

    #[test]
    fn the_section_switch_is_recorded_in_the_host() {
        let fixture = data_fixture().expect("固定数据可读");
        let state = DataHostState::new(&fixture, Runtime::new().reactive_scope());
        assert_eq!(state.section.get(), DataSection::Data);
        state.select_section("user");
        assert_eq!(state.section.get(), DataSection::User);
        state.select_section("nope");
        assert_eq!(
            state.section.get(),
            DataSection::Data,
            "未知 id ⇒ 落回默认分区"
        );
    }
}
