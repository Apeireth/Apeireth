//! 表格操作工具面 (`tool.sheet_ops`, 声明名 `sheet_ops`) —— 受控读写族 (K3)。
//!
//! 设计第一律「AI 先用」的表格面: 模型可直接读 CSV 表、查摘要、筛行、排序、
//! 聚合、加计算列、导出 CSV, 同一内核未来再接人类界面出口。本面 = **工具面先行**:
//! 模型可调用、过治理、入名册、随自述上报。
//!
//! # 操作词表 (闭合, 未知动作拒绝即帧)
//!
//! | 动作 | 面 | 一句话 |
//! |---|---|---|
//! | `read` | 读 | 表头 + 行预览 (offset/limit, 超出显式 `truncated`) |
//! | `summary` | 读 | 行列数 / 列名 / 逐列类型猜测 (int/float/text/mixed/empty) |
//! | `filter` | 读(可写) | 列条件筛行 (eq/ne/lt/lte/gt/gte/contains 闭合谓词) |
//! | `sort` | 读(可写) | 按列稳定排序 (asc/desc, 空值恒排最后) |
//! | `aggregate` | 读 | 按列聚合 sum/avg/count/min/max (f64 如实) |
//! | `formula` | 读(可写) | 四则运算计算列 (新列追加, 原文不改) |
//! | `export_csv` | 写 | 规范 CSV 再序列化导出 (方言归一) |
//!
//! 「读(可写)」= 带 `output` 参数即写面 (结果落新文件), 不带即内存返回行预览。
//!
//! # 语义钉 (本面把口径钉死, 演进须改测试)
//!
//! - **数值词面**: 只认 `[-+]?(digits[.digits?] | .digits)([eE][-+]?digits)?` 且
//!   解析结果**有限**的词面为数值; `NaN` / `inf` / `1e999`(溢出) / `1,000` /
//!   `0x10` 一律按**文本**处理 —— NaN 永不进入数值轴。
//! - **类型混**: 排序与数值谓词/数值聚合要求目标列非空单元同一类型
//!   (全数值或全文本), 混型**拒绝即帧** (`sheet_ops.type_mixed`), 不静默跳行。
//! - **空值**: 空单元在 filter 中恒不匹配、在 sort 中恒排最后 (asc/desc 同)、
//!   在 sum/avg/min/max 中视为缺位不参与、在 count 中不计数。
//! - **空聚合**: sum(空集)=0.0 且 `count:0` 如实; avg/min/max(空集) 拒绝即帧
//!   (`sheet_ops.aggregate_empty`), 不冒充 0。
//! - **浮点如实**: 数值轴是 f64 —— 求和按行序裸 f64 累加 (不改写精度口径),
//!   结果以 f64 真值输出 (`0.1+0.2` 如实回 `0.30000000000000004`, 不静默取整);
//!   计算列文本取 Rust f64 最短往返表示。
//! - **NaN/inf 零产出**: 公式除零、溢出一律拒绝即帧 (`sheet_ops.formula_error`),
//!   输出永远不含 NaN/inf 词面。
//! - **空表**: 空文件 (零行) = 缺表头拒绝; 仅表头 = 0 数据行 (read/summary/
//!   filter/sort/formula 正常回 0 行, aggregate 按空聚合语义)。
//! - **行宽**: 每行单元数必须与表头一致, 参差行拒绝即帧 (不静默补空)。
//!
//! # 受控面 (工作区限域 + 敏感面拒 + 防炸弹表 + 原子写)
//!
//! - **工作区限域**: 读写路径一律解析进工作区根, 词法层 (`guardrail`) +
//!   现实层 (canonicalize) 双检, 越界拒绝即帧 (帧内不回显路径)。
//! - **敏感面拒**: 凭据/密钥面路径 (`sensitive_path` 族词表) 读写都拒,
//!   拒绝信息即帧 (`pipeline.pre_deny` 语义)。
//! - **防炸弹表**: 输入字节上限 [`DEFAULT_MAX_INPUT_BYTES`] + 数据行上限
//!   [`DEFAULT_MAX_ROWS`] + 列数上限 [`DEFAULT_MAX_COLS`], 三层都**拒绝即帧**
//!   (宁拒不截); 行预览另有 [`DEFAULT_MAX_PREVIEW_ROWS`] 显式 `truncated` 上限。
//! - **原子写**: 输出走 `storage_atomic` 原子替换 (临时文件 + 换名, 崩溃不留
//!   半截文件); **只创建不覆盖** —— 目标已存在拒绝即帧 (create-if-absent 门禁)。
//! - **解析子集**: 本批 CSV 面只认 **UTF-8** (可带 BOM) + RFC 4180 逗号方言
//!   (引号字段含逗号/换行, `""` 转义); 坏词面 (未闭合引号 / 引号裸奔 / 孤
//!   CR / 参差行 / 重名列 / 空列名) 一律拒绝即帧, 不静默修复。
//!
//! # 治理 (受控读写族, 与 `tool.doc_ops` 同族同档)
//!
//! 注册即闸 [`SHEET_OPS_ENABLE_ENV`] (`APEIRETH_ENABLE_SHEET_OPS`, 默认关
//! fail-closed, 见 [`sheet_ops_enabled_from_env`])。读面 grant 即用 (零审批);
//! **写面双重约束**: ① 受宪法级 `file_write` 唯一写总闸约束
//! ([`SheetOpsConfig::with_write_gate`], 关 = 拒写不拒读); ② **每次人批**
//! ([`SheetOpsApprovalHook`], 无自动放行档)。边界 Guard (工作区限域 / 敏感面 /
//! 上限 / 写总闸) 单调只拒不放, 自动放行也不绕过硬边界。
//!
//! # 归档裁定 (读码定归档: 表格写入族 or 文件写入族 → **文件写入族**)
//! 治理层族词表是冻结面, 表格输出写 = 写文件, 不新造「表格写入族」——
//! 唯一写总闸 (`APEIRETH_ENABLE_FILE_WRITE`) 的宪法语义是「文件写入只有唯一
//! 写总闸」, 故本面写动作受写总闸约束 (组装根注入同一开关值) 且每次人批,
//! 与 `tool.doc_ops` 写面同档 (创建类原子写, 永不覆盖)。
//!
//! # 脱敏口径 (路径不出帧)
//! 自产帧与结果帧里的路径只回工作区相对拼写; 越界 / 敏感面 / 词法攻击面的
//! 拒绝帧不回显路径; 自述创作节只回计数, 零路径零文件名。
//!
//! # 表格文件面 (读: CSV + xlsx; 写: 仅 CSV)
//! - **读面**认两种输入格式, 按**后缀词面**判定 (`.xlsx` → xlsx 面, 其余 →
//!   CSV 面): CSV = UTF-8 (可带 BOM) + RFC 4180 逗号方言 (见上); xlsx =
//!   OOXML 表格容器 (ZIP + XML, 与文档面 DOCX 同族容器, **复用 `doc_ops` 已报备
//!   ZIP 读端**, 解压预算同族), 单元格认三形态 —— 共享字符串 (`t="s"`) /
//!   内联字符串 (`t="inlineStr"`) / 数值格 (`t="n"` 或缺省, 原词面如实实在),
//!   字符串结果格 `t="str"` 按其 `<v>` 文本; 其余类型 (`b`/`e`/`d`/未知) 显式
//!   拒绝不猜值。工作表取 `workbook.xml` 首个 `<sheet>` 声明 (经 rels 解析);
//!   xlsx 稀疏格网缺格**补空** (格式语义), 落在表头宽度之外的格**拒绝即帧**。
//! - **公式格口径钉死**: `<f>` 在场 = 公式格, 一律取**缓存值**(`<v>`)且不重算
//!   —— 共享公式不展开计算, 帧内 `formula_cells` 计数如实标注; **无缓存值的
//!   公式格拒绝即帧** (不编造值)。
//! - **写面**只出 **CSV** (见 [`serialize_csv`]); xlsx 写出**后置**, 本批不接、
//!   不冒充 (xlsx 输入经变换/导出落盘 = 规范 CSV)。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{
    Action, Decision, GovernanceHook, GovernanceRequest, Permission, PermissionPolicy, ToolGuard,
    ToolGuardRequest,
};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::exec_pipeline::{
    AroundPolicy, OutputSchema, PreExecuteHook, PreExecuteRequest, PreVerdict, SchemaKind,
    DEFAULT_MAX_TIMEOUT_MS,
};
use crate::observed_gate::{
    gated_write_atomic, FsVersionProbe, ObservedGate, WriteKind, WriteRequest,
};
use crate::self_status::{SheetOpsOpCount, SheetOpsStatus};
use crate::sensitive_path::{credential_surface_refusal, is_sensitive_path};

// ---------------------------------------------------------------------------
// 身份与词表
// ---------------------------------------------------------------------------

/// 稳定能力标识 (注册表 / 治理授权 / 风险映射共用)。
pub const SHEET_OPS_CAPABILITY_ID: &str = "tool.sheet_ops";

/// 模型侧声明名 (短名, 每次请求都进上下文)。
pub const SHEET_OPS_TOOL_NAME: &str = "sheet_ops";

/// 生产执行时限 (毫秒): 解析/筛排聚合是纯内存工作, 到期即归 `timeout.*`。
pub const SHEET_OPS_TIMEOUT_MS: u64 = 30_000;

/// 表格操作面总开关 (`APEIRETH_ENABLE_SHEET_OPS`): 注册即闸, 默认关 fail-closed。
///
/// 只认显式 `"1"` (前后空白容忍); 未设 / `"0"` / 其它任何取值 = 关。
pub const SHEET_OPS_ENABLE_ENV: &str = "APEIRETH_ENABLE_SHEET_OPS";

/// 动作: `read` (行预览)。
pub const ACTION_READ: &str = "read";
/// 动作: `summary` (行列数 / 列名 / 类型猜测)。
pub const ACTION_SUMMARY: &str = "summary";
/// 动作: `filter` (列条件筛行; 带 `output` 即写面)。
pub const ACTION_FILTER: &str = "filter";
/// 动作: `sort` (按列稳定排序; 带 `output` 即写面)。
pub const ACTION_SORT: &str = "sort";
/// 动作: `aggregate` (按列聚合)。
pub const ACTION_AGGREGATE: &str = "aggregate";
/// 动作: `formula` (四则运算计算列; 带 `output` 即写面)。
pub const ACTION_FORMULA: &str = "formula";
/// 动作: `export_csv` (规范 CSV 导出, 写面)。
pub const ACTION_EXPORT_CSV: &str = "export_csv";

/// 闭合动作词表 (7 个)。
pub const SHEET_OPS_ACTIONS: &[&str] = &[
    ACTION_READ,
    ACTION_SUMMARY,
    ACTION_FILTER,
    ACTION_SORT,
    ACTION_AGGREGATE,
    ACTION_FORMULA,
    ACTION_EXPORT_CSV,
];

/// 纯读动作 (grant 即用零审批; 不带 `output` 的变换动作也是纯读)。
pub const SHEET_OPS_READ_ACTIONS: &[&str] = &[ACTION_READ, ACTION_SUMMARY, ACTION_AGGREGATE];

/// 变换动作 (不带 `output` = 读; 带 `output` = 写面)。
pub const SHEET_OPS_TRANSFORM_ACTIONS: &[&str] = &[ACTION_FILTER, ACTION_SORT, ACTION_FORMULA];

/// 恒写动作 (`export_csv` 必落盘)。
pub const SHEET_OPS_WRITE_ACTIONS: &[&str] = &[ACTION_EXPORT_CSV];

/// filter 谓词词表 (闭合)。
pub const FILTER_PREDICATES: &[&str] = &["eq", "ne", "lt", "lte", "gt", "gte", "contains"];

/// aggregate 函数词表 (闭合)。
pub const AGGREGATE_FUNCTIONS: &[&str] = &["sum", "avg", "count", "min", "max"];

/// sort 顺序词表 (闭合)。
pub const SORT_ORDERS: &[&str] = &["asc", "desc"];

/// 默认输入字节上限 (8 MiB): 按实读判定, 防炸弹表第一层。
pub const DEFAULT_MAX_INPUT_BYTES: u64 = 8 * 1024 * 1024;

/// 默认数据行上限 (行): 表头之外的数据行超限拒绝即帧。
pub const DEFAULT_MAX_ROWS: usize = 50_000;

/// 默认列数上限 (列): 超限拒绝即帧。
pub const DEFAULT_MAX_COLS: usize = 256;

/// 默认行预览上限 (行): read / 变换动作的内存返回行数上限, 超出显式
/// `truncated` (要全量就落盘, 不冒充全量)。
pub const DEFAULT_MAX_PREVIEW_ROWS: usize = 1_000;

/// 表达式长度上限 (字符): 防超长公式拖垮单次调用。
pub const MAX_EXPRESSION_CHARS: usize = 512;

/// 表达式括号嵌套上限 (层): 防深括号炸弹。
pub const MAX_EXPRESSION_DEPTH: usize = 16;

/// 自述创作节计数口径注记 (进程内计数, 不落账不造账)。
pub const SHEET_OPS_COUNT_SCOPE_NOTE: &str =
    "进程内计数: 本进程生命周期内的最近操作计数 (重启清零), 不落账不造账";

/// 工具描述全文 (注册声明与文档引用同一份文本)。
pub const SHEET_OPS_TOOL_DESCRIPTION: &str = "Read and transform tables inside \
  the workspace root (controlled read/write family). Inputs: .csv files and \
  .xlsx workbooks (the first worksheet; shared-string, inline-string and \
  numeric cells; formula cells report their cached values, never recomputed). \
  Closed vocabulary: read (header + row preview), summary (shape, column names, \
  type guesses), filter (eq/ne/lt/lte/gt/gte/contains on one column), sort \
  (stable, asc/desc, empty cells always last), aggregate \
  (sum/avg/count/min/max on one column), formula (a four-function computed \
  column), export_csv (canonical CSV out). filter/sort/formula write a new file \
  only when `output` is given; every write is created atomically, never \
  overwrites an existing file, and waits for a human approval; outputs are CSV \
  (xlsx writing is not offered yet). Paths must stay inside the workspace root; \
  credential surfaces are refused; oversized or bomb-like tables \
  (bytes/rows/columns) are refused, never silently truncated. Numbers are plain \
  f64 and are reported faithfully (no silent rounding); NaN/inf never appear in \
  results.";

// ---------------------------------------------------------------------------
// 错误 (拒绝即帧: 真因如实, 不静默降级)
// ---------------------------------------------------------------------------

/// 表格操作失败分类 (帧 = `sheet_ops.<kind>: <reason>`; 稳定 code + 真因)。
///
/// 脱敏口径: 越界/敏感/词法面不回显路径; 共享件回传文案经工具面剥根前缀后
/// 才入帧。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetOpsError {
    /// 参数校验拒绝 (词表外动作/谓词/函数 / 缺参 / 多余参 / 越界值)。
    ParamRejected(String),
    /// CSV 解析拒绝 (非 UTF-8 / 坏词面 / 参差行 / 重名列 / 空列名 / 空表无表头)。
    ParseFailed(String),
    /// 字节/行/列上限拒绝 (防炸弹表, 宁拒不截)。
    LimitExceeded(String),
    /// 类型混拒绝 (排序/数值谓词/数值聚合要求目标列同型)。
    TypeMixed(String),
    /// 公式拒绝 (未知列 / 坏表达式 / 除零 / 溢出非有限值 / 非数值单元)。
    FormulaError(String),
    /// 空聚合拒绝 (avg/min/max 空集不冒充 0)。
    AggregateEmpty(String),
    /// 词法攻击面 / 敏感系统路径 (不回显路径)。
    PathRejected,
    /// 目标越出工作区 (不回显路径)。
    OutsideWorkspace,
    /// 凭据面 (沿用读工具同一帧)。
    SensitiveSurface,
    /// 输入读取失败 (错误类别, 无绝对路径)。
    ReadFailed(String),
    /// 写面受 `file_write` 唯一写总闸约束: 总闸未开, 写动作拒绝即帧
    /// (读面不受影响)。
    WriteGateClosed(String),
    /// 输出目标已存在 (只创建不覆盖, 原子写门禁)。
    OutputExists(String),
    /// 原子写失败 (文案已剥根前缀)。
    WriteFailed(String),
}

impl SheetOpsError {
    /// 帧的稳定 code。
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ParamRejected(_) => "sheet_ops.param_rejected",
            Self::ParseFailed(_) => "sheet_ops.parse_error",
            Self::LimitExceeded(_) => "sheet_ops.limit_exceeded",
            Self::TypeMixed(_) => "sheet_ops.type_mixed",
            Self::FormulaError(_) => "sheet_ops.formula_error",
            Self::AggregateEmpty(_) => "sheet_ops.aggregate_empty",
            Self::PathRejected => "sheet_ops.path_rejected",
            Self::OutsideWorkspace => "sheet_ops.outside_workspace",
            Self::SensitiveSurface => "pipeline.pre_deny",
            Self::ReadFailed(_) => "sheet_ops.read_failed",
            Self::WriteGateClosed(_) => "sheet_ops.write_gate_closed",
            Self::OutputExists(_) => "sheet_ops.output_exists",
            Self::WriteFailed(_) => "sheet_ops.write_failed",
        }
    }

    /// 帧的真因 (越界/敏感/词法面不回显路径)。
    pub fn reason(&self) -> String {
        match self {
            Self::ParamRejected(detail)
            | Self::ParseFailed(detail)
            | Self::LimitExceeded(detail)
            | Self::TypeMixed(detail)
            | Self::FormulaError(detail)
            | Self::AggregateEmpty(detail)
            | Self::ReadFailed(detail)
            | Self::WriteGateClosed(detail)
            | Self::OutputExists(detail)
            | Self::WriteFailed(detail) => detail.clone(),
            Self::PathRejected => "requested path is refused by the path guard (traversal or \
                 protected system path); 路径不回显 (脱敏口径)"
                .to_string(),
            Self::OutsideWorkspace => "requested path resolves outside the workspace root; \
                 路径不回显 (脱敏口径)"
                .to_string(),
            Self::SensitiveSurface => credential_surface_refusal(),
        }
    }

    /// 渲染成帧 (稳定 code + 真因; 敏感面整帧沿用读工具凭据帧)。
    pub fn frame(&self) -> String {
        if matches!(self, Self::SensitiveSurface) {
            return credential_surface_refusal();
        }
        format!("{}: {}", self.code(), self.reason())
    }
}

// ---------------------------------------------------------------------------
// 配置与开关 (注册即闸, 默认关 fail-closed)
// ---------------------------------------------------------------------------

/// 表格操作面配置 (工作区根 + 四道上限 + 写总闸注入)。
///
/// 组装根按 [`SheetOpsConfig::from_env`] 注入 (注册即闸): 缺配置 =
/// `tool.sheet_ops` 不注册, 自述名册 `sheet_ops` 行照实 false。**写总闸** =
/// 宪法级 `file_write` 唯一写总闸的同一开关值 (组装根注入, 见
/// [`SheetOpsConfig::with_write_gate`]): 关 = 写动作拒绝即帧, 读面不受影响。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetOpsConfig {
    /// 工作区根 (读写限域)。
    pub workspace_root: PathBuf,
    /// 输入字节上限 (字节)。
    pub max_input_bytes: u64,
    /// 数据行上限 (行)。
    pub max_rows: usize,
    /// 列数上限 (列)。
    pub max_cols: usize,
    /// 行预览上限 (行)。
    pub max_preview_rows: usize,
    /// 写面写总闸 (默认关 fail-closed; 组装根注入 `file_write` 同一开关值)。
    pub write_gate_open: bool,
}

impl SheetOpsConfig {
    /// 工作区根 + 默认四上限; 写总闸默认关 (fail-closed)。
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            max_input_bytes: DEFAULT_MAX_INPUT_BYTES,
            max_rows: DEFAULT_MAX_ROWS,
            max_cols: DEFAULT_MAX_COLS,
            max_preview_rows: DEFAULT_MAX_PREVIEW_ROWS,
            write_gate_open: false,
        }
    }

    /// 覆盖四上限 (测试/宿主微调; 命名即承诺: 值即生效权力)。
    #[must_use]
    pub fn with_limits(
        mut self,
        max_input_bytes: u64,
        max_rows: usize,
        max_cols: usize,
        max_preview_rows: usize,
    ) -> Self {
        self.max_input_bytes = max_input_bytes;
        self.max_rows = max_rows;
        self.max_cols = max_cols;
        self.max_preview_rows = max_preview_rows;
        self
    }

    /// 注入写总闸 (`file_write` 唯一写总闸的同一开关值; 关 = 写动作拒绝即帧)。
    #[must_use]
    pub fn with_write_gate(mut self, open: bool) -> Self {
        self.write_gate_open = open;
        self
    }

    /// 开关实况: `APEIRETH_ENABLE_SHEET_OPS` 只认显式 `"1"` (默认关 fail-closed)。
    pub fn enabled_from_env() -> bool {
        sheet_ops_enabled_from_env()
    }

    /// 从 env 构造: 开关开才 `Some` (默认关 → [`None`], fail-closed)。
    pub fn from_env(workspace_root: impl Into<PathBuf>) -> Option<Self> {
        sheet_ops_enabled_from_env().then(|| Self::new(workspace_root))
    }
}

/// 表格操作面开关 (`APEIRETH_ENABLE_SHEET_OPS`) 实况: 只认显式 `"1"`。
///
/// 未设 / `"0"` / `"true"` / 其它任何取值 = 关 (fail-closed, 命名即承诺:
/// 没有这把钥匙就是没有表格操作面)。
pub fn sheet_ops_enabled_from_env() -> bool {
    std::env::var(SHEET_OPS_ENABLE_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

// ---------------------------------------------------------------------------
// 受控面: 工作区限域 + 敏感面拒 + 原子写
// ---------------------------------------------------------------------------

/// 把请求路径解析进工作区根并证明包含 (词法层 + 现实层双检)。
///
/// `must_exist` 用于输入面 (canonicalize 整条路径); 输出面目标尚不存在,
/// 只 canonicalize 最近存在的祖先再拼回, 保持同一把尺。拒绝帧不回显路径。
fn resolve_contained(
    root: &Path,
    requested: &str,
    must_exist: bool,
) -> Result<PathBuf, SheetOpsError> {
    if requested.trim().is_empty() {
        return Err(SheetOpsError::ParamRejected("路径不得为空".to_string()));
    }
    let canonical_root = std::fs::canonicalize(root).map_err(|_| {
        SheetOpsError::ReadFailed("workspace root is not accessible; 工作区根不可访问".to_string())
    })?;
    let candidate = if Path::new(requested).is_absolute() {
        PathBuf::from(requested)
    } else {
        canonical_root.join(requested)
    };

    crate::guardrail::ToolGuardrail::verify_path_access(&canonical_root, &candidate).map_err(
        |error| match error {
            crate::guardrail::PreCallGuardError::OutsideWorkspace(_) => {
                SheetOpsError::OutsideWorkspace
            }
            crate::guardrail::PreCallGuardError::PathTraversal(_)
            | crate::guardrail::PreCallGuardError::DangerousCommandInjection(_) => {
                SheetOpsError::PathRejected
            }
        },
    )?;

    let resolved = if must_exist {
        match std::fs::canonicalize(&candidate) {
            Ok(canonical) => canonical,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // 缺席目标也先做现实层包含判定 (最近存在祖先): 越界拼写照拒,
                // 不让「文件不存在」盖过「越界」。
                let ancestor_resolved = resolve_output_spelling(&candidate)?;
                if !ancestor_resolved.starts_with(&canonical_root) {
                    return Err(SheetOpsError::OutsideWorkspace);
                }
                return Err(SheetOpsError::ReadFailed(
                    "input table not found; 输入表不存在".to_string(),
                ));
            }
            Err(error) => {
                return Err(SheetOpsError::ReadFailed(format!(
                    "input table unreadable ({})",
                    error.kind()
                )));
            }
        }
    } else {
        resolve_output_spelling(&candidate)?
    };

    if !resolved.starts_with(&canonical_root) {
        return Err(SheetOpsError::OutsideWorkspace);
    }
    if is_sensitive_path(&canonical_root, &candidate)
        || is_sensitive_path(&canonical_root, &resolved)
    {
        return Err(SheetOpsError::SensitiveSurface);
    }
    Ok(resolved)
}

/// 输出面路径拼写: 最近存在的祖先 canonicalize, 尾段按原拼写接回 (目标可缺席)。
fn resolve_output_spelling(candidate: &Path) -> Result<PathBuf, SheetOpsError> {
    let mut ancestor = candidate.to_path_buf();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    loop {
        match std::fs::canonicalize(&ancestor) {
            Ok(canonical) => {
                let mut resolved = canonical;
                for component in tail.iter().rev() {
                    resolved.push(component);
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let Some(parent) = ancestor.parent() else {
                    return Err(SheetOpsError::OutsideWorkspace);
                };
                let Some(name) = ancestor.file_name() else {
                    return Err(SheetOpsError::OutsideWorkspace);
                };
                tail.push(name.to_os_string());
                ancestor = parent.to_path_buf();
            }
            Err(error) => {
                return Err(SheetOpsError::WriteFailed(format!(
                    "output directory is not accessible ({}); 输出目录不可访问",
                    error.kind()
                )));
            }
        }
    }
}

/// 原子创建一个输出文件 (只创建不覆盖: 目标已存在拒绝即帧)。
///
/// 走 [`gated_write_atomic`] 的 create-if-absent 门禁 + `storage_atomic`
/// 原子替换 (崩溃不留半截文件)。
fn write_output_create_only(path: &Path, bytes: &[u8]) -> Result<(), SheetOpsError> {
    let gate = ObservedGate::new();
    let request = WriteRequest {
        path: path.to_path_buf(),
        kind: WriteKind::Create,
        replace_if_version: None,
        create_if_absent: true,
    };
    gated_write_atomic(
        &gate,
        &FsVersionProbe,
        &request,
        bytes,
        apeireth_core::storage_atomic::DEFAULT_FILE_MODE,
    )
    .map_err(|error| match error {
        crate::observed_gate::GatedWriteError::Denied(denial) => {
            SheetOpsError::OutputExists(denial.message())
        }
        crate::observed_gate::GatedWriteError::Io(detail) => SheetOpsError::WriteFailed(detail),
    })
}

/// 读入一个受控输入 (限域 + 敏感面 + 字节上限按实读判定)。
fn read_input(
    config: &SheetOpsConfig,
    requested: &str,
) -> Result<(PathBuf, Vec<u8>), SheetOpsError> {
    let path = resolve_contained(&config.workspace_root, requested, true)?;
    let mut bytes = Vec::new();
    let file = std::fs::File::open(&path).map_err(|error| {
        SheetOpsError::ReadFailed(format!("input open failed ({})", error.kind()))
    })?;
    let mut limited = file.take(config.max_input_bytes.saturating_add(1));
    limited.read_to_end(&mut bytes).map_err(|error| {
        SheetOpsError::ReadFailed(format!("input read failed ({})", error.kind()))
    })?;
    if bytes.len() as u64 > config.max_input_bytes {
        return Err(SheetOpsError::LimitExceeded(format!(
            "input exceeds the {} byte limit (按实读判定); 输入字节超上限",
            config.max_input_bytes
        )));
    }
    Ok((path, bytes))
}

/// 相对拼写 (脱敏口径: 报告只回相对路径)。
fn relative_display(canonical: &Path, root: &Path) -> String {
    match canonical.strip_prefix(root) {
        Ok(relative) if relative.as_os_str().is_empty() => ".".to_string(),
        Ok(relative) => relative.to_string_lossy().replace('\\', "/"),
        Err(_) => "<path>".to_string(),
    }
}

// ---------------------------------------------------------------------------
// CSV 面 (RFC 4180 逗号方言, UTF-8; 坏词面拒绝即帧, 不静默修复)
// ---------------------------------------------------------------------------

/// 一张 CSV 表 (表头 + 数据行; 单元一律原文本, 数值轴另算)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sheet {
    /// 表头列名 (非空、互不重复)。
    pub columns: Vec<String>,
    /// 数据行 (每行单元数 = 列数)。
    pub rows: Vec<Vec<String>>,
}

impl Sheet {
    /// 列名下标 (不存在回 `None`)。
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|column| column == name)
    }

    /// 取一列的全部单元 (原文本)。
    pub fn column_cells(&self, index: usize) -> Vec<&str> {
        self.rows.iter().map(|row| row[index].as_str()).collect()
    }
}

/// 解析 CSV 表 (UTF-8 可带 BOM; 严格 RFC 4180 词面, 上限三道)。
pub fn parse_csv_table(bytes: &[u8], config: &SheetOpsConfig) -> Result<Sheet, SheetOpsError> {
    let bytes = bytes.strip_prefix(&[0xEFu8, 0xBB, 0xBF]).unwrap_or(bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| {
        SheetOpsError::ParseFailed(
            "输入不是 UTF-8 (本批 CSV 面只认 UTF-8, 不猜编码不转码)".to_string(),
        )
    })?;

    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut field_was_quoted = false;
    let mut chars = text.chars().peekable();
    let mut line = 1usize;

    macro_rules! end_field {
        ($row:expr, $field:expr) => {{
            $row.push(std::mem::take(&mut $field));
        }};
    }

    while let Some(ch) = chars.next() {
        if in_quotes {
            match ch {
                '"' => {
                    if chars.peek() == Some(&'"') {
                        chars.next();
                        field.push('"');
                    } else {
                        in_quotes = false;
                    }
                }
                '\n' => {
                    line += 1;
                    field.push('\n');
                }
                '\r' => field.push('\r'),
                other => field.push(other),
            }
            continue;
        }
        match ch {
            '"' => {
                if !field.is_empty() {
                    return Err(SheetOpsError::ParseFailed(format!(
                        "第 {line} 行: 引号不得出现在未引字段中间 (坏词面拒绝, 不静默修复)"
                    )));
                }
                if field_was_quoted {
                    return Err(SheetOpsError::ParseFailed(format!(
                        "第 {line} 行: 已闭合引号字段后不得再接字符 (坏词面拒绝)"
                    )));
                }
                in_quotes = true;
                field_was_quoted = true;
            }
            ',' => {
                end_field!(row, field);
                field_was_quoted = false;
            }
            '\n' => {
                end_field!(row, field);
                field_was_quoted = false;
                rows.push(std::mem::take(&mut row));
                line += 1;
            }
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                    end_field!(row, field);
                    field_was_quoted = false;
                    rows.push(std::mem::take(&mut row));
                    line += 1;
                } else {
                    return Err(SheetOpsError::ParseFailed(format!(
                        "第 {line} 行: 孤立 CR 行尾不在支持面 (只认 LF / CRLF)"
                    )));
                }
            }
            other => {
                if field_was_quoted {
                    return Err(SheetOpsError::ParseFailed(format!(
                        "第 {line} 行: 已闭合引号字段后不得再接字符 (坏词面拒绝)"
                    )));
                }
                field.push(other);
            }
        }
    }
    if in_quotes {
        return Err(SheetOpsError::ParseFailed(
            "引号未闭合 (坏词面拒绝, 不静默修复)".to_string(),
        ));
    }
    // 收尾: 最后一行无换行也算一行; 空文本零行。
    if !field.is_empty() || field_was_quoted || !row.is_empty() {
        end_field!(row, field);
        rows.push(row);
    }

    if rows.is_empty() {
        return Err(SheetOpsError::ParseFailed(
            "空表无表头 (零行输入拒绝; 仅表头 = 0 数据行)".to_string(),
        ));
    }
    let columns = rows.remove(0);
    if columns.is_empty() {
        return Err(SheetOpsError::ParseFailed(
            "表头零列 (至少一列)".to_string(),
        ));
    }
    for (index, name) in columns.iter().enumerate() {
        if name.trim().is_empty() {
            return Err(SheetOpsError::ParseFailed(format!(
                "表头第 {} 列列名为空 (列须可点名)",
                index + 1
            )));
        }
        if columns.iter().take(index).any(|earlier| earlier == name) {
            return Err(SheetOpsError::ParseFailed(format!(
                "表头列名 {name:?} 重复 (列名须唯一可点名)"
            )));
        }
    }
    if columns.len() > config.max_cols {
        return Err(SheetOpsError::LimitExceeded(format!(
            "表有 {} 列, 超过列上限 {} (防炸弹表, 宁拒不截)",
            columns.len(),
            config.max_cols
        )));
    }
    let width = columns.len();
    for (index, row) in rows.iter().enumerate() {
        if row.len() != width {
            return Err(SheetOpsError::ParseFailed(format!(
                "第 {} 行有 {} 单元, 表头 {} 列 (参差行拒绝, 不静默补空)",
                index + 2,
                row.len(),
                width
            )));
        }
    }
    if rows.len() > config.max_rows {
        return Err(SheetOpsError::LimitExceeded(format!(
            "表有 {} 数据行, 超过行上限 {} (防炸弹表, 宁拒不截)",
            rows.len(),
            config.max_rows
        )));
    }
    Ok(Sheet { columns, rows })
}

/// 规范 CSV 序列化 (RFC 4180: 需要时引号包裹, `""` 转义, LF 行尾, 末尾换行)。
pub fn serialize_csv(sheet: &Sheet) -> String {
    let mut out = String::new();
    let mut write_row = |cells: &[String], out: &mut String| {
        for (index, cell) in cells.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            if cell.contains(',')
                || cell.contains('"')
                || cell.contains('\n')
                || cell.contains('\r')
            {
                out.push('"');
                for ch in cell.chars() {
                    if ch == '"' {
                        out.push('"');
                    }
                    out.push(ch);
                }
                out.push('"');
            } else {
                out.push_str(cell);
            }
        }
        out.push('\n');
    };
    write_row(&sheet.columns, &mut out);
    for row in &sheet.rows {
        write_row(row, &mut out);
    }
    out
}

// ---------------------------------------------------------------------------
// xlsx (OOXML 表格) 读端: ZIP 容器 (复用 doc_ops 已报备 ZIP 读端) + XML 格网
// ---------------------------------------------------------------------------

/// 输入格式 (按**后缀词面**判定: `.xlsx` → xlsx 面, 其余 → CSV 面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetInputFormat {
    /// CSV (UTF-8 + RFC 4180 逗号方言)。
    Csv,
    /// xlsx (OOXML 表格容器: ZIP + XML)。
    Xlsx,
}

impl SheetInputFormat {
    /// 词面名 (`csv` / `xlsx`, 帧内 `input_format` 同值)。
    pub const fn name(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Xlsx => "xlsx",
        }
    }

    /// 从请求路径的后缀词面判定 (大小写不敏感; 无后缀 / 其它后缀 = CSV 面)。
    pub fn from_requested(requested: &str) -> Self {
        let is_xlsx = Path::new(requested)
            .extension()
            .is_some_and(|extension| extension.to_string_lossy().eq_ignore_ascii_case("xlsx"));
        if is_xlsx {
            Self::Xlsx
        } else {
            Self::Csv
        }
    }
}

/// 表格载入元数据 (格式 + xlsx 公式格如实计数)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetLoadMeta {
    /// 输入格式 (后缀词面判定)。
    pub format: SheetInputFormat,
    /// xlsx 公式格计数 (取缓存值口径, 未重算; CSV 恒 0)。
    pub formula_cells: u64,
}

/// ZIP 读端错误 → 表格错误 (解压炸弹归上限拒, 其余归解析拒; 真因如实)。
fn map_container_error(error: crate::doc_ops::DocOpsError) -> SheetOpsError {
    match error {
        crate::doc_ops::DocOpsError::UncompressedTooLarge { bytes, cap } => {
            SheetOpsError::LimitExceeded(format!(
                "xlsx 容器解压后 {bytes} 字节超过解压预算 {cap} 字节 (防解压炸弹, 宁拒不截)"
            ))
        }
        other => SheetOpsError::ParseFailed(format!("xlsx 容器: {}", other.message())),
    }
}

/// XML 实体解码 (预定义实体 + 数字引用; 未知实体按缺字显式 `?`, 不编字形)。
fn decode_xml_entity(entity: &str) -> String {
    match entity {
        "amp" => "&".to_string(),
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "quot" => "\"".to_string(),
        "apos" => "'".to_string(),
        other => {
            let digits = other.strip_prefix('#').unwrap_or_default();
            let code = if let Some(hex) = digits
                .strip_prefix('x')
                .or_else(|| digits.strip_prefix('X'))
            {
                u32::from_str_radix(hex, 16).ok()
            } else {
                digits.parse::<u32>().ok()
            };
            code.and_then(char::from_u32)
                .map(String::from)
                .unwrap_or("?".to_string())
        }
    }
}

/// xlsx 共享字符串表 (`xl/sharedStrings.xml`): 每 `<si>` 一串 (run 文本拼接)。
fn xlsx_shared_strings(xml: &str) -> Result<Vec<String>, SheetOpsError> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut strings: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_text = false;
    loop {
        let event = reader.read_event().map_err(|error| {
            SheetOpsError::ParseFailed(format!("sharedStrings.xml 标记损坏: {error}"))
        })?;
        match event {
            Event::Start(start) => match start.local_name().as_ref() {
                b"si" => current.clear(),
                b"t" => in_text = true,
                _ => {}
            },
            Event::Text(text) => {
                if in_text {
                    current.push_str(&text.decode().map_err(|error| {
                        SheetOpsError::ParseFailed(format!("共享字符串文本解码失败: {error}"))
                    })?);
                }
            }
            Event::GeneralRef(reference) => {
                if in_text {
                    let raw = reference.into_inner();
                    let entity = std::str::from_utf8(&raw).unwrap_or_default();
                    current.push_str(&decode_xml_entity(entity));
                }
            }
            Event::End(end) => match end.local_name().as_ref() {
                b"t" => in_text = false,
                b"si" => strings.push(std::mem::take(&mut current)),
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(strings)
}

/// `workbook.xml` 首个 `<sheet>` 声明的关系 id (`r:id` / `id` 属性)。
fn xlsx_first_sheet_rel_id(xml: &str) -> Result<String, SheetOpsError> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    loop {
        let event = reader.read_event().map_err(|error| {
            SheetOpsError::ParseFailed(format!("workbook.xml 标记损坏: {error}"))
        })?;
        match event {
            Event::Start(start) | Event::Empty(start) => {
                if start.local_name().as_ref() == b"sheet" {
                    for attribute in start.attributes() {
                        let attribute = attribute.map_err(|error| {
                            SheetOpsError::ParseFailed(format!("workbook.xml 属性损坏: {error}"))
                        })?;
                        if attribute.key.local_name().as_ref() == b"id" {
                            return Ok(attribute
                                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                .map_err(|error| {
                                    SheetOpsError::ParseFailed(format!(
                                        "workbook.xml 关系 id 损坏: {error}"
                                    ))
                                })?
                                .into_owned());
                        }
                    }
                    return Err(SheetOpsError::ParseFailed(
                        "workbook.xml 首个 <sheet> 缺关系 id (无法定位工作表)".to_string(),
                    ));
                }
            }
            Event::Eof => {
                return Err(SheetOpsError::ParseFailed(
                    "workbook.xml 无 <sheet> 声明 (不是工作簿)".to_string(),
                ));
            }
            _ => {}
        }
    }
}

/// `xl/_rels/workbook.xml.rels`: 关系 id → 工作表包内路径 (子集只认工作表目标)。
fn xlsx_sheet_target(xml: &str, rel_id: &str) -> Result<String, SheetOpsError> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    loop {
        let event = reader.read_event().map_err(|error| {
            SheetOpsError::ParseFailed(format!("workbook.xml.rels 标记损坏: {error}"))
        })?;
        match event {
            Event::Start(start) | Event::Empty(start) => {
                if start.local_name().as_ref() == b"Relationship" {
                    let mut id = String::new();
                    let mut target = String::new();
                    for attribute in start.attributes() {
                        let attribute = attribute.map_err(|error| {
                            SheetOpsError::ParseFailed(format!(
                                "workbook.xml.rels 属性损坏: {error}"
                            ))
                        })?;
                        match attribute.key.local_name().as_ref() {
                            b"Id" => {
                                id = attribute
                                    .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                    .map_err(|error| {
                                        SheetOpsError::ParseFailed(format!(
                                            "workbook.xml.rels Id 损坏: {error}"
                                        ))
                                    })?
                                    .into_owned();
                            }
                            b"Target" => {
                                target = attribute
                                    .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                    .map_err(|error| {
                                        SheetOpsError::ParseFailed(format!(
                                            "workbook.xml.rels Target 损坏: {error}"
                                        ))
                                    })?
                                    .into_owned();
                            }
                            _ => {}
                        }
                    }
                    if id == rel_id {
                        if target.contains("..") {
                            return Err(SheetOpsError::ParseFailed(
                                "工作表关系目标含 .. (包内穿越拒绝)".to_string(),
                            ));
                        }
                        let package_path = if let Some(absolute) = target.strip_prefix('/') {
                            absolute.to_string()
                        } else {
                            format!("xl/{target}")
                        };
                        return Ok(package_path);
                    }
                }
            }
            Event::Eof => {
                return Err(SheetOpsError::ParseFailed(format!(
                    "workbook.xml.rels 无关系 {rel_id} (无法定位工作表)"
                )));
            }
            _ => {}
        }
    }
}

/// 单元格列引用 (`A1` 形: 前导字母 = 列) → 0 基列号。
fn xlsx_column_of(reference: &str) -> Result<usize, SheetOpsError> {
    let letters: String = reference
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .collect();
    let rest = &reference[letters.len()..];
    if letters.is_empty() || rest.is_empty() || !rest.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(SheetOpsError::ParseFailed(format!(
            "单元格引用 {reference:?} 不是合法 A1 形 (拒绝不猜位)"
        )));
    }
    if letters.len() > 7 {
        return Err(SheetOpsError::ParseFailed(format!(
            "单元格引用 {reference:?} 列字母超界 (拒绝不猜位)"
        )));
    }
    let mut column = 0usize;
    for character in letters.chars() {
        column = column * 26 + (character.to_ascii_uppercase() as usize - 'A' as usize + 1);
    }
    Ok(column - 1)
}

/// 一行的稀疏格 (0 基列号 → 单元文本)。
type XlsxRow = Vec<(usize, String)>;

/// 解析工作表 XML → 稀疏行序列 + 公式格计数 (口径见模块文档: 公式格取缓存值)。
fn xlsx_parse_sheet(xml: &str, shared: &[String]) -> Result<(Vec<XlsxRow>, u64), SheetOpsError> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    #[derive(Default)]
    struct CellState {
        column: usize,
        kind: Option<String>,
        value: String,
        inline: String,
        has_value: bool,
        has_inline: bool,
        formula: bool,
    }

    let mut reader = Reader::from_str(xml);
    let mut rows: Vec<XlsxRow> = Vec::new();
    let mut row: XlsxRow = Vec::new();
    let mut next_column = 0usize;
    let mut row_open = false;
    let mut cell: Option<CellState> = None;
    let mut in_value = false;
    let mut in_text = false;
    let mut in_inline = false;
    let mut formula_cells = 0u64;

    loop {
        let event = reader
            .read_event()
            .map_err(|error| SheetOpsError::ParseFailed(format!("worksheet 标记损坏: {error}")))?;
        match event {
            Event::Start(start) => match start.local_name().as_ref() {
                b"row" => {
                    row_open = true;
                    row = XlsxRow::new();
                    next_column = 0;
                }
                b"c" => {
                    let mut column = next_column;
                    let mut kind: Option<String> = None;
                    for attribute in start.attributes() {
                        let attribute = attribute.map_err(|error| {
                            SheetOpsError::ParseFailed(format!("单元格属性损坏: {error}"))
                        })?;
                        match attribute.key.local_name().as_ref() {
                            b"r" => {
                                let reference = attribute
                                    .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                    .map_err(|error| {
                                        SheetOpsError::ParseFailed(format!(
                                            "单元格引用损坏: {error}"
                                        ))
                                    })?
                                    .into_owned();
                                column = xlsx_column_of(&reference)?;
                            }
                            b"t" => {
                                kind = Some(
                                    attribute
                                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                        .map_err(|error| {
                                            SheetOpsError::ParseFailed(format!(
                                                "单元格类型损坏: {error}"
                                            ))
                                        })?
                                        .into_owned(),
                                );
                            }
                            _ => {}
                        }
                    }
                    if let Some(kind) = kind.as_deref() {
                        if !matches!(kind, "s" | "inlineStr" | "str" | "n") {
                            return Err(SheetOpsError::ParseFailed(format!(
                                "单元格类型 t={kind:?} 不在子集 (只认 s/inlineStr/str/n/缺省), \
                                 显式拒绝不猜值"
                            )));
                        }
                    }
                    next_column = column + 1;
                    cell = Some(CellState {
                        column,
                        kind,
                        ..CellState::default()
                    });
                }
                b"v" => in_value = true,
                b"f" => {
                    if let Some(state) = cell.as_mut() {
                        state.formula = true;
                    }
                }
                b"is" => in_inline = true,
                b"t" => {
                    if in_inline {
                        in_text = true;
                    }
                }
                _ => {}
            },
            Event::Empty(empty) => match empty.local_name().as_ref() {
                b"row" => {
                    // 显式空行: 文档序中占一行 (全空单元)。
                    rows.push(XlsxRow::new());
                }
                b"c" => {
                    // 空单元: 占位不取值 (稀疏格网语义)。
                    if row_open {
                        let mut column = next_column;
                        for attribute in empty.attributes() {
                            let attribute = attribute.map_err(|error| {
                                SheetOpsError::ParseFailed(format!("单元格属性损坏: {error}"))
                            })?;
                            if attribute.key.local_name().as_ref() == b"r" {
                                let reference = attribute
                                    .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                    .map_err(|error| {
                                        SheetOpsError::ParseFailed(format!(
                                            "单元格引用损坏: {error}"
                                        ))
                                    })?
                                    .into_owned();
                                column = xlsx_column_of(&reference)?;
                            }
                        }
                        next_column = column + 1;
                    }
                }
                b"f" => {
                    if let Some(state) = cell.as_mut() {
                        state.formula = true;
                    }
                }
                b"is" => {}
                b"t" | b"v" => {}
                _ => {}
            },
            Event::Text(text) => {
                let decoded = text.decode().map_err(|error| {
                    SheetOpsError::ParseFailed(format!("文本解码失败: {error}"))
                })?;
                if in_value {
                    if let Some(state) = cell.as_mut() {
                        state.value.push_str(&decoded);
                        state.has_value = true;
                    }
                } else if in_text {
                    if let Some(state) = cell.as_mut() {
                        state.inline.push_str(&decoded);
                        state.has_inline = true;
                    }
                }
            }
            Event::GeneralRef(reference) => {
                if in_value || in_text {
                    let raw = reference.into_inner();
                    let entity = std::str::from_utf8(&raw).unwrap_or_default();
                    let decoded = decode_xml_entity(entity);
                    if in_value {
                        if let Some(state) = cell.as_mut() {
                            state.value.push_str(&decoded);
                            state.has_value = true;
                        }
                    } else if let Some(state) = cell.as_mut() {
                        state.inline.push_str(&decoded);
                        state.has_inline = true;
                    }
                }
            }
            Event::End(end) => match end.local_name().as_ref() {
                b"v" => in_value = false,
                b"t" => in_text = false,
                b"is" => in_inline = false,
                b"c" => {
                    let state = cell.take().ok_or_else(|| {
                        SheetOpsError::ParseFailed("单元格闭合无开 (标记损坏)".to_string())
                    })?;
                    if row.iter().any(|(column, _)| *column == state.column) {
                        return Err(SheetOpsError::ParseFailed(format!(
                            "同一行重复列引用 (列 {}), 拒绝不猜位",
                            state.column + 1
                        )));
                    }
                    let text = match state.kind.as_deref() {
                        Some("s") => {
                            let index: usize = state.value.trim().parse().map_err(|_| {
                                SheetOpsError::ParseFailed(format!(
                                    "共享字符串索引 {:?} 不是合法整数 (拒绝不猜值)",
                                    state.value
                                ))
                            })?;
                            shared.get(index).cloned().ok_or_else(|| {
                                SheetOpsError::ParseFailed(format!(
                                    "共享字符串索引 {index} 越界 (表共 {} 项)",
                                    shared.len()
                                ))
                            })?
                        }
                        Some("inlineStr") => state.inline,
                        Some("str") => state.value,
                        Some("n") | None => state.value,
                        Some(other) => {
                            return Err(SheetOpsError::ParseFailed(format!(
                                "单元格类型 t={other:?} 不在子集, 显式拒绝不猜值"
                            )));
                        }
                    };
                    if state.formula {
                        formula_cells += 1;
                        if !state.has_value && !state.has_inline {
                            return Err(SheetOpsError::ParseFailed(format!(
                                "公式格 (列 {}) 无缓存值: 本面不重算公式 (取缓存值 + 标注口径), \
                                 不编造值",
                                state.column + 1
                            )));
                        }
                    }
                    row.push((state.column, text));
                }
                b"row" => {
                    row_open = false;
                    rows.push(std::mem::take(&mut row));
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    if cell.is_some() {
        return Err(SheetOpsError::ParseFailed(
            "worksheet 末尾有未闭合单元格 (标记损坏)".to_string(),
        ));
    }
    Ok((rows, formula_cells))
}

/// 稀疏行序列 → 密实 [`Sheet`] (表头规则与 CSV 同一把尺: 非空、唯一;
/// xlsx 缺格补空, 越宽拒绝即帧)。
fn xlsx_rows_to_sheet(rows: Vec<XlsxRow>, config: &SheetOpsConfig) -> Result<Sheet, SheetOpsError> {
    if rows.is_empty() {
        return Err(SheetOpsError::ParseFailed(
            "空表无表头 (零行输入拒绝; 仅表头 = 0 数据行)".to_string(),
        ));
    }
    let header = &rows[0];
    let width = header.iter().map(|(column, _)| *column).max().unwrap_or(0) + 1;
    let mut columns = vec![String::new(); width];
    for (column, name) in header {
        columns[*column] = name.clone();
    }
    for (index, name) in columns.iter().enumerate() {
        if name.trim().is_empty() {
            return Err(SheetOpsError::ParseFailed(format!(
                "表头第 {} 列列名为空 (列须可点名; xlsx 缺格不补位)",
                index + 1
            )));
        }
        if columns.iter().take(index).any(|earlier| earlier == name) {
            return Err(SheetOpsError::ParseFailed(format!(
                "表头列名 {name:?} 重复 (列名须唯一可点名)"
            )));
        }
    }
    if columns.len() > config.max_cols {
        return Err(SheetOpsError::LimitExceeded(format!(
            "表有 {} 列, 超过列上限 {} (防炸弹表, 宁拒不截)",
            columns.len(),
            config.max_cols
        )));
    }
    let mut out_rows: Vec<Vec<String>> = Vec::with_capacity(rows.len().saturating_sub(1));
    for (offset, row) in rows.iter().skip(1).enumerate() {
        if let Some((column, _)) = row.iter().find(|(column, _)| *column >= width) {
            return Err(SheetOpsError::ParseFailed(format!(
                "第 {} 行第 {} 列落在表头 {width} 列之外 (xlsx 缺格补空, 越宽拒绝即帧)",
                offset + 2,
                column + 1
            )));
        }
        let mut dense = vec![String::new(); width];
        for (column, text) in row {
            dense[*column] = text.clone();
        }
        out_rows.push(dense);
    }
    if out_rows.len() > config.max_rows {
        return Err(SheetOpsError::LimitExceeded(format!(
            "表有 {} 数据行, 超过行上限 {} (防炸弹表, 宁拒不截)",
            out_rows.len(),
            config.max_rows
        )));
    }
    Ok(Sheet {
        columns,
        rows: out_rows,
    })
}

/// 解析一份 xlsx 表 (限域外的字节安全由调用方承担): 返回 (表, 公式格计数)。
///
/// 子集钉死: ZIP 容器 (stored/deflate, 复用 `doc_ops` ZIP 读端, 解压预算同族) +
/// `workbook.xml` 首个工作表 (rels 解析) + 单元格三形态; 其余显式拒绝不降级。
pub fn parse_xlsx_table(
    bytes: &[u8],
    config: &SheetOpsConfig,
) -> Result<(Sheet, u64), SheetOpsError> {
    let budget = config
        .max_input_bytes
        .saturating_mul(crate::doc_ops::ZIP_DECOMPRESSION_BUDGET_FACTOR);
    let entries = crate::doc_ops::zip_read_entries(bytes, budget).map_err(map_container_error)?;
    let entry = |name: &str| -> Option<&crate::doc_ops::ZipEntry> {
        entries.iter().find(|entry| entry.name == name)
    };
    let workbook = entry("xl/workbook.xml").ok_or_else(|| {
        SheetOpsError::ParseFailed("xlsx 容器缺 xl/workbook.xml (不是工作簿)".to_string())
    })?;
    let rels = entry("xl/_rels/workbook.xml.rels").ok_or_else(|| {
        SheetOpsError::ParseFailed(
            "xlsx 容器缺 xl/_rels/workbook.xml.rels (无法定位工作表)".to_string(),
        )
    })?;
    let workbook_xml = std::str::from_utf8(&workbook.data)
        .map_err(|_| SheetOpsError::ParseFailed("xl/workbook.xml 不是 UTF-8".to_string()))?;
    let rels_xml = std::str::from_utf8(&rels.data).map_err(|_| {
        SheetOpsError::ParseFailed("xl/_rels/workbook.xml.rels 不是 UTF-8".to_string())
    })?;
    let rel_id = xlsx_first_sheet_rel_id(workbook_xml)?;
    let target = xlsx_sheet_target(rels_xml, &rel_id)?;
    let worksheet = entry(&target).ok_or_else(|| {
        SheetOpsError::ParseFailed(format!(
            "xlsx 容器缺工作表条目 {target} (workbook.xml 首个 sheet 声明所指)"
        ))
    })?;
    let worksheet_xml = std::str::from_utf8(&worksheet.data)
        .map_err(|_| SheetOpsError::ParseFailed("工作表 XML 不是 UTF-8".to_string()))?;
    let shared = match entry("xl/sharedStrings.xml") {
        Some(entry) => {
            let xml = std::str::from_utf8(&entry.data).map_err(|_| {
                SheetOpsError::ParseFailed("sharedStrings.xml 不是 UTF-8".to_string())
            })?;
            xlsx_shared_strings(xml)?
        }
        None => Vec::new(),
    };
    let (rows, formula_cells) = xlsx_parse_sheet(worksheet_xml, &shared)?;
    let sheet = xlsx_rows_to_sheet(rows, config)?;
    Ok((sheet, formula_cells))
}

// ---------------------------------------------------------------------------
// 数值轴 (词面 → f64; NaN/inf/溢出词面永远是文本)
// ---------------------------------------------------------------------------

/// 数值词面解析 (钉死口径): `[-+]?(digits[.digits?] | .digits)([eE][-+]?digits)?`
/// 且结果**有限**; 其余 (`NaN`/`inf`/`1e999`/`1,000`/`0x10`/空) 一律 `None`。
pub fn parse_number(text: &str) -> Option<f64> {
    let bytes = text.as_bytes();
    let mut index = 0usize;
    if index < bytes.len() && matches!(bytes[index], b'+' | b'-') {
        index += 1;
    }
    let mut digits_before = 0usize;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
        digits_before += 1;
    }
    let mut digits_after = 0usize;
    if index < bytes.len() && bytes[index] == b'.' {
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
            digits_after += 1;
        }
    }
    if digits_before + digits_after == 0 {
        return None;
    }
    if index < bytes.len() && matches!(bytes[index], b'e' | b'E') {
        index += 1;
        if index < bytes.len() && matches!(bytes[index], b'+' | b'-') {
            index += 1;
        }
        let mut exponent_digits = 0usize;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
            exponent_digits += 1;
        }
        if exponent_digits == 0 {
            return None;
        }
    }
    if index != bytes.len() {
        return None;
    }
    text.parse::<f64>().ok().filter(|value| value.is_finite())
}

/// 整数字面 (纯 `[-+]?digits`, 无小数点无指数)。
fn is_integer_literal(text: &str) -> bool {
    let body = text
        .strip_prefix('+')
        .or_else(|| text.strip_prefix('-'))
        .unwrap_or(text);
    !body.is_empty() && body.bytes().all(|byte| byte.is_ascii_digit())
}

/// 逐列类型猜测 (钉死口径)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    /// 全空列。
    Empty,
    /// 非空单元全是整数字面。
    Int,
    /// 非空单元全数值但含非整数字面。
    Float,
    /// 非空单元全非数值。
    Text,
    /// 数值与非数值混在。
    Mixed,
}

impl ColumnType {
    /// 词面名 (`empty` / `int` / `float` / `text` / `mixed`)。
    pub const fn name(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Int => "int",
            Self::Float => "float",
            Self::Text => "text",
            Self::Mixed => "mixed",
        }
    }
}

/// 一列的类型猜测 (非空单元判型; 全空 = `empty`)。
pub fn column_type(cells: &[&str]) -> ColumnType {
    let mut any_numeric = false;
    let mut any_text = false;
    let mut all_integer = true;
    for cell in cells {
        if cell.is_empty() {
            continue;
        }
        match parse_number(cell) {
            Some(_) => {
                any_numeric = true;
                if !is_integer_literal(cell) {
                    all_integer = false;
                }
            }
            None => any_text = true,
        }
    }
    match (any_numeric, any_text) {
        (false, false) => ColumnType::Empty,
        (true, false) if all_integer => ColumnType::Int,
        (true, false) => ColumnType::Float,
        (false, true) => ColumnType::Text,
        (true, true) => ColumnType::Mixed,
    }
}

// ---------------------------------------------------------------------------
// 公式面 (闭合文法: 四则 + 括号 + 一元符号; NaN/inf 零产出)
// ---------------------------------------------------------------------------

/// 公式语法树。
#[derive(Debug, Clone, PartialEq)]
enum Expr {
    /// 数值字面。
    Number(f64),
    /// 列引用 (已解析下标)。
    Column(usize),
    /// 一元符号。
    Unary(bool, Box<Expr>),
    /// 四则二元 (`true` = 乘, `false` = 除; 加减同构存 op 字符)。
    Binary(char, Box<Expr>, Box<Expr>),
}

/// 公式解析 (闭合文法): `expr := term (('+'|'-') term)*`,
/// `term := factor (('*'|'/') factor)*`,
/// `factor := ('+'|'-') factor | '(' expr ')' | NUMBER | COLUMN`。
///
/// 列引用 = 非空白非运算符的词面且精确匹配表头列名; 数字词面优先按数值。
fn parse_expression(expression: &str, columns: &[String]) -> Result<Expr, SheetOpsError> {
    if expression.trim().is_empty() {
        return Err(SheetOpsError::ParamRejected(
            "formula: expression 不得为空".to_string(),
        ));
    }
    if expression.chars().count() > MAX_EXPRESSION_CHARS {
        return Err(SheetOpsError::ParamRejected(format!(
            "formula: expression 超过 {MAX_EXPRESSION_CHARS} 字符上限"
        )));
    }
    let tokens = tokenize_expression(expression)?;
    let mut parser = ExprParser {
        tokens,
        position: 0,
        depth: 0,
    };
    let expr = parser.parse_expr(columns)?;
    if parser.position != parser.tokens.len() {
        return Err(SheetOpsError::FormulaError(format!(
            "表达式在词面 {:?} 处多余内容",
            parser.tokens[parser.position]
        )));
    }
    Ok(expr)
}

/// 公式词面。
#[derive(Debug, Clone, PartialEq)]
enum Token {
    /// 加。
    Plus,
    /// 减。
    Minus,
    /// 乘。
    Star,
    /// 除。
    Slash,
    /// 左括号。
    LParen,
    /// 右括号。
    RParen,
    /// 数值/列名词面 (判定在解析期)。
    Atom(String),
}

fn tokenize_expression(expression: &str) -> Result<Vec<Token>, SheetOpsError> {
    let mut tokens = Vec::new();
    let mut chars: Vec<char> = expression.chars().collect();
    chars.reverse();
    while let Some(&ch) = chars.last() {
        if ch.is_whitespace() {
            chars.pop();
            continue;
        }
        match ch {
            '+' => {
                chars.pop();
                tokens.push(Token::Plus);
            }
            '-' => {
                chars.pop();
                tokens.push(Token::Minus);
            }
            '*' => {
                chars.pop();
                tokens.push(Token::Star);
            }
            '/' => {
                chars.pop();
                tokens.push(Token::Slash);
            }
            '(' => {
                chars.pop();
                tokens.push(Token::LParen);
            }
            ')' => {
                chars.pop();
                tokens.push(Token::RParen);
            }
            _ => {
                let mut atom = String::new();
                while let Some(&next) = chars.last() {
                    let after_e = atom
                        .chars()
                        .next_back()
                        .is_some_and(|previous| previous == 'e' || previous == 'E');
                    let is_operator = matches!(next, '+' | '-' | '*' | '/' | '(' | ')')
                        && !(after_e && matches!(next, '+' | '-'));
                    if next.is_whitespace() || is_operator {
                        break;
                    }
                    atom.push(next);
                    chars.pop();
                }
                if atom.is_empty() {
                    return Err(SheetOpsError::FormulaError(format!(
                        "表达式含非法字符 {ch:?}"
                    )));
                }
                tokens.push(Token::Atom(atom));
            }
        }
    }
    Ok(tokens)
}

struct ExprParser {
    tokens: Vec<Token>,
    position: usize,
    depth: usize,
}

impl ExprParser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn parse_expr(&mut self, columns: &[String]) -> Result<Expr, SheetOpsError> {
        let mut left = self.parse_term(columns)?;
        while matches!(self.peek(), Some(Token::Plus) | Some(Token::Minus)) {
            let op = if self.peek() == Some(&Token::Plus) {
                '+'
            } else {
                '-'
            };
            self.position += 1;
            let right = self.parse_term(columns)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_term(&mut self, columns: &[String]) -> Result<Expr, SheetOpsError> {
        let mut left = self.parse_factor(columns)?;
        while matches!(self.peek(), Some(Token::Star) | Some(Token::Slash)) {
            let op = if self.peek() == Some(&Token::Star) {
                '*'
            } else {
                '/'
            };
            self.position += 1;
            let right = self.parse_factor(columns)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_factor(&mut self, columns: &[String]) -> Result<Expr, SheetOpsError> {
        match self.peek().cloned() {
            Some(Token::Plus) => {
                self.position += 1;
                self.parse_factor(columns)
            }
            Some(Token::Minus) => {
                self.position += 1;
                let inner = self.parse_factor(columns)?;
                Ok(Expr::Unary(true, Box::new(inner)))
            }
            Some(Token::LParen) => {
                self.position += 1;
                self.depth += 1;
                if self.depth > MAX_EXPRESSION_DEPTH {
                    return Err(SheetOpsError::FormulaError(format!(
                        "表达式括号嵌套超过 {MAX_EXPRESSION_DEPTH} 层"
                    )));
                }
                let inner = self.parse_expr(columns)?;
                self.depth -= 1;
                if self.peek() != Some(&Token::RParen) {
                    return Err(SheetOpsError::FormulaError(
                        "括号未闭合 (表达式文法拒绝)".to_string(),
                    ));
                }
                self.position += 1;
                Ok(inner)
            }
            Some(Token::Atom(atom)) => {
                self.position += 1;
                if let Some(value) = parse_number(&atom) {
                    return Ok(Expr::Number(value));
                }
                match columns.iter().position(|column| *column == atom) {
                    Some(index) => Ok(Expr::Column(index)),
                    None => Err(SheetOpsError::FormulaError(format!(
                        "未知列 {atom:?} (列引用须精确匹配表头列名; 数字名列不可引用)"
                    ))),
                }
            }
            Some(Token::RParen) => Err(SheetOpsError::FormulaError(
                "多余右括号 (表达式文法拒绝)".to_string(),
            )),
            Some(Token::Star) | Some(Token::Slash) => Err(SheetOpsError::FormulaError(
                "运算符缺操作数 (表达式文法拒绝)".to_string(),
            )),
            None => Err(SheetOpsError::FormulaError(
                "表达式意外结束 (文法拒绝)".to_string(),
            )),
        }
    }
}

/// 求值 (逐行): 非有限结果 / 除零 / 非数值单元一律拒绝即帧 —— NaN/inf 零产出。
fn eval_expression(expr: &Expr, row: &[String], row_number: usize) -> Result<f64, SheetOpsError> {
    let finite = |value: f64, at: &str| -> Result<f64, SheetOpsError> {
        if value.is_finite() {
            Ok(value)
        } else {
            Err(SheetOpsError::FormulaError(format!(
                "第 {row_number} 行: {at} 运算结果非有限值 (inf/NaN 永不产出)"
            )))
        }
    };
    match expr {
        Expr::Number(value) => Ok(*value),
        Expr::Column(index) => {
            let cell = &row[*index];
            match parse_number(cell) {
                Some(value) => Ok(value),
                None => Err(SheetOpsError::FormulaError(format!(
                    "第 {row_number} 行: 列单元 {cell:?} 非数值或为空 (计算列要求引用列全是数值)"
                ))),
            }
        }
        Expr::Unary(_, inner) => {
            let value = eval_expression(inner, row, row_number)?;
            finite(-value, "取负")
        }
        Expr::Binary(op, left, right) => {
            let lhs = eval_expression(left, row, row_number)?;
            let rhs = eval_expression(right, row, row_number)?;
            let value = match op {
                '+' => lhs + rhs,
                '-' => lhs - rhs,
                '*' => lhs * rhs,
                '/' => {
                    if rhs == 0.0 {
                        return Err(SheetOpsError::FormulaError(format!(
                            "第 {row_number} 行: 除零拒绝 (不产出 inf/NaN)"
                        )));
                    }
                    lhs / rhs
                }
                _ => unreachable!("闭合运算符集"),
            };
            finite(value, "四则")
        }
    }
}

// ---------------------------------------------------------------------------
// 调用参数与参数矩阵 (纯判定, 工具面与边界 Guard 同一真源)
// ---------------------------------------------------------------------------

/// 调用参数 (未知字段拒绝)。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SheetOpsParams {
    /// 动作 (闭合词表)。
    pub action: Option<String>,
    /// 输入表 (工作区相对拼写或其内绝对拼写; 后缀 `.xlsx` 走 xlsx 读端,
    /// 其余走 CSV 读端)。
    pub source: Option<String>,
    /// 输出 CSV (变换动作可选; `export_csv` 必填) —— 在场即写面。
    pub output: Option<String>,
    /// 目标列名 (filter / sort / aggregate)。
    pub column: Option<String>,
    /// filter 谓词 (闭合词表)。
    pub predicate: Option<String>,
    /// filter 比较值 (非空; eq/ne 数值轴或文本轴, lt/lte/gt/gte 数值轴,
    /// contains 文本包含)。
    pub value: Option<String>,
    /// sort 顺序 (`asc` / `desc`, 缺省 `asc`)。
    pub order: Option<String>,
    /// aggregate 函数 (闭合词表)。
    pub function: Option<String>,
    /// formula 表达式 (四则 + 括号 + 列引用)。
    pub expression: Option<String>,
    /// formula 新列名 (不得与既有列重名)。
    pub new_column: Option<String>,
    /// read 行偏移 (缺省 0)。
    pub offset: Option<u64>,
    /// read 行预算 (缺省 = 配置预览上限; 超上限拒绝即帧)。
    pub limit: Option<u64>,
}

/// 写面判定 (闭合词表; `export_csv` 恒写, 变换动作带 `output` 即写)。
pub fn is_sheet_ops_write_action(action: &str, has_output: bool) -> bool {
    if SHEET_OPS_WRITE_ACTIONS.contains(&action) {
        return true;
    }
    SHEET_OPS_TRANSFORM_ACTIONS.contains(&action) && has_output
}

/// 逐动作字段集合 (恰好允许这些字段; 多余/缺失拒绝即帧)。
fn allowed_fields(action: &str) -> &'static [&'static str] {
    match action {
        ACTION_READ => &["offset", "limit"],
        ACTION_SUMMARY => &[],
        ACTION_FILTER => &["column", "predicate", "value", "output"],
        ACTION_SORT => &["column", "order", "output"],
        ACTION_AGGREGATE => &["column", "function"],
        ACTION_FORMULA => &["expression", "new_column", "output"],
        _ => &["output"],
    }
}

/// 参数矩阵校验 (纯判定): 返回 `Some(拒绝帧)` 即拒。
///
/// 逐项: ① 动作在闭合词表; ② 每动作的必填/禁填字段矩阵; ③ 谓词/函数/顺序
/// 在闭合词表; ④ 写面受写总闸约束 (关 = 拒写不拒读); ⑤ 路径工作区限域 +
/// 敏感面拒; ⑥ 输入字节上限预检; ⑦ 输出只创建不覆盖预检。列存在性/表解析
/// 在执行面判定 (需要读表)。
pub fn sheet_ops_params_refusal(
    config: &SheetOpsConfig,
    params: &SheetOpsParams,
) -> Option<SheetOpsError> {
    let Some(action) = params.action.as_deref() else {
        return Some(SheetOpsError::ParamRejected(format!(
            "必须显式给 action (闭合词表: {})",
            SHEET_OPS_ACTIONS.join(" / ")
        )));
    };
    if !SHEET_OPS_ACTIONS.contains(&action) {
        return Some(SheetOpsError::ParamRejected(format!(
            "未知动作 {action:?}; 闭合词表是 {}",
            SHEET_OPS_ACTIONS.join(" / ")
        )));
    }
    if params
        .source
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return Some(SheetOpsError::ParamRejected(format!(
            "{action} 必须给非空 source"
        )));
    }

    // 字段集合矩阵: 只认该动作的字段, 多余/缺失拒绝即帧。
    let allowed = allowed_fields(action);
    let present: Vec<(&'static str, bool)> = vec![
        ("output", params.output.is_some()),
        ("column", params.column.is_some()),
        ("predicate", params.predicate.is_some()),
        ("value", params.value.is_some()),
        ("order", params.order.is_some()),
        ("function", params.function.is_some()),
        ("expression", params.expression.is_some()),
        ("new_column", params.new_column.is_some()),
        ("offset", params.offset.is_some()),
        ("limit", params.limit.is_some()),
    ];
    for (field, is_present) in &present {
        if *is_present && !allowed.contains(field) {
            return Some(SheetOpsError::ParamRejected(format!(
                "{action}: 不接受参数 [{field}] (该动作只认 {:?})",
                allowed
            )));
        }
    }
    // 必填字段校验 (闭包回拒绝帧; 命中即返回, 不吞真因)。
    let require = |field: &'static str, condition: bool| -> Option<SheetOpsError> {
        if condition {
            None
        } else {
            Some(SheetOpsError::ParamRejected(format!(
                "{action}: 缺参数 [{field}]"
            )))
        }
    };
    macro_rules! require {
        ($field:expr, $condition:expr) => {
            if let Some(refusal) = require($field, $condition) {
                return Some(refusal);
            }
        };
    }

    match action {
        ACTION_READ => {
            if let Some(limit) = params.limit {
                if limit == 0 || limit as usize > config.max_preview_rows {
                    return Some(SheetOpsError::ParamRejected(format!(
                        "read: limit 必须在 1..={} (行预览上限), 得到 {limit}",
                        config.max_preview_rows
                    )));
                }
            }
        }
        ACTION_SUMMARY => {}
        ACTION_FILTER => {
            require!(
                "column",
                params
                    .column
                    .as_deref()
                    .is_some_and(|c| !c.trim().is_empty())
            );
            require!("predicate", params.predicate.is_some());
            let predicate = params.predicate.as_deref().unwrap_or_default();
            if !FILTER_PREDICATES.contains(&predicate) {
                return Some(SheetOpsError::ParamRejected(format!(
                    "filter: predicate 只认 {FILTER_PREDICATES:?}, 得到 {predicate:?}"
                )));
            }
            require!(
                "value",
                params.value.as_deref().is_some_and(|v| !v.is_empty())
            );
        }
        ACTION_SORT => {
            require!(
                "column",
                params
                    .column
                    .as_deref()
                    .is_some_and(|c| !c.trim().is_empty())
            );
            if let Some(order) = params.order.as_deref() {
                if !SORT_ORDERS.contains(&order) {
                    return Some(SheetOpsError::ParamRejected(format!(
                        "sort: order 只认 {SORT_ORDERS:?}, 得到 {order:?}"
                    )));
                }
            }
        }
        ACTION_AGGREGATE => {
            require!(
                "column",
                params
                    .column
                    .as_deref()
                    .is_some_and(|c| !c.trim().is_empty())
            );
            require!("function", params.function.is_some());
            let function = params.function.as_deref().unwrap_or_default();
            if !AGGREGATE_FUNCTIONS.contains(&function) {
                return Some(SheetOpsError::ParamRejected(format!(
                    "aggregate: function 只认 {AGGREGATE_FUNCTIONS:?}, 得到 {function:?}"
                )));
            }
        }
        ACTION_FORMULA => {
            require!(
                "expression",
                params
                    .expression
                    .as_deref()
                    .is_some_and(|e| !e.trim().is_empty())
            );
            require!(
                "new_column",
                params
                    .new_column
                    .as_deref()
                    .is_some_and(|c| !c.trim().is_empty())
            );
            if let Some(name) = params.new_column.as_deref() {
                if name.trim() != name {
                    return Some(SheetOpsError::ParamRejected(
                        "formula: new_column 首尾不得含空白".to_string(),
                    ));
                }
            }
        }
        _ => {
            require!(
                "output",
                params
                    .output
                    .as_deref()
                    .is_some_and(|o| !o.trim().is_empty())
            );
        }
    }

    // 写面受宪法级 `file_write` 唯一写总闸约束: 关 = 拒写不拒读。
    let write_face = is_sheet_ops_write_action(action, params.output.is_some());
    if write_face && !config.write_gate_open {
        return Some(SheetOpsError::WriteGateClosed(
            "表格写面受 file_write 唯一写总闸约束: 总闸未开, 写动作拒绝即帧 (读面不受影响)"
                .to_string(),
        ));
    }

    // 路径面: 输入限域 + 敏感面 + 字节上限预检; 输出限域 + 敏感面 + 不覆盖预检。
    let source = params.source.as_deref().unwrap_or_default();
    match resolve_contained(&config.workspace_root, source, true) {
        Ok(path) => {
            let bytes = std::fs::metadata(&path)
                .map(|meta| meta.len())
                .unwrap_or_default();
            if bytes > config.max_input_bytes {
                return Some(SheetOpsError::LimitExceeded(format!(
                    "input exceeds the {} byte limit; 输入字节超上限",
                    config.max_input_bytes
                )));
            }
        }
        Err(error) => return Some(error),
    }
    if let Some(output) = params.output.as_deref() {
        if output.trim().is_empty() {
            return Some(SheetOpsError::ParamRejected("output 不得为空".to_string()));
        }
        match resolve_contained(&config.workspace_root, output, false) {
            Ok(path) => {
                if path.exists() {
                    return Some(SheetOpsError::OutputExists(
                        "输出目标已存在, 只创建不覆盖 (请换名或先移走现有文件)".to_string(),
                    ));
                }
            }
            Err(error) => return Some(error),
        }
    }
    None
}

// ---------------------------------------------------------------------------
// 自述随行 (创作节 K3: 进程内计数, 只计数不回路径)
// ---------------------------------------------------------------------------

/// 表格操作面进程内计数 (创作节真值源: 与 `tool.sheet_ops` 工具面同一实例)。
///
/// **进程内计数** (本进程生命周期内的最近操作, 重启清零), 不落账不造账;
/// 成功按闭合词表逐动作计数, 拒绝即帧照计 ([`SheetOpsCounters::denied`])。
#[derive(Debug, Default)]
pub struct SheetOpsCounters {
    calls: std::sync::atomic::AtomicU64,
    rows_touched: std::sync::atomic::AtomicU64,
    denied: std::sync::atomic::AtomicU64,
    per_action: Mutex<std::collections::BTreeMap<String, u64>>,
    last_op_at_ms: Mutex<Option<i64>>,
    last_action: Mutex<Option<String>>,
}

impl SheetOpsCounters {
    /// 计数器 (进程内, 重启清零)。
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// 记一次调用 (成败都计)。
    fn record_call(&self) {
        self.calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// 记一次成功 (动作词表名 + 处理行数 + 时刻)。
    fn record_success(&self, action: &str, rows_touched: u64, at_ms: i64) {
        self.rows_touched
            .fetch_add(rows_touched, std::sync::atomic::Ordering::Relaxed);
        let mut per_action = self
            .per_action
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *per_action.entry(action.to_string()).or_insert(0) += 1;
        drop(per_action);
        *self
            .last_op_at_ms
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(at_ms);
        *self
            .last_action
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(action.to_string());
    }

    /// 记一次拒绝即帧 (错误即帧也计数, 不藏拒绝)。
    fn record_denied(&self) {
        self.denied
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// 投影自述创作节 (真值如实; 尚无记录 = 显式 null + 如实注)。
    pub fn status(&self) -> SheetOpsStatus {
        use std::sync::atomic::Ordering;
        let per_action = self
            .per_action
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let by_operation: Vec<SheetOpsOpCount> = SHEET_OPS_ACTIONS
            .iter()
            .map(|action| SheetOpsOpCount {
                operation: (*action).to_string(),
                count: per_action.get(*action).copied().unwrap_or(0),
            })
            .collect();
        let last_op_at_ms = *self
            .last_op_at_ms
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let last_action = self
            .last_action
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let note = if self.calls.load(Ordering::Relaxed) == 0 {
            format!("尚无操作记录; {SHEET_OPS_COUNT_SCOPE_NOTE}")
        } else {
            SHEET_OPS_COUNT_SCOPE_NOTE.to_string()
        };
        SheetOpsStatus {
            calls: self.calls.load(Ordering::Relaxed),
            rows_touched: self.rows_touched.load(Ordering::Relaxed),
            denied: self.denied.load(Ordering::Relaxed),
            by_operation,
            last_op_at_ms,
            last_action,
            note: Some(note),
            reason: None,
        }
    }
}

fn now_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// 执行面 (受控读写族的全部行为)
// ---------------------------------------------------------------------------

/// 表格操作执行面 (工具面与测试直调口同一真源)。
pub struct SheetOpsRunner {
    /// 生效配置 (工作区根 + 四上限 + 写总闸)。
    config: SheetOpsConfig,
    /// 进程内计数 (自述创作节真值源)。
    counters: Arc<SheetOpsCounters>,
}

impl SheetOpsRunner {
    /// 绑定配置 (独立计数器)。
    pub fn new(config: SheetOpsConfig) -> Self {
        Self {
            config,
            counters: SheetOpsCounters::new(),
        }
    }

    /// 绑定配置 + 共享计数器 (工具面与自述节同一实例)。
    pub fn with_counters(config: SheetOpsConfig, counters: Arc<SheetOpsCounters>) -> Self {
        Self { config, counters }
    }

    /// 生效配置。
    pub fn config(&self) -> &SheetOpsConfig {
        &self.config
    }

    /// 进程内计数 (自述创作节同源)。
    pub fn counters(&self) -> &Arc<SheetOpsCounters> {
        &self.counters
    }

    /// 执行一个动作 (参数矩阵先拒, 真因即帧; 计数如实)。
    pub fn run(&self, params: &SheetOpsParams) -> Result<Value, SheetOpsError> {
        self.counters.record_call();
        let outcome = self.run_inner(params);
        match &outcome {
            Ok((_, rows)) => {
                let action = params.action.as_deref().unwrap_or_default();
                self.counters.record_success(action, *rows, now_epoch_ms());
            }
            Err(_) => self.counters.record_denied(),
        }
        outcome.map(|(frame, _)| frame)
    }

    /// 执行内核 (无计数副作用; 回 (结果帧, 处理行数))。
    fn run_inner(&self, params: &SheetOpsParams) -> Result<(Value, u64), SheetOpsError> {
        if let Some(refusal) = sheet_ops_params_refusal(&self.config, params) {
            return Err(refusal);
        }
        let action = params.action.as_deref().unwrap_or_default();
        let source = params.source.as_deref().unwrap_or_default();
        let (sheet, meta) = self.load_sheet(source)?;
        let outcome = match action {
            ACTION_READ => self.read_rows(params, source, &sheet),
            ACTION_SUMMARY => Ok((self.summary(source, &sheet), sheet.rows.len() as u64)),
            ACTION_FILTER => self.filter(params, source, &sheet),
            ACTION_SORT => self.sort(params, source, &sheet),
            ACTION_AGGREGATE => self.aggregate(params, source, &sheet),
            ACTION_FORMULA => self.formula(params, source, &sheet),
            ACTION_EXPORT_CSV => self.export_csv(params, source, &sheet),
            other => Err(SheetOpsError::ParamRejected(format!(
                "未知动作 {other:?} (闭合词表外)"
            ))),
        };
        outcome.map(|(mut frame, rows)| {
            if let Value::Object(map) = &mut frame {
                annotate_sheet_meta(map, &meta);
            }
            (frame, rows)
        })
    }

    /// 解析一份受控表格输入 (限域 + 敏感面 + 字节上限 + 防炸弹表三道)。
    ///
    /// 格式按**后缀词面**判定 ([`SheetInputFormat::from_requested`]): `.xlsx`
    /// 走 xlsx 读端, 其余走 CSV 读端。
    fn load_sheet(&self, requested: &str) -> Result<(Sheet, SheetLoadMeta), SheetOpsError> {
        let (_path, bytes) = read_input(&self.config, requested)?;
        match SheetInputFormat::from_requested(requested) {
            SheetInputFormat::Csv => {
                let sheet = parse_csv_table(&bytes, &self.config)?;
                Ok((
                    sheet,
                    SheetLoadMeta {
                        format: SheetInputFormat::Csv,
                        formula_cells: 0,
                    },
                ))
            }
            SheetInputFormat::Xlsx => {
                let (sheet, formula_cells) = parse_xlsx_table(&bytes, &self.config)?;
                Ok((
                    sheet,
                    SheetLoadMeta {
                        format: SheetInputFormat::Xlsx,
                        formula_cells,
                    },
                ))
            }
        }
    }

    /// 目标列下标 (不存在拒绝即帧, 点名列)。
    fn require_column(
        &self,
        sheet: &Sheet,
        params: &SheetOpsParams,
        action: &str,
    ) -> Result<usize, SheetOpsError> {
        let name = params.column.as_deref().unwrap_or_default();
        sheet.column_index(name).ok_or_else(|| {
            SheetOpsError::ParamRejected(format!(
                "{action}: 列 {name:?} 不在表头 {:?}",
                sheet.columns
            ))
        })
    }

    /// `read`: 表头 + 行预览 (offset/limit, 超出显式 truncated)。
    fn read_rows(
        &self,
        params: &SheetOpsParams,
        source: &str,
        sheet: &Sheet,
    ) -> Result<(Value, u64), SheetOpsError> {
        let offset = params.offset.unwrap_or(0) as usize;
        let limit = params
            .limit
            .map(|value| value as usize)
            .unwrap_or(self.config.max_preview_rows);
        if offset > sheet.rows.len() {
            return Err(SheetOpsError::ParamRejected(format!(
                "read: offset {offset} 超过数据行数 {} (不回空帧冒充)",
                sheet.rows.len()
            )));
        }
        let end = offset.saturating_add(limit).min(sheet.rows.len());
        let rows: Vec<Value> = sheet.rows[offset..end]
            .iter()
            .map(|row| Value::Array(row.iter().map(|cell| Value::String(cell.clone())).collect()))
            .collect();
        let returned = end - offset;
        let frame = json!({
            "action": ACTION_READ,
            "status": "ok",
            "source": source,
            "columns": sheet.columns,
            "rows": rows,
            "row_count": sheet.rows.len(),
            "offset": offset,
            "returned": returned,
            "truncated": end < sheet.rows.len(),
        });
        Ok((frame, returned as u64))
    }

    /// `summary`: 行列数 / 列名 / 逐列类型猜测 (真值, 不推断)。
    fn summary(&self, source: &str, sheet: &Sheet) -> Value {
        let columns: Vec<Value> = sheet
            .columns
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let cells = sheet.column_cells(index);
                let empty_cells = cells.iter().filter(|cell| cell.is_empty()).count();
                json!({
                    "name": name,
                    "index": index,
                    "type": column_type(&cells).name(),
                    "empty_cells": empty_cells,
                })
            })
            .collect();
        json!({
            "action": ACTION_SUMMARY,
            "status": "ok",
            "source": source,
            "row_count": sheet.rows.len(),
            "column_count": sheet.columns.len(),
            "columns": columns,
        })
    }

    /// `filter`: 列条件筛行 (谓词语义钉死, 空值恒不匹配)。
    fn filter(
        &self,
        params: &SheetOpsParams,
        source: &str,
        sheet: &Sheet,
    ) -> Result<(Value, u64), SheetOpsError> {
        let index = self.require_column(sheet, params, ACTION_FILTER)?;
        let predicate = params.predicate.as_deref().unwrap_or_default();
        let value = params.value.as_deref().unwrap_or_default();
        let value_number = parse_number(value);

        // 数值谓词 (lt/lte/gt/gte) 只走数值轴: 目标列非空单元必须全数值。
        if matches!(predicate, "lt" | "lte" | "gt" | "gte") {
            let cells = sheet.column_cells(index);
            if !matches!(column_type(&cells), ColumnType::Int | ColumnType::Float) {
                return Err(SheetOpsError::TypeMixed(format!(
                    "filter: 数值谓词 {predicate} 要求列 {:?} 非空单元全数值 (类型混/非数值拒绝, 不静默跳行)",
                    sheet.columns[index]
                )));
            }
            if value_number.is_none() {
                return Err(SheetOpsError::ParamRejected(format!(
                    "filter: 数值谓词 {predicate} 的 value 必须是数值词面, 得到 {value:?}"
                )));
            }
        }

        let mut matched_rows: Vec<Vec<String>> = Vec::new();
        for row in &sheet.rows {
            let cell = row[index].as_str();
            if cell.is_empty() {
                continue; // 空值恒不匹配 (钉死口径)。
            }
            let cell_number = parse_number(cell);
            let matched = match predicate {
                "eq" => match (cell_number, value_number) {
                    (Some(left), Some(right)) => left == right,
                    _ => cell == value,
                },
                "ne" => match (cell_number, value_number) {
                    (Some(left), Some(right)) => left != right,
                    _ => cell != value,
                },
                "contains" => cell.contains(value),
                "lt" | "lte" | "gt" | "gte" => {
                    let left = cell_number.ok_or_else(|| {
                        SheetOpsError::TypeMixed(format!(
                            "filter: 数值谓词遇到非数值单元 {cell:?} (拒绝, 不静默跳行)"
                        ))
                    })?;
                    let right = value_number.expect("value validated numeric above");
                    match predicate {
                        "lt" => left < right,
                        "lte" => left <= right,
                        "gt" => left > right,
                        _ => left >= right,
                    }
                }
                _ => unreachable!("谓词词表闭合"),
            };
            if matched {
                matched_rows.push(row.clone());
            }
        }
        let matched = matched_rows.len();
        self.emit_rows(
            params,
            &sheet.columns,
            matched_rows,
            into_map(json!({
                "action": ACTION_FILTER,
                "status": "ok",
                "source": source,
                "column": sheet.columns[index],
                "predicate": predicate,
                "value": value,
                "row_count": sheet.rows.len(),
                "matched": matched,
            })),
            matched as u64,
        )
    }

    /// `sort`: 按列稳定排序 (数值/文本轴按同型裁定, 空值恒排最后)。
    fn sort(
        &self,
        params: &SheetOpsParams,
        source: &str,
        sheet: &Sheet,
    ) -> Result<(Value, u64), SheetOpsError> {
        let index = self.require_column(sheet, params, ACTION_SORT)?;
        let order = params.order.as_deref().unwrap_or("asc");
        let cells = sheet.column_cells(index);
        let numeric = match column_type(&cells) {
            ColumnType::Int | ColumnType::Float => true,
            ColumnType::Text => false,
            ColumnType::Empty => false,
            ColumnType::Mixed => {
                return Err(SheetOpsError::TypeMixed(format!(
                    "sort: 列 {:?} 数值与文本混型, 排序轴无法诚实裁定 (拒绝, 不静默分轴)",
                    sheet.columns[index]
                )));
            }
        };
        let desc = order == "desc";
        let mut sorted: Vec<Vec<String>> = sheet.rows.clone();
        sorted.sort_by(|left, right| {
            let (a, b) = (left[index].as_str(), right[index].as_str());
            match (a.is_empty(), b.is_empty()) {
                (true, true) => std::cmp::Ordering::Equal,
                (true, false) => std::cmp::Ordering::Greater, // 空值恒排最后
                (false, true) => std::cmp::Ordering::Less,
                (false, false) => {
                    let ordering = if numeric {
                        let (x, y) = (
                            parse_number(a).expect("numeric column"),
                            parse_number(b).expect("numeric column"),
                        );
                        x.partial_cmp(&y).expect("finite numbers compare")
                    } else {
                        a.cmp(b)
                    };
                    if desc {
                        ordering.reverse()
                    } else {
                        ordering
                    }
                }
            }
        });
        self.emit_rows(
            params,
            &sheet.columns,
            sorted,
            into_map(json!({
                "action": ACTION_SORT,
                "status": "ok",
                "source": source,
                "column": sheet.columns[index],
                "order": order,
                "row_count": sheet.rows.len(),
            })),
            sheet.rows.len() as u64,
        )
    }

    /// `aggregate`: 按列聚合 (sum/avg/count/min/max; 空聚合语义钉死)。
    fn aggregate(
        &self,
        params: &SheetOpsParams,
        source: &str,
        sheet: &Sheet,
    ) -> Result<(Value, u64), SheetOpsError> {
        let index = self.require_column(sheet, params, ACTION_AGGREGATE)?;
        let function = params.function.as_deref().unwrap_or_default();
        let cells = sheet.column_cells(index);

        if function == "count" {
            let count = cells.iter().filter(|cell| !cell.is_empty()).count();
            return Ok((
                json!({
                    "action": ACTION_AGGREGATE,
                    "status": "ok",
                    "source": source,
                    "column": sheet.columns[index],
                    "function": function,
                    "count": count,
                    "value": count,
                }),
                sheet.rows.len() as u64,
            ));
        }

        // 数值聚合: 非空单元必须全数值 (类型混拒绝), 空单元视为缺位。
        let mut values: Vec<f64> = Vec::new();
        for cell in &cells {
            if cell.is_empty() {
                continue;
            }
            match parse_number(cell) {
                Some(value) => values.push(value),
                None => {
                    return Err(SheetOpsError::TypeMixed(format!(
                        "aggregate: 列 {:?} 的单元 {cell:?} 非数值 (类型混拒绝, 不静默跳行)",
                        sheet.columns[index]
                    )));
                }
            }
        }
        let count = values.len();
        let value = match function {
            "sum" => values.iter().fold(0.0f64, |acc, value| acc + value),
            "avg" | "min" | "max" if count == 0 => {
                return Err(SheetOpsError::AggregateEmpty(format!(
                    "aggregate: {function} 的可聚合值为空集 (不冒充 0; sum/count 可空)"
                )));
            }
            "avg" => values.iter().sum::<f64>() / count as f64,
            "min" => values.iter().copied().fold(f64::INFINITY, f64::min),
            "max" => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            _ => unreachable!("函数词表闭合"),
        };
        Ok((
            json!({
                "action": ACTION_AGGREGATE,
                "status": "ok",
                "source": source,
                "column": sheet.columns[index],
                "function": function,
                "count": count,
                "value": value,
            }),
            sheet.rows.len() as u64,
        ))
    }

    /// `formula`: 四则运算计算列 (新列追加, 原文不改; f64 最短往返文本)。
    fn formula(
        &self,
        params: &SheetOpsParams,
        source: &str,
        sheet: &Sheet,
    ) -> Result<(Value, u64), SheetOpsError> {
        let expression = params.expression.as_deref().unwrap_or_default();
        let new_column = params.new_column.as_deref().unwrap_or_default();
        if sheet.column_index(new_column).is_some() {
            return Err(SheetOpsError::ParamRejected(format!(
                "formula: new_column {new_column:?} 与既有列重名 (不静默覆盖列)"
            )));
        }
        let expr = parse_expression(expression, &sheet.columns)?;
        let mut columns = sheet.columns.clone();
        columns.push(new_column.to_string());
        let mut rows: Vec<Vec<String>> = Vec::with_capacity(sheet.rows.len());
        for (offset, row) in sheet.rows.iter().enumerate() {
            let value = eval_expression(&expr, row, offset + 1)?;
            let mut cells = row.clone();
            // f64 最短往返表示 (浮点如实: 不静默取整, 不补位)。
            cells.push(format!("{value}"));
            rows.push(cells);
        }
        self.emit_rows(
            params,
            &columns,
            rows,
            into_map(json!({
                "action": ACTION_FORMULA,
                "status": "ok",
                "source": source,
                "expression": expression,
                "new_column": new_column,
                "row_count": sheet.rows.len(),
            })),
            sheet.rows.len() as u64,
        )
    }

    /// `export_csv`: 规范 CSV 再序列化导出 (方言归一; 只创建不覆盖)。
    fn export_csv(
        &self,
        params: &SheetOpsParams,
        source: &str,
        sheet: &Sheet,
    ) -> Result<(Value, u64), SheetOpsError> {
        let output = params.output.as_deref().unwrap_or_default();
        let bytes = serialize_csv(sheet).into_bytes();
        let path = resolve_contained(&self.config.workspace_root, output, false)?;
        write_output_create_only(&path, &bytes)?;
        let root = std::fs::canonicalize(&self.config.workspace_root).map_err(|_| {
            SheetOpsError::ReadFailed(
                "workspace root is not accessible; 工作区根不可访问".to_string(),
            )
        })?;
        Ok((
            json!({
                "action": ACTION_EXPORT_CSV,
                "status": "ok",
                "source": source,
                "output": relative_display(&path, &root),
                "row_count": sheet.rows.len(),
                "rows_written": sheet.rows.len(),
                "bytes_written": bytes.len() as u64,
            }),
            sheet.rows.len() as u64,
        ))
    }

    /// 变换产出的出口: 带 `output` = 创建类原子写 (只创建不覆盖), 不带 = 内存
    /// 行预览 (上限 [`SheetOpsConfig::max_preview_rows`], 超出显式 `truncated`)。
    fn emit_rows(
        &self,
        params: &SheetOpsParams,
        columns: &[String],
        rows: Vec<Vec<String>>,
        mut frame: serde_json::Map<String, Value>,
        rows_touched: u64,
    ) -> Result<(Value, u64), SheetOpsError> {
        if let Some(output) = params.output.as_deref() {
            let bytes = serialize_csv(&Sheet {
                columns: columns.to_vec(),
                rows: rows.clone(),
            })
            .into_bytes();
            let path = resolve_contained(&self.config.workspace_root, output, false)?;
            write_output_create_only(&path, &bytes)?;
            let root = std::fs::canonicalize(&self.config.workspace_root).map_err(|_| {
                SheetOpsError::ReadFailed(
                    "workspace root is not accessible; 工作区根不可访问".to_string(),
                )
            })?;
            frame.insert(
                "output".to_string(),
                Value::String(relative_display(&path, &root)),
            );
            frame.insert("rows_written".to_string(), json!(rows.len()));
            frame.insert("bytes_written".to_string(), json!(bytes.len() as u64));
            frame.insert("truncated".to_string(), Value::Bool(false));
            return Ok((Value::Object(frame), rows_touched));
        }
        let preview_cap = self.config.max_preview_rows;
        let truncated = rows.len() > preview_cap;
        let preview: Vec<Value> = rows
            .iter()
            .take(preview_cap)
            .map(|row| Value::Array(row.iter().map(|cell| Value::String(cell.clone())).collect()))
            .collect();
        frame.insert(
            "columns".to_string(),
            Value::Array(
                columns
                    .iter()
                    .map(|name| Value::String(name.clone()))
                    .collect(),
            ),
        );
        frame.insert("rows".to_string(), Value::Array(preview));
        frame.insert("returned".to_string(), json!(preview_cap.min(rows.len())));
        frame.insert("truncated".to_string(), Value::Bool(truncated));
        Ok((Value::Object(frame), rows_touched))
    }
}

/// `json!` 对象 → map (帧字段增补用)。
fn into_map(value: Value) -> serde_json::Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

/// 帧随行: 输入格式恒注 (`input_format`); xlsx 另注 `formula_cells` 计数,
/// 有公式格时带缓存值口径注记 (CSV 恒 0, 不注)。
fn annotate_sheet_meta(frame: &mut serde_json::Map<String, Value>, meta: &SheetLoadMeta) {
    frame.insert(
        "input_format".to_string(),
        Value::String(meta.format.name().to_string()),
    );
    if meta.format == SheetInputFormat::Xlsx {
        frame.insert("formula_cells".to_string(), json!(meta.formula_cells));
        if meta.formula_cells > 0 {
            frame.insert(
                "notes".to_string(),
                json!([format!(
                    "公式格 {} 个: 一律取缓存值未重算 (共享公式不展开, 如实标注口径)",
                    meta.formula_cells
                )]),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 工具面 (声明名 `sheet_ops`, 五段流水线)
// ---------------------------------------------------------------------------

/// 表格操作工具 (`tool.sheet_ops`, 声明名 `sheet_ops`)。
pub struct SheetOpsTool {
    id: CapabilityId,
    runner: SheetOpsRunner,
}

impl SheetOpsTool {
    /// 绑定受控配置 (独立计数器)。
    pub fn new(config: SheetOpsConfig) -> Self {
        Self::with_counters(config, SheetOpsCounters::new())
    }

    /// 绑定受控配置 + 共享计数器 (自述创作节同源)。
    pub fn with_counters(config: SheetOpsConfig, counters: Arc<SheetOpsCounters>) -> Self {
        Self {
            id: CapabilityId::new(SHEET_OPS_CAPABILITY_ID).expect("valid capability id"),
            runner: SheetOpsRunner::with_counters(config, counters),
        }
    }

    /// 输出归一合同: 结构化输出必须是对象且 `action` / `status` 恒在场。
    pub fn output_schema() -> OutputSchema {
        OutputSchema::new()
            .require("action", SchemaKind::String)
            .require("status", SchemaKind::String)
    }

    /// 执行面 (测试直调口, 与工具面同一真源)。
    pub fn runner(&self) -> &SheetOpsRunner {
        &self.runner
    }

    /// 进程内计数 (自述创作节同源)。
    pub fn counters(&self) -> &Arc<SheetOpsCounters> {
        self.runner.counters()
    }

    /// 拒绝即帧 (真因如实, 不静默降级)。
    fn refuse(&self, call: &ToolCall, error: SheetOpsError) -> ToolResult {
        ToolResult::permanent_error(&call.id, error.frame()).with_name(SHEET_OPS_TOOL_NAME)
    }
}

#[async_trait]
impl ToolCapability for SheetOpsTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut parameters = ToolParameters::new();
        parameters.extend(
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": SHEET_OPS_ACTIONS,
                        "description": "Closed vocabulary: read / summary / aggregate (reads); filter / sort / formula (reads, or writes when `output` is given); export_csv (write, each waits for a human approval)."
                    },
                    "source": {
                        "type": "string",
                        "description": "Input table path (workspace-relative or absolute inside the workspace root). A `.xlsx` suffix reads an xlsx workbook (first worksheet); any other suffix reads CSV."
                    },
                    "output": {
                        "type": "string",
                        "description": "Optional output CSV for filter/sort/formula; required for export_csv. Created atomically, never overwrites an existing file."
                    },
                    "column": {
                        "type": "string",
                        "description": "Target column name for filter/sort/aggregate (must match a header name exactly)."
                    },
                    "predicate": {
                        "type": "string",
                        "enum": FILTER_PREDICATES,
                        "description": "filter only: closed predicate vocabulary (eq/ne compare numerically when both sides are numeric, else textually; lt/lte/gt/gte are numeric-only; contains is a text substring test). Empty cells never match."
                    },
                    "value": {
                        "type": "string",
                        "description": "filter only: non-empty comparison value."
                    },
                    "order": {
                        "type": "string",
                        "enum": SORT_ORDERS,
                        "description": "sort only: asc (default) or desc; empty cells always sort last."
                    },
                    "function": {
                        "type": "string",
                        "enum": AGGREGATE_FUNCTIONS,
                        "description": "aggregate only: sum/avg/count/min/max. count counts non-empty cells; the others need numeric cells (empty cells skipped, mixed types refused)."
                    },
                    "expression": {
                        "type": "string",
                        "description": "formula only: arithmetic expression over column names and numeric literals (+ - * /, parentheses). Division by zero and non-finite results are refused."
                    },
                    "new_column": {
                        "type": "string",
                        "description": "formula only: name of the appended column (must not collide with an existing column)."
                    },
                    "offset": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "read only: 0-based row offset (default 0)."
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "read only: row budget (default and capped at the configured preview limit)."
                    }
                },
                "required": ["action", "source"],
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new(SHEET_OPS_TOOL_NAME)
            .with_description(SHEET_OPS_TOOL_DESCRIPTION.to_string())
            .with_parameters(parameters)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let arguments = match &call.arguments {
            Value::Null => json!({}),
            other => other.clone(),
        };
        let params = match serde_json::from_value::<SheetOpsParams>(arguments) {
            Ok(params) => params,
            Err(error) => {
                return self.refuse(
                    call,
                    SheetOpsError::ParamRejected(format!("invalid sheet_ops parameters: {error}")),
                );
            }
        };
        match self.runner.run(&params) {
            Ok(frame) => ToolResult::ok(&call.id, frame).with_name(SHEET_OPS_TOOL_NAME),
            Err(error) => self.refuse(call, error),
        }
    }
}

// ---------------------------------------------------------------------------
// 治理 (风险映射 + 审批档 + 单调边界 Guard)
// ---------------------------------------------------------------------------

/// 写面审批理由 (审批卡载荷同文; 零路径)。
pub fn sheet_ops_approval_reason(action: &str) -> String {
    format!(
        "表格操作写面 {action}: 落盘是文件创建面 (只创建不覆盖), \
         受 file_write 唯一写总闸约束, 每次写面都停人工审批 (无自动放行档)"
    )
}

/// 表格操作审批档钩子 (治理链, 与本地审批面板 / IM 审批卡同链)。
///
/// **写面每次人批** (带 `output` 的变换动作与 `export_csv`, 无自动放行档);
/// 纯读放行; 词表外动作 fail-closed 停审批 (不静默放行)。
#[derive(Debug, Clone, Copy, Default)]
pub struct SheetOpsApprovalHook;

impl SheetOpsApprovalHook {
    /// 空构造 (无状态钩子)。
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl GovernanceHook for SheetOpsApprovalHook {
    fn name(&self) -> &str {
        "sheet_ops_write_risk"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        let Action::CapabilityDispatch {
            capability,
            arguments,
        } = &request.action
        else {
            return Decision::Allow;
        };
        if capability.as_str() != SHEET_OPS_CAPABILITY_ID {
            return Decision::Allow;
        }
        let action = arguments
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let has_output = arguments
            .get("output")
            .is_some_and(|value| !value.is_null());
        if is_sheet_ops_write_action(action, has_output) {
            return Decision::require_approval(sheet_ops_approval_reason(action));
        }
        if SHEET_OPS_READ_ACTIONS.contains(&action) || SHEET_OPS_TRANSFORM_ACTIONS.contains(&action)
        {
            return Decision::Allow;
        }
        Decision::require_approval(format!(
            "表格操作动作 {action:?} 不在闭合词表: fail-closed 停人工审批"
        ))
    }
}

/// 写入风险映射 (pre-execute 瀑布): 未显式授权 = require-approval 档。
///
/// 与文档面风险映射同一语义: 显式授权 (grant 且未标审批) → 放行;
/// 授权被标人工审批 / 无授权 → `ask` (拒绝即帧 `pipeline.pre_ask`)。
pub struct SheetOpsRiskMappingHook {
    policy: Arc<Mutex<PermissionPolicy>>,
}

impl SheetOpsRiskMappingHook {
    /// 挂在共享授权策略上的风险映射钩子。
    pub fn new(policy: Arc<Mutex<PermissionPolicy>>) -> Self {
        Self { policy }
    }
}

impl PreExecuteHook for SheetOpsRiskMappingHook {
    fn name(&self) -> &str {
        "sheet_ops_risk_mapping"
    }

    fn pre_verdict(&self, request: &PreExecuteRequest<'_>) -> PreVerdict {
        let decision = self
            .policy
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .decision_for_capability(request.capability.as_str());
        match decision {
            Decision::Allow => PreVerdict::Allow,
            Decision::RequireApproval { reason } => PreVerdict::Ask { reason },
            Decision::Deny { .. } => PreVerdict::Ask {
                reason: format!(
                    "sheet_ops tool {} sits at the controlled read/write level: \
                     no explicit authorization for it",
                    request.capability.as_str()
                ),
            },
        }
    }
}

/// 单调边界 Guard (只拒不放): 参数矩阵 / 工作区限域 / 敏感面 / 上限 / 写总闸。
///
/// 自动放行档放行的调用也照穿本 Guard —— 自动放行从不绕过硬边界; 任何后续
/// 阶段都无法把这里的拒绝翻回放行 (单调性)。
pub struct SheetOpsBoundaryGuard {
    config: SheetOpsConfig,
}

impl SheetOpsBoundaryGuard {
    /// 绑定受控配置的边界 Guard。
    pub fn new(config: SheetOpsConfig) -> Self {
        Self { config }
    }
}

impl ToolGuard for SheetOpsBoundaryGuard {
    fn name(&self) -> &str {
        "sheet_ops_boundary"
    }

    fn deny(&self, request: &ToolGuardRequest<'_>) -> Option<String> {
        if request.capability.as_str() != SHEET_OPS_CAPABILITY_ID {
            return None;
        }
        // 解析失败不归边界 Guard (由工具即帧报解析错误), 只判硬边界。
        let Ok(params) = serde_json::from_value::<SheetOpsParams>(request.arguments.clone()) else {
            return None;
        };
        sheet_ops_params_refusal(&self.config, &params).map(|error| error.frame())
    }
}

/// 生产授权播种: 已注册的表格操作面即获显式授权 (授权 = 「这次调用有人拍板」)。
///
/// 风险档位 (写面每次人批) 不在工具策略里 —— 由 [`SheetOpsApprovalHook`] 承载,
/// 与本地审批面板 / IM 审批卡同链。
pub fn authorized_sheet_ops_policy() -> Arc<Mutex<PermissionPolicy>> {
    let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
    policy
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .grant(Permission::ExecuteTool(SHEET_OPS_CAPABILITY_ID.to_string()));
    policy
}

/// 组装表格操作五段流水线: 风险映射 → 单调边界 Guard → 超时 → 输出归一合同。
pub fn sheet_ops_pipeline(
    config: SheetOpsConfig,
) -> Arc<crate::exec_pipeline::ToolExecutionPipeline> {
    let guard: Arc<dyn ToolGuard> = Arc::new(SheetOpsBoundaryGuard::new(config));
    Arc::new(
        crate::exec_pipeline::ToolExecutionPipeline::new()
            .with_pre_hook(Arc::new(SheetOpsRiskMappingHook::new(
                authorized_sheet_ops_policy(),
            )))
            .with_guard(guard)
            .with_around(
                AroundPolicy::new().with_timeout(SHEET_OPS_TIMEOUT_MS, DEFAULT_MAX_TIMEOUT_MS),
            )
            .with_output_schema(SheetOpsTool::output_schema()),
    )
}

/// 组装生产工具面: 受控读写族 + 五段流水线 (模型侧一个能力)。
pub fn sheet_ops_capability(config: SheetOpsConfig) -> Arc<dyn ToolCapability> {
    sheet_ops_capability_shared(config, SheetOpsCounters::new())
}

/// 组装生产工具面 + 共享计数器 (自述创作节与工具面同一真值源)。
pub fn sheet_ops_capability_shared(
    config: SheetOpsConfig,
    counters: Arc<SheetOpsCounters>,
) -> Arc<dyn ToolCapability> {
    let tool: Arc<dyn ToolCapability> =
        Arc::new(SheetOpsTool::with_counters(config.clone(), counters));
    Arc::new(crate::exec_pipeline::PipelinedCapability::new(
        tool,
        sheet_ops_pipeline(config),
    ))
}

// ---------------------------------------------------------------------------
// 单元测试 (词表/词面/文法/开关/写总闸的行为钉)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// 开关 fail-closed: 只认显式 "1", 其余取值一律关。
    #[test]
    fn sheet_ops_switch_fails_closed_on_anything_but_one() {
        std::env::remove_var(SHEET_OPS_ENABLE_ENV);
        assert!(!sheet_ops_enabled_from_env(), "未设 = 关");
        for value in ["0", "true", "yes", "on", " 2 ", "11", "1x"] {
            std::env::set_var(SHEET_OPS_ENABLE_ENV, value);
            assert!(!sheet_ops_enabled_from_env(), "must refuse {value:?}");
        }
        std::env::set_var(SHEET_OPS_ENABLE_ENV, "1");
        assert!(sheet_ops_enabled_from_env());
        std::env::set_var(SHEET_OPS_ENABLE_ENV, " 1 ");
        assert!(sheet_ops_enabled_from_env(), "前后空白容忍");
        std::env::remove_var(SHEET_OPS_ENABLE_ENV);
    }

    /// 数值词面钉死: NaN/inf/溢出/带逗号/十六进制一律文本; 合法字面精确解析。
    #[test]
    fn numeric_literal_grammar_pins_nan_and_inf_as_text() {
        for text in [
            "NaN", "nan", "inf", "Infinity", "-inf", "1e999", "1,000", "0x10", "", " 1", "1 ",
            "1.2.3", "e5",
        ] {
            assert!(
                parse_number(text).is_none(),
                "{text:?} 必须不是数值词面 (NaN/inf/溢出永不入数值轴)"
            );
        }
        assert_eq!(parse_number("3"), Some(3.0));
        assert_eq!(parse_number("-0"), Some(-0.0));
        assert_eq!(parse_number("+2.5"), Some(2.5));
        assert_eq!(parse_number(".5"), Some(0.5));
        assert_eq!(parse_number("3."), Some(3.0));
        assert_eq!(parse_number("1e-3"), Some(0.001));
        assert_eq!(parse_number("0.1"), Some(0.1));
    }

    /// 类型猜测钉死: empty/int/float/text/mixed 各归其位。
    #[test]
    fn column_type_guessing_is_pinned() {
        assert_eq!(column_type(&["", ""]), ColumnType::Empty);
        assert_eq!(column_type(&["1", "-2", "+3"]), ColumnType::Int);
        assert_eq!(column_type(&["1", "2.5"]), ColumnType::Float);
        assert_eq!(column_type(&["NaN", "inf"]), ColumnType::Text);
        assert_eq!(column_type(&["1", "x"]), ColumnType::Mixed);
    }

    /// CSV 词面严格性: 引号转义/内嵌换行可解; 坏词面 (裸引号/参差行/重名列/
    /// 空列名/孤 CR/空表) 拒绝即帧。
    #[test]
    fn csv_parsing_is_strict_and_roundtrips_quoting() {
        let cfg = SheetOpsConfig::new(".");
        let good = b"a,b\n\"x,\"\"y\"\"\",\"line1\nline2\"\n";
        let sheet = parse_csv_table(good, &cfg).expect("合法词面可解");
        assert_eq!(sheet.columns, vec!["a", "b"]);
        assert_eq!(
            sheet.rows,
            vec![vec!["x,\"y\"".to_string(), "line1\nline2".to_string()]]
        );
        assert_eq!(
            parse_csv_table(serialize_csv(&sheet).as_bytes(), &cfg),
            Ok(sheet.clone())
        );

        assert!(
            parse_csv_table(b"a,b\nx\"y,z\n", &cfg).is_err(),
            "裸引号拒绝"
        );
        assert!(parse_csv_table(b"a,b\n1\n", &cfg).is_err(), "参差行拒绝");
        assert!(parse_csv_table(b"a,a\n1,2\n", &cfg).is_err(), "重名列拒绝");
        assert!(
            parse_csv_table(b"a,,\n1,2,3\n", &cfg).is_err(),
            "空列名拒绝"
        );
        assert!(parse_csv_table(b"a,b\r1,2\r", &cfg).is_err(), "孤 CR 拒绝");
        assert!(parse_csv_table(b"", &cfg).is_err(), "空表无表头拒绝");
        let header_only = parse_csv_table(b"a,b\n", &cfg).expect("仅表头 = 0 数据行");
        assert!(header_only.rows.is_empty());
    }

    /// 公式文法与求值: 优先级/括号/一元符号; 除零与溢出拒绝即帧 (NaN/inf 零产出)。
    #[test]
    fn formula_evaluation_pins_refusal_of_zero_division_and_overflow() {
        let columns = vec!["a".to_string(), "b".to_string()];
        let row = vec!["3".to_string(), "2".to_string()];
        let eval = |expression: &str| -> Result<f64, SheetOpsError> {
            let expr = parse_expression(expression, &columns)?;
            eval_expression(&expr, &row, 1)
        };
        assert_eq!(eval("a + b * 2"), Ok(7.0), "乘法优先");
        assert_eq!(eval("(a + b) * 2"), Ok(10.0), "括号压优先级");
        assert_eq!(eval("-a + b"), Ok(-1.0), "一元符号");
        assert_eq!(eval("a / b"), Ok(1.5));
        assert!(
            matches!(eval("a / (b - b)"), Err(SheetOpsError::FormulaError(_))),
            "除零拒绝"
        );
        assert!(matches!(eval("a - 4"), Ok(_)), "负值允许");
        assert!(
            matches!(eval("zz + 1"), Err(SheetOpsError::FormulaError(_))),
            "未知列拒绝"
        );
        assert!(
            matches!(eval("a +"), Err(SheetOpsError::FormulaError(_))),
            "缺操作数拒绝"
        );
        // 溢出到 inf: 连乘大数。
        let big = parse_expression("1e300 * 1e300", &columns).expect("可解析");
        assert!(
            matches!(
                eval_expression(&big, &row, 1),
                Err(SheetOpsError::FormulaError(_))
            ),
            "溢出非有限值拒绝 (不产出 inf)"
        );
    }

    /// 写总闸钉死: 关 = 拒写不拒读 (fail-closed)。
    #[test]
    fn the_write_gate_refuses_writes_but_never_reads() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("t.csv"), "a\n1\n").expect("fixture");

        let read = SheetOpsParams {
            action: Some(ACTION_READ.to_string()),
            source: Some("t.csv".to_string()),
            ..SheetOpsParams::default()
        };
        let write = SheetOpsParams {
            action: Some(ACTION_EXPORT_CSV.to_string()),
            source: Some("t.csv".to_string()),
            output: Some("out.csv".to_string()),
            ..SheetOpsParams::default()
        };

        let closed = SheetOpsConfig::new(dir.path()).with_write_gate(false);
        assert_eq!(
            sheet_ops_params_refusal(&closed, &read),
            None,
            "读面不受写总闸约束"
        );
        let refusal = sheet_ops_params_refusal(&closed, &write).expect("写面必须被拒");
        assert!(
            matches!(refusal, SheetOpsError::WriteGateClosed(_)),
            "写总闸关 = 拒写: {}",
            refusal.frame()
        );

        let open = SheetOpsConfig::new(dir.path()).with_write_gate(true);
        assert_eq!(
            sheet_ops_params_refusal(&open, &write),
            None,
            "写总闸开 = 放行到审批档"
        );
    }

    /// 写面判定钉死: export_csv 恒写, 变换动作带 output 才写。
    #[test]
    fn write_face_classification_is_pinned() {
        assert!(is_sheet_ops_write_action(ACTION_EXPORT_CSV, false));
        for action in SHEET_OPS_TRANSFORM_ACTIONS {
            assert!(
                !is_sheet_ops_write_action(action, false),
                "{action} 不带 output = 读"
            );
            assert!(
                is_sheet_ops_write_action(action, true),
                "{action} 带 output = 写"
            );
        }
        for action in SHEET_OPS_READ_ACTIONS {
            assert!(
                !is_sheet_ops_write_action(action, true),
                "{action} 恒读 (多余 output 已被参数矩阵拒)"
            );
        }
    }

    // ---- xlsx 读面行为钉 (K3 留缝收口: 三形态 / 公式口径 / 坏拒 / 上限) ----

    /// 手写 ZIP 容器 (stored 条目; 独立于生产写端手写, 防写读同模自证)。
    fn zip_stored(entries: &[(&str, &[u8])]) -> Vec<u8> {
        fn crc32(data: &[u8]) -> u32 {
            let mut crc = 0xFFFF_FFFFu32;
            for byte in data {
                crc ^= u32::from(*byte);
                for _ in 0..8 {
                    let mask = (crc & 1).wrapping_neg();
                    crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
                }
            }
            !crc
        }
        let mut out = Vec::new();
        let mut central = Vec::new();
        for (name, data) in entries {
            let offset = out.len() as u32;
            let crc = crc32(data);
            out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            out.extend_from_slice(&20u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(data);

            central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&crc.to_le_bytes());
            central.extend_from_slice(&(data.len() as u32).to_le_bytes());
            central.extend_from_slice(&(data.len() as u32).to_le_bytes());
            central.extend_from_slice(&(name.len() as u16).to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u32.to_le_bytes());
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(name.as_bytes());
        }
        let cd_offset = out.len() as u32;
        out.extend_from_slice(&central);
        let cd_size = central.len() as u32;
        out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    /// `xl/workbook.xml` (首个 `<sheet>` 声明 `r:id="rId1"`)。
    fn workbook_xml() -> String {
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" \
         xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\
         <sheets><sheet name=\"Sheet1\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>"
            .to_string()
    }

    /// `xl/_rels/workbook.xml.rels` (`rId1` → `worksheets/sheet1.xml`)。
    fn rels_xml() -> String {
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
         <Relationship Id=\"rId1\" \
         Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" \
         Target=\"worksheets/sheet1.xml\"/></Relationships>"
            .to_string()
    }

    /// 最小 xlsx 容器 (workbook + rels + sheet1, 可选 sharedStrings)。
    fn xlsx_book(sheet_xml: &str, shared: Option<&[&str]>) -> Vec<u8> {
        let worksheet = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
             <worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">\
             <sheetData>{sheet_xml}</sheetData></worksheet>"
        );
        let shared_xml = shared.map(|items| {
            let body: String = items
                .iter()
                .map(|item| format!("<si><t>{item}</t></si>"))
                .collect();
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
                 <sst xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">\
                 {body}</sst>"
            )
        });
        let workbook = workbook_xml();
        let rels = rels_xml();
        let mut entries: Vec<(&str, &[u8])> = vec![
            ("xl/workbook.xml", workbook.as_bytes()),
            ("xl/_rels/workbook.xml.rels", rels.as_bytes()),
            ("xl/worksheets/sheet1.xml", worksheet.as_bytes()),
        ];
        if let Some(xml) = &shared_xml {
            entries.push(("xl/sharedStrings.xml", xml.as_bytes()));
        }
        zip_stored(&entries)
    }

    /// 装配 xlsx 工作区 (fixture 落盘, 回 runner)。
    fn xlsx_runner(
        bytes: &[u8],
        limits: Option<(u64, usize, usize)>,
    ) -> (SheetOpsRunner, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("t.xlsx"), bytes).expect("落 fixture");
        let mut config = SheetOpsConfig::new(dir.path()).with_write_gate(true);
        if let Some((max_input_bytes, max_rows, max_cols)) = limits {
            config = config.with_limits(max_input_bytes, max_rows, max_cols, 100);
        }
        (SheetOpsRunner::new(config), dir)
    }

    /// 快捷调用 (只用 action + source 的读面)。
    fn run_on(runner: &SheetOpsRunner, action: &str, source: &str) -> Result<Value, SheetOpsError> {
        runner.run(&SheetOpsParams {
            action: Some(action.to_string()),
            source: Some(source.to_string()),
            ..SheetOpsParams::default()
        })
    }

    /// ㉙ xlsx 读面: 三形态单元格如实读入 (共享字符串 / 内联字符串 / 数值格原词面,
    /// 外加字符串结果格 `t="str"`), read/summary 同口径。
    #[test]
    fn xlsx_reads_shared_string_inline_string_and_numeric_cells() {
        let rows = "<row r=\"1\">\
            <c r=\"A1\" t=\"s\"><v>0</v></c>\
            <c r=\"B1\" t=\"inlineStr\"><is><t>数量</t></is></c>\
            <c r=\"C1\" t=\"s\"><v>1</v></c></row>\
            <row r=\"2\">\
            <c r=\"A2\" t=\"s\"><v>2</v></c>\
            <c r=\"B2\"><v>42</v></c>\
            <c r=\"C2\" t=\"inlineStr\"><is><t>ok &amp; fine</t></is></c></row>\
            <row r=\"3\">\
            <c r=\"A3\" t=\"str\"><v>raw</v></c>\
            <c r=\"B3\" t=\"n\"><v>3.5</v></c>\
            <c r=\"C3\"/></row>";
        let (runner, _dir) = xlsx_runner(
            &xlsx_book(rows, Some(&["名称", "备注", "shared-value"])),
            None,
        );
        let frame = run_on(&runner, ACTION_READ, "t.xlsx").expect("xlsx 可读");
        assert_eq!(frame["input_format"], "xlsx");
        assert_eq!(frame["formula_cells"], 0);
        assert_eq!(frame["columns"], json!(["名称", "数量", "备注"]));
        assert_eq!(frame["row_count"], 2);
        assert_eq!(
            frame["rows"],
            json!([["shared-value", "42", "ok & fine"], ["raw", "3.5", ""],]),
            "三形态 + 缺格补空, 数值格原词面如实"
        );
        let frame = run_on(&runner, ACTION_SUMMARY, "t.xlsx").expect("summary 可读");
        assert_eq!(frame["columns"][0]["type"], "text");
        assert_eq!(
            frame["columns"][1]["type"], "float",
            "42 与 3.5 同列 = float (逐列类型猜测口径不变)"
        );
        assert_eq!(frame["columns"][2]["type"], "text");
        assert_eq!(frame["columns"][2]["empty_cells"], 1);
    }

    /// ㉚ xlsx 公式格口径钉死: 一律取缓存值未重算 (共享公式不展开), 帧内如实标注。
    #[test]
    fn xlsx_formula_cells_take_cached_values_and_are_annotated() {
        let rows = "<row r=\"1\">\
            <c r=\"A1\" t=\"inlineStr\"><is><t>x</t></is></c>\
            <c r=\"B1\" t=\"inlineStr\"><is><t>sum</t></is></c></row>\
            <row r=\"2\">\
            <c r=\"A2\"><v>41</v></c>\
            <c r=\"B2\"><f>A2*2</f><v>82</v></c></row>\
            <row r=\"3\">\
            <c r=\"A3\"><v>42</v></c>\
            <c r=\"B3\"><f t=\"shared\" si=\"0\"/><v>84</v></c></row>";
        let (runner, _dir) = xlsx_runner(&xlsx_book(rows, None), None);
        let frame = run_on(&runner, ACTION_READ, "t.xlsx").expect("可读");
        assert_eq!(
            frame["rows"],
            json!([["41", "82"], ["42", "84"]]),
            "公式格取缓存值 (不重算); 共享公式格同样取缓存值"
        );
        assert_eq!(frame["formula_cells"], 2);
        let notes = frame["notes"].as_array().expect("notes 在场");
        let note = notes[0].as_str().expect("注记文本");
        assert!(note.contains("缓存值"), "{note}");
        assert!(note.contains("共享公式不展开"), "{note}");
    }

    /// ㉛ xlsx 公式格无缓存值 = 拒绝即帧 (不重算公式, 不编造值)。
    #[test]
    fn xlsx_formula_cells_without_cached_values_are_refused() {
        let rows = "<row r=\"1\"><c r=\"A1\" t=\"inlineStr\"><is><t>x</t></is></c></row>\
            <row r=\"2\"><c r=\"A2\"><f>B2*2</f></c></row>";
        let (runner, _dir) = xlsx_runner(&xlsx_book(rows, None), None);
        let error = run_on(&runner, ACTION_READ, "t.xlsx").expect_err("无缓存值公式格必拒");
        assert!(
            error.frame().contains("缓存值") && error.frame().contains("不编造值"),
            "{}",
            error.frame()
        );
    }

    /// ㉜ 坏 xlsx 拒 (容器层): 非 ZIP / 缺 workbook / 缺 rels / 关系对不上,
    /// 各自拒绝即帧 (真因如实, 不降级)。
    #[test]
    fn bad_xlsx_containers_are_refused_with_honest_frames() {
        let worksheet = "<worksheet><sheetData/></worksheet>";
        // 非 ZIP 容器。
        let (runner, _dir) = xlsx_runner(b"not a zip container", None);
        let error = run_on(&runner, ACTION_READ, "t.xlsx").expect_err("非容器必拒");
        assert!(
            error.frame().contains("sheet_ops.parse_error"),
            "{}",
            error.frame()
        );
        // 缺 workbook。
        let (runner, _dir) = xlsx_runner(
            &zip_stored(&[("xl/worksheets/sheet1.xml", worksheet.as_bytes())]),
            None,
        );
        let error = run_on(&runner, ACTION_READ, "t.xlsx").expect_err("缺 workbook 必拒");
        assert!(
            error.frame().contains("xl/workbook.xml"),
            "{}",
            error.frame()
        );
        // 缺 rels。
        let (runner, _dir) = xlsx_runner(
            &zip_stored(&[
                ("xl/workbook.xml", workbook_xml().as_bytes()),
                ("xl/worksheets/sheet1.xml", worksheet.as_bytes()),
            ]),
            None,
        );
        let error = run_on(&runner, ACTION_READ, "t.xlsx").expect_err("缺 rels 必拒");
        assert!(
            error.frame().contains("workbook.xml.rels"),
            "{}",
            error.frame()
        );
        // 关系 id 对不上 (工作表定位不了)。
        let wrong_rels = rels_xml().replace("rId1", "rId9");
        let (runner, _dir) = xlsx_runner(
            &zip_stored(&[
                ("xl/workbook.xml", workbook_xml().as_bytes()),
                ("xl/_rels/workbook.xml.rels", wrong_rels.as_bytes()),
                ("xl/worksheets/sheet1.xml", worksheet.as_bytes()),
            ]),
            None,
        );
        let error = run_on(&runner, ACTION_READ, "t.xlsx").expect_err("坏关系必拒");
        assert!(
            error.frame().contains("无法定位工作表"),
            "{}",
            error.frame()
        );
    }

    /// ㉝ 坏 xlsx 拒 (单元格层): 未知类型 / 共享索引越界 / 坏引用 / 同行重复列,
    /// 各自拒绝即帧 (显式拒, 不猜值)。
    #[test]
    fn bad_xlsx_cells_are_refused_with_honest_frames() {
        let cases: &[(&str, &str, Option<&[&str]>, &str)] = &[
            (
                "bool-type",
                "<row r=\"1\"><c r=\"A1\" t=\"b\"><v>1</v></c></row>",
                None,
                "不在子集",
            ),
            (
                "shared-index",
                "<row r=\"1\"><c r=\"A1\" t=\"s\"><v>9</v></c></row>",
                Some(&["only"]),
                "越界",
            ),
            (
                "bad-reference",
                "<row r=\"1\"><c r=\"1A\"><v>1</v></c></row>",
                None,
                "A1 形",
            ),
            (
                "duplicate-column",
                "<row r=\"1\"><c r=\"A1\"><v>1</v></c><c r=\"A1\"><v>2</v></c></row>",
                None,
                "重复列引用",
            ),
        ];
        for (tag, rows, shared, needle) in cases {
            let (runner, _dir) = xlsx_runner(&xlsx_book(rows, *shared), None);
            let error = run_on(&runner, ACTION_READ, "t.xlsx")
                .err()
                .unwrap_or_else(|| panic!("{tag} 必须拒绝即帧"));
            assert!(
                error.frame().contains(needle),
                "{tag} 帧必须点名真因 {needle:?}: {}",
                error.frame()
            );
        }
    }

    /// ㉞ xlsx 稀疏格网口径: 缺格补空 (格式语义), 越宽拒绝即帧, 表头空列名/重名拒。
    #[test]
    fn xlsx_sparse_grid_fills_gaps_and_refuses_wider_rows() {
        let sparse = "<row r=\"1\">\
            <c r=\"A1\" t=\"inlineStr\"><is><t>ha</t></is></c>\
            <c r=\"B1\" t=\"inlineStr\"><is><t>hb</t></is></c>\
            <c r=\"C1\" t=\"inlineStr\"><is><t>hc</t></is></c></row>\
            <row r=\"2\">\
            <c r=\"A2\" t=\"inlineStr\"><is><t>x</t></is></c>\
            <c r=\"C2\" t=\"inlineStr\"><is><t>y</t></is></c></row>\
            <row r=\"3\"><c r=\"B3\" t=\"inlineStr\"><is><t>z</t></is></c></row>";
        let (runner, _dir) = xlsx_runner(&xlsx_book(sparse, None), None);
        let frame = run_on(&runner, ACTION_READ, "t.xlsx").expect("稀疏格网可读");
        assert_eq!(
            frame["rows"],
            json!([["x", "", "y"], ["", "z", ""]]),
            "缺格补空"
        );

        let wider = "<row r=\"1\">\
            <c r=\"A1\" t=\"inlineStr\"><is><t>ha</t></is></c>\
            <c r=\"B1\" t=\"inlineStr\"><is><t>hb</t></is></c></row>\
            <row r=\"2\"><c r=\"C2\"><v>1</v></c></row>";
        let (runner, _dir) = xlsx_runner(&xlsx_book(wider, None), None);
        let error = run_on(&runner, ACTION_READ, "t.xlsx").expect_err("越宽行必拒");
        assert!(error.frame().contains("之外"), "{}", error.frame());

        let gap_header = "<row r=\"1\">\
            <c r=\"A1\" t=\"inlineStr\"><is><t>ha</t></is></c>\
            <c r=\"C1\" t=\"inlineStr\"><is><t>hc</t></is></c></row>";
        let (runner, _dir) = xlsx_runner(&xlsx_book(gap_header, None), None);
        let error = run_on(&runner, ACTION_READ, "t.xlsx").expect_err("表头缺格必拒");
        assert!(error.frame().contains("列名为空"), "{}", error.frame());
    }

    /// ㉟ 防炸弹表: xlsx 解压预算 / 数据行 / 列数三层上限各自拒绝即帧 (宁拒不截)。
    #[test]
    fn xlsx_caps_refuse_bomb_workbooks() {
        // 解压预算: 超高压缩比条目在小预算配置下拒绝 (防解压炸弹)。
        let bomb_row = format!(
            "<row r=\"1\"><c r=\"A1\" t=\"inlineStr\"><is><t>{}</t></is></c></row>",
            "a".repeat(100_000)
        );
        let worksheet = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
             <worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">\
             <sheetData>{bomb_row}</sheetData></worksheet>"
        );
        let entries = [
            crate::doc_ops::ZipEntry {
                name: "xl/workbook.xml".to_string(),
                data: workbook_xml().into_bytes(),
            },
            crate::doc_ops::ZipEntry {
                name: "xl/_rels/workbook.xml.rels".to_string(),
                data: rels_xml().into_bytes(),
            },
            crate::doc_ops::ZipEntry {
                name: "xl/worksheets/sheet1.xml".to_string(),
                data: worksheet.into_bytes(),
            },
        ];
        let bomb =
            crate::doc_ops::zip_write_entries(&entries, crate::doc_ops::ZipCompression::Deflate)
                .expect("生产写端可产出炸弹表 fixture");
        assert!(
            bomb.len() < 8_192,
            "fixture 压缩后须小 (才触发解压预算而非字节上限): {}",
            bomb.len()
        );
        let (runner, _dir) = xlsx_runner(&bomb, Some((8_192, 100, 10)));
        let error = run_on(&runner, ACTION_READ, "t.xlsx").expect_err("解压超预算必拒");
        assert!(
            error.frame().contains("解压预算") && error.frame().contains("limit_exceeded"),
            "{}",
            error.frame()
        );

        // 数据行上限。
        let rows = "<row r=\"1\"><c r=\"A1\" t=\"inlineStr\"><is><t>h</t></is></c></row>\
            <row r=\"2\"><c r=\"A2\"><v>1</v></c></row>\
            <row r=\"3\"><c r=\"A3\"><v>2</v></c></row>";
        let (runner, _dir) = xlsx_runner(&xlsx_book(rows, None), Some((8 * 1024 * 1024, 1, 10)));
        let error = run_on(&runner, ACTION_READ, "t.xlsx").expect_err("行超上限必拒");
        assert!(error.frame().contains("行上限"), "{}", error.frame());

        // 列上限。
        let cols = "<row r=\"1\">\
            <c r=\"A1\" t=\"inlineStr\"><is><t>a</t></is></c>\
            <c r=\"B1\" t=\"inlineStr\"><is><t>b</t></is></c>\
            <c r=\"C1\" t=\"inlineStr\"><is><t>c</t></is></c></row>";
        let (runner, _dir) = xlsx_runner(&xlsx_book(cols, None), Some((8 * 1024 * 1024, 10, 2)));
        let error = run_on(&runner, ACTION_READ, "t.xlsx").expect_err("列超上限必拒");
        assert!(error.frame().contains("列上限"), "{}", error.frame());
    }

    /// ㊱ xlsx 输入的写面口径: 产物是规范 CSV (xlsx 写出后置如实), 写面词表
    /// 零 xlsx 写动作。
    #[test]
    fn xlsx_input_exports_canonical_csv_and_the_write_vocab_is_csv_only() {
        assert_eq!(
            SHEET_OPS_WRITE_ACTIONS,
            ["export_csv"].as_slice(),
            "写面词表只有 export_csv (xlsx 写出后置, 不冒充)"
        );
        let rows = "<row r=\"1\">\
            <c r=\"A1\" t=\"s\"><v>0</v></c>\
            <c r=\"B1\" t=\"inlineStr\"><is><t>数量</t></is></c></row>\
            <row r=\"2\">\
            <c r=\"A2\" t=\"s\"><v>1</v></c>\
            <c r=\"B2\"><v>42</v></c></row>";
        let (runner, dir) = xlsx_runner(&xlsx_book(rows, Some(&["名称", "值"])), None);
        let frame = runner
            .run(&SheetOpsParams {
                action: Some(ACTION_EXPORT_CSV.to_string()),
                source: Some("t.xlsx".to_string()),
                output: Some("out.csv".to_string()),
                ..SheetOpsParams::default()
            })
            .expect("xlsx 读入可导出规范 CSV");
        assert_eq!(frame["input_format"], "xlsx");
        assert_eq!(frame["rows_written"], 1);
        let csv = std::fs::read_to_string(dir.path().join("out.csv")).expect("产物已落盘");
        assert_eq!(csv, "名称,数量\n值,42\n", "规范 CSV (LF 行尾, 末尾换行)");
    }
}
