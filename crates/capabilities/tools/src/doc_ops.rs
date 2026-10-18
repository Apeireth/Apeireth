//! 文档操作工具面 (`tool.doc_ops`, 声明名 `doc_ops`) —— 受控读写族 (W2)。
//!
//! 设计第一律「AI 先用」的文档面: 模型可直接读 PDF / DOCX 文档并做基础 PDF
//! 写操作, 同一内核未来再接人类界面出口。本面 = **工具面先行**: 模型可调用、
//! 过治理、入名册、随自述上报。
//!
//! # 操作词表 (闭合, 未知动作拒绝即帧)
//!
//! | 动作 | 面 | 一句话 |
//! |---|---|---|
//! | `pdf_info` | 读 | 页数 / 版本 / 逐页尺寸 (真值, 不推断) |
//! | `pdf_extract_text` | 读 | 逐页文本提取 (受字符上限, 超限显式 `truncated`) |
//! | `docx_extract_text` | 读 | DOCX 正文 → Markdown 转出 |
//! | `docx_outline` | 读 | DOCX 标题层级大纲 |
//! | `pdf_merge` | 写 | 多入一出: 依序合并页 |
//! | `pdf_split` | 写 | 一入多出: 逐页拆分为独立 PDF (批量管道) |
//! | `pdf_extract_pages` | 写 | 多页选页: 1 起页码列表 → 单一 PDF |
//! | `pdf_watermark_text` | 写 | 逐页叠对角水印文字 (WinAnsi 可编码文本) |
//! | `docx_from_markdown` | 写 | Markdown → DOCX 最小生成 (标题/段落/列表/粗斜体) |
//! | `docx_append` | 写 | 追加段落到副本 (原容器条目原样, 只创建不覆盖) |
//!
//! # 受控面 (四道硬边界 + 两道写语义)
//!
//! - **工作区限域**: 读写路径一律解析进工作区根, 词法层 (`guardrail`) +
//!   现实层 (canonicalize) 双检, 越界拒绝即帧 (带一次性升级引导)。
//! - **敏感面拒**: 凭据/密钥面路径 (`sensitive_path` 族词表) 读写都拒,
//!   拒绝信息即帧 (`pipeline.pre_deny` 语义)。
//! - **大小上限**: 输入超 [`DocOpsConfig::max_input_bytes`] 拒绝 (不静默截断)。
//! - **页数上限**: PDF 页数超 [`DocOpsConfig::max_pages`] 拒绝 (含合并总页数)。
//! - **原子写**: 输出走 `storage_atomic` 原子替换 (崩溃不留半截文件);
//!   **只创建不覆盖** —— 目标已存在拒绝即帧 (create-if-absent 门禁)。
//! - **批量管道**: 多入一出 / 一入多出 / 多页选页三种批量形态, 全部走
//!   五段流水线 (风险映射 → 单调边界 Guard → 超时 → 输出归一合同)。
//!
//! # 声明子集 (0 装诚实: 子集外拒绝即帧, 不静默降级)
//!
//! - **PDF 读**: 经典对象扫描 (含 xref 流文件的对象布局), 页树继承属性物化;
//!   `/Encrypt` 在场 = 加密文档**无解密面**, 显式拒绝; 内容流滤器只认
//!   `FlateDecode` / 无滤器, 其余显式拒绝并报滤器名。
//! - **PDF 文本提取**: 显示算子 `Tj` / `TJ` / `'` / `"`; 字体编码只认
//!   WinAnsi 系简单字体 (含 `/Differences`) 与 `/ToUnicode` CMap
//!   (bfchar/bfrange); 字体编码在子集外 = 该串按显式缺字 `?` 记并入备注,
//!   不编字形。
//! - **PDF 写**: 对象级搬运 (页对象 + 引用闭包原样复制, 压缩流不解不重压),
//!   新页树 + 经典 xref 表输出; 水印文字须 WinAnsi 可编码 (子集外拒绝即帧)。
//! - **DOCX 读**: ZIP 容器 (stored / deflate 两种条目, ZIP64 显式拒绝) +
//!   `word/document.xml` (OOXML 文字处理标记, UTF-8); 标题层级取段落样式
//!   `HeadingN`, 列表取 `w:numPr`。
//! - **DOCX 写** (结构面): `docx_from_markdown` 认 Markdown 结构子集
//!   (ATX 标题 `#`..`######` / 逐行一段 / 列表项 `-` `*` `+` 与 `N.` `N)` /
//!   行内 `**粗**` `*斜*` `_斜_`), 生成最小 OOXML 包 (内容类型 + 根关系 + 正文
//!   关系 + `word/document.xml` + `word/styles.xml` + `word/numbering.xml` 六件);
//!   **样式层最小如实**: 样式表只给 Normal / Heading1..6 (带大纲级别 +
//!   16/14/13/12/11/11pt 字号阶梯) / ListParagraph / 粗斜体字符样式 + 默认字体族
//!   与正文字号, **无主题、无字体美化**; **编号层最小如实**: `numbering.xml` 只给
//!   一级项目符号 (`w:numFmt bullet` + `w:lvlText "•"` + 左缩进 720 / 悬挂 360
//!   twips), 正文列表条目以 `w:numPr` → `w:numId="1"` 指向它, 故项目符号由包内
//!   定义可见; **有序列表当前与无序列表同样渲染为项目符号** —— 解析层尚无
//!   ordered/bullet 分型 (`N.` `N)` 与 `-` `*` `+` 落同一 `numId=1`, 读回归一
//!   `- ` 的既有口径不变), 不冒充编号支持。`docx_append` 在副本的
//!   `word/document.xml` body 尾部追加纯文本段落 (逐行一段), 其余容器条目原样
//!   搬动 (只动 `word/document.xml`); **只创建不覆盖**, 与写族同一原子写门禁。
//!   嵌套列表 / 引用块 / 表格 / 图片 / 表式排版不在子集, 标记不成对按字面保留
//!   (不猜语义)。
//!
//! # 治理 (受控读写族)
//!
//! 注册即闸 (`APEIRETH_ENABLE_DOC_OPS`, 默认关 fail-closed, 见
//! [`doc_ops_enabled_from_env`])。读面 grant 即用 (零审批); **写面双重约束**:
//! ① 写动作受宪法级 `file_write` 唯一写总闸约束 ([`DocOpsConfig::with_write_gate`],
//! 组装根注入同一开关值, 关 = 写动作拒绝即帧、读面不受影响); ② 每次写人批
//! ([`DocOpsApprovalHook`], 无自动放行档) —— 写面产物是新文件 (创建类原子写,
//! 永不覆盖), 但批量落盘仍停人工审批。边界 Guard (工作区限域 / 敏感面 /
//! 上限 / 写总闸) 是单调只拒不放, 自动放行也不绕过硬边界。
//!
//! # 依赖报备 (取材口径: 复用 workspace 既有闭包, 0 新外部包)
//!
//! - `flate2` (Cargo.lock 既有传递依赖, 同版本复用): PDF 内容流 zlib 解压 +
//!   ZIP deflate 条目解压/压缩;
//! - `quick-xml` (Cargo.lock 既有传递依赖, 同版本复用): DOCX XML 解析;
//! - ZIP 容器读写端为本文件内纯 Rust 最小实现 (读: stored + deflate; 写:
//!   stored / deflate 两档, 同一子集对称, 无第三方 zip 包)。
//!   两者均为锁定文件既有 `[[package]]` 的直接化引用, 版本零漂移。

use std::collections::{BTreeMap, HashMap};
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
use crate::self_status::{DocOpsOpCount, DocOpsStatus};
use crate::sensitive_path::{credential_surface_refusal, is_sensitive_path};

// ---------------------------------------------------------------------------
// 身份与词表
// ---------------------------------------------------------------------------

/// 稳定能力标识 (注册表 / 治理授权 / 风险映射共用)。
pub const DOC_OPS_CAPABILITY_ID: &str = "tool.doc_ops";

/// 模型侧声明名 (短名, 每次请求都进上下文)。
pub const DOC_OPS_TOOL_NAME: &str = "doc_ops";

/// 生产执行时限 (毫秒): 解析/搬运是纯内存工作, 到期即归 `timeout.*`。
pub const DOC_OPS_TIMEOUT_MS: u64 = 30_000;

/// 文档操作面总开关 (`APEIRETH_ENABLE_DOC_OPS`): 注册即闸, 默认关 fail-closed。
///
/// 只认显式 `"1"` (前后空白容忍); 未设 / `"0"` / 其它任何取值 = 关。
pub const DOC_OPS_ENABLE_ENV: &str = "APEIRETH_ENABLE_DOC_OPS";

/// 读面动作: `pdf_info`。
pub const ACTION_PDF_INFO: &str = "pdf_info";
/// 读面动作: `pdf_extract_text`。
pub const ACTION_PDF_EXTRACT_TEXT: &str = "pdf_extract_text";
/// 读面动作: `docx_extract_text` (Markdown 转出)。
pub const ACTION_DOCX_EXTRACT_TEXT: &str = "docx_extract_text";
/// 读面动作: `docx_outline`。
pub const ACTION_DOCX_OUTLINE: &str = "docx_outline";
/// 写面动作: `pdf_merge` (多入一出)。
pub const ACTION_PDF_MERGE: &str = "pdf_merge";
/// 写面动作: `pdf_split` (一入多出, 批量管道)。
pub const ACTION_PDF_SPLIT: &str = "pdf_split";
/// 写面动作: `pdf_extract_pages` (多页选页)。
pub const ACTION_PDF_EXTRACT_PAGES: &str = "pdf_extract_pages";
/// 写面动作: `pdf_watermark_text`。
pub const ACTION_PDF_WATERMARK_TEXT: &str = "pdf_watermark_text";
/// 写面动作: `docx_from_markdown` (Markdown → DOCX 最小生成)。
pub const ACTION_DOCX_FROM_MARKDOWN: &str = "docx_from_markdown";
/// 写面动作: `docx_append` (副本追加段落)。
pub const ACTION_DOCX_APPEND: &str = "docx_append";

/// 闭合动作词表 (10 个)。
pub const DOC_OPS_ACTIONS: &[&str] = &[
    ACTION_PDF_INFO,
    ACTION_PDF_EXTRACT_TEXT,
    ACTION_DOCX_EXTRACT_TEXT,
    ACTION_DOCX_OUTLINE,
    ACTION_PDF_MERGE,
    ACTION_PDF_SPLIT,
    ACTION_PDF_EXTRACT_PAGES,
    ACTION_PDF_WATERMARK_TEXT,
    ACTION_DOCX_FROM_MARKDOWN,
    ACTION_DOCX_APPEND,
];

/// 读面词表 (grant 即用零审批)。
pub const DOC_OPS_READ_ACTIONS: &[&str] = &[
    ACTION_PDF_INFO,
    ACTION_PDF_EXTRACT_TEXT,
    ACTION_DOCX_EXTRACT_TEXT,
    ACTION_DOCX_OUTLINE,
];

/// 写面词表 (每次人批, 无自动放行档)。
pub const DOC_OPS_WRITE_ACTIONS: &[&str] = &[
    ACTION_PDF_MERGE,
    ACTION_PDF_SPLIT,
    ACTION_PDF_EXTRACT_PAGES,
    ACTION_PDF_WATERMARK_TEXT,
    ACTION_DOCX_FROM_MARKDOWN,
    ACTION_DOCX_APPEND,
];

/// 工具描述全文 (注册声明与文档引用同一份文本)。
pub const DOC_OPS_TOOL_DESCRIPTION: &str = "Read and transform documents inside \
  the workspace root (controlled read/write family). PDF face: pdf_info, \
  pdf_extract_text (reads); pdf_merge (join several PDFs in order), pdf_split \
  (one input to one file per page), pdf_extract_pages (pick 1-based pages into \
  one PDF), pdf_watermark_text (diagonal text watermark on every page) - the \
  write face creates new files atomically and never overwrites an existing one \
  (every write waits for a human approval). DOCX face: docx_extract_text \
  (markdown out) and docx_outline (heading levels) as reads; docx_from_markdown \
  (minimal markdown-to-DOCX generation: headings, paragraphs, lists, bold/italic \
  structure) and docx_append (append text paragraphs to a copy) as writes - the \
  generated package carries a minimal style layer (Normal, Heading1..6 with \
  outline levels and a stepwise size ladder, ListParagraph, bold/italic character \
  styles, default font family and body size) and a minimal numbering layer \
  (word/numbering.xml: one bullet level whose numPr numId 1 the list paragraphs \
  reference, so lists show a visible bullet); there is no theme and no font \
  beautification, and ordered lists currently render as the same bullet (the \
  parser does not distinguish ordered from bullet items, and the read side \
  normalizes every list item to a `- ` item), so no numbered-list support is \
  claimed; the markdown subset refuses nothing silently (unmatched markers stay \
  literal). Paths must stay inside the \
  workspace root; credential surfaces are refused; inputs over the size cap and \
  PDFs over the page cap are refused, never silently truncated. Encrypted PDFs \
  and streams outside the declared subset are refused with an explicit reason.";

// ---------------------------------------------------------------------------
// 错误 (拒绝即帧: 真因如实, 不静默降级)
// ---------------------------------------------------------------------------

/// 文档操作失败 (面向模型的稳定帧文本由 [`DocOpsError::message`] 给出)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocOpsError {
    /// 参数校验不过 (词表外动作 / 缺字段 / 越界页码 / 非法取值)。
    InvalidParam(String),
    /// 请求路径在工作区之外 (带一次性升级引导)。
    OutsideWorkspace(String),
    /// 凭据/密钥敏感面路径 (拒绝信息即帧, 安全契约)。
    SensitiveSurface,
    /// 输入超过大小上限。
    InputTooLarge {
        /// 请求路径 (显示用)。
        path: String,
        /// 实际字节数。
        bytes: u64,
        /// 生效上限。
        cap: u64,
    },
    /// PDF 页数超过页数上限 (含合并总页数)。
    TooManyPages {
        /// 实际页数。
        pages: u64,
        /// 生效上限。
        cap: u64,
    },
    /// 容器解压后总字节超过解压预算 (防解压炸弹, 宁拒不截)。
    UncompressedTooLarge {
        /// 实际解压字节。
        bytes: u64,
        /// 生效预算。
        cap: u64,
    },
    /// 解析失败 (坏容器/坏结构, 真因在描述里)。
    Parse(String),
    /// 子集外 (加密文档 / 未支持滤器 / 未支持编码 / ZIP64): 显式拒绝不降级。
    Unsupported(String),
    /// 文档写面受 `file_write` 唯一写总闸约束: 总闸未开, 写动作拒绝即帧
    /// (读面不受影响)。
    WriteGateClosed(String),
    /// 输出目标已存在 (只创建不覆盖, 原子写门禁)。
    OutputExists(String),
    /// 落盘 IO 失败 (来自 `storage_atomic` 原子写)。
    Io(String),
}

impl DocOpsError {
    /// 面向模型的拒绝帧 (稳定前缀 + 真因)。
    pub fn message(&self) -> String {
        match self {
            Self::InvalidParam(detail) => format!("拒绝文档操作: {detail}"),
            Self::OutsideWorkspace(detail) => format!("拒绝文档操作: {detail}"),
            Self::SensitiveSurface => credential_surface_refusal(),
            Self::InputTooLarge { path, bytes, cap } => format!(
                "拒绝文档操作: 输入 {path} 为 {bytes} 字节, 超过大小上限 {cap} 字节 \
                 (宁拒不截: 请拆分文档或提高上限)"
            ),
            Self::TooManyPages { pages, cap } => format!(
                "拒绝文档操作: PDF 页数 {pages} 超过页数上限 {cap} (宁拒不截: \
                 请分批处理或提高上限)"
            ),
            Self::UncompressedTooLarge { bytes, cap } => format!(
                "拒绝文档操作: 容器解压后 {bytes} 字节超过解压预算 {cap} 字节 \
                 (防解压炸弹, 宁拒不截)"
            ),
            Self::Parse(detail) => format!("拒绝文档操作: 解析失败 —— {detail}"),
            Self::Unsupported(detail) => {
                format!("拒绝文档操作: 子集外显式拒绝 (不静默降级) —— {detail}")
            }
            Self::WriteGateClosed(detail) => {
                format!("拒绝文档操作: 文档写面受 file_write 唯一写总闸约束 —— {detail}")
            }
            Self::OutputExists(path) => format!(
                "拒绝文档操作: 输出目标已存在, 只创建不覆盖 —— {path}; \
                 请换名或先移走现有文件"
            ),
            Self::Io(detail) => format!("文档操作原子写失败: {detail}"),
        }
    }
}

// ---------------------------------------------------------------------------
// 配置与开关 (注册即闸, 默认关 fail-closed)
// ---------------------------------------------------------------------------

/// 默认输入大小上限 (字节): 32 MiB (常量带 `DOC_OPS_` 前缀: 根导出命名空间
/// 与其它创作面的同名上限常量分立, 命名即承诺)。
pub const DOC_OPS_MAX_INPUT_BYTES: u64 = 32 * 1024 * 1024;

/// 默认 PDF 页数上限 (页; 合并时按总页数计)。
pub const DOC_OPS_MAX_PAGES: u64 = 1_000;

/// 默认提取文本返回上限 (字符): 超限显式 `truncated`, 不悄悄丢。
pub const DOC_OPS_MAX_TEXT_CHARS: usize = 200_000;

/// Markdown / 追加文源文字符上限 (字符): 超限拒绝即帧 (宁拒不截, 不截断生成)。
pub const DOC_OPS_MAX_SOURCE_CHARS: usize = 200_000;

/// Markdown / 追加文段落上限 (段): 超限拒绝即帧 (防超长段落流拖垮单次调用)。
pub const DOC_OPS_MAX_SOURCE_PARAGRAPHS: usize = 10_000;

/// ZIP 解压预算系数: 解压预算 = 输入字节上限 × 本系数 (防解压炸弹, 宁拒不截)。
pub(crate) const ZIP_DECOMPRESSION_BUDGET_FACTOR: u64 = 8;

/// 文档操作面配置 (工作区根 + 三道上限 + 写总闸注入)。
///
/// 组装根按 [`DocOpsConfig::from_env`] 注入 (注册即闸): 缺配置 =
/// `tool.doc_ops` 不注册, 自述名册 `doc_ops` 行照实 false。**写总闸** =
/// 宪法级 `file_write` 唯一写总闸的同一开关值 (组装根注入, 见
/// [`DocOpsConfig::with_write_gate`]): 关 = 写动作拒绝即帧, 读面不受影响。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocOpsConfig {
    /// 工作区根 (读写限域)。
    pub workspace_root: PathBuf,
    /// 输入大小上限 (字节)。
    pub max_input_bytes: u64,
    /// PDF 页数上限 (页)。
    pub max_pages: u64,
    /// 提取文本返回上限 (字符)。
    pub max_text_chars: usize,
    /// 文档写面写总闸 (默认关 fail-closed; 组装根注入 `file_write` 同一开关值)。
    pub write_gate_open: bool,
}

impl DocOpsConfig {
    /// 工作区根 + 默认三上限; 写总闸默认关 (fail-closed)。
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            max_input_bytes: DOC_OPS_MAX_INPUT_BYTES,
            max_pages: DOC_OPS_MAX_PAGES,
            max_text_chars: DOC_OPS_MAX_TEXT_CHARS,
            write_gate_open: false,
        }
    }

    /// 覆盖三上限 (测试/宿主微调; 命名即承诺: 值即生效权力)。
    pub fn with_limits(
        mut self,
        max_input_bytes: u64,
        max_pages: u64,
        max_text_chars: usize,
    ) -> Self {
        self.max_input_bytes = max_input_bytes;
        self.max_pages = max_pages;
        self.max_text_chars = max_text_chars;
        self
    }

    /// 注入写总闸 (`file_write` 唯一写总闸的同一开关值; 关 = 写动作拒绝即帧)。
    pub fn with_write_gate(mut self, open: bool) -> Self {
        self.write_gate_open = open;
        self
    }

    /// 开关实况: `APEIRETH_ENABLE_DOC_OPS` 只认显式 `"1"` (默认关 fail-closed)。
    pub fn enabled_from_env() -> bool {
        doc_ops_enabled_from_env()
    }

    /// 从 env 构造: 开关开才 `Some` (默认关 → [`None`], fail-closed)。
    pub fn from_env(workspace_root: impl Into<PathBuf>) -> Option<Self> {
        doc_ops_enabled_from_env().then(|| Self::new(workspace_root))
    }
}

/// 文档操作面开关 (`APEIRETH_ENABLE_DOC_OPS`) 实况: 只认显式 `"1"`。
///
/// 未设 / `"0"` / `"true"` / 其它任何取值 = 关 (fail-closed, 命名即承诺:
/// 没有这把钥匙就是没有文档操作面)。
pub fn doc_ops_enabled_from_env() -> bool {
    std::env::var(DOC_OPS_ENABLE_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

// ---------------------------------------------------------------------------
// 受控面: 工作区限域 + 敏感面拒 + 原子写
// ---------------------------------------------------------------------------

/// 把请求路径解析进工作区根并证明包含 (词法层 + 现实层双检)。
///
/// `must_exist` 用于输入面 (canonicalize 整条路径); 输出面目标尚不存在,
/// 只 canonicalize 最近存在的祖先再拼回, 保持同一把尺。
fn resolve_contained(
    root: &Path,
    requested: &str,
    must_exist: bool,
) -> Result<PathBuf, DocOpsError> {
    if requested.trim().is_empty() {
        return Err(DocOpsError::InvalidParam("路径不能为空".to_string()));
    }
    let canonical_root = std::fs::canonicalize(root).map_err(|error| {
        DocOpsError::InvalidParam(format!("工作区根 {} 不可访问: {error}", root.display()))
    })?;
    let candidate = if Path::new(requested).is_absolute() {
        PathBuf::from(requested)
    } else {
        canonical_root.join(requested)
    };

    // 词法层守门 (相对穿越 / 绝对敏感系统路径 / root 包含) 在 canonicalize 之前,
    // 与现实层 (symlink 逃逸) 构成双层。
    crate::guardrail::ToolGuardrail::verify_path_access(&canonical_root, &candidate).map_err(
        |error| {
            let message = match error {
                crate::guardrail::PreCallGuardError::OutsideWorkspace(detail) => format!(
                    "{detail}; {}",
                    crate::escalation::UpgradeHint::out_of_workspace_access()
                ),
                ref other => other.to_string(),
            };
            DocOpsError::OutsideWorkspace(message)
        },
    )?;

    let resolved = if must_exist {
        std::fs::canonicalize(&candidate).map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => {
                DocOpsError::InvalidParam(format!("输入 {} 不存在", candidate.display()))
            }
            other => DocOpsError::Io(format!("{}: {other}", candidate.display())),
        })?
    } else {
        resolve_output_spelling(&candidate)?
    };

    if !resolved.starts_with(&canonical_root) {
        return Err(DocOpsError::OutsideWorkspace(format!(
            "{} 解析到工作区根之外; {}",
            candidate.display(),
            crate::escalation::UpgradeHint::out_of_workspace_access()
        )));
    }
    if is_sensitive_path(&canonical_root, &candidate)
        || is_sensitive_path(&canonical_root, &resolved)
    {
        return Err(DocOpsError::SensitiveSurface);
    }
    Ok(resolved)
}

/// 输出面路径拼写: 最近存在的祖先 canonicalize, 尾段按原拼写接回 (目标可缺席)。
fn resolve_output_spelling(candidate: &Path) -> Result<PathBuf, DocOpsError> {
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
                    return Err(DocOpsError::OutsideWorkspace(format!(
                        "输出 {} 无可解析的祖先目录",
                        candidate.display()
                    )));
                };
                let Some(name) = ancestor.file_name() else {
                    return Err(DocOpsError::OutsideWorkspace(format!(
                        "输出 {} 无可解析的祖先目录",
                        candidate.display()
                    )));
                };
                tail.push(name.to_os_string());
                ancestor = parent.to_path_buf();
            }
            Err(error) => return Err(DocOpsError::Io(format!("{}: {error}", candidate.display()))),
        }
    }
}

/// 原子创建一个输出文件 (只创建不覆盖: 目标已存在拒绝即帧)。
///
/// 走 [`gated_write_atomic`] 的 create-if-absent 门禁 + `storage_atomic`
/// 原子替换 (崩溃不留半截文件); 写后同会话连写凭回填观测无需重读。
fn write_output_create_only(path: &Path, bytes: &[u8]) -> Result<(), DocOpsError> {
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
            DocOpsError::OutputExists(format!("{}: {}", path.display(), denial.message()))
        }
        crate::observed_gate::GatedWriteError::Io(detail) => DocOpsError::Io(detail),
    })
}

/// 共享 ZIP 解压预算 (输入字节上限 × [`ZIP_DECOMPRESSION_BUDGET_FACTOR`])。
fn zip_decompression_budget(config: &DocOpsConfig) -> u64 {
    config
        .max_input_bytes
        .saturating_mul(ZIP_DECOMPRESSION_BUDGET_FACTOR)
}

/// 读入一个受控输入 (限域 + 敏感面 + 大小上限)。
fn read_input(config: &DocOpsConfig, requested: &str) -> Result<(PathBuf, Vec<u8>), DocOpsError> {
    let path = resolve_contained(&config.workspace_root, requested, true)?;
    let bytes = std::fs::read(&path)
        .map_err(|error| DocOpsError::Io(format!("{}: {error}", path.display())))?;
    if bytes.len() as u64 > config.max_input_bytes {
        return Err(DocOpsError::InputTooLarge {
            path: requested.to_string(),
            bytes: bytes.len() as u64,
            cap: config.max_input_bytes,
        });
    }
    Ok((path, bytes))
}

// ---------------------------------------------------------------------------
// ZIP 最小读写端 (纯 Rust: 读 stored + deflate / 写 stored + deflate;
// ZIP64 显式拒绝; 读写同一子集对称)
// ---------------------------------------------------------------------------

/// ZIP 容器里的一个条目 (已解出数据)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZipEntry {
    /// 条目名 (含路径, `/` 分隔)。
    pub(crate) name: String,
    /// 条目数据 (已按压缩方法展开)。
    pub(crate) data: Vec<u8>,
}

/// 读端中央目录条目 (定位 + 压缩信息)。
struct ZipDirEntry {
    name: String,
    method: u16,
    compressed_size: u64,
    uncompressed_size: u64,
    local_header_offset: u64,
}

/// 从 ZIP 字节流读出全部条目 (只认 stored / deflate, ZIP64 显式拒绝)。
///
/// `decompressed_budget` 是**解压预算** (全部条目展开后总字节上限, 防解压炸弹):
/// 超预算拒绝即帧 [`DocOpsError::UncompressedTooLarge`] (宁拒不截, 不落盘不截断)。
pub(crate) fn zip_read_entries(
    bytes: &[u8],
    decompressed_budget: u64,
) -> Result<Vec<ZipEntry>, DocOpsError> {
    let eocd_at = find_eocd(bytes).ok_or_else(|| {
        DocOpsError::Parse("ZIP 容器缺中央目录结尾记录 (不是有效 docx 容器)".to_string())
    })?;
    let entry_count = u64::from(read_u16(bytes, eocd_at + 10));
    let cd_size = u64::from(read_u32(bytes, eocd_at + 12));
    let cd_offset = u64::from(read_u32(bytes, eocd_at + 16));
    if entry_count == 0xFFFF || cd_size == 0xFFFF_FFFF || cd_offset == 0xFFFF_FFFF {
        return Err(DocOpsError::Unsupported(
            "ZIP64 容器不在本批子集 (显式拒绝, 不降级)".to_string(),
        ));
    }

    let mut entries = Vec::new();
    let mut cursor = cd_offset as usize;
    for _ in 0..entry_count {
        if read_u32(bytes, cursor) != 0x0201_4b50 {
            return Err(DocOpsError::Parse(format!(
                "ZIP 中央目录条目签名损坏 (偏移 {cursor})"
            )));
        }
        let method = read_u16(bytes, cursor + 10);
        let compressed_size = u64::from(read_u32(bytes, cursor + 20));
        let uncompressed_size = u64::from(read_u32(bytes, cursor + 24));
        let name_len = read_u16(bytes, cursor + 28) as usize;
        let extra_len = read_u16(bytes, cursor + 30) as usize;
        let comment_len = read_u16(bytes, cursor + 32) as usize;
        let local_header_offset = u64::from(read_u32(bytes, cursor + 42));
        if compressed_size == 0xFFFF_FFFF
            || uncompressed_size == 0xFFFF_FFFF
            || local_header_offset == 0xFFFF_FFFF
        {
            return Err(DocOpsError::Unsupported(
                "ZIP64 条目不在本批子集 (显式拒绝, 不降级)".to_string(),
            ));
        }
        let name_start = cursor + 46;
        let name_bytes = slice(bytes, name_start, name_len)?;
        let name = String::from_utf8(name_bytes.to_vec())
            .map_err(|_| DocOpsError::Parse("ZIP 条目名不是 UTF-8".to_string()))?;
        entries.push(ZipDirEntry {
            name,
            method,
            compressed_size,
            uncompressed_size,
            local_header_offset,
        });
        cursor = name_start + name_len + extra_len + comment_len;
    }

    let mut out = Vec::with_capacity(entries.len());
    let mut total_uncompressed: u64 = 0;
    for entry in entries {
        if entry.name.ends_with('/') {
            continue;
        }
        let local_at = entry.local_header_offset as usize;
        if read_u32(bytes, local_at) != 0x0403_4b50 {
            return Err(DocOpsError::Parse(format!(
                "ZIP 条目 {} 的本地头签名损坏",
                entry.name
            )));
        }
        let name_len = read_u16(bytes, local_at + 26) as usize;
        let extra_len = read_u16(bytes, local_at + 28) as usize;
        let data_at = local_at + 30 + name_len + extra_len;
        let raw = slice(bytes, data_at, entry.compressed_size as usize)?;
        let data = match entry.method {
            0 => {
                if raw.len() as u64 != entry.uncompressed_size {
                    return Err(DocOpsError::Parse(format!(
                        "ZIP 条目 {} 大小不符",
                        entry.name
                    )));
                }
                raw.to_vec()
            }
            8 => {
                let remaining = decompressed_budget.saturating_sub(total_uncompressed);
                let mut decoder = flate2::read::DeflateDecoder::new(raw).take(remaining + 1);
                let mut expanded = Vec::new();
                decoder.read_to_end(&mut expanded).map_err(|error| {
                    DocOpsError::Parse(format!("ZIP 条目 {} 解压失败: {error}", entry.name))
                })?;
                expanded
            }
            other => {
                return Err(DocOpsError::Unsupported(format!(
                    "ZIP 条目 {} 压缩方法 {other} 不在子集 (只认 stored/deflate)",
                    entry.name
                )))
            }
        };
        total_uncompressed = total_uncompressed.saturating_add(data.len() as u64);
        if total_uncompressed > decompressed_budget {
            return Err(DocOpsError::UncompressedTooLarge {
                bytes: total_uncompressed,
                cap: decompressed_budget,
            });
        }
        out.push(ZipEntry {
            name: entry.name,
            data,
        });
    }
    Ok(out)
}

/// 从文件尾回扫中央目录结尾记录签名。
fn find_eocd(bytes: &[u8]) -> Option<usize> {
    let window_start = bytes.len().saturating_sub(66_000);
    let mut at = bytes.len().saturating_sub(22);
    loop {
        if at < window_start {
            return None;
        }
        if read_u32(bytes, at) == 0x0605_4b50 {
            return Some(at);
        }
        if at == window_start {
            return None;
        }
        at -= 1;
    }
}

/// 小端 u16 (越界按 0 处理, 由调用方的签名检查兜底)。
fn read_u16(bytes: &[u8], at: usize) -> u16 {
    if at + 2 > bytes.len() {
        return 0;
    }
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

/// 小端 u32 (越界按 0 处理, 由调用方的签名检查兜底)。
fn read_u32(bytes: &[u8], at: usize) -> u32 {
    if at + 4 > bytes.len() {
        return 0;
    }
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// 受控切片 (越界 = 解析失败, 不 panic)。
fn slice(bytes: &[u8], at: usize, len: usize) -> Result<&[u8], DocOpsError> {
    let end = at
        .checked_add(len)
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| DocOpsError::Parse("ZIP 条目数据越界".to_string()))?;
    Ok(&bytes[at..end])
}

/// 写端压缩档位 (与读端子集同一对称面: stored / deflate 两档)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZipCompression {
    /// 不压缩 (method 0)。
    Stored,
    /// 原始 deflate (method 8)。
    Deflate,
}

/// ZIP CRC-32 (条目完整性字段; 与读端同一校验口径)。
fn zip_crc32(data: &[u8]) -> u32 {
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

/// 把条目序列写成合法 ZIP 容器 (本地头 + 中央目录 + 结尾记录; 名字按 UTF-8
/// 声明 (通用标志位 bit 11); 时间戳零值如实, 不编造时间)。
///
/// 物理字段上限如实钉死 (名字 ≤ 64 KiB - 1, 数据 / 偏移 ≤ 4 GiB - 1, 条目数
/// ≤ 65535): 超出**拒绝即帧**, 不截断不换 ZIP64。字节量级的防炸弹预检由调用方
/// 参数矩阵 (输入上限 / 源文上限) 承担, 本端只钉容器物理上限。
pub(crate) fn zip_write_entries(
    entries: &[ZipEntry],
    compression: ZipCompression,
) -> Result<Vec<u8>, DocOpsError> {
    if entries.len() > u16::MAX as usize {
        return Err(DocOpsError::Parse(format!(
            "ZIP 写端: 条目数 {} 超过 65535 (ZIP64 不在子集, 拒绝不截断)",
            entries.len()
        )));
    }
    let method: u16 = match compression {
        ZipCompression::Stored => 0,
        ZipCompression::Deflate => 8,
    };
    let mut out = Vec::new();
    let mut central = Vec::new();
    for entry in entries {
        let name = entry.name.as_bytes();
        if name.len() > u16::MAX as usize {
            return Err(DocOpsError::Parse(
                "ZIP 写端: 条目名超过 65535 字节 (拒绝不截断)".to_string(),
            ));
        }
        let data: &[u8] = &entry.data;
        let raw: Vec<u8> = match compression {
            ZipCompression::Stored => data.to_vec(),
            ZipCompression::Deflate => {
                let mut encoder =
                    flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
                std::io::Write::write_all(&mut encoder, data).map_err(|error| {
                    DocOpsError::Io(format!("ZIP 写端 deflate 压缩失败: {error}"))
                })?;
                encoder.finish().map_err(|error| {
                    DocOpsError::Io(format!("ZIP 写端 deflate 收尾失败: {error}"))
                })?
            }
        };
        if raw.len() > u32::MAX as usize || data.len() > u32::MAX as usize {
            return Err(DocOpsError::Parse(
                "ZIP 写端: 条目数据超过 4 GiB (ZIP64 不在子集, 拒绝不截断)".to_string(),
            ));
        }
        if out.len() > u32::MAX as usize {
            return Err(DocOpsError::Parse(
                "ZIP 写端: 容器偏移超过 4 GiB (ZIP64 不在子集, 拒绝不截断)".to_string(),
            ));
        }
        let offset = out.len() as u32;
        let crc = zip_crc32(data);

        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0x0800u16.to_le_bytes()); // 名字按 UTF-8 声明
        out.extend_from_slice(&method.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // mod time (零值如实)
        out.extend_from_slice(&0u16.to_le_bytes()); // mod date (零值如实)
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(raw.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra length
        out.extend_from_slice(name);
        out.extend_from_slice(&raw);

        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes()); // version made by
        central.extend_from_slice(&20u16.to_le_bytes()); // version needed
        central.extend_from_slice(&0x0800u16.to_le_bytes());
        central.extend_from_slice(&method.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&(raw.len() as u32).to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // extra length
        central.extend_from_slice(&0u16.to_le_bytes()); // comment length
        central.extend_from_slice(&0u16.to_le_bytes()); // disk number start
        central.extend_from_slice(&0u16.to_le_bytes()); // internal attributes
        central.extend_from_slice(&0u32.to_le_bytes()); // external attributes
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name);
    }
    if central.len() > u32::MAX as usize {
        return Err(DocOpsError::Parse(
            "ZIP 写端: 中央目录超过 4 GiB (ZIP64 不在子集, 拒绝不截断)".to_string(),
        ));
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
    Ok(out)
}

// ---------------------------------------------------------------------------
// DOCX (OOXML) 读端: 正文提取 (Markdown 转出) + 标题大纲
// ---------------------------------------------------------------------------

/// 一个文字处理段落 (抽取中转形状)。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct DocxParagraph {
    /// 段落样式名 (`w:pStyle w:val`), 无样式 = 空。
    style: String,
    /// 是否列表项 (`w:numPr` 在场)。
    numbered: bool,
    /// 段落文本 (run 文本 + tab/br 拼接)。
    text: String,
}

/// 标题级别 (`HeadingN` → N); 非标题样式返回 [`None`]。
fn heading_level(style: &str) -> Option<u32> {
    let trimmed = style.trim();
    let lower = trimmed.to_ascii_lowercase();
    let rest = lower.strip_prefix("heading")?;
    let digits: String = rest
        .trim()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    if digits.is_empty() {
        return None;
    }
    let level: u32 = digits.parse().ok()?;
    (1..=9).contains(&level).then_some(level)
}

/// 从 DOCX 容器字节流抽出段落序列 (样式 / 列表标记 / 文本)。
///
/// `decompressed_budget` 是容器解压预算 (见 [`zip_read_entries`])。
fn docx_parse_paragraphs(
    bytes: &[u8],
    decompressed_budget: u64,
) -> Result<Vec<DocxParagraph>, DocOpsError> {
    let entries = zip_read_entries(bytes, decompressed_budget)?;
    let document = entries
        .iter()
        .find(|entry| entry.name == "word/document.xml")
        .ok_or_else(|| {
            DocOpsError::Parse("DOCX 容器缺 word/document.xml (不是文字处理文档)".to_string())
        })?;
    let xml = std::str::from_utf8(&document.data)
        .map_err(|_| DocOpsError::Unsupported("word/document.xml 不是 UTF-8".to_string()))?;
    parse_wordprocessingml(xml)
}

/// 扫描 OOXML 文字处理标记的段落层 (只认 `w:p` / `w:pStyle` / `w:numPr` /
/// `w:t` / `w:tab` / `w:br`, 其余元素透明跳过)。
fn parse_wordprocessingml(xml: &str) -> Result<Vec<DocxParagraph>, DocOpsError> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut paragraphs: Vec<DocxParagraph> = Vec::new();
    let mut current: Option<DocxParagraph> = None;
    let mut in_text = false;
    loop {
        let event = reader
            .read_event()
            .map_err(|error| DocOpsError::Parse(format!("word/document.xml 标记损坏: {error}")))?;
        match event {
            Event::Start(start) => match start.local_name().as_ref() {
                b"p" => {
                    if let Some(done) = current.replace(DocxParagraph::default()) {
                        paragraphs.push(done);
                    }
                }
                b"pStyle" => {
                    if let Some(paragraph) = current.as_mut() {
                        for attribute in start.attributes() {
                            let attribute = attribute.map_err(|error| {
                                DocOpsError::Parse(format!("w:pStyle 属性损坏: {error}"))
                            })?;
                            if attribute.key.local_name().as_ref() == b"val" {
                                paragraph.style = attribute
                                    .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                    .map_err(|error| {
                                        DocOpsError::Parse(format!("w:pStyle 值损坏: {error}"))
                                    })?
                                    .into_owned();
                            }
                        }
                    }
                }
                b"numPr" => {
                    if let Some(paragraph) = current.as_mut() {
                        paragraph.numbered = true;
                    }
                }
                b"t" => in_text = true,
                _ => {}
            },
            Event::Empty(empty) => match empty.local_name().as_ref() {
                b"pStyle" => {
                    if let Some(paragraph) = current.as_mut() {
                        for attribute in empty.attributes() {
                            let attribute = attribute.map_err(|error| {
                                DocOpsError::Parse(format!("w:pStyle 属性损坏: {error}"))
                            })?;
                            if attribute.key.local_name().as_ref() == b"val" {
                                paragraph.style = attribute
                                    .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                    .map_err(|error| {
                                        DocOpsError::Parse(format!("w:pStyle 值损坏: {error}"))
                                    })?
                                    .into_owned();
                            }
                        }
                    }
                }
                b"numPr" => {
                    if let Some(paragraph) = current.as_mut() {
                        paragraph.numbered = true;
                    }
                }
                b"tab" => {
                    if let Some(paragraph) = current.as_mut() {
                        paragraph.text.push('\t');
                    }
                }
                b"br" => {
                    if let Some(paragraph) = current.as_mut() {
                        paragraph.text.push('\n');
                    }
                }
                _ => {}
            },
            Event::Text(text) => {
                if in_text {
                    let decoded = text.decode().map_err(|error| {
                        DocOpsError::Parse(format!("w:t 文本解码失败: {error}"))
                    })?;
                    if let Some(paragraph) = current.as_mut() {
                        paragraph.text.push_str(&decoded);
                    }
                }
            }
            Event::GeneralRef(reference) => {
                if in_text {
                    let raw = reference.into_inner();
                    let entity = std::str::from_utf8(&raw).unwrap_or_default();
                    if let Some(paragraph) = current.as_mut() {
                        paragraph.text.push_str(&decode_xml_entity(entity));
                    }
                }
            }
            Event::End(end) => match end.local_name().as_ref() {
                b"t" => in_text = false,
                b"p" => {
                    if let Some(done) = current.take() {
                        paragraphs.push(done);
                    }
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    if let Some(done) = current.take() {
        paragraphs.push(done);
    }
    Ok(paragraphs)
}

/// 实体引用解码 (预定义实体 + 数字引用; 未知实体按缺字显式 `?`, 不编字形)。
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

/// 段落序列 → Markdown 转出 (标题 `#` 级 / 列表 `- ` 项 / 正文行; 空段丢弃)。
fn docx_paragraphs_to_markdown(paragraphs: &[DocxParagraph]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for paragraph in paragraphs {
        let text = paragraph.text.trim();
        if text.is_empty() {
            continue;
        }
        if let Some(level) = heading_level(&paragraph.style) {
            lines.push(format!("{} {text}", "#".repeat(level as usize)));
        } else if paragraph.numbered {
            lines.push(format!("- {text}"));
        } else {
            lines.push(text.to_string());
        }
    }
    lines.join("\n")
}

/// 段落序列 → 标题大纲 (只收标题级段落, 层级 1..=9)。
fn docx_paragraphs_to_outline(paragraphs: &[DocxParagraph]) -> Vec<(u32, String)> {
    paragraphs
        .iter()
        .filter_map(|paragraph| {
            let text = paragraph.text.trim();
            if text.is_empty() {
                return None;
            }
            heading_level(&paragraph.style).map(|level| (level, text.to_string()))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// DOCX (OOXML) 写端: Markdown → DOCX 最小生成 + 副本追加段落 (结构面)
// ---------------------------------------------------------------------------

/// 行内 run (着色最小面: 粗 / 斜; 标记不嵌套不重叠)。
#[derive(Debug, Clone, PartialEq, Eq)]
struct DocxRun {
    /// run 文本 (原文, 已解行内标记)。
    text: String,
    /// 粗体 (`**...**`)。
    bold: bool,
    /// 斜体 (`*...*` / `_..._`)。
    italic: bool,
}

/// Markdown 结构块 (标题 / 列表项 / 正文段; 逐行一块)。
#[derive(Debug, Clone, PartialEq, Eq)]
struct DocxBlock {
    /// 标题级别 (1..=6, ATX `#`..`######`); 非标题 = [`None`]。
    heading: Option<u32>,
    /// 列表项 (`-` `*` `+` 或 `N.` `N)`; 落 `w:numPr numId=1` 标记, 外观由
    /// `numbering.xml` 的一级项目符号定义给出 —— 有序与无序当前同外观, 不区分)。
    list: bool,
    /// 行内 run 序列。
    runs: Vec<DocxRun>,
}

/// XML 1.0 非法控制字符 (拒绝即帧, 不静默剔除不猜)。
fn is_xml_illegal_char(character: char) -> bool {
    matches!(
        character,
        '\u{0}'..='\u{8}' | '\u{B}' | '\u{C}' | '\u{E}'..='\u{1F}'
    )
}

/// XML 文本转义 (预定义五实体; 其余码位原样)。
fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}

/// 行内强调解析 (子集钉死: `**粗**` / `*斜*` / `_斜_`)。
///
/// 最左配对、内容非空且不跨行; 不成对 / 重叠标记按字面保留 (不猜语义,
/// 不支持嵌套着色)。
fn parse_inline_runs(text: &str) -> Vec<DocxRun> {
    let chars: Vec<char> = text.chars().collect();
    let mut runs: Vec<DocxRun> = Vec::new();
    let mut literal = String::new();
    let mut index = 0usize;
    while index < chars.len() {
        let (marker, marker_len): (&str, usize) =
            if chars[index] == '*' && chars.get(index + 1) == Some(&'*') {
                ("**", 2)
            } else if chars[index] == '*' {
                ("*", 1)
            } else if chars[index] == '_' {
                ("_", 1)
            } else {
                literal.push(chars[index]);
                index += 1;
                continue;
            };
        let open = index + marker_len;
        let mut found = None;
        let mut scan = open;
        while scan + marker_len <= chars.len() {
            let candidate: String = chars[scan..scan + marker_len].iter().collect();
            if candidate == marker {
                found = Some(scan);
                break;
            }
            scan += 1;
        }
        match found {
            Some(close) if close > open => {
                let body: String = chars[open..close].iter().collect();
                if body.is_empty() {
                    // 空内容不成对 (如 `****`): 按字面保留, 不造空 run。
                    literal.push_str(marker);
                    index = open;
                    continue;
                }
                if !literal.is_empty() {
                    runs.push(DocxRun {
                        text: std::mem::take(&mut literal),
                        bold: false,
                        italic: false,
                    });
                }
                runs.push(DocxRun {
                    text: body,
                    bold: marker == "**",
                    italic: marker != "**",
                });
                index = close + marker_len;
            }
            _ => {
                literal.push_str(marker);
                index = open;
            }
        }
    }
    if !literal.is_empty() {
        runs.push(DocxRun {
            text: literal,
            bold: false,
            italic: false,
        });
    }
    runs
}

/// Markdown 结构面解析 (子集钉死): ATX 标题 `#`..`######` / 逐行一段 /
/// 列表项 / 行内粗斜。空行 = 段落分隔; 空结构 (零段落) 拒绝即帧。
fn parse_markdown_blocks(text: &str) -> Result<Vec<DocxBlock>, DocOpsError> {
    let mut blocks: Vec<DocxBlock> = Vec::new();
    for raw_line in text.split('\n') {
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        let trimmed = line.trim_start();
        let hashes = trimmed.chars().take_while(|&c| c == '#').count();
        if (1..=6).contains(&hashes) && trimmed[hashes..].starts_with(' ') {
            let content = trimmed[hashes..].trim();
            if content.is_empty() {
                continue; // 空标题丢弃 (不编造标题文本)。
            }
            blocks.push(DocxBlock {
                heading: Some(hashes as u32),
                list: false,
                runs: parse_inline_runs(content),
            });
            continue;
        }
        if ["- ", "* ", "+ "]
            .iter()
            .any(|marker| trimmed.starts_with(marker))
        {
            let content = trimmed[2..].trim();
            if content.is_empty() {
                continue; // 空列表项丢弃 (不编造项文本)。
            }
            blocks.push(DocxBlock {
                heading: None,
                list: true,
                runs: parse_inline_runs(content),
            });
            continue;
        }
        let digits: String = trimmed.chars().take_while(char::is_ascii_digit).collect();
        if !digits.is_empty() {
            let rest = &trimmed[digits.len()..];
            if rest.starts_with(". ") || rest.starts_with(") ") {
                let content = rest[2..].trim();
                if content.is_empty() {
                    continue; // 空列表项丢弃 (不编造项文本)。
                }
                blocks.push(DocxBlock {
                    heading: None,
                    list: true,
                    runs: parse_inline_runs(content),
                });
                continue;
            }
        }
        blocks.push(DocxBlock {
            heading: None,
            list: false,
            runs: parse_inline_runs(line),
        });
    }
    if blocks.is_empty() {
        return Err(DocOpsError::InvalidParam(
            "源文无段落 (空结构拒绝, 不生成空文档)".to_string(),
        ));
    }
    Ok(blocks)
}

/// 追加文逐行一段 (空行忽略; CRLF 容忍)。
fn source_paragraph_lines(text: &str) -> Vec<String> {
    text.split('\n')
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| !line.trim().is_empty())
        .map(str::to_string)
        .collect()
}

/// run → OOXML run 标记 (着色最小面: 粗 / 斜)。
fn docx_run_xml(run: &DocxRun) -> String {
    let props = match (run.bold, run.italic) {
        (false, false) => String::new(),
        (true, false) => "<w:rPr><w:b/></w:rPr>".to_string(),
        (false, true) => "<w:rPr><w:i/></w:rPr>".to_string(),
        (true, true) => "<w:rPr><w:b/><w:i/></w:rPr>".to_string(),
    };
    format!(
        "<w:r>{props}<w:t xml:space=\"preserve\">{}</w:t></w:r>",
        xml_escape(&run.text)
    )
}

/// 块 → 段落标记 (标题 = `w:pStyle HeadingN`; 列表 = `w:numPr`)。
fn docx_paragraph_xml(block: &DocxBlock) -> String {
    let mut props = String::new();
    if let Some(level) = block.heading {
        props.push_str(&format!("<w:pStyle w:val=\"Heading{level}\"/>"));
    }
    if block.list {
        props.push_str("<w:numPr><w:numId w:val=\"1\"/></w:numPr>");
    }
    let ppr = if props.is_empty() {
        String::new()
    } else {
        format!("<w:pPr>{props}</w:pPr>")
    };
    let runs: String = block.runs.iter().map(docx_run_xml).collect();
    format!("<w:p>{ppr}{runs}</w:p>")
}

/// 块序列 → `word/document.xml` 全文 (最小 OOXML 文字处理标记)。
fn docx_document_xml(blocks: &[DocxBlock]) -> String {
    let body: String = blocks.iter().map(docx_paragraph_xml).collect();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
         <w:body>{body}</w:body></w:document>"
    )
}

/// `Heading1..6` 样式条目 (标题 = 加粗 + 字号阶梯 + 大纲级别; 纯结构, 零字体美化)。
fn docx_heading_styles_xml() -> String {
    // 半磅值 (1/2 pt): 16 / 14 / 13 / 12 / 11 / 11pt 阶梯。
    const SIZES: [u32; 6] = [32, 28, 26, 24, 22, 22];
    // 段前距 (twips): 逐级收窄。
    const BEFORE: [u32; 6] = [240, 200, 160, 120, 120, 120];
    let mut out = String::new();
    for level in 1..=6u32 {
        let index = (level - 1) as usize;
        let before = BEFORE[index];
        let size = SIZES[index];
        let outline = level - 1;
        out.push_str(&format!(
            "<w:style w:type=\"paragraph\" w:styleId=\"Heading{level}\">\
             <w:name w:val=\"heading {level}\"/>\
             <w:basedOn w:val=\"Normal\"/>\
             <w:next w:val=\"Normal\"/>\
             <w:qFormat/>\
             <w:pPr><w:keepNext/><w:spacing w:before=\"{before}\" w:after=\"120\"/>\
             <w:outlineLvl w:val=\"{outline}\"/></w:pPr>\
             <w:rPr><w:b/><w:sz w:val=\"{size}\"/><w:szCs w:val=\"{size}\"/></w:rPr>\
             </w:style>"
        ));
    }
    out
}

/// 最小样式层 (`word/styles.xml`): 文档默认 (Calibri 11pt 正文, 行距 1.1 + 段后
/// 160 twips) + `Normal` + `Heading1..6` + `ListParagraph` + `Strong`/`Emphasis`
/// 字符样式。
///
/// **最小如实 (0 装边界)**: 无主题、无字体美化 (不换字体族/不加颜色/不加阴影)、
/// 样式层本身**不含编号定义** (列表外观独立成 `numbering.xml`, 见
/// [`docx_numbering_xml`]; 此处只给段落/字符样式)。样式名与正文里 `w:pStyle`
/// 的引用同名同源, 消费端按名解析即得标题层级观感; 渲染结果仍取决于消费端样式表
/// 实现。
fn docx_styles_xml() -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
         <w:docDefaults>\
         <w:rPrDefault><w:rPr>\
         <w:rFonts w:ascii=\"Calibri\" w:hAnsi=\"Calibri\" w:cs=\"Calibri\"/>\
         <w:sz w:val=\"22\"/><w:szCs w:val=\"22\"/>\
         </w:rPr></w:rPrDefault>\
         <w:pPrDefault><w:pPr>\
         <w:spacing w:after=\"160\" w:line=\"264\" w:lineRule=\"auto\"/>\
         </w:pPr></w:pPrDefault>\
         </w:docDefaults>\
         <w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\">\
         <w:name w:val=\"Normal\"/><w:qFormat/></w:style>\
         {}\
         <w:style w:type=\"paragraph\" w:styleId=\"ListParagraph\">\
         <w:name w:val=\"List Paragraph\"/><w:basedOn w:val=\"Normal\"/>\
         <w:qFormat/><w:pPr><w:ind w:left=\"720\"/></w:pPr></w:style>\
         <w:style w:type=\"character\" w:default=\"1\" w:styleId=\"DefaultParagraphFont\">\
         <w:name w:val=\"Default Paragraph Font\"/><w:uiPriority w:val=\"1\"/>\
         <w:semiHidden/><w:unhideWhenUsed/></w:style>\
         <w:style w:type=\"character\" w:styleId=\"Strong\">\
         <w:name w:val=\"Strong\"/><w:basedOn w:val=\"DefaultParagraphFont\"/>\
         <w:qFormat/><w:rPr><w:b/></w:rPr></w:style>\
         <w:style w:type=\"character\" w:styleId=\"Emphasis\">\
         <w:name w:val=\"Emphasis\"/><w:basedOn w:val=\"DefaultParagraphFont\"/>\
         <w:qFormat/><w:rPr><w:i/></w:rPr></w:style>\
         </w:styles>",
        docx_heading_styles_xml()
    )
}

/// 最小编号层 (`word/numbering.xml`): 一个 `w:abstractNum` + 一个 `w:num`。
///
/// 正文列表条目以 `w:numPr > w:numId w:val="1"` 引用本文件的 `w:num w:numId="1"`,
/// 后者经 `w:abstractNumId w:val="0"` 落到 `w:abstractNum w:abstractNumId="0"` 的
/// `w:lvl w:ilvl="0"` (项目符号 `•`, 左缩进 720 + 悬挂 360 twips)。
///
/// **最小如实 (0 装边界)**: 只给一级 bullet, 且**有序列表当前与无序列表同样渲染为
/// 项目符号** —— 解析层尚无 ordered/bullet 分型 (Markdown 的 `-` `*` `+` 与
/// `N.` `N)` 都落同一 `numId=1`, 读回归一为 `- `), 故此处不写 `w:numFmt
/// w:val="decimal"` 也不写多级 `w:lvl`: 编号支持要等解析层先有分型, 本层不假装
/// 提供。消费端按关系 (rId2) 与内容类型找到本部件即得项目符号外观。
fn docx_numbering_xml() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
     <w:numbering xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
     <w:abstractNum w:abstractNumId=\"0\">\
     <w:lvl w:ilvl=\"0\">\
     <w:numFmt w:val=\"bullet\"/>\
     <w:lvlText w:val=\"•\"/>\
     <w:lvlJc w:val=\"left\"/>\
     <w:pPr><w:ind w:left=\"720\" w:hanging=\"360\"/></w:pPr>\
     </w:lvl>\
     </w:abstractNum>\
     <w:num w:numId=\"1\"><w:abstractNumId w:val=\"0\"/></w:num>\
     </w:numbering>"
        .to_string()
}

/// 最小 OOXML 包 (六件: 内容类型 + 根关系 + 正文关系 + 正文 + 样式层 + 编号层)。
///
/// 2026-10-18 样式层补齐: `word/_rels/document.xml.rels` 把正文指向 `styles.xml`,
/// `[Content_Types].xml` 登记样式部件内容类型 —— 缺一即 Word 判「样式缺失」。
/// 编号层同口径: 正文关系再给 rId2 → `numbering.xml`, 内容类型同步登记编号部件,
/// 正文里的 `w:numPr numId="1"` 才解析得到项目符号 (悬空 numId = 消费端不显示
/// 符号), 故三处 (正文标记 / 关系 / 内容类型) 一起给, 不靠消费端容错。
fn docx_package(document_xml: &str) -> Result<Vec<u8>, DocOpsError> {
    let types = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
         <Default Extension=\"rels\" \
         ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
         <Default Extension=\"xml\" ContentType=\"application/xml\"/>\
         <Override PartName=\"/word/document.xml\" \
         ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/>\
         <Override PartName=\"/word/styles.xml\" \
         ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/>\
         <Override PartName=\"/word/numbering.xml\" \
         ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml\"/>\
         </Types>";
    let rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
         <Relationship Id=\"rId1\" \
         Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" \
         Target=\"word/document.xml\"/></Relationships>";
    let document_rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
         <Relationship Id=\"rId1\" \
         Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" \
         Target=\"styles.xml\"/>\
         <Relationship Id=\"rId2\" \
         Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering\" \
         Target=\"numbering.xml\"/></Relationships>";
    zip_write_entries(
        &[
            ZipEntry {
                name: "[Content_Types].xml".to_string(),
                data: types.as_bytes().to_vec(),
            },
            ZipEntry {
                name: "_rels/.rels".to_string(),
                data: rels.as_bytes().to_vec(),
            },
            ZipEntry {
                name: "word/_rels/document.xml.rels".to_string(),
                data: document_rels.as_bytes().to_vec(),
            },
            ZipEntry {
                name: "word/document.xml".to_string(),
                data: document_xml.as_bytes().to_vec(),
            },
            ZipEntry {
                name: "word/styles.xml".to_string(),
                data: docx_styles_xml().into_bytes(),
            },
            ZipEntry {
                name: "word/numbering.xml".to_string(),
                data: docx_numbering_xml().into_bytes(),
            },
        ],
        ZipCompression::Deflate,
    )
}

/// 副本追加段落: 只动 `word/document.xml` 的 body 尾部 (节属性 `w:sectPr`
/// 若在 body 尾则插它之前 —— 依规节属性恒是 body 尾子), 其余标记一字不动。
fn docx_append_paragraphs(document_xml: &str, lines: &[String]) -> Result<String, DocOpsError> {
    let body_end = document_xml.rfind("</w:body>").ok_or_else(|| {
        DocOpsError::Parse("word/document.xml 缺 </w:body> (不是合法文字处理文档)".to_string())
    })?;
    let mut insert_at = body_end;
    let before = &document_xml[..body_end];
    if let Some(sect_close) = before.rfind("</w:sectPr>") {
        if before[sect_close + "</w:sectPr>".len()..].trim().is_empty() {
            let sect_open = before[..sect_close].rfind("<w:sectPr").ok_or_else(|| {
                DocOpsError::Parse(
                    "word/document.xml 节属性标记不成对 (有 </w:sectPr> 无 <w:sectPr; 拒绝不猜)"
                        .to_string(),
                )
            })?;
            insert_at = sect_open;
        }
    }
    let mut added = String::new();
    for line in lines {
        added.push_str(&format!(
            "<w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
            xml_escape(line)
        ));
    }
    let mut out = String::with_capacity(document_xml.len() + added.len());
    out.push_str(&document_xml[..insert_at]);
    out.push_str(&added);
    out.push_str(&document_xml[insert_at..]);
    Ok(out)
}

// ---------------------------------------------------------------------------
// PDF 子集读端 (对象扫描; 声明子集之外显式拒绝)
// ---------------------------------------------------------------------------

/// PDF 间接引用 (`N G R`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PdfRef {
    /// 对象号。
    num: u32,
    /// 代号。
    gen: u32,
}

/// PDF 对象 (解析中转形状; 流保留原始压缩数据, 不解不重压)。
#[derive(Debug, Clone, PartialEq)]
enum PdfObject {
    /// `null`。
    Null,
    /// 布尔。
    Bool(bool),
    /// 整数。
    Int(i64),
    /// 实数。
    Real(f64),
    /// 名字 (不含前导 `/`)。
    Name(String),
    /// 字符串原始字节 (未解码)。
    Str(Vec<u8>),
    /// 数组。
    Array(Vec<PdfObject>),
    /// 字典 (键不含前导 `/`)。
    Dict(BTreeMap<String, PdfObject>),
    /// 流 (字典 + 原始数据)。
    Stream {
        /// 流字典。
        dict: BTreeMap<String, PdfObject>,
        /// 流原始数据 (按 `/Length` 或 `endstream` 回扫截取)。
        data: Vec<u8>,
    },
    /// 间接引用。
    Ref(PdfRef),
}

impl PdfObject {
    /// 取字典视图 (流取其字典)。
    fn as_dict(&self) -> Option<&BTreeMap<String, PdfObject>> {
        match self {
            Self::Dict(dict) | Self::Stream { dict, .. } => Some(dict),
            _ => None,
        }
    }

    /// 取名字视图。
    fn as_name(&self) -> Option<&str> {
        match self {
            Self::Name(name) => Some(name),
            _ => None,
        }
    }

    /// 取整数视图 (实数按四舍五入兼容坏生成器)。
    fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(value) => Some(*value),
            Self::Real(value) => Some(value.round() as i64),
            _ => None,
        }
    }

    /// 取数组视图。
    fn as_array(&self) -> Option<&[PdfObject]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    /// 取引用视图。
    fn as_ref_id(&self) -> Option<PdfRef> {
        match self {
            Self::Ref(reference) => Some(*reference),
            _ => None,
        }
    }

    /// 取字符串原始字节。
    fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Str(bytes) => Some(bytes),
            _ => None,
        }
    }
}

/// 解析出的 PDF 文档 (对象表 + 目录 + 版本)。
#[derive(Debug, Clone)]
struct PdfDocument {
    /// 对象表 (按 `N G` 键)。
    objects: BTreeMap<(u32, u32), PdfObject>,
    /// 文档目录对象 (trailer `/Root`, 缺席时按 `/Type /Catalog` 认)。
    catalog: PdfRef,
    /// 头部版本串 (如 `1.7`)。
    version: String,
}

/// 递归引用解析深度上限 (防恶意深链)。
const RESOLVE_DEPTH_CAP: u32 = 32;

impl PdfDocument {
    /// 取对象 (缺号 = 解析失败)。
    fn get(&self, reference: PdfRef) -> Result<&PdfObject, DocOpsError> {
        self.objects
            .get(&(reference.num, reference.gen))
            .ok_or_else(|| {
                DocOpsError::Parse(format!(
                    "PDF 间接引用 {} {} R 悬空 (对象缺号)",
                    reference.num, reference.gen
                ))
            })
    }

    /// 跟随引用链取实体对象。
    fn resolve<'a>(&'a self, object: &'a PdfObject) -> Result<&'a PdfObject, DocOpsError> {
        let mut current = object;
        for _ in 0..RESOLVE_DEPTH_CAP {
            match current {
                PdfObject::Ref(reference) => current = self.get(*reference)?,
                other => return Ok(other),
            }
        }
        Err(DocOpsError::Parse(
            "PDF 间接引用链超深 (疑似环形引用)".to_string(),
        ))
    }

    /// 字典项解析后的实体值。
    fn dict_resolved<'a>(
        &'a self,
        dict: &'a BTreeMap<String, PdfObject>,
        key: &str,
    ) -> Result<Option<&'a PdfObject>, DocOpsError> {
        match dict.get(key) {
            None => Ok(None),
            Some(value) => Ok(Some(self.resolve(value)?)),
        }
    }

    /// 页树拍平 (继承属性物化: /Resources /MediaBox /CropBox /Rotate)。
    fn collect_pages(&self) -> Result<Vec<PdfPage>, DocOpsError> {
        let catalog = self.get(self.catalog)?;
        let catalog_dict = catalog
            .as_dict()
            .ok_or_else(|| DocOpsError::Parse("PDF 目录对象不是字典".to_string()))?;
        let pages_ref = catalog_dict
            .get("Pages")
            .and_then(PdfObject::as_ref_id)
            .ok_or_else(|| DocOpsError::Parse("PDF 目录缺 /Pages 页树".to_string()))?;
        let mut pages = Vec::new();
        self.walk_pages(pages_ref, &BTreeMap::new(), &mut pages, 0)?;
        if pages.is_empty() {
            return Err(DocOpsError::Parse("PDF 页树没有页对象".to_string()));
        }
        Ok(pages)
    }

    /// 页树递归 (节点继承属性向叶传递, 叶上物化)。
    fn walk_pages(
        &self,
        node_ref: PdfRef,
        inherited: &BTreeMap<String, PdfObject>,
        out: &mut Vec<PdfPage>,
        depth: u32,
    ) -> Result<(), DocOpsError> {
        if depth > 64 {
            return Err(DocOpsError::Parse("PDF 页树嵌套超深".to_string()));
        }
        let node = self.get(node_ref)?;
        let node_dict = node
            .as_dict()
            .ok_or_else(|| DocOpsError::Parse("PDF 页树节点不是字典".to_string()))?;
        let mut merged = inherited.clone();
        for key in ["Resources", "MediaBox", "CropBox", "Rotate"] {
            if let Some(value) = node_dict.get(key) {
                merged.insert(key.to_string(), value.clone());
            }
        }
        if let Some(kids) = node_dict
            .get("Kids")
            .map(|value| self.resolve(value))
            .transpose()?
            .and_then(PdfObject::as_array)
        {
            for kid in kids {
                let kid_ref = kid
                    .as_ref_id()
                    .ok_or_else(|| DocOpsError::Parse("页树 /Kids 项不是间接引用".to_string()))?;
                self.walk_pages(kid_ref, &merged, out, depth + 1)?;
            }
            return Ok(());
        }
        let mut dict = node_dict.clone();
        for (key, value) in merged {
            dict.entry(key).or_insert(value);
        }
        out.push(PdfPage {
            obj_ref: PdfRef {
                num: node_ref.num,
                gen: node_ref.gen,
            },
            dict,
        });
        Ok(())
    }

    /// 页的内容流字节 (多流按序拼接; 滤器只认 FlateDecode/无)。
    fn page_content_bytes(&self, page: &PdfPage) -> Result<Vec<u8>, DocOpsError> {
        let Some(contents) = page.dict.get("Contents") else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        let resolved = self.resolve(contents)?;
        let streams: Vec<&PdfObject> = match resolved {
            PdfObject::Array(items) => {
                let mut collected = Vec::with_capacity(items.len());
                for item in items {
                    collected.push(self.resolve(item)?);
                }
                collected
            }
            other => vec![other],
        };
        for stream in streams {
            let PdfObject::Stream { dict, data } = stream else {
                return Err(DocOpsError::Parse(
                    "页 /Contents 指向的不是流对象".to_string(),
                ));
            };
            out.extend_from_slice(&decode_stream(dict, data)?);
            out.push(b'\n');
        }
        Ok(out)
    }
}

/// 页 (对象引用 + 物化后的页字典)。
#[derive(Debug, Clone)]
struct PdfPage {
    /// 页对象引用。
    obj_ref: PdfRef,
    /// 页字典 (继承属性已物化)。
    dict: BTreeMap<String, PdfObject>,
}

/// 流解码: 无滤器原样 / `FlateDecode` zlib 解压; 其余显式拒绝 (报滤器名)。
fn decode_stream(dict: &BTreeMap<String, PdfObject>, data: &[u8]) -> Result<Vec<u8>, DocOpsError> {
    let filters: Vec<String> = match dict.get("Filter") {
        None | Some(PdfObject::Null) => Vec::new(),
        Some(PdfObject::Name(name)) => vec![name.clone()],
        Some(PdfObject::Array(items)) => items
            .iter()
            .filter_map(|item| match item {
                PdfObject::Name(name) => Some(name.clone()),
                PdfObject::Ref(_) => Some(String::from("<ref-filter>")),
                _ => None,
            })
            .collect(),
        Some(_) => vec![String::from("<unknown-filter>")],
    };
    let mut current = data.to_vec();
    for filter in &filters {
        match filter.as_str() {
            "FlateDecode" => {
                let mut decoder = flate2::read::ZlibDecoder::new(current.as_slice());
                let mut expanded = Vec::new();
                decoder.read_to_end(&mut expanded).map_err(|error| {
                    DocOpsError::Parse(format!("FlateDecode 解压失败: {error}"))
                })?;
                current = expanded;
            }
            other => {
                return Err(DocOpsError::Unsupported(format!(
                    "内容流滤器 {other} 不在子集 (只认 FlateDecode / 无滤器)"
                )))
            }
        }
    }
    Ok(current)
}

/// 解析 PDF 字节流 (对象扫描版; 加密文档无解密面显式拒绝)。
fn parse_pdf(bytes: &[u8]) -> Result<PdfDocument, DocOpsError> {
    let header_at = find_pdf_header(bytes)
        .ok_or_else(|| DocOpsError::Parse("缺 %PDF- 头 (不是 PDF 文档)".to_string()))?;
    let version = String::from_utf8_lossy(&bytes[header_at + 5..header_at + 8]).to_string();
    if contains_token(bytes, b"/Encrypt") {
        return Err(DocOpsError::Unsupported(
            "加密 PDF 无解密面 (本批不做加密/解密, 显式拒绝)".to_string(),
        ));
    }

    let mut objects = BTreeMap::new();
    for (num, gen, body_at) in find_object_headers(bytes) {
        let mut cursor = ParsedCursor { bytes, at: body_at };
        let object = cursor.parse_value()?;
        let object = match object {
            PdfObject::Dict(dict) => {
                cursor.skip_ws();
                if bytes[cursor.at..].starts_with(b"stream") {
                    let mut stream_at = cursor.at + 6;
                    if bytes.get(stream_at) == Some(&b'\r') {
                        stream_at += 1;
                    }
                    if bytes.get(stream_at) == Some(&b'\n') {
                        stream_at += 1;
                    }
                    let length = dict
                        .get("Length")
                        .and_then(PdfObject::as_int)
                        .filter(|value| *value >= 0)
                        .map(|value| value as usize);
                    let (data, _end) = match length {
                        Some(len) if stream_at + len <= bytes.len() => {
                            (bytes[stream_at..stream_at + len].to_vec(), stream_at + len)
                        }
                        _ => {
                            let end =
                                find_keyword(bytes, stream_at, b"endstream").ok_or_else(|| {
                                    DocOpsError::Parse("流缺 endstream 关键字".to_string())
                                })?;
                            let mut trimmed = end;
                            if trimmed > stream_at && bytes[trimmed - 1] == b'\n' {
                                trimmed -= 1;
                            }
                            if trimmed > stream_at && bytes[trimmed - 1] == b'\r' {
                                trimmed -= 1;
                            }
                            (bytes[stream_at..trimmed].to_vec(), end)
                        }
                    };
                    PdfObject::Stream { dict, data }
                } else {
                    PdfObject::Dict(dict)
                }
            }
            other => other,
        };
        objects.insert((num, gen), object);
    }
    if objects.is_empty() {
        return Err(DocOpsError::Parse("PDF 没有可解析对象".to_string()));
    }

    let document = PdfDocument {
        objects,
        catalog: PdfRef { num: 0, gen: 0 },
        version,
    };
    let catalog = find_catalog(bytes, &document)?;
    Ok(PdfDocument {
        catalog,
        ..document
    })
}

/// 从 trailer `/Root` 或 `/Type /Catalog` 认目录对象。
fn find_catalog(bytes: &[u8], document: &PdfDocument) -> Result<PdfRef, DocOpsError> {
    if let Some(trailer_at) = find_keyword(bytes, 0, b"trailer") {
        let mut cursor = ParsedCursor {
            bytes,
            at: trailer_at + b"trailer".len(),
        };
        if let Ok(PdfObject::Dict(trailer)) = cursor.parse_value() {
            if let Some(PdfObject::Ref(reference)) = trailer.get("Root") {
                return Ok(*reference);
            }
        }
    }
    for ((num, gen), object) in &document.objects {
        if let Some(dict) = object.as_dict() {
            if dict.get("Type").and_then(PdfObject::as_name) == Some("Catalog") {
                return Ok(PdfRef {
                    num: *num,
                    gen: *gen,
                });
            }
        }
    }
    Err(DocOpsError::Parse(
        "PDF 缺文档目录 (trailer /Root 与 /Type /Catalog 都没找到)".to_string(),
    ))
}

/// 头部 `%PDF-` 位置 (允许前导少量无关字节, 与阅读器宽容度一致)。
fn find_pdf_header(bytes: &[u8]) -> Option<usize> {
    let window = bytes.len().min(1024);
    (0..window).find(|at| bytes[*at..].starts_with(b"%PDF-"))
}

/// 全文是否出现字面 token (加密标记检测用, 宁拒勿漏)。
fn contains_token(bytes: &[u8], token: &[u8]) -> bool {
    bytes.windows(token.len()).any(|window| window == token)
}

/// 从某处起找关键字 (前面须为分隔边界)。
fn find_keyword(bytes: &[u8], from: usize, keyword: &[u8]) -> Option<usize> {
    let mut at = from;
    while at + keyword.len() <= bytes.len() {
        if bytes[at..].starts_with(keyword) {
            let before_ok = at == 0 || is_separator(bytes[at - 1]);
            let after = at + keyword.len();
            let after_ok = after >= bytes.len() || is_separator(bytes[after]);
            if before_ok && after_ok {
                return Some(at);
            }
        }
        at += 1;
    }
    None
}

/// 词法分隔符 (对象/关键字边界的判据)。
fn is_separator(byte: u8) -> bool {
    byte.is_ascii_whitespace()
        || matches!(
            byte,
            b'<' | b'>' | b'[' | b']' | b'(' | b')' | b'/' | b'%' | b'{' | b'}'
        )
}

/// 扫描全部 `N G obj` 对象头, 返回 (对象号, 代号, 对象体起点)。
fn find_object_headers(bytes: &[u8]) -> Vec<(u32, u32, usize)> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while at + 3 <= bytes.len() {
        if bytes[at..].starts_with(b"obj") {
            if let Some(header) = backward_object_header(bytes, at) {
                out.push((header.0, header.1, at + 3));
                at += 3;
                continue;
            }
        }
        at += 1;
    }
    out
}

/// 从 `obj` 前推 `N G` 两段数字 (带词法边界检查)。
fn backward_object_header(bytes: &[u8], obj_at: usize) -> Option<(u32, u32)> {
    let mut end = obj_at;
    while end > 0 && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    let gen_end = end;
    while end > 0 && bytes[end - 1].is_ascii_digit() {
        end -= 1;
    }
    if end == gen_end {
        return None;
    }
    let gen: u32 = std::str::from_utf8(&bytes[end..gen_end])
        .ok()?
        .parse()
        .ok()?;
    let mut num_end = end;
    while num_end > 0 && bytes[num_end - 1].is_ascii_whitespace() {
        num_end -= 1;
    }
    let mut num_start = num_end;
    while num_start > 0 && bytes[num_start - 1].is_ascii_digit() {
        num_start -= 1;
    }
    if num_start == num_end {
        return None;
    }
    if num_start > 0 && !is_separator(bytes[num_start - 1]) {
        return None;
    }
    let num: u32 = std::str::from_utf8(&bytes[num_start..num_end])
        .ok()?
        .parse()
        .ok()?;
    Some((num, gen))
}

/// PDF 值解析游标 (单向前进, 越界即解析失败)。
struct ParsedCursor<'a> {
    /// 全文字节。
    bytes: &'a [u8],
    /// 当前位置。
    at: usize,
}

impl<'a> ParsedCursor<'a> {
    /// 跳过空白与 `%` 注释。
    fn skip_ws(&mut self) {
        loop {
            while self.at < self.bytes.len() && self.bytes[self.at].is_ascii_whitespace() {
                self.at += 1;
            }
            if self.at < self.bytes.len() && self.bytes[self.at] == b'%' {
                while self.at < self.bytes.len() && self.bytes[self.at] != b'\n' {
                    self.at += 1;
                }
                continue;
            }
            break;
        }
    }

    /// 解析一个 PDF 值。
    fn parse_value(&mut self) -> Result<PdfObject, DocOpsError> {
        self.skip_ws();
        let Some(byte) = self.bytes.get(self.at).copied() else {
            return Err(DocOpsError::Parse("PDF 值未预期结束".to_string()));
        };
        match byte {
            b'<' => {
                if self.bytes.get(self.at + 1) == Some(&b'<') {
                    self.at += 2;
                    self.parse_dict()
                } else {
                    self.at += 1;
                    self.parse_hex_string()
                }
            }
            b'(' => self.parse_literal_string(),
            b'[' => {
                self.at += 1;
                let mut items = Vec::new();
                loop {
                    self.skip_ws();
                    if self.bytes.get(self.at) == Some(&b']') {
                        self.at += 1;
                        return Ok(PdfObject::Array(items));
                    }
                    items.push(self.parse_value()?);
                }
            }
            b'/' => {
                self.at += 1;
                let start = self.at;
                while self.at < self.bytes.len() && !is_separator(self.bytes[self.at]) {
                    self.at += 1;
                }
                let raw = &self.bytes[start..self.at];
                Ok(PdfObject::Name(decode_name(raw)))
            }
            b't' if self.bytes[self.at..].starts_with(b"true") => {
                self.at += 4;
                Ok(PdfObject::Bool(true))
            }
            b'f' if self.bytes[self.at..].starts_with(b"false") => {
                self.at += 5;
                Ok(PdfObject::Bool(false))
            }
            b'n' if self.bytes[self.at..].starts_with(b"null") => {
                self.at += 4;
                Ok(PdfObject::Null)
            }
            b'0'..=b'9' | b'+' | b'-' | b'.' => self.parse_number_or_ref(),
            other => Err(DocOpsError::Parse(format!(
                "PDF 值遇到意外字节 {other:#04x} (偏移 {})",
                self.at
            ))),
        }
    }

    /// 解析字典 (`<<` 已消费)。
    fn parse_dict(&mut self) -> Result<PdfObject, DocOpsError> {
        let mut dict = BTreeMap::new();
        loop {
            self.skip_ws();
            if self.bytes[self.at..].starts_with(b">>") {
                self.at += 2;
                return Ok(PdfObject::Dict(dict));
            }
            if self.bytes.get(self.at) != Some(&b'/') {
                return Err(DocOpsError::Parse(format!(
                    "PDF 字典键不是名字 (偏移 {})",
                    self.at
                )));
            }
            self.at += 1;
            let start = self.at;
            while self.at < self.bytes.len() && !is_separator(self.bytes[self.at]) {
                self.at += 1;
            }
            let key = decode_name(&self.bytes[start..self.at]);
            let value = self.parse_value()?;
            dict.insert(key, value);
        }
    }

    /// 解析 `<hex>` 字符串 (`<` 已消费)。
    fn parse_hex_string(&mut self) -> Result<PdfObject, DocOpsError> {
        let mut digits = Vec::new();
        loop {
            match self.bytes.get(self.at).copied() {
                Some(b'>') => {
                    self.at += 1;
                    break;
                }
                Some(byte) if byte.is_ascii_hexdigit() => {
                    digits.push(byte);
                    self.at += 1;
                }
                Some(byte) if byte.is_ascii_whitespace() => self.at += 1,
                Some(other) => {
                    return Err(DocOpsError::Parse(format!(
                        "PDF 十六进制串含非法字节 {other:#04x}"
                    )))
                }
                None => return Err(DocOpsError::Parse("PDF 十六进制串未闭合".to_string())),
            }
        }
        if digits.len() % 2 == 1 {
            digits.push(b'0');
        }
        let mut out = Vec::with_capacity(digits.len() / 2);
        for pair in digits.chunks(2) {
            let text = std::str::from_utf8(pair).unwrap_or_default();
            out.push(u8::from_str_radix(text, 16).unwrap_or(0));
        }
        Ok(PdfObject::Str(out))
    }

    /// 解析 `(literal)` 字符串 (`(` 已消费)。
    fn parse_literal_string(&mut self) -> Result<PdfObject, DocOpsError> {
        self.at += 1;
        let mut out = Vec::new();
        let mut depth = 1usize;
        loop {
            let Some(byte) = self.bytes.get(self.at).copied() else {
                return Err(DocOpsError::Parse("PDF 字面串未闭合".to_string()));
            };
            self.at += 1;
            match byte {
                b'(' => {
                    depth += 1;
                    out.push(byte);
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(PdfObject::Str(out));
                    }
                    out.push(byte);
                }
                b'\\' => {
                    let Some(escaped) = self.bytes.get(self.at).copied() else {
                        return Err(DocOpsError::Parse("PDF 字面串转义未闭合".to_string()));
                    };
                    self.at += 1;
                    match escaped {
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'b' => out.push(0x08),
                        b'f' => out.push(0x0c),
                        b'(' | b')' | b'\\' => out.push(escaped),
                        b'\n' => {}
                        b'\r' => {
                            if self.bytes.get(self.at) == Some(&b'\n') {
                                self.at += 1;
                            }
                        }
                        b'0'..=b'7' => {
                            let mut value = escaped - b'0';
                            for _ in 0..2 {
                                if let Some(next) = self.bytes.get(self.at).copied() {
                                    if (b'0'..=b'7').contains(&next) {
                                        self.at += 1;
                                        value = value * 8 + (next - b'0');
                                    } else {
                                        break;
                                    }
                                }
                            }
                            out.push(value);
                        }
                        other => out.push(other),
                    }
                }
                other => out.push(other),
            }
        }
    }

    /// 解析数字; 识别 `N G R` 三元组转引用 (第二段不是 `R` 则原样回退,
    /// 不吞下一个操作数)。
    fn parse_number_or_ref(&mut self) -> Result<PdfObject, DocOpsError> {
        let start = self.at;
        while self
            .bytes
            .get(self.at)
            .is_some_and(|byte| matches!(byte, b'0'..=b'9' | b'+' | b'-' | b'.'))
        {
            self.at += 1;
        }
        let text = std::str::from_utf8(&self.bytes[start..self.at])
            .map_err(|_| DocOpsError::Parse("PDF 数字不是 ASCII".to_string()))?;
        if text.contains('.') {
            return Ok(PdfObject::Real(text.parse().map_err(|_| {
                DocOpsError::Parse(format!("PDF 实数损坏: {text}"))
            })?));
        }
        let first: i64 = text
            .parse()
            .map_err(|_| DocOpsError::Parse(format!("PDF 整数损坏: {text}")))?;
        // 试探 `N G R` 三元组: 第二段整数 + 关键字 `R`; 不成立则游标回退,
        // 第二段作为独立操作数留给调用方。
        let save = self.at;
        self.skip_ws();
        let gen_start = self.at;
        while self
            .bytes
            .get(self.at)
            .is_some_and(|byte| byte.is_ascii_digit())
        {
            self.at += 1;
        }
        if self.at > gen_start {
            let gen_text = std::str::from_utf8(&self.bytes[gen_start..self.at]).unwrap_or_default();
            self.skip_ws();
            if self.bytes.get(self.at) == Some(&b'R')
                && self
                    .bytes
                    .get(self.at + 1)
                    .is_none_or(|byte| is_separator(*byte))
                && first >= 0
            {
                if let Ok(gen) = gen_text.parse::<u32>() {
                    self.at += 1;
                    return Ok(PdfObject::Ref(PdfRef {
                        num: first as u32,
                        gen,
                    }));
                }
            }
        }
        self.at = save;
        Ok(PdfObject::Int(first))
    }
}

/// 名字解码 (`#XX` 十六进制转义展开)。
fn decode_name(raw: &[u8]) -> String {
    let mut out = Vec::with_capacity(raw.len());
    let mut at = 0;
    while at < raw.len() {
        if raw[at] == b'#' && at + 2 < raw.len() {
            let text = std::str::from_utf8(&raw[at + 1..at + 3]).unwrap_or_default();
            if let Ok(byte) = u8::from_str_radix(text, 16) {
                out.push(byte);
                at += 3;
                continue;
            }
        }
        out.push(raw[at]);
        at += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

// ---------------------------------------------------------------------------
// PDF 文本提取 (Tj / TJ / ' / "; WinAnsi 系简单字体 + /ToUnicode)
// ---------------------------------------------------------------------------

/// 单页提取结果 (文本 + 如实备注)。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ExtractedPage {
    /// 页文本。
    text: String,
    /// 如实备注 (缺字/子集边界), 不编字形。
    notes: Vec<String>,
}

/// 字体解码器 (串 → 文本; 缺字显式 `?` 并计备注)。
#[derive(Debug, Clone)]
struct FontDecoder {
    /// 双字节码 (Type0 / ToUnicode 宽度推断)。
    two_byte: bool,
    /// `/ToUnicode` 映射 (码 → 文本)。
    tounicode: Option<BTreeMap<u16, String>>,
    /// 简单字体 256 表 (码 → 码位, 0 = 缺字)。
    simple: Option<Box<[u16; 256]>>,
}

impl FontDecoder {
    /// 缺字解码器 (Type0 无 /ToUnicode): 每码显式 `?`。
    fn unmapped(two_byte: bool) -> Self {
        Self {
            two_byte,
            tounicode: None,
            simple: None,
        }
    }

    /// 解码一串字节 (返回文本 + 缺字计数)。
    fn decode(&self, raw: &[u8]) -> (String, u64) {
        let mut out = String::new();
        let mut unmapped = 0u64;
        if self.two_byte {
            for pair in raw.chunks(2) {
                if pair.len() < 2 {
                    unmapped += 1;
                    out.push('?');
                    continue;
                }
                let code = u16::from_be_bytes([pair[0], pair[1]]);
                self.push_code(code, &mut out, &mut unmapped);
            }
        } else {
            for byte in raw {
                self.push_code(u16::from(*byte), &mut out, &mut unmapped);
            }
        }
        (out, unmapped)
    }

    /// 单码映射 (ToUnicode 优先, 其次简单表)。
    fn push_code(&self, code: u16, out: &mut String, unmapped: &mut u64) {
        if let Some(map) = &self.tounicode {
            if let Some(text) = map.get(&code) {
                out.push_str(text);
                return;
            }
            *unmapped += 1;
            out.push('?');
            return;
        }
        if let Some(table) = &self.simple {
            if (0..256).contains(&code) {
                let point = table[code as usize];
                if point == 0 {
                    *unmapped += 1;
                    out.push('?');
                } else if let Some(character) = char::from_u32(u32::from(point)) {
                    out.push(character);
                } else {
                    *unmapped += 1;
                    out.push('?');
                }
                return;
            }
        }
        *unmapped += 1;
        out.push('?');
    }
}

/// WinAnsi 码位映射 (0x20-0x7E / 0xA0-0xFF 同 Latin-1, 0x80-0x9F 特殊区)。
fn winansi_point(code: u8) -> u16 {
    match code {
        0x20..=0x7E | 0xA0..=0xFF => u16::from(code),
        0x80 => 0x20AC,
        0x82 => 0x201A,
        0x83 => 0x0192,
        0x84 => 0x201E,
        0x85 => 0x2026,
        0x86 => 0x2020,
        0x87 => 0x2021,
        0x88 => 0x02C6,
        0x89 => 0x2030,
        0x8A => 0x0160,
        0x8B => 0x2039,
        0x8C => 0x0152,
        0x8E => 0x017D,
        0x91 => 0x2018,
        0x92 => 0x2019,
        0x93 => 0x201C,
        0x94 => 0x201D,
        0x95 => 0x2022,
        0x96 => 0x2013,
        0x97 => 0x2014,
        0x98 => 0x02DC,
        0x99 => 0x2122,
        0x9A => 0x0161,
        0x9B => 0x203A,
        0x9C => 0x0153,
        0x9E => 0x017E,
        0x9F => 0x0178,
        _ => 0,
    }
}

/// 字形名 → 码位 (AGL 常用子集 + `uniXXXX` / `uXXXX` 数字形; 未知 = 缺字)。
fn glyph_name_point(name: &str) -> u16 {
    if name.len() == 1 {
        return name
            .chars()
            .next()
            .map(|character| character as u32)
            .filter(|point| *point <= u32::from(u16::MAX))
            .map(|point| point as u16)
            .unwrap_or(0);
    }
    if let Some(hex) = name.strip_prefix("uni") {
        if hex.len() == 4 {
            if let Ok(point) = u32::from_str_radix(hex, 16) {
                return point.min(u32::from(u16::MAX)) as u16;
            }
        }
    }
    if let Some(hex) = name.strip_prefix('u') {
        if let Ok(point) = u32::from_str_radix(hex, 16) {
            return point.min(u32::from(u16::MAX)) as u16;
        }
    }
    let point = match name {
        "space" => 0x20,
        "exclam" => 0x21,
        "quotedbl" => 0x22,
        "numbersign" => 0x23,
        "dollar" => 0x24,
        "percent" => 0x25,
        "ampersand" => 0x26,
        "quotesingle" => 0x27,
        "parenleft" => 0x28,
        "parenright" => 0x29,
        "asterisk" => 0x2A,
        "plus" => 0x2B,
        "comma" => 0x2C,
        "hyphen" => 0x2D,
        "period" => 0x2E,
        "slash" => 0x2F,
        "zero" => 0x30,
        "one" => 0x31,
        "two" => 0x32,
        "three" => 0x33,
        "four" => 0x34,
        "five" => 0x35,
        "six" => 0x36,
        "seven" => 0x37,
        "eight" => 0x38,
        "nine" => 0x39,
        "colon" => 0x3A,
        "semicolon" => 0x3B,
        "less" => 0x3C,
        "equal" => 0x3D,
        "greater" => 0x3E,
        "question" => 0x3F,
        "at" => 0x40,
        "bracketleft" => 0x5B,
        "backslash" => 0x5C,
        "bracketright" => 0x5D,
        "asciicircum" => 0x5E,
        "underscore" => 0x5F,
        "grave" => 0x60,
        "braceleft" => 0x7B,
        "bar" => 0x7C,
        "braceright" => 0x7D,
        "asciitilde" => 0x7E,
        "quoteleft" => 0x2018,
        "quoteright" => 0x2019,
        "quotedblleft" => 0x201C,
        "quotedblright" => 0x201D,
        "quotesinglbase" => 0x201A,
        "quotedblbase" => 0x201E,
        "bullet" => 0x2022,
        "endash" => 0x2013,
        "emdash" => 0x2014,
        "dagger" => 0x2020,
        "daggerdbl" => 0x2021,
        "ellipsis" => 0x2026,
        "perthousand" => 0x2030,
        "Euro" => 0x20AC,
        "trademark" => 0x2122,
        "registered" => 0x00AE,
        "copyright" => 0x00A9,
        "sterling" => 0x00A3,
        "yen" => 0x00A5,
        "currency" => 0x00A4,
        "section" => 0x00A7,
        "degree" => 0x00B0,
        "plusminus" => 0x00B1,
        "multiply" => 0x00D7,
        "divide" => 0x00F7,
        "minus" => 0x2212,
        _ => 0,
    };
    point
}

/// 构造字体解码器 (ToUnicode 优先; WinAnsi 系简单字体次之; 其余显式缺字)。
fn font_decoder(
    document: &PdfDocument,
    font_object: &PdfObject,
) -> Result<FontDecoder, DocOpsError> {
    let font_dict = font_object
        .as_dict()
        .ok_or_else(|| DocOpsError::Parse("字体对象不是字典".to_string()))?;
    let subtype = font_dict
        .get("Subtype")
        .map(|value| document.resolve(value))
        .transpose()?
        .and_then(PdfObject::as_name)
        .unwrap_or_default();
    let two_byte = subtype == "Type0";

    if let Some(tounicode_object) = font_dict.get("ToUnicode") {
        let tounicode_object = document.resolve(tounicode_object)?;
        if let PdfObject::Stream { dict, data } = tounicode_object {
            let cmap = decode_stream(dict, data)?;
            let cmap_text = String::from_utf8_lossy(&cmap).to_string();
            let (map, width_hint) = parse_tounicode_cmap(&cmap_text);
            let two_byte = two_byte || width_hint > 1;
            return Ok(FontDecoder {
                two_byte,
                tounicode: Some(map),
                simple: None,
            });
        }
    }

    if two_byte {
        return Ok(FontDecoder::unmapped(true));
    }

    let mut table: Box<[u16; 256]> = Box::new([0u16; 256]);
    for (code, slot) in table.iter_mut().enumerate() {
        *slot = winansi_point(code as u8);
    }
    match font_dict.get("Encoding") {
        None | Some(PdfObject::Null) => {}
        Some(PdfObject::Name(name)) => {
            if name != "WinAnsiEncoding" {
                return Err(DocOpsError::Unsupported(format!(
                    "字体基编码 {name} 不在子集 (只认 WinAnsiEncoding 系)"
                )));
            }
        }
        Some(encoding) => {
            let encoding = document.resolve(encoding)?;
            let encoding_dict = encoding
                .as_dict()
                .ok_or_else(|| DocOpsError::Parse("字体 /Encoding 不是名字或字典".to_string()))?;
            match encoding_dict.get("BaseEncoding") {
                None | Some(PdfObject::Null) => {}
                Some(PdfObject::Name(name)) if name == "WinAnsiEncoding" => {}
                Some(PdfObject::Name(name)) => {
                    return Err(DocOpsError::Unsupported(format!(
                        "字体基编码 {name} 不在子集 (只认 WinAnsiEncoding 系)"
                    )))
                }
                Some(_) => {
                    return Err(DocOpsError::Parse(
                        "字体 /BaseEncoding 形状损坏".to_string(),
                    ))
                }
            }
            if let Some(differences) = encoding_dict.get("Differences") {
                let differences = document.resolve(differences)?;
                let items = differences
                    .as_array()
                    .ok_or_else(|| DocOpsError::Parse("字体 /Differences 不是数组".to_string()))?;
                let mut code = 0usize;
                for item in items {
                    match item {
                        PdfObject::Int(value) if *value >= 0 => code = *value as usize,
                        PdfObject::Name(name) => {
                            if code < 256 {
                                table[code] = glyph_name_point(name);
                            }
                            code += 1;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(FontDecoder {
        two_byte: false,
        tounicode: None,
        simple: Some(table),
    })
}

/// 解析 `/ToUnicode` CMap (bfchar / bfrange; 返回映射 + 码宽提示)。
fn parse_tounicode_cmap(cmap: &str) -> (BTreeMap<u16, String>, usize) {
    let mut map: BTreeMap<u16, String> = BTreeMap::new();
    let mut width_hint = 1usize;
    let tokens = tokenize_cmap(cmap);
    let mut at = 0usize;
    while at < tokens.len() {
        match tokens[at].as_str() {
            "beginbfchar" => {
                at += 1;
                while at + 1 < tokens.len() && tokens[at] != "endbfchar" {
                    let source = parse_hex_code(&tokens[at]).unwrap_or(0);
                    let target = hex_to_text(&tokens[at + 1]);
                    width_hint = width_hint.max(hex_token_width(&tokens[at]));
                    map.insert(source, target);
                    at += 2;
                }
            }
            "beginbfrange" => {
                at += 1;
                while at + 2 < tokens.len() && tokens[at] != "endbfrange" {
                    let low = parse_hex_code(&tokens[at]).unwrap_or(0);
                    let high = parse_hex_code(&tokens[at + 1]).unwrap_or(0);
                    width_hint = width_hint.max(hex_token_width(&tokens[at]));
                    if tokens[at + 2] == "[" {
                        at += 3;
                        let mut code = low;
                        while at < tokens.len() && tokens[at] != "]" {
                            map.insert(code, hex_to_text(&tokens[at]));
                            code = code.saturating_add(1);
                            at += 1;
                        }
                    } else {
                        let base = parse_hex_code(&tokens[at + 2]).unwrap_or(0);
                        for (offset, code) in
                            (low..=high.min(low.saturating_add(10_000))).enumerate()
                        {
                            let point = u32::from(base) + offset as u32;
                            let text = char::from_u32(point)
                                .map(String::from)
                                .unwrap_or_else(|| "?".to_string());
                            map.insert(code, text);
                        }
                        at += 3;
                    }
                }
            }
            _ => {}
        }
        at += 1;
    }
    (map, width_hint)
}

/// CMap 词法: `<hex>` 串 / 括号 / 裸词 (空白分隔)。
fn tokenize_cmap(cmap: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let bytes = cmap.as_bytes();
    let mut at = 0usize;
    while at < bytes.len() {
        let byte = bytes[at];
        if byte.is_ascii_whitespace() {
            at += 1;
            continue;
        }
        if byte == b'<' {
            let start = at;
            at += 1;
            while at < bytes.len() && bytes[at] != b'>' {
                at += 1;
            }
            at = (at + 1).min(bytes.len());
            tokens.push(cmap[start..at].to_string());
            continue;
        }
        if byte == b'[' || byte == b']' {
            tokens.push((byte as char).to_string());
            at += 1;
            continue;
        }
        let start = at;
        while at < bytes.len()
            && !bytes[at].is_ascii_whitespace()
            && !matches!(bytes[at], b'<' | b'>' | b'[' | b']')
        {
            at += 1;
        }
        tokens.push(cmap[start..at].to_string());
    }
    tokens
}

/// `<hex>` token → 码 (取低 16 位)。
fn parse_hex_code(token: &str) -> Option<u16> {
    let inner = token.strip_prefix('<')?.strip_suffix('>')?;
    if inner.is_empty() {
        return None;
    }
    u16::from_str_radix(inner, 16).ok()
}

/// `<hex>` token → 码字节宽 (供双字节判定)。
fn hex_token_width(token: &str) -> usize {
    token
        .strip_prefix('<')
        .and_then(|rest| rest.strip_suffix('>'))
        .map(|inner| inner.len() / 2)
        .unwrap_or(1)
}

/// `<hex>` token → UTF-16BE 文本 (缺字显式 `?`)。
fn hex_to_text(token: &str) -> String {
    let Some(inner) = token
        .strip_prefix('<')
        .and_then(|rest| rest.strip_suffix('>'))
    else {
        return "?".to_string();
    };
    let mut bytes = Vec::with_capacity(inner.len() / 2);
    let mut at = 0;
    let chars: Vec<char> = inner.chars().collect();
    while at + 1 < chars.len() {
        let pair: String = chars[at..at + 2].iter().collect();
        match u8::from_str_radix(&pair, 16) {
            Ok(byte) => bytes.push(byte),
            Err(_) => return "?".to_string(),
        }
        at += 2;
    }
    let mut out = String::new();
    for unit in bytes.chunks(2) {
        if unit.len() < 2 {
            out.push('?');
            continue;
        }
        let code = u16::from_be_bytes([unit[0], unit[1]]);
        match char::from_u32(u32::from(code)) {
            Some(character) => out.push(character),
            None => out.push('?'),
        }
    }
    out
}

/// 内容流词法游标 (只认算子与操作数骨架)。
struct ContentCursor<'a> {
    /// 内容流字节。
    bytes: &'a [u8],
    /// 当前位置。
    at: usize,
}

impl<'a> ContentCursor<'a> {
    /// 跳过空白与注释。
    fn skip_ws(&mut self) {
        loop {
            while self.at < self.bytes.len() && self.bytes[self.at].is_ascii_whitespace() {
                self.at += 1;
            }
            if self.at < self.bytes.len() && self.bytes[self.at] == b'%' {
                while self.at < self.bytes.len() && self.bytes[self.at] != b'\n' {
                    self.at += 1;
                }
                continue;
            }
            break;
        }
    }

    /// 下一个词法单元: 操作数 (值) 或算子 (名字)。
    fn next_unit(&mut self) -> Option<ContentUnit<'a>> {
        self.skip_ws();
        let byte = self.bytes.get(self.at).copied()?;
        match byte {
            b'<' if self.bytes.get(self.at + 1) == Some(&b'<') => {
                // 字典操作数 (如 BDC 属性): 跳到配对 `>>`。
                let mut depth = 0i32;
                while self.at < self.bytes.len() {
                    if self.bytes[self.at..].starts_with(b"<<") {
                        depth += 1;
                        self.at += 2;
                    } else if self.bytes[self.at..].starts_with(b">>") {
                        depth -= 1;
                        self.at += 2;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        self.at += 1;
                    }
                }
                Some(ContentUnit::Operand(PdfObject::Null))
            }
            b'<' => {
                let mut cursor = ParsedCursor {
                    bytes: self.bytes,
                    at: self.at,
                };
                let value = cursor.parse_value().ok()?;
                self.at = cursor.at;
                Some(ContentUnit::Operand(value))
            }
            b'(' => {
                let mut cursor = ParsedCursor {
                    bytes: self.bytes,
                    at: self.at,
                };
                let value = cursor.parse_value().ok()?;
                self.at = cursor.at;
                Some(ContentUnit::Operand(value))
            }
            b'[' => {
                let mut cursor = ParsedCursor {
                    bytes: self.bytes,
                    at: self.at,
                };
                let value = cursor.parse_value().ok()?;
                self.at = cursor.at;
                Some(ContentUnit::Operand(value))
            }
            b'/' => {
                let start = self.at + 1;
                self.at += 1;
                while self.at < self.bytes.len() && !is_separator(self.bytes[self.at]) {
                    self.at += 1;
                }
                Some(ContentUnit::Operand(PdfObject::Name(decode_name(
                    &self.bytes[start..self.at],
                ))))
            }
            b'0'..=b'9' | b'+' | b'-' | b'.' => {
                let mut cursor = ParsedCursor {
                    bytes: self.bytes,
                    at: self.at,
                };
                let value = cursor.parse_value().ok()?;
                self.at = cursor.at;
                Some(ContentUnit::Operand(value))
            }
            b'}' | b'{' => {
                self.at += 1;
                Some(ContentUnit::Operand(PdfObject::Null))
            }
            byte if byte.is_ascii_alphabetic() || byte == b'\'' || byte == b'"' || byte == b'*' => {
                let start = self.at;
                if byte.is_ascii_alphabetic() {
                    while self.at < self.bytes.len()
                        && (self.bytes[self.at].is_ascii_alphanumeric()
                            || self.bytes[self.at] == b'*'
                            || self.bytes[self.at] == b'\''
                            || self.bytes[self.at] == b'"')
                    {
                        self.at += 1;
                    }
                } else {
                    self.at += 1;
                }
                Some(ContentUnit::Operator(&self.bytes[start..self.at]))
            }
            _ => {
                self.at += 1;
                Some(ContentUnit::Operand(PdfObject::Null))
            }
        }
    }
}

/// 内容流词法单元。
enum ContentUnit<'a> {
    /// 操作数。
    Operand(PdfObject),
    /// 算子关键字。
    Operator(&'a [u8]),
}

/// 页的字体解码表 (资源名 → 解码器; 同对象只构造一次)。
fn page_font_decoders(
    document: &PdfDocument,
    page: &PdfPage,
) -> Result<HashMap<String, FontDecoder>, DocOpsError> {
    let mut out: HashMap<String, FontDecoder> = HashMap::new();
    let mut cache: HashMap<(u32, u32), FontDecoder> = HashMap::new();
    let Some(resources) = document.dict_resolved(&page.dict, "Resources")? else {
        return Ok(out);
    };
    let Some(resources_dict) = resources.as_dict() else {
        return Ok(out);
    };
    let Some(fonts_object) = document.dict_resolved(resources_dict, "Font")? else {
        return Ok(out);
    };
    let Some(mapping) = fonts_object.as_dict() else {
        return Ok(out);
    };
    for (name, value) in mapping {
        let font_object = document.resolve(value)?;
        let decoder = match font_object {
            PdfObject::Ref(reference) => {
                let key = (reference.num, reference.gen);
                if !cache.contains_key(&key) {
                    cache.insert(key, font_decoder(document, font_object)?);
                }
                cache[&key].clone()
            }
            _ => font_decoder(document, font_object)?,
        };
        out.insert(name.clone(), decoder);
    }
    Ok(out)
}

/// 单页文本提取 (显示算子词表 + 字体映射; 备注如实)。
fn extract_page_text(document: &PdfDocument, page: &PdfPage) -> Result<ExtractedPage, DocOpsError> {
    let content = document.page_content_bytes(page)?;
    let fonts = page_font_decoders(document, page)?;

    let mut out = String::new();
    let mut notes: Vec<String> = Vec::new();
    let mut operands: Vec<PdfObject> = Vec::new();
    let mut current_font: Option<String> = None;
    let mut pending_newline = false;
    let mut cursor = ContentCursor {
        bytes: &content,
        at: 0,
    };
    while let Some(unit) = cursor.next_unit() {
        match unit {
            ContentUnit::Operand(value) => operands.push(value),
            ContentUnit::Operator(operator) => {
                let name = std::str::from_utf8(operator).unwrap_or_default();
                match name {
                    "Tf" => {
                        if let Some(PdfObject::Name(resource)) = operands.first() {
                            current_font = Some(resource.clone());
                        }
                    }
                    "Td" | "TD" | "Tm" | "T*" => pending_newline = true,
                    "Tj" => {
                        if let Some(PdfObject::Str(raw)) = operands.last() {
                            push_shown_text(
                                &fonts,
                                current_font.as_deref(),
                                raw,
                                &mut out,
                                &mut notes,
                                &mut pending_newline,
                            );
                        }
                    }
                    "TJ" => {
                        if let Some(PdfObject::Array(items)) = operands.last() {
                            for item in items {
                                match item {
                                    PdfObject::Str(raw) => push_shown_text(
                                        &fonts,
                                        current_font.as_deref(),
                                        raw,
                                        &mut out,
                                        &mut notes,
                                        &mut pending_newline,
                                    ),
                                    // TJ 数组的大负间隙 = 词距启发 (1000em 单位),
                                    // 行为钉测试锁定阈值 -180。
                                    PdfObject::Real(value) if *value < -180.0 => {
                                        if !out.is_empty() && !out.ends_with(' ') {
                                            out.push(' ');
                                        }
                                    }
                                    PdfObject::Int(value) if *value < -180 => {
                                        if !out.is_empty() && !out.ends_with(' ') {
                                            out.push(' ');
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    "'" | "\"" => {
                        pending_newline = true;
                        if let Some(PdfObject::Str(raw)) = operands.last() {
                            push_shown_text(
                                &fonts,
                                current_font.as_deref(),
                                raw,
                                &mut out,
                                &mut notes,
                                &mut pending_newline,
                            );
                        }
                    }
                    _ => {}
                }
                operands.clear();
            }
        }
    }
    Ok(ExtractedPage { text: out, notes })
}

/// 追加一次显示的文本 (换行待定 + 缺字如实备注)。
fn push_shown_text(
    fonts: &HashMap<String, FontDecoder>,
    current_font: Option<&str>,
    raw: &[u8],
    out: &mut String,
    notes: &mut Vec<String>,
    pending_newline: &mut bool,
) {
    if *pending_newline && !out.is_empty() {
        out.push('\n');
    }
    *pending_newline = false;
    let decoder = current_font.and_then(|name| fonts.get(name));
    match decoder {
        Some(decoder) => {
            let (text, unmapped) = decoder.decode(raw);
            if unmapped > 0 {
                let note = format!(
                    "字体 {} 有 {unmapped} 个码在子集外 (显式 ? 缺字, 不编字形)",
                    current_font.unwrap_or_default()
                );
                if !notes.contains(&note) {
                    notes.push(note);
                }
            }
            out.push_str(&text);
        }
        None => {
            // 无字体资源 (或 Tf 缺席): 按 WinAnsi 单字节如实映射, 控制码显式缺字。
            let mut fallback_table: Box<[u16; 256]> = Box::new([0u16; 256]);
            for (code, slot) in fallback_table.iter_mut().enumerate() {
                *slot = winansi_point(code as u8);
            }
            let fallback = FontDecoder {
                two_byte: false,
                tounicode: None,
                simple: Some(fallback_table),
            };
            let (text, unmapped) = fallback.decode(raw);
            if unmapped > 0 {
                let note = format!("无字体资源, {unmapped} 个码显式缺字 (不编字形)");
                if !notes.contains(&note) {
                    notes.push(note);
                }
            }
            out.push_str(&text);
        }
    }
}

// ---------------------------------------------------------------------------
// PDF 写端 (对象级搬运: 引用闭包原样复制 + 新页树 + 经典 xref)
// ---------------------------------------------------------------------------

/// 写端对象槽 (编号 = 槽位 + 1; `None` = 预留未填)。
#[derive(Default)]
struct PdfWriter {
    /// 对象槽。
    slots: Vec<Option<PdfObject>>,
}

impl PdfWriter {
    /// 预留一个新对象号 (环形引用先占位)。
    fn alloc(&mut self) -> u32 {
        self.slots.push(None);
        self.slots.len() as u32
    }

    /// 填入对象体。
    fn set(&mut self, num: u32, object: PdfObject) {
        if let Some(slot) = self.slots.get_mut(num as usize - 1) {
            *slot = Some(object);
        }
    }

    /// 序列化整份输出 (头部 + 对象 + 经典 xref + trailer)。
    fn finish(&self, root_num: u32) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
        let mut offsets: Vec<usize> = Vec::with_capacity(self.slots.len());
        for (index, slot) in self.slots.iter().enumerate() {
            offsets.push(out.len());
            let num = index as u32 + 1;
            out.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
            if let Some(object) = slot {
                match object {
                    PdfObject::Stream { dict, data } => {
                        let mut with_length = dict.clone();
                        with_length.insert("Length".to_string(), PdfObject::Int(data.len() as i64));
                        out.extend_from_slice(
                            serialize_value(&PdfObject::Dict(with_length)).as_bytes(),
                        );
                        out.extend_from_slice(b"\nstream\n");
                        out.extend_from_slice(data);
                        out.extend_from_slice(b"\nendstream");
                    }
                    other => out.extend_from_slice(serialize_value(other).as_bytes()),
                }
            } else {
                out.extend_from_slice(b"null");
            }
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref_at = out.len();
        let size = self.slots.len() + 1;
        out.extend_from_slice(format!("xref\n0 {size}\n").as_bytes());
        out.extend_from_slice(b"0000000000 65535 f \n");
        for offset in &offsets {
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {size} /Root {root_num} 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
            )
            .as_bytes(),
        );
        out
    }
}

/// 序列化一个 PDF 值 (引用都写新代号 0)。
fn serialize_value(object: &PdfObject) -> String {
    match object {
        PdfObject::Null => "null".to_string(),
        PdfObject::Bool(value) => value.to_string(),
        PdfObject::Int(value) => value.to_string(),
        PdfObject::Real(value) => {
            if value.fract() == 0.0 && value.abs() < 1e15 {
                format!("{}", *value as i64)
            } else {
                format!("{value:.4}")
            }
        }
        PdfObject::Name(name) => {
            let mut out = String::from("/");
            for byte in name.bytes() {
                if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'+' | b'.' | b'*')
                {
                    out.push(byte as char);
                } else {
                    out.push_str(&format!("#{byte:02X}"));
                }
            }
            out
        }
        PdfObject::Str(bytes) => serialize_string(bytes),
        PdfObject::Array(items) => {
            let inner: Vec<String> = items.iter().map(serialize_value).collect();
            format!("[ {} ]", inner.join(" "))
        }
        PdfObject::Dict(dict) => {
            let mut out = String::from("<<");
            for (key, value) in dict {
                out.push(' ');
                out.push_str(&serialize_value(&PdfObject::Name(key.clone())));
                out.push(' ');
                out.push_str(&serialize_value(value));
            }
            out.push_str(" >>");
            out
        }
        PdfObject::Stream { dict, data } => {
            let mut with_length = dict.clone();
            with_length.insert("Length".to_string(), PdfObject::Int(data.len() as i64));
            format!(
                "{}\nstream\n{}\nendstream",
                serialize_value(&PdfObject::Dict(with_length)),
                String::from_utf8_lossy(data)
            )
        }
        PdfObject::Ref(reference) => format!("{} {} R", reference.num, reference.gen),
    }
}

/// 字符串序列化: 可打印 ASCII 走字面串, 其余走十六进制串 (无损)。
fn serialize_string(bytes: &[u8]) -> String {
    let printable = bytes
        .iter()
        .all(|byte| (0x20..0x7F).contains(byte) && !matches!(byte, b'(' | b')' | b'\\'));
    if printable {
        let mut out = String::from("(");
        for byte in bytes {
            out.push(*byte as char);
        }
        out.push(')');
        out
    } else {
        let mut out = String::from("<");
        for byte in bytes {
            out.push_str(&format!("{byte:02X}"));
        }
        out.push('>');
        out
    }
}

/// 一个待写页 (来源文档 + 页)。
struct PageSource<'a> {
    /// 来源文档。
    doc: &'a PdfDocument,
    /// 来源页。
    page: &'a PdfPage,
}

/// 搬运一个引用闭包 (递归复制对象, 环形引用先占位)。
fn copy_closure(
    document: &PdfDocument,
    writer: &mut PdfWriter,
    memo: &mut HashMap<(u32, u32), u32>,
    reference: PdfRef,
    depth: u32,
) -> Result<u32, DocOpsError> {
    if depth > RESOLVE_DEPTH_CAP {
        return Err(DocOpsError::Parse("对象引用闭包嵌套超深".to_string()));
    }
    if let Some(existing) = memo.get(&(reference.num, reference.gen)) {
        return Ok(*existing);
    }
    let object = document.get(reference)?.clone();
    let new_num = writer.alloc();
    memo.insert((reference.num, reference.gen), new_num);
    let copied = copy_value(document, writer, memo, &object, depth + 1)?;
    writer.set(new_num, copied);
    Ok(new_num)
}

/// 搬运一个值 (内部引用换成新号)。
fn copy_value(
    document: &PdfDocument,
    writer: &mut PdfWriter,
    memo: &mut HashMap<(u32, u32), u32>,
    object: &PdfObject,
    depth: u32,
) -> Result<PdfObject, DocOpsError> {
    Ok(match object {
        PdfObject::Ref(reference) => PdfObject::Ref(PdfRef {
            num: copy_closure(document, writer, memo, *reference, depth + 1)?,
            gen: 0,
        }),
        PdfObject::Array(items) => {
            let mut copied = Vec::with_capacity(items.len());
            for item in items {
                copied.push(copy_value(document, writer, memo, item, depth + 1)?);
            }
            PdfObject::Array(copied)
        }
        PdfObject::Dict(dict) => {
            let mut copied = BTreeMap::new();
            for (key, value) in dict {
                copied.insert(
                    key.clone(),
                    copy_value(document, writer, memo, value, depth + 1)?,
                );
            }
            PdfObject::Dict(copied)
        }
        PdfObject::Stream { dict, data } => {
            let mut copied = BTreeMap::new();
            for (key, value) in dict {
                copied.insert(
                    key.clone(),
                    copy_value(document, writer, memo, value, depth + 1)?,
                );
            }
            PdfObject::Stream {
                dict: copied,
                data: data.clone(),
            }
        }
        other => other.clone(),
    })
}

/// 生成新 PDF: 指定页序列 (+ 可选水印) → 新页树 + 对象闭包搬运。
fn write_pdf_with_pages(
    pages: &[PageSource<'_>],
    watermark_text: Option<&str>,
) -> Result<Vec<u8>, DocOpsError> {
    let mut writer = PdfWriter::default();
    let pages_num = writer.alloc();
    let catalog_num = writer.alloc();
    let watermark_font_num = watermark_text.map(|_| writer.alloc());

    let mut kids: Vec<PdfObject> = Vec::with_capacity(pages.len());
    // 每个来源文档独立 memo: 闭包不跨文档, 对象号冲突天然隔离。
    let mut memos: HashMap<usize, HashMap<(u32, u32), u32>> = HashMap::new();
    for (index, source) in pages.iter().enumerate() {
        let memo = memos.entry(index).or_default();
        let parent = PdfRef {
            num: pages_num,
            gen: 0,
        };
        let page_num = copy_page_object(
            source.doc,
            source.page,
            &mut writer,
            memo,
            parent,
            watermark_text,
            watermark_font_num.map(|num| PdfRef { num, gen: 0 }),
        )?;
        kids.push(PdfObject::Ref(PdfRef {
            num: page_num,
            gen: 0,
        }));
    }

    let mut pages_dict = BTreeMap::new();
    pages_dict.insert("Type".to_string(), PdfObject::Name("Pages".to_string()));
    pages_dict.insert("Kids".to_string(), PdfObject::Array(kids));
    pages_dict.insert("Count".to_string(), PdfObject::Int(pages.len() as i64));
    writer.set(pages_num, PdfObject::Dict(pages_dict));

    let mut catalog_dict = BTreeMap::new();
    catalog_dict.insert("Type".to_string(), PdfObject::Name("Catalog".to_string()));
    catalog_dict.insert(
        "Pages".to_string(),
        PdfObject::Ref(PdfRef {
            num: pages_num,
            gen: 0,
        }),
    );
    writer.set(catalog_num, PdfObject::Dict(catalog_dict));

    if let (Some(_text), Some(font_num)) = (watermark_text, watermark_font_num) {
        let mut font_dict = BTreeMap::new();
        font_dict.insert("Type".to_string(), PdfObject::Name("Font".to_string()));
        font_dict.insert("Subtype".to_string(), PdfObject::Name("Type1".to_string()));
        font_dict.insert(
            "BaseFont".to_string(),
            PdfObject::Name("Helvetica".to_string()),
        );
        font_dict.insert(
            "Encoding".to_string(),
            PdfObject::Name("WinAnsiEncoding".to_string()),
        );
        writer.set(font_num, PdfObject::Dict(font_dict));
    }

    Ok(writer.finish(catalog_num))
}

/// 搬运一页对象 (页字典手工展开: /Parent 换新树, /Contents 水印可拼)。
fn copy_page_object(
    document: &PdfDocument,
    page: &PdfPage,
    writer: &mut PdfWriter,
    memo: &mut HashMap<(u32, u32), u32>,
    parent: PdfRef,
    watermark_text: Option<&str>,
    watermark_font_ref: Option<PdfRef>,
) -> Result<u32, DocOpsError> {
    let page_num = writer.alloc();
    let mut copied: BTreeMap<String, PdfObject> = BTreeMap::new();
    for (key, value) in &page.dict {
        match key.as_str() {
            "Parent" => {}
            "Contents" if watermark_text.is_some() => {}
            _ => {
                copied.insert(key.clone(), copy_value(document, writer, memo, value, 0)?);
            }
        }
    }
    copied.insert("Parent".to_string(), PdfObject::Ref(parent));

    if let Some(text) = watermark_text {
        let font_ref = watermark_font_ref.ok_or_else(|| {
            DocOpsError::InvalidParam("水印字体对象缺位 (内部装配错误)".to_string())
        })?;
        // /Resources 物化 + 合并水印字体 (不覆盖原字体表)。
        let mut resources: BTreeMap<String, PdfObject> = document
            .dict_resolved(&page.dict, "Resources")?
            .and_then(PdfObject::as_dict)
            .cloned()
            .unwrap_or_default();
        let mut font_table: BTreeMap<String, PdfObject> = match resources.get("Font") {
            Some(value) => document
                .resolve(value)?
                .as_dict()
                .cloned()
                .unwrap_or_default(),
            None => BTreeMap::new(),
        };
        font_table.insert("F_ApeirethWm".to_string(), PdfObject::Ref(font_ref));
        resources.insert("Font".to_string(), PdfObject::Dict(font_table));
        let mut copied_resources = BTreeMap::new();
        for (key, value) in resources {
            copied_resources.insert(key, copy_value(document, writer, memo, &value, 0)?);
        }
        copied.insert("Resources".to_string(), PdfObject::Dict(copied_resources));

        // /Contents: 原内容流引用闭包 + 新水印流 (数组拼接, 不动原流)。
        let mut contents: Vec<PdfObject> = Vec::new();
        if let Some(existing) = page.dict.get("Contents") {
            let resolved = document.resolve(existing)?;
            match resolved {
                PdfObject::Array(items) => {
                    for item in items {
                        contents.push(PdfObject::Ref(PdfRef {
                            num: copy_closure(
                                document,
                                writer,
                                memo,
                                item.as_ref_id().ok_or_else(|| {
                                    DocOpsError::Parse("页 /Contents 数组项不是引用".to_string())
                                })?,
                                0,
                            )?,
                            gen: 0,
                        }));
                    }
                }
                PdfObject::Stream { .. } => {
                    let reference = existing.as_ref_id().ok_or_else(|| {
                        DocOpsError::Parse("页 /Contents 流不是间接引用".to_string())
                    })?;
                    contents.push(PdfObject::Ref(PdfRef {
                        num: copy_closure(document, writer, memo, reference, 0)?,
                        gen: 0,
                    }));
                }
                _ => {}
            }
        }
        let watermark_stream = PdfObject::Stream {
            dict: BTreeMap::new(),
            data: watermark_content(text)?,
        };
        let watermark_num = writer.alloc();
        writer.set(watermark_num, watermark_stream);
        contents.push(PdfObject::Ref(PdfRef {
            num: watermark_num,
            gen: 0,
        }));
        copied.insert("Contents".to_string(), PdfObject::Array(contents));
    }

    writer.set(page_num, PdfObject::Dict(copied));
    Ok(page_num)
}

/// 水印内容流 (对角文字 + 灰阶; 文本须 WinAnsi 可编码, 子集外拒绝即帧)。
fn watermark_content(text: &str) -> Result<Vec<u8>, DocOpsError> {
    let mut encoded: Vec<u8> = Vec::with_capacity(text.len());
    for character in text.chars() {
        let point = character as u32;
        if point == 0x20 || (0x21..=0x7E).contains(&point) || (0xA0..=0xFF).contains(&point) {
            encoded.push(point as u8);
        } else {
            return Err(DocOpsError::Unsupported(format!(
                "水印文字含非 WinAnsi 可编码字符 {character:?} (本批水印子集只认 WinAnsi 文本)"
            )));
        }
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"q\n0.55 g\nBT\n/F_ApeirethWm 48 Tf\n");
    out.extend_from_slice(b"0.7071 0.7071 -0.7071 0.7071 120 360 Tm\n");
    out.push(b'(');
    for byte in encoded {
        match byte {
            b'(' | b')' | b'\\' => {
                out.push(b'\\');
                out.push(byte);
            }
            other => out.push(other),
        }
    }
    out.extend_from_slice(b") Tj\nET\nQ\n");
    Ok(out)
}

// ---------------------------------------------------------------------------
// 操作实现 (批量管道: 多入一出 / 一入多出 / 多页选页)
// ---------------------------------------------------------------------------

/// 调用参数 (未知字段拒绝)。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocOpsParams {
    /// 动作 (闭合词表)。
    pub action: Option<String>,
    /// 单输入 (读面四动作 / split / extract_pages / watermark_text / docx_append)。
    pub input: Option<String>,
    /// 多输入 (`pdf_merge`)。
    pub inputs: Option<Vec<String>>,
    /// 单输出文件 (merge / extract_pages / watermark_text / docx_from_markdown /
    /// docx_append)。
    pub output: Option<String>,
    /// 输出目录 (`pdf_split`)。
    pub output_dir: Option<String>,
    /// 选页 (1 起页码, `pdf_extract_pages`)。
    pub pages: Option<Vec<u64>>,
    /// 水印文字 (WinAnsi 可编码, `pdf_watermark_text`)。
    pub watermark_text: Option<String>,
    /// 提取字符预算 (≤ 配置上限, 读面两提取动作)。
    pub max_chars: Option<u64>,
    /// 源文 (`docx_from_markdown`: Markdown 源文; `docx_append`: 追加段落文,
    /// 逐行一段)。
    pub text: Option<String>,
}

/// 参数矩阵校验 (纯判定, 工具面与边界 Guard 同一真源): 返回 `Some(拒绝错误)` 即拒。
///
/// 逐项: ① 动作在闭合词表; ② 每动作的必填/禁填字段矩阵; ③ 选页合法
/// (≥1 / 无重复 / 不超上限); ④ 路径工作区限域 + 敏感面拒; ⑤ 输入大小上限;
/// ⑥ 输出文件只创建不覆盖。错误按真因分类 (敏感面/限域/上限各有其帧)。
pub fn doc_ops_params_refusal(config: &DocOpsConfig, params: &DocOpsParams) -> Option<DocOpsError> {
    let Some(action) = params.action.as_deref() else {
        return Some(DocOpsError::InvalidParam(format!(
            "必须显式给 action (闭合词表: {})",
            DOC_OPS_ACTIONS.join(" / ")
        )));
    };
    if !DOC_OPS_ACTIONS.contains(&action) {
        return Some(DocOpsError::InvalidParam(format!(
            "未知动作 {action:?}; 闭合词表是 {}",
            DOC_OPS_ACTIONS.join(" / ")
        )));
    }

    let require_text = |value: Option<&String>, field: &str| -> Option<DocOpsError> {
        match value.map(|value| value.trim()) {
            Some(text) if !text.is_empty() => None,
            _ => Some(DocOpsError::InvalidParam(format!(
                "{action} 必须给非空 {field}"
            ))),
        }
    };

    match action {
        ACTION_PDF_INFO
        | ACTION_PDF_EXTRACT_TEXT
        | ACTION_DOCX_EXTRACT_TEXT
        | ACTION_DOCX_OUTLINE => {
            if let Some(refusal) = require_text(params.input.as_ref(), "input") {
                return Some(refusal);
            }
        }
        ACTION_PDF_MERGE => {
            let inputs = params.inputs.as_deref().unwrap_or_default();
            if !(2..=32).contains(&inputs.len()) {
                return Some(DocOpsError::InvalidParam(format!(
                    "pdf_merge 必须给 2-32 个 inputs (多入一出), 实给 {}",
                    inputs.len()
                )));
            }
            for input in inputs {
                if input.trim().is_empty() {
                    return Some(DocOpsError::InvalidParam(
                        "pdf_merge 的 inputs 含空路径".to_string(),
                    ));
                }
            }
            if let Some(refusal) = require_text(params.output.as_ref(), "output") {
                return Some(refusal);
            }
        }
        ACTION_PDF_SPLIT => {
            if let Some(refusal) = require_text(params.input.as_ref(), "input") {
                return Some(refusal);
            }
            if let Some(refusal) = require_text(params.output_dir.as_ref(), "output_dir") {
                return Some(refusal);
            }
        }
        ACTION_PDF_EXTRACT_PAGES => {
            if let Some(refusal) = require_text(params.input.as_ref(), "input") {
                return Some(refusal);
            }
            if let Some(refusal) = require_text(params.output.as_ref(), "output") {
                return Some(refusal);
            }
            let pages = params.pages.as_deref().unwrap_or_default();
            if pages.is_empty() {
                return Some(DocOpsError::InvalidParam(
                    "pdf_extract_pages 必须给 pages (1 起页码列表, 非空)".to_string(),
                ));
            }
            if pages.len() as u64 > config.max_pages {
                return Some(DocOpsError::TooManyPages {
                    pages: pages.len() as u64,
                    cap: config.max_pages,
                });
            }
            let mut seen = std::collections::BTreeSet::new();
            for page in pages {
                if *page == 0 {
                    return Some(DocOpsError::InvalidParam(
                        "pages 页码从 1 起 (0 非法)".to_string(),
                    ));
                }
                if !seen.insert(*page) {
                    return Some(DocOpsError::InvalidParam(format!(
                        "pages 含重复页码 {page}"
                    )));
                }
            }
        }
        ACTION_PDF_WATERMARK_TEXT => {
            if let Some(refusal) = require_text(params.input.as_ref(), "input") {
                return Some(refusal);
            }
            if let Some(refusal) = require_text(params.output.as_ref(), "output") {
                return Some(refusal);
            }
            if let Some(refusal) = require_text(params.watermark_text.as_ref(), "watermark_text") {
                return Some(refusal);
            }
        }
        ACTION_DOCX_FROM_MARKDOWN => {
            if let Some(refusal) = require_text(params.text.as_ref(), "text") {
                return Some(refusal);
            }
            if let Some(refusal) = require_text(params.output.as_ref(), "output") {
                return Some(refusal);
            }
        }
        ACTION_DOCX_APPEND => {
            if let Some(refusal) = require_text(params.input.as_ref(), "input") {
                return Some(refusal);
            }
            if let Some(refusal) = require_text(params.text.as_ref(), "text") {
                return Some(refusal);
            }
            if let Some(refusal) = require_text(params.output.as_ref(), "output") {
                return Some(refusal);
            }
        }
        _ => {}
    }

    // 源文面 (DOCX 写面两动作): 字符 / 段落上限宁拒不截; XML 非法控制字符
    // 拒绝即帧 (不静默剔除不猜)。
    if matches!(action, ACTION_DOCX_FROM_MARKDOWN | ACTION_DOCX_APPEND) {
        let source = params.text.as_deref().unwrap_or_default();
        let chars = source.chars().count();
        if chars > DOC_OPS_MAX_SOURCE_CHARS {
            return Some(DocOpsError::InvalidParam(format!(
                "{action}: 源文 {chars} 字符超过 {DOC_OPS_MAX_SOURCE_CHARS} 字符上限 \
                 (宁拒不截, 不截断生成)"
            )));
        }
        // 段落口径钉死: 非空行 = 一段 (与解析面逐行一段同一把尺)。
        let paragraphs = source
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count();
        if paragraphs > DOC_OPS_MAX_SOURCE_PARAGRAPHS {
            return Some(DocOpsError::InvalidParam(format!(
                "{action}: 源文 {paragraphs} 段超过 {DOC_OPS_MAX_SOURCE_PARAGRAPHS} 段上限 \
                 (宁拒不截)"
            )));
        }
        if source.chars().any(is_xml_illegal_char) {
            return Some(DocOpsError::InvalidParam(format!(
                "{action}: 源文含 XML 非法控制字符 (拒绝即帧, 不静默剔除不猜)"
            )));
        }
    }

    // 写面写总闸 (宪法级 `file_write` 唯一写总闸, 组装根注入): 关 = 写动作
    // 拒绝即帧, 读面不受影响 (与写族工具同一把闸)。
    if is_doc_ops_write_action(action) && !config.write_gate_open {
        return Some(DocOpsError::WriteGateClosed(format!(
            "总闸未开, 写动作 {action} 拒绝即帧 (读面不受影响)"
        )));
    }

    // 路径面: 输入限域 + 敏感面 + 大小上限; 输出文件限域 + 敏感面 + 不覆盖预检
    // (输出目录允许已存在, 只做限域 + 敏感面)。
    let mut inputs: Vec<&String> = Vec::new();
    if let Some(input) = params.input.as_ref() {
        inputs.push(input);
    }
    if let Some(list) = params.inputs.as_deref() {
        inputs.extend(list.iter());
    }
    for input in inputs {
        match resolve_contained(&config.workspace_root, input, true) {
            Ok(path) => {
                let bytes = std::fs::metadata(&path)
                    .map(|meta| meta.len())
                    .unwrap_or_default();
                if bytes > config.max_input_bytes {
                    return Some(DocOpsError::InputTooLarge {
                        path: input.clone(),
                        bytes,
                        cap: config.max_input_bytes,
                    });
                }
            }
            Err(error) => return Some(error),
        }
    }
    if let Some(output) = params.output.as_ref() {
        match resolve_contained(&config.workspace_root, output, false) {
            Ok(path) => {
                if path.exists() {
                    return Some(DocOpsError::OutputExists(output.clone()));
                }
            }
            Err(error) => return Some(error),
        }
    }
    if let Some(output_dir) = params.output_dir.as_ref() {
        if let Err(error) = resolve_contained(&config.workspace_root, output_dir, false) {
            return Some(error);
        }
    }
    None
}

/// 页数上限预检 (解析后按真页数判)。
fn check_page_cap(config: &DocOpsConfig, pages: u64) -> Result<(), DocOpsError> {
    if pages > config.max_pages {
        return Err(DocOpsError::TooManyPages {
            pages,
            cap: config.max_pages,
        });
    }
    Ok(())
}

/// 提取字符预算 (参数可收紧, 配置是上限)。
fn text_budget(config: &DocOpsConfig, requested: Option<u64>) -> usize {
    requested
        .map(|value| value.min(config.max_text_chars as u64) as usize)
        .unwrap_or(config.max_text_chars)
}

/// 输入文件名主干 (输出命名用; 非法字符归 `_`)。
fn file_stem(requested: &str) -> String {
    let name = Path::new(requested)
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| "document".to_string());
    let cleaned: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "document".to_string()
    } else {
        cleaned
    }
}

/// 文档操作执行面 (受控读写族的全部行为)。
pub struct DocOpsRunner {
    /// 生效配置 (工作区根 + 三上限)。
    config: DocOpsConfig,
    /// 进程内计数 (自述创作节真值源)。
    counters: Arc<DocOpsCounters>,
}

impl DocOpsRunner {
    /// 绑定配置 (独立计数器)。
    pub fn new(config: DocOpsConfig) -> Self {
        Self {
            config,
            counters: DocOpsCounters::new(),
        }
    }

    /// 绑定配置 + 共享计数器 (工具面与自述节同一实例)。
    pub fn with_counters(config: DocOpsConfig, counters: Arc<DocOpsCounters>) -> Self {
        Self { config, counters }
    }

    /// 生效配置。
    pub fn config(&self) -> &DocOpsConfig {
        &self.config
    }

    /// 进程内计数 (自述创作节同源)。
    pub fn counters(&self) -> &Arc<DocOpsCounters> {
        &self.counters
    }

    /// 执行一个动作 (参数矩阵先拒, 真因即帧; 计数如实)。
    pub fn run(&self, params: &DocOpsParams) -> Result<Value, DocOpsError> {
        self.counters.record_call();
        let outcome = self.run_inner(params);
        match &outcome {
            Ok(frame) => {
                let action = params.action.as_deref().unwrap_or_default();
                self.counters
                    .record_success(action, frame_pages_touched(frame), now_epoch_ms());
            }
            Err(_) => self.counters.record_denied(),
        }
        outcome
    }

    /// 执行内核 (无计数副作用)。
    fn run_inner(&self, params: &DocOpsParams) -> Result<Value, DocOpsError> {
        if let Some(refusal) = doc_ops_params_refusal(&self.config, params) {
            return Err(refusal);
        }
        let action = params.action.as_deref().unwrap_or_default();
        match action {
            ACTION_PDF_INFO => self.pdf_info(params),
            ACTION_PDF_EXTRACT_TEXT => self.pdf_extract_text(params),
            ACTION_DOCX_EXTRACT_TEXT => self.docx_extract_text(params),
            ACTION_DOCX_OUTLINE => self.docx_outline(params),
            ACTION_PDF_MERGE => self.pdf_merge(params),
            ACTION_PDF_SPLIT => self.pdf_split(params),
            ACTION_PDF_EXTRACT_PAGES => self.pdf_extract_pages(params),
            ACTION_PDF_WATERMARK_TEXT => self.pdf_watermark_text(params),
            ACTION_DOCX_FROM_MARKDOWN => self.docx_from_markdown(params),
            ACTION_DOCX_APPEND => self.docx_append(params),
            other => Err(DocOpsError::InvalidParam(format!(
                "未知动作 {other:?} (闭合词表外)"
            ))),
        }
    }

    /// 解析一份受控 PDF 输入。
    fn load_pdf(
        &self,
        requested: &str,
    ) -> Result<(String, PdfDocument, Vec<PdfPage>), DocOpsError> {
        let (_path, bytes) = read_input(&self.config, requested)?;
        let document = parse_pdf(&bytes)?;
        let pages = document.collect_pages()?;
        check_page_cap(&self.config, pages.len() as u64)?;
        Ok((requested.to_string(), document, pages))
    }

    /// `pdf_info`: 页数 / 版本 / 逐页尺寸 (真值)。
    fn pdf_info(&self, params: &DocOpsParams) -> Result<Value, DocOpsError> {
        let requested = params.input.as_deref().unwrap_or_default();
        let (input, document, pages) = self.load_pdf(requested)?;
        let mut sizes: Vec<Value> = Vec::with_capacity(pages.len());
        let mut notes: Vec<String> = Vec::new();
        for (index, page) in pages.iter().enumerate() {
            match page_box(&document, page) {
                Ok((width, height)) => sizes.push(json!([width, height])),
                Err(reason) => {
                    sizes.push(Value::Null);
                    notes.push(format!("第 {} 页尺寸未报: {reason}", index + 1));
                }
            }
        }
        Ok(json!({
            "action": ACTION_PDF_INFO,
            "status": "ok",
            "input": input,
            "version": document.version,
            "pages": pages.len(),
            "page_sizes": sizes,
            "notes": notes,
        }))
    }

    /// `pdf_extract_text`: 逐页文本 (字符预算, 超限显式 `truncated`)。
    fn pdf_extract_text(&self, params: &DocOpsParams) -> Result<Value, DocOpsError> {
        let requested = params.input.as_deref().unwrap_or_default();
        let (input, document, pages) = self.load_pdf(requested)?;
        let budget = text_budget(&self.config, params.max_chars);
        let mut rendered: Vec<Value> = Vec::new();
        let mut notes: Vec<String> = Vec::new();
        let mut chars = 0usize;
        let mut truncated = false;
        for (index, page) in pages.iter().enumerate() {
            let extracted = extract_page_text(&document, page)?;
            let length = extracted.text.chars().count();
            if chars + length > budget {
                truncated = true;
                notes.push(format!(
                    "字符预算 {budget} 用尽: 第 {} 页起共 {} 页未返回 (显式 truncated, 不编文)",
                    index + 1,
                    pages.len() - index
                ));
                break;
            }
            chars += length;
            for note in extracted.notes {
                let full = format!("第 {} 页: {note}", index + 1);
                if !notes.contains(&full) {
                    notes.push(full);
                }
            }
            rendered.push(json!({"page": index + 1, "text": extracted.text}));
        }
        Ok(json!({
            "action": ACTION_PDF_EXTRACT_TEXT,
            "status": "ok",
            "input": input,
            "pages": rendered,
            "chars": chars,
            "truncated": truncated,
            "notes": notes,
        }))
    }

    /// `docx_extract_text`: 正文 → Markdown 转出。
    fn docx_extract_text(&self, params: &DocOpsParams) -> Result<Value, DocOpsError> {
        let requested = params.input.as_deref().unwrap_or_default();
        let (_path, bytes) = read_input(&self.config, requested)?;
        let paragraphs = docx_parse_paragraphs(&bytes, zip_decompression_budget(&self.config))?;
        let markdown = docx_paragraphs_to_markdown(&paragraphs);
        let budget = text_budget(&self.config, params.max_chars);
        let mut truncated = false;
        let mut notes: Vec<String> = Vec::new();
        let full_chars = markdown.chars().count();
        let markdown = if full_chars > budget {
            truncated = true;
            notes.push(format!(
                "字符预算 {budget} 用尽: Markdown 转出截断于预算处 (显式 truncated, 不编文)"
            ));
            markdown.chars().take(budget).collect()
        } else {
            markdown
        };
        let chars = markdown.chars().count();
        Ok(json!({
            "action": ACTION_DOCX_EXTRACT_TEXT,
            "status": "ok",
            "input": requested,
            "markdown": markdown,
            "chars": chars,
            "paragraphs": paragraphs.len(),
            "truncated": truncated,
            "notes": notes,
        }))
    }

    /// `docx_outline`: 标题层级大纲。
    fn docx_outline(&self, params: &DocOpsParams) -> Result<Value, DocOpsError> {
        let requested = params.input.as_deref().unwrap_or_default();
        let (_path, bytes) = read_input(&self.config, requested)?;
        let paragraphs = docx_parse_paragraphs(&bytes, zip_decompression_budget(&self.config))?;
        let outline = docx_paragraphs_to_outline(&paragraphs);
        let entries: Vec<Value> = outline
            .iter()
            .map(|(level, text)| json!({"level": level, "text": text}))
            .collect();
        Ok(json!({
            "action": ACTION_DOCX_OUTLINE,
            "status": "ok",
            "input": requested,
            "outline": entries,
            "headings": outline.len(),
        }))
    }

    /// `docx_from_markdown`: Markdown → DOCX 最小生成 (结构面; 样式层最小如实)。
    fn docx_from_markdown(&self, params: &DocOpsParams) -> Result<Value, DocOpsError> {
        let text = params.text.as_deref().unwrap_or_default();
        let output = params.output.as_deref().unwrap_or_default();
        let blocks = parse_markdown_blocks(text)?;
        let document_xml = docx_document_xml(&blocks);
        let bytes = docx_package(&document_xml)?;
        let path = resolve_contained(&self.config.workspace_root, output, false)?;
        write_output_create_only(&path, &bytes)?;
        Ok(json!({
            "action": ACTION_DOCX_FROM_MARKDOWN,
            "status": "ok",
            "output": output,
            "paragraphs": blocks.len(),
            "bytes_written": bytes.len() as u64,
            "notes": [
                "样式层与编号层最小如实: 标题以 HeadingN 引用同源样式表 (Normal/\
                 Heading1..6/ListParagraph/粗斜字符样式); 列表以 numPr numId=1 引用 \
                 numbering.xml 的一级项目符号 (左缩进 720 + 悬挂 360 twips); 无主题、\
                 无字体美化; 有序列表当前同样渲染为项目符号 (解析层无 ordered/bullet \
                 分型, 读回归一 `- `), 不冒充编号支持"
            ],
        }))
    }

    /// `docx_append`: 副本追加段落 (逐行一段, 空行忽略)。
    ///
    /// 只动 `word/document.xml` (body 尾部插入, 节属性前让位), 其余容器条目
    /// **内容**原样搬动 (重组容器压缩档位统一 deflate, 内容字节零改动);
    /// 只创建不覆盖 (创建类原子写, 与写族同一门禁)。
    fn docx_append(&self, params: &DocOpsParams) -> Result<Value, DocOpsError> {
        let input = params.input.as_deref().unwrap_or_default();
        let output = params.output.as_deref().unwrap_or_default();
        let text = params.text.as_deref().unwrap_or_default();
        let lines = source_paragraph_lines(text);
        let (_path, bytes) = read_input(&self.config, input)?;
        let mut entries = zip_read_entries(&bytes, zip_decompression_budget(&self.config))?;
        let Some(document) = entries
            .iter_mut()
            .find(|entry| entry.name == "word/document.xml")
        else {
            return Err(DocOpsError::Parse(
                "DOCX 容器缺 word/document.xml (不是文字处理文档)".to_string(),
            ));
        };
        let updated = {
            let original_xml = std::str::from_utf8(&document.data).map_err(|_| {
                DocOpsError::Unsupported("word/document.xml 不是 UTF-8".to_string())
            })?;
            docx_append_paragraphs(original_xml, &lines)?
        };
        document.data = updated.into_bytes();
        let packed = zip_write_entries(&entries, ZipCompression::Deflate)?;
        let path = resolve_contained(&self.config.workspace_root, output, false)?;
        write_output_create_only(&path, &packed)?;
        Ok(json!({
            "action": ACTION_DOCX_APPEND,
            "status": "ok",
            "input": input,
            "output": output,
            "paragraphs_appended": lines.len(),
            "bytes_written": packed.len() as u64,
            "notes": [
                "只动 word/document.xml: 其余容器条目内容原样搬动 (重组统一 deflate 档, \
                 内容字节零改动)"
            ],
        }))
    }

    /// `pdf_merge`: 多入一出 (依序合并页)。
    fn pdf_merge(&self, params: &DocOpsParams) -> Result<Value, DocOpsError> {
        let inputs = params.inputs.as_deref().unwrap_or_default();
        let output = params.output.as_deref().unwrap_or_default();
        let mut sources: Vec<(String, PdfDocument, Vec<PdfPage>)> = Vec::new();
        for input in inputs {
            sources.push(self.load_pdf(input)?);
        }
        let total: u64 = sources.iter().map(|(_, _, pages)| pages.len() as u64).sum();
        check_page_cap(&self.config, total)?;
        let mut page_sources: Vec<PageSource<'_>> = Vec::new();
        for (index, source) in sources.iter().enumerate() {
            for page in &source.2 {
                page_sources.push(PageSource {
                    doc: &sources[index].1,
                    page,
                });
            }
        }
        let bytes = write_pdf_with_pages(&page_sources, None)?;
        let path = resolve_contained(&self.config.workspace_root, output, false)?;
        write_output_create_only(&path, &bytes)?;
        Ok(json!({
            "action": ACTION_PDF_MERGE,
            "status": "ok",
            "inputs": inputs,
            "output": output,
            "pages": total,
        }))
    }

    /// `pdf_split`: 一入多出 (逐页独立 PDF, 批量管道; 先全量预检再落盘)。
    fn pdf_split(&self, params: &DocOpsParams) -> Result<Value, DocOpsError> {
        let requested = params.input.as_deref().unwrap_or_default();
        let output_dir = params.output_dir.as_deref().unwrap_or_default();
        let (input, document, pages) = self.load_pdf(requested)?;
        let dir = resolve_contained(&self.config.workspace_root, output_dir, false)?;
        if dir.exists() && !dir.is_dir() {
            return Err(DocOpsError::InvalidParam(format!(
                "输出目录位置已有同名文件: {output_dir}"
            )));
        }
        let stem = file_stem(requested);
        let mut targets: Vec<(usize, PathBuf, String)> = Vec::with_capacity(pages.len());
        for index in 0..pages.len() {
            let name = format!("{stem}-page-{:04}.pdf", index + 1);
            let target = dir.join(&name);
            if target.exists() {
                return Err(DocOpsError::OutputExists(target.display().to_string()));
            }
            targets.push((index, target, name));
        }
        if !dir.exists() {
            std::fs::create_dir_all(&dir)
                .map_err(|error| DocOpsError::Io(format!("{}: {error}", dir.display())))?;
        }
        let mut files: Vec<String> = Vec::with_capacity(targets.len());
        for (index, target, name) in targets {
            let bytes = write_pdf_with_pages(
                &[PageSource {
                    doc: &document,
                    page: &pages[index],
                }],
                None,
            )?;
            write_output_create_only(&target, &bytes)?;
            files.push(name);
        }
        Ok(json!({
            "action": ACTION_PDF_SPLIT,
            "status": "ok",
            "input": input,
            "output_dir": output_dir,
            "pages": pages.len(),
            "files": files,
        }))
    }

    /// `pdf_extract_pages`: 多页选页 → 单一 PDF。
    fn pdf_extract_pages(&self, params: &DocOpsParams) -> Result<Value, DocOpsError> {
        let requested = params.input.as_deref().unwrap_or_default();
        let output = params.output.as_deref().unwrap_or_default();
        let (input, document, pages) = self.load_pdf(requested)?;
        let selected = params.pages.as_deref().unwrap_or_default();
        let mut page_sources: Vec<PageSource<'_>> = Vec::with_capacity(selected.len());
        for number in selected {
            let index = usize::try_from(*number - 1)
                .map_err(|_| DocOpsError::InvalidParam(format!("页码 {number} 超出可寻址范围")))?;
            let Some(page) = pages.get(index) else {
                return Err(DocOpsError::InvalidParam(format!(
                    "页码 {number} 超出页数范围 1..={}",
                    pages.len()
                )));
            };
            page_sources.push(PageSource {
                doc: &document,
                page,
            });
        }
        let bytes = write_pdf_with_pages(&page_sources, None)?;
        let path = resolve_contained(&self.config.workspace_root, output, false)?;
        write_output_create_only(&path, &bytes)?;
        Ok(json!({
            "action": ACTION_PDF_EXTRACT_PAGES,
            "status": "ok",
            "input": input,
            "output": output,
            "pages_total": pages.len(),
            "pages": selected,
        }))
    }

    /// `pdf_watermark_text`: 逐页对角水印 (WinAnsi 可编码文字)。
    fn pdf_watermark_text(&self, params: &DocOpsParams) -> Result<Value, DocOpsError> {
        let requested = params.input.as_deref().unwrap_or_default();
        let output = params.output.as_deref().unwrap_or_default();
        let text = params.watermark_text.as_deref().unwrap_or_default();
        let (input, document, pages) = self.load_pdf(requested)?;
        let page_sources: Vec<PageSource<'_>> = pages
            .iter()
            .map(|page| PageSource {
                doc: &document,
                page,
            })
            .collect();
        let bytes = write_pdf_with_pages(&page_sources, Some(text))?;
        let path = resolve_contained(&self.config.workspace_root, output, false)?;
        write_output_create_only(&path, &bytes)?;
        Ok(json!({
            "action": ACTION_PDF_WATERMARK_TEXT,
            "status": "ok",
            "input": input,
            "output": output,
            "pages": pages.len(),
            "watermark": text,
        }))
    }
}

/// 页尺寸 (MediaBox 真值; 缺失如实报因)。
fn page_box(document: &PdfDocument, page: &PdfPage) -> Result<(f64, f64), String> {
    let Some(media) = page.dict.get("MediaBox") else {
        return Err("MediaBox 缺失 (含继承)".to_string());
    };
    let resolved = document.resolve(media).map_err(|error| error.message())?;
    let Some(items) = resolved.as_array() else {
        return Err("MediaBox 不是数组".to_string());
    };
    if items.len() != 4 {
        return Err(format!("MediaBox 有 {} 项 (应为 4)", items.len()));
    }
    let values: Vec<f64> = items
        .iter()
        .map(|item| match item {
            PdfObject::Int(value) => *value as f64,
            PdfObject::Real(value) => *value,
            _ => 0.0,
        })
        .collect();
    Ok((values[2] - values[0], values[3] - values[1]))
}

// ---------------------------------------------------------------------------
// 工具面 (声明名 `doc_ops`, 五段流水线)
// ---------------------------------------------------------------------------

/// 文档操作工具 (`tool.doc_ops`, 声明名 `doc_ops`)。
pub struct DocOpsTool {
    id: CapabilityId,
    runner: DocOpsRunner,
}

impl DocOpsTool {
    /// 绑定受控配置 (独立计数器)。
    pub fn new(config: DocOpsConfig) -> Self {
        Self::with_counters(config, DocOpsCounters::new())
    }

    /// 绑定受控配置 + 共享计数器 (自述创作节同源)。
    pub fn with_counters(config: DocOpsConfig, counters: Arc<DocOpsCounters>) -> Self {
        Self {
            id: CapabilityId::new(DOC_OPS_CAPABILITY_ID).expect("valid capability id"),
            runner: DocOpsRunner::with_counters(config, counters),
        }
    }

    /// 输出归一合同: 结构化输出必须是对象且 `action` / `status` 恒在场。
    pub fn output_schema() -> OutputSchema {
        OutputSchema::new()
            .require("action", SchemaKind::String)
            .require("status", SchemaKind::String)
    }

    /// 执行面 (测试直调口, 与工具面同一真源)。
    pub fn runner(&self) -> &DocOpsRunner {
        &self.runner
    }

    /// 进程内计数 (自述创作节同源)。
    pub fn counters(&self) -> &Arc<DocOpsCounters> {
        self.runner.counters()
    }

    /// 拒绝即帧 (真因如实, 不静默降级)。
    fn refuse(&self, call: &ToolCall, error: DocOpsError) -> ToolResult {
        ToolResult::permanent_error(&call.id, error.message()).with_name(DOC_OPS_TOOL_NAME)
    }
}

#[async_trait]
impl ToolCapability for DocOpsTool {
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
                        "enum": DOC_OPS_ACTIONS,
                        "description": "Closed vocabulary: pdf_info / pdf_extract_text / docx_extract_text / docx_outline (reads); pdf_merge / pdf_split / pdf_extract_pages / pdf_watermark_text / docx_from_markdown / docx_append (writes, each waits for a human approval)."
                    },
                    "input": {
                        "type": "string",
                        "description": "Single input path (workspace-relative or absolute inside the workspace root): the four read actions, pdf_split, pdf_extract_pages, pdf_watermark_text, and the existing DOCX that docx_append copies from."
                    },
                    "inputs": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "pdf_merge only: 2-32 input paths, joined in order."
                    },
                    "output": {
                        "type": "string",
                        "description": "Output file for pdf_merge / pdf_extract_pages / pdf_watermark_text / docx_from_markdown / docx_append. Created atomically, never overwrites an existing file."
                    },
                    "output_dir": {
                        "type": "string",
                        "description": "pdf_split only: output directory for one PDF per page (created if missing)."
                    },
                    "pages": {
                        "type": "array",
                        "items": { "type": "integer", "minimum": 1 },
                        "description": "pdf_extract_pages only: 1-based page numbers, no duplicates."
                    },
                    "watermark_text": {
                        "type": "string",
                        "description": "pdf_watermark_text only: WinAnsi-encodable watermark text (non-ASCII beyond WinAnsi is refused, never guessed)."
                    },
                    "max_chars": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional tighter character budget for the two extract actions (the configured cap still applies)."
                    },
                    "text": {
                        "type": "string",
                        "description": "Source text for the two DOCX write actions. docx_from_markdown: markdown subset (ATX headings #..######, one paragraph per line, list items - * + or N. N), inline **bold** / *italic* / _italic_; unmatched markers stay literal). docx_append: paragraphs to append, one per line (blank lines skipped). Refused over the source caps or with XML-illegal control characters."
                    }
                },
                "required": ["action"],
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new(DOC_OPS_TOOL_NAME)
            .with_description(DOC_OPS_TOOL_DESCRIPTION.to_string())
            .with_parameters(parameters)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let arguments = match &call.arguments {
            Value::Null => json!({}),
            other => other.clone(),
        };
        let params = match serde_json::from_value::<DocOpsParams>(arguments) {
            Ok(params) => params,
            Err(error) => {
                return self.refuse(
                    call,
                    DocOpsError::InvalidParam(format!("invalid doc_ops parameters: {error}")),
                );
            }
        };
        match self.runner.run(&params) {
            Ok(frame) => ToolResult::ok(&call.id, frame).with_name(DOC_OPS_TOOL_NAME),
            Err(error) => self.refuse(call, error),
        }
    }
}

// ---------------------------------------------------------------------------
// 治理 (风险映射 + 审批档 + 单调边界 Guard)
// ---------------------------------------------------------------------------

/// 写面判定 (闭合词表; 词表外不当写面, 由调用方拒绝即帧)。
pub fn is_doc_ops_write_action(action: &str) -> bool {
    DOC_OPS_WRITE_ACTIONS.contains(&action)
}

/// 写面审批理由 (审批卡载荷同文)。
pub fn doc_ops_approval_reason(action: &str) -> String {
    format!(
        "文档操作写面 {action}: 批量落盘是文件创建面 (只创建不覆盖), \
         每次写面都停人工审批 (无自动放行档)"
    )
}

/// 文档操作审批档钩子 (治理链, 与本地审批面板 / IM 审批卡同链)。
///
/// **写面每次人批** ([`DOC_OPS_WRITE_ACTIONS`], 无自动放行档); 读面放行;
/// 词表外动作 fail-closed 停审批 (不静默放行)。
#[derive(Debug, Clone, Copy, Default)]
pub struct DocOpsApprovalHook;

impl DocOpsApprovalHook {
    /// 空构造 (无状态钩子)。
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl GovernanceHook for DocOpsApprovalHook {
    fn name(&self) -> &str {
        "doc_ops_write_risk"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        let Action::CapabilityDispatch {
            capability,
            arguments,
        } = &request.action
        else {
            return Decision::Allow;
        };
        if capability.as_str() != DOC_OPS_CAPABILITY_ID {
            return Decision::Allow;
        }
        let action = arguments
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if is_doc_ops_write_action(action) {
            return Decision::require_approval(doc_ops_approval_reason(action));
        }
        if DOC_OPS_READ_ACTIONS.contains(&action) {
            return Decision::Allow;
        }
        Decision::require_approval(format!(
            "文档操作动作 {action:?} 不在闭合词表: fail-closed 停人工审批"
        ))
    }
}

/// 写入风险映射 (pre-execute 瀑布): 未显式授权 = require-approval 档。
///
/// 与外部工具风险映射同一语义: 显式授权 (grant 且未标审批) → 放行;
/// 授权被标人工审批 / 无授权 → `ask` (拒绝即帧 `pipeline.pre_ask`)。
pub struct DocOpsRiskMappingHook {
    policy: Arc<Mutex<PermissionPolicy>>,
}

impl DocOpsRiskMappingHook {
    /// 挂在共享授权策略上的风险映射钩子。
    pub fn new(policy: Arc<Mutex<PermissionPolicy>>) -> Self {
        Self { policy }
    }
}

impl PreExecuteHook for DocOpsRiskMappingHook {
    fn name(&self) -> &str {
        "doc_ops_risk_mapping"
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
                    "doc_ops tool {} sits at the controlled read/write level: \
                     no explicit authorization for it",
                    request.capability.as_str()
                ),
            },
        }
    }
}

/// 单调边界 Guard (只拒不放): 参数矩阵 / 工作区限域 / 敏感面 / 上限。
///
/// 自动放行档放行的调用也照穿本 Guard —— 自动放行从不绕过硬边界; 任何后续
/// 阶段都无法把这里的拒绝翻回放行 (单调性)。
pub struct DocOpsBoundaryGuard {
    config: DocOpsConfig,
}

impl DocOpsBoundaryGuard {
    /// 绑定受控配置的边界 Guard。
    pub fn new(config: DocOpsConfig) -> Self {
        Self { config }
    }
}

impl ToolGuard for DocOpsBoundaryGuard {
    fn name(&self) -> &str {
        "doc_ops_boundary"
    }

    fn deny(&self, request: &ToolGuardRequest<'_>) -> Option<String> {
        if request.capability.as_str() != DOC_OPS_CAPABILITY_ID {
            return None;
        }
        // 解析失败不归边界 Guard (由工具即帧报解析错误), 只判硬边界。
        let Ok(params) = serde_json::from_value::<DocOpsParams>(request.arguments.clone()) else {
            return None;
        };
        doc_ops_params_refusal(&self.config, &params).map(|error| error.message())
    }
}

/// 生产授权播种: 已注册的文档操作面即获显式授权 (授权 = 「这次调用有人拍板」)。
///
/// 风险档位 (写面每次人批) 不在工具策略里 —— 由 [`DocOpsApprovalHook`] 承载,
/// 与本地审批面板 / IM 审批卡同链。
pub fn authorized_doc_ops_policy() -> Arc<Mutex<PermissionPolicy>> {
    let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
    policy
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .grant(Permission::ExecuteTool(DOC_OPS_CAPABILITY_ID.to_string()));
    policy
}

/// 组装文档操作五段流水线: 风险映射 → 单调边界 Guard → 超时 → 输出归一合同。
pub fn doc_ops_pipeline(config: DocOpsConfig) -> Arc<crate::exec_pipeline::ToolExecutionPipeline> {
    let guard: Arc<dyn ToolGuard> = Arc::new(DocOpsBoundaryGuard::new(config));
    Arc::new(
        crate::exec_pipeline::ToolExecutionPipeline::new()
            .with_pre_hook(Arc::new(DocOpsRiskMappingHook::new(
                authorized_doc_ops_policy(),
            )))
            .with_guard(guard)
            .with_around(
                AroundPolicy::new().with_timeout(DOC_OPS_TIMEOUT_MS, DEFAULT_MAX_TIMEOUT_MS),
            )
            .with_output_schema(DocOpsTool::output_schema()),
    )
}

/// 组装生产工具面: 受控读写族 + 五段流水线 (模型侧一个能力)。
pub fn doc_ops_capability(config: DocOpsConfig) -> Arc<dyn ToolCapability> {
    doc_ops_capability_shared(config, DocOpsCounters::new())
}

/// 组装生产工具面 + 共享计数器 (自述创作节与工具面同一真值源)。
pub fn doc_ops_capability_shared(
    config: DocOpsConfig,
    counters: Arc<DocOpsCounters>,
) -> Arc<dyn ToolCapability> {
    let tool: Arc<dyn ToolCapability> =
        Arc::new(DocOpsTool::with_counters(config.clone(), counters));
    Arc::new(crate::exec_pipeline::PipelinedCapability::new(
        tool,
        doc_ops_pipeline(config),
    ))
}

// ---------------------------------------------------------------------------
// 自述随行 (创作节 W2: 进程内计数, 只计数不回路径)
// ---------------------------------------------------------------------------

/// 文档操作面进程内计数 (创作节真值源: 与 `tool.doc_ops` 工具面同一实例)。
///
/// **进程内计数** (本进程生命周期内的最近操作, 重启清零), 不落账不造账;
/// 成功按闭合词表逐操作计数, 拒绝即帧照计 ([`DocOpsCounters::denied`])。
#[derive(Debug, Default)]
pub struct DocOpsCounters {
    calls: std::sync::atomic::AtomicU64,
    pages_touched: std::sync::atomic::AtomicU64,
    denied: std::sync::atomic::AtomicU64,
    per_operation: Mutex<std::collections::BTreeMap<String, u64>>,
    last_op_at_ms: Mutex<Option<i64>>,
    last_action: Mutex<Option<String>>,
}

impl DocOpsCounters {
    /// 计数器 (进程内, 重启清零)。
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// 记一次调用 (成败都计)。
    fn record_call(&self) {
        self.calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// 记一次成功 (动作词表名 + 处理页数 + 时刻)。
    fn record_success(&self, action: &str, pages_touched: u64, at_ms: i64) {
        self.pages_touched
            .fetch_add(pages_touched, std::sync::atomic::Ordering::Relaxed);
        let mut per_operation = self
            .per_operation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *per_operation.entry(action.to_string()).or_insert(0) += 1;
        drop(per_operation);
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
    pub fn status(&self) -> DocOpsStatus {
        use std::sync::atomic::Ordering;
        let per_operation = self
            .per_operation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let by_operation: Vec<DocOpsOpCount> = DOC_OPS_ACTIONS
            .iter()
            .map(|action| DocOpsOpCount {
                operation: (*action).to_string(),
                count: per_operation.get(*action).copied().unwrap_or(0),
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
        let note = (last_op_at_ms.is_none()).then(|| "尚无操作记录".to_string());
        DocOpsStatus {
            calls: self.calls.load(Ordering::Relaxed),
            pages_touched: self.pages_touched.load(Ordering::Relaxed),
            denied: self.denied.load(Ordering::Relaxed),
            by_operation,
            last_op_at_ms,
            last_action,
            note,
            reason: None,
        }
    }
}

/// 从帧里读处理页数 (u64 = 页数; 数组 = 返回页行数)。
fn frame_pages_touched(frame: &Value) -> u64 {
    match frame.get("pages") {
        Some(Value::Number(number)) => number.as_u64().unwrap_or(0),
        Some(Value::Array(items)) => items.len() as u64,
        _ => 0,
    }
}

/// 当前时刻 (epoch 毫秒); 时钟回拨不编造, 如实取系统读数。
fn now_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// 测试 (真 fixture 生成: 手写 PDF/ZIP 字节, 与生产写端独立, 防循环自证)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{SessionId, TraceId};

    // ---- 真 fixture 生成 (raw bytes, 不用生产写端) ----

    /// 手写 PDF: N 页, 每页若干行文字 (Tj + Td 定位)。返回真实 PDF 字节。
    fn fixture_pdf(pages: &[&[&str]]) -> Vec<u8> {
        fixture_pdf_with_trailer(pages, None)
    }

    /// 手写 PDF + 可选 trailer 附加项 (加密标记检测用真形状)。
    fn fixture_pdf_with_trailer(pages: &[&[&str]], trailer_extra: Option<&str>) -> Vec<u8> {
        let n = pages.len();
        let font_num = 3 + 2 * n;
        let mut objects: Vec<(u32, Vec<u8>)> = Vec::new();

        let kids: Vec<String> = (0..n)
            .map(|index| format!("{} 0 R", 3 + 2 * index))
            .collect();
        objects.push((1, format!("<< /Type /Catalog /Pages 2 0 R >>").into_bytes()));
        objects.push((
            2,
            format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")).into_bytes(),
        ));
        for (index, lines) in pages.iter().enumerate() {
            let page_num = 3 + 2 * index;
            let content_num = page_num + 1;
            objects.push((
                page_num as u32,
                format!(
                    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
                     /Resources << /Font << /F1 {font_num} 0 R >> >> \
                     /Contents {content_num} 0 R >>"
                )
                .into_bytes(),
            ));
            let mut body = String::from("BT\n/F1 12 Tf\n72 720 Td\n");
            for (line_index, line) in lines.iter().enumerate() {
                if line_index > 0 {
                    body.push_str("0 -14 Td\n");
                }
                body.push('(');
                body.push_str(line);
                body.push_str(") Tj\n");
            }
            body.push_str("ET\n");
            objects.push((
                content_num as u32,
                format!("<< /Length {} >>\nstream\n{body}endstream", body.len()).into_bytes(),
            ));
        }
        objects.push((
            font_num as u32,
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
                .to_vec(),
        ));
        assemble_pdf(&objects, trailer_extra)
    }

    /// 手写 PDF, 单页内容流用 FlateDecode (zlib) 压缩 —— 真压缩 fixture。
    fn fixture_pdf_flate(lines: &[&str]) -> Vec<u8> {
        use std::io::Write;
        let mut body = String::from("BT\n/F1 12 Tf\n72 720 Td\n");
        for (index, line) in lines.iter().enumerate() {
            if index > 0 {
                body.push_str("0 -14 Td\n");
            }
            body.push('(');
            body.push_str(line);
            body.push_str(") Tj\n");
        }
        body.push_str("ET\n");
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(body.as_bytes()).expect("压缩内容流");
        let compressed = encoder.finish().expect("收尾压缩器");
        let objects: Vec<(u32, Vec<u8>)> = vec![
            (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
            (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec()),
            (
                3,
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
                   /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
                    .to_vec(),
            ),
            (
                4,
                format!(
                    "<< /Length {} /Filter /FlateDecode >>\nstream\n",
                    compressed.len()
                )
                .into_bytes(),
            ),
            (
                5,
                b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
                    .to_vec(),
            ),
        ];
        // 压缩流要拼进对象 4: 手工组装 (数据在流关键字之后)。
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(b"%PDF-1.4\n");
        let mut offsets = vec![0usize; objects.len() + 1];
        for (num, body_bytes) in &objects {
            offsets[*num as usize] = out.len();
            out.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
            out.extend_from_slice(body_bytes);
            if *num == 4 {
                out.extend_from_slice(&compressed);
                out.extend_from_slice(b"\nendstream");
            }
            out.extend_from_slice(b"\nendobj\n");
        }
        append_xref(&mut out, &offsets, objects.len(), None);
        out
    }

    /// 按对象表拼 PDF (算 xref 偏移)。
    fn assemble_pdf(objects: &[(u32, Vec<u8>)], trailer_extra: Option<&str>) -> Vec<u8> {
        let count = objects.iter().map(|(num, _)| *num).max().unwrap_or(0);
        let mut out: Vec<u8> = b"%PDF-1.4\n".to_vec();
        let mut offsets = vec![0usize; count as usize + 1];
        for (num, body) in objects {
            offsets[*num as usize] = out.len();
            out.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        append_xref(&mut out, &offsets, count as usize, trailer_extra);
        out
    }

    /// 追加 xref 表 + trailer + startxref。
    fn append_xref(
        out: &mut Vec<u8>,
        offsets: &[usize],
        count: usize,
        trailer_extra: Option<&str>,
    ) {
        let xref_at = out.len();
        out.extend_from_slice(format!("xref\n0 {}\n", count + 1).as_bytes());
        out.extend_from_slice(b"0000000000 65535 f \n");
        for offset in &offsets[1..] {
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        let extra = trailer_extra.unwrap_or_default();
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R {extra}>>\nstartxref\n{xref_at}\n%%EOF\n",
                count + 1
            )
            .as_bytes(),
        );
    }

    /// ZIP CRC32 (真 fixture 的完整性字段)。
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

    /// 手写 ZIP 容器 (stored 条目, 合法 local header + 中央目录 + 结尾记录)。
    fn zip_stored(entries: &[(&str, &[u8])]) -> Vec<u8> {
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

    /// 一个文字处理段落标记。
    fn para(style: Option<&str>, numbered: bool, text: &str) -> String {
        let mut props = String::new();
        if let Some(style) = style {
            props.push_str(&format!("<w:pStyle w:val=\"{style}\"/>"));
        }
        if numbered {
            props.push_str("<w:numPr><w:numId w:val=\"1\"/></w:numPr>");
        }
        let ppr = if props.is_empty() {
            String::new()
        } else {
            format!("<w:pPr>{props}</w:pPr>")
        };
        format!("<w:p>{ppr}<w:r><w:t>{text}</w:t></w:r></w:p>")
    }

    /// 手写 DOCX 容器 (真 ZIP + 真 OOXML 标记)。
    fn fixture_docx(paragraphs: &[String]) -> Vec<u8> {
        let body = paragraphs.join("");
        let document = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
             <w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
             <w:body>{body}</w:body></w:document>"
        );
        let types = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
             <Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
             <Default Extension=\"xml\" ContentType=\"application/xml\"/></Types>";
        zip_stored(&[
            ("[Content_Types].xml", types.as_bytes()),
            ("word/document.xml", document.as_bytes()),
        ])
    }

    // ---- 测试装配 ----

    /// 每测试独立临时工作区。
    fn workspace(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("apeireth-doc-ops-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("创建测试工作区");
        dir
    }

    /// 写 fixture 进工作区, 返回 (配置, 文件名)。
    ///
    /// 测试装配显式开写总闸 (生产由组装根注入 `file_write` 同一开关值)。
    fn with_input(tag: &str, name: &str, bytes: &[u8]) -> (DocOpsConfig, String) {
        let root = workspace(tag);
        std::fs::write(root.join(name), bytes).expect("落 fixture");
        (
            DocOpsConfig::new(root).with_write_gate(true),
            name.to_string(),
        )
    }

    /// 参数快捷构造。
    fn params(action: &str, input: &str) -> DocOpsParams {
        DocOpsParams {
            action: Some(action.to_string()),
            input: Some(input.to_string()),
            ..DocOpsParams::default()
        }
    }

    /// PDF 字节 → 逐页文本 (复用生产读端做行为核对)。
    fn pages_text(bytes: &[u8]) -> Vec<String> {
        let document = parse_pdf(bytes).expect("输出 PDF 可解析");
        let pages = document.collect_pages().expect("输出页树可读");
        pages
            .iter()
            .map(|page| extract_page_text(&document, page).expect("提取").text)
            .collect()
    }

    /// PDF 字节 → 页数。
    fn page_count(bytes: &[u8]) -> usize {
        let document = parse_pdf(bytes).expect("输出 PDF 可解析");
        document.collect_pages().expect("输出页树可读").len()
    }

    // ---- PDF 读面行为钉 ----

    /// ① pdf_info: 页数 / 版本 / 逐页尺寸真值。
    #[test]
    fn pdf_info_reports_page_count_version_and_page_sizes() {
        let bytes = fixture_pdf(&[&["alpha line"], &["beta line"]]);
        let (config, name) = with_input("info", "sample.pdf", &bytes);
        let runner = DocOpsRunner::new(config);
        let frame = runner.run(&params(ACTION_PDF_INFO, &name)).expect("info");
        assert_eq!(frame["action"], ACTION_PDF_INFO);
        assert_eq!(frame["status"], "ok");
        assert_eq!(frame["pages"], 2);
        assert_eq!(frame["version"], "1.4");
        assert_eq!(frame["page_sizes"][0][0], 612.0);
        assert_eq!(frame["page_sizes"][0][1], 792.0);
        assert_eq!(frame["page_sizes"][1][1], 792.0);
    }

    /// ② pdf_extract_text: 逐页行文本按 Td 换行 (真 fixture 行为钉)。
    #[test]
    fn pdf_extract_text_recovers_pinned_lines_with_a_real_fixture() {
        let bytes = fixture_pdf(&[&["Hello Apeireth", "second line"], &["page two text"]]);
        let (config, name) = with_input("extract", "sample.pdf", &bytes);
        let runner = DocOpsRunner::new(config);
        let frame = runner
            .run(&params(ACTION_PDF_EXTRACT_TEXT, &name))
            .expect("extract");
        assert_eq!(frame["truncated"], false);
        assert_eq!(frame["pages"][0]["text"], "Hello Apeireth\nsecond line");
        assert_eq!(frame["pages"][1]["text"], "page two text");
        assert_eq!(
            frame["chars"],
            "Hello Apeireth\nsecond linepage two text".chars().count() as u64
        );
    }

    /// ③ pdf_extract_text: FlateDecode 压缩内容流 (真 zlib fixture)。
    #[test]
    fn pdf_extract_text_decodes_flate_streams() {
        let bytes = fixture_pdf_flate(&["compressed line"]);
        let (config, name) = with_input("flate", "flate.pdf", &bytes);
        let runner = DocOpsRunner::new(config);
        let frame = runner
            .run(&params(ACTION_PDF_EXTRACT_TEXT, &name))
            .expect("extract flate");
        assert_eq!(frame["pages"][0]["text"], "compressed line");
    }

    /// ④ pdf_merge: 多入一出依序合页 (输出重解析行为钉)。
    #[test]
    fn pdf_merge_joins_pages_in_order() {
        let root = workspace("merge");
        let config = DocOpsConfig::new(&root).with_write_gate(true);
        std::fs::write(root.join("a.pdf"), fixture_pdf(&[&["alpha line"]])).unwrap();
        std::fs::write(
            root.join("b.pdf"),
            fixture_pdf(&[&["beta one"], &["beta two"]]),
        )
        .unwrap();
        let runner = DocOpsRunner::new(config);
        let mut call = DocOpsParams {
            action: Some(ACTION_PDF_MERGE.to_string()),
            inputs: Some(vec!["a.pdf".to_string(), "b.pdf".to_string()]),
            output: Some("merged.pdf".to_string()),
            ..DocOpsParams::default()
        };
        let frame = runner.run(&call).expect("merge");
        assert_eq!(frame["pages"], 3);
        let merged = std::fs::read(root.join("merged.pdf")).unwrap();
        assert_eq!(page_count(&merged), 3);
        assert_eq!(
            pages_text(&merged),
            vec!["alpha line", "beta one", "beta two"]
        );
        call.inputs = Some(vec!["a.pdf".to_string()]);
        assert!(runner.run(&call).is_err(), "单输入 merge 必须拒绝");
    }

    /// ⑤ pdf_split: 一入多出逐页独立 (批量管道, 文件名钉)。
    #[test]
    fn pdf_split_writes_one_pdf_per_page() {
        let bytes = fixture_pdf(&[&["first page"], &["second page"]]);
        let (config, name) = with_input("split", "book.pdf", &bytes);
        let root = config.workspace_root.clone();
        let runner = DocOpsRunner::new(config);
        let call = DocOpsParams {
            action: Some(ACTION_PDF_SPLIT.to_string()),
            input: Some(name.clone()),
            output_dir: Some("out".to_string()),
            ..DocOpsParams::default()
        };
        let frame = runner.run(&call).expect("split");
        assert_eq!(frame["pages"], 2);
        assert_eq!(
            frame["files"],
            json!(["book-page-0001.pdf", "book-page-0002.pdf"])
        );
        let first = std::fs::read(root.join("out/book-page-0001.pdf")).unwrap();
        let second = std::fs::read(root.join("out/book-page-0002.pdf")).unwrap();
        assert_eq!(pages_text(&first), vec!["first page"]);
        assert_eq!(pages_text(&second), vec!["second page"]);
    }

    /// ⑥ pdf_extract_pages: 多页选页 → 单一 PDF (1 起页码)。
    #[test]
    fn pdf_extract_pages_keeps_only_the_selected_pages() {
        let bytes = fixture_pdf(&[&["first page"], &["second page"], &["third page"]]);
        let (config, name) = with_input("subset", "book.pdf", &bytes);
        let root = config.workspace_root.clone();
        let runner = DocOpsRunner::new(config);
        let call = DocOpsParams {
            action: Some(ACTION_PDF_EXTRACT_PAGES.to_string()),
            input: Some(name),
            output: Some("picked.pdf".to_string()),
            pages: Some(vec![3, 1]),
            ..DocOpsParams::default()
        };
        let frame = runner.run(&call).expect("extract pages");
        assert_eq!(frame["pages_total"], 3);
        let picked = std::fs::read(root.join("picked.pdf")).unwrap();
        assert_eq!(page_count(&picked), 2);
        assert_eq!(pages_text(&picked), vec!["third page", "first page"]);
    }

    /// ⑦ pdf_watermark_text: 逐页叠字不丢页 (输出重提取含原文与水印)。
    #[test]
    fn pdf_watermark_text_overlays_without_losing_pages() {
        let bytes = fixture_pdf(&[&["keep me"], &["keep me too"]]);
        let (config, name) = with_input("watermark", "memo.pdf", &bytes);
        let root = config.workspace_root.clone();
        let runner = DocOpsRunner::new(config);
        let call = DocOpsParams {
            action: Some(ACTION_PDF_WATERMARK_TEXT.to_string()),
            input: Some(name),
            output: Some("marked.pdf".to_string()),
            watermark_text: Some("DRAFT".to_string()),
            ..DocOpsParams::default()
        };
        let frame = runner.run(&call).expect("watermark");
        assert_eq!(frame["pages"], 2);
        let marked = std::fs::read(root.join("marked.pdf")).unwrap();
        assert_eq!(page_count(&marked), 2);
        for text in pages_text(&marked) {
            assert!(text.contains("keep me"), "原文必须保留: {text}");
            assert!(text.contains("DRAFT"), "水印必须上页: {text}");
        }
    }

    /// ⑧ pdf_watermark_text: 非 WinAnsi 文字拒绝即帧 (子集外不降级)。
    #[test]
    fn pdf_watermark_refuses_text_outside_the_declared_subset() {
        let bytes = fixture_pdf(&[&["keep me"]]);
        let (config, name) = with_input("watermark-han", "memo.pdf", &bytes);
        let runner = DocOpsRunner::new(config);
        let call = DocOpsParams {
            action: Some(ACTION_PDF_WATERMARK_TEXT.to_string()),
            input: Some(name),
            output: Some("marked.pdf".to_string()),
            watermark_text: Some("水印".to_string()),
            ..DocOpsParams::default()
        };
        let error = runner.run(&call).expect_err("非 WinAnsi 必须拒绝");
        assert!(
            error.message().contains("子集外"),
            "拒绝帧必须说明子集边界: {}",
            error.message()
        );
    }

    /// ⑨ 加密 PDF: 无解密面显式拒绝 (真 trailer 形状)。
    #[test]
    fn encrypted_pdfs_are_refused_with_an_honest_frame() {
        let bytes = fixture_pdf_with_trailer(&[&["secret"]], Some("/Encrypt 9 0 R "));
        let (config, name) = with_input("encrypted", "locked.pdf", &bytes);
        let runner = DocOpsRunner::new(config);
        let error = runner
            .run(&params(ACTION_PDF_INFO, &name))
            .expect_err("加密文档必须拒绝");
        assert!(
            error.message().contains("加密"),
            "拒绝帧必须如实说加密: {}",
            error.message()
        );
    }

    // ---- DOCX 读面行为钉 ----

    /// ⑩ docx_extract_text: 段落 → Markdown 转出 (标题/列表/正文钉)。
    #[test]
    fn docx_extract_text_turns_paragraphs_into_markdown() {
        let bytes = fixture_docx(&[
            para(Some("Heading1"), false, "标题一"),
            para(None, false, "正文段"),
            para(Some("Heading2"), false, "小节"),
            para(None, true, "列表项"),
            para(None, false, "   "),
        ]);
        let (config, name) = with_input("docx", "paper.docx", &bytes);
        let runner = DocOpsRunner::new(config);
        let frame = runner
            .run(&params(ACTION_DOCX_EXTRACT_TEXT, &name))
            .expect("docx extract");
        assert_eq!(frame["markdown"], "# 标题一\n正文段\n## 小节\n- 列表项");
        assert_eq!(frame["paragraphs"], 5);
        assert_eq!(frame["truncated"], false);
    }

    /// ⑪ docx_outline: 标题层级大纲 (只收标题级段落)。
    #[test]
    fn docx_outline_lists_heading_levels() {
        let bytes = fixture_docx(&[
            para(Some("Heading1"), false, "顶层"),
            para(None, false, "正文不出现在大纲"),
            para(Some("Heading2"), false, "二层"),
            para(Some("Heading3"), false, "三层"),
        ]);
        let (config, name) = with_input("docx-outline", "plan.docx", &bytes);
        let runner = DocOpsRunner::new(config);
        let frame = runner
            .run(&params(ACTION_DOCX_OUTLINE, &name))
            .expect("outline");
        assert_eq!(frame["headings"], 3);
        assert_eq!(
            frame["outline"],
            json!([
                {"level": 1, "text": "顶层"},
                {"level": 2, "text": "二层"},
                {"level": 3, "text": "三层"}
            ])
        );
    }

    // ---- 受控面行为钉 ----

    /// ⑫ 参数校验矩阵: 词表外动作 / 缺字段 / 非法取值逐项拒绝即帧。
    #[test]
    fn the_parameter_matrix_refuses_bad_calls() {
        let bytes = fixture_pdf(&[&["alpha"]]);
        let (config, name) = with_input("matrix", "sample.pdf", &bytes);
        let cases: Vec<DocOpsParams> = vec![
            DocOpsParams::default(),
            DocOpsParams {
                action: Some("rm_rf".to_string()),
                ..DocOpsParams::default()
            },
            params(ACTION_PDF_INFO, ""),
            DocOpsParams {
                action: Some(ACTION_PDF_MERGE.to_string()),
                inputs: Some(vec![name.clone()]),
                output: Some("out.pdf".to_string()),
                ..DocOpsParams::default()
            },
            DocOpsParams {
                action: Some(ACTION_PDF_EXTRACT_PAGES.to_string()),
                input: Some(name.clone()),
                output: Some("out.pdf".to_string()),
                pages: Some(vec![0]),
                ..DocOpsParams::default()
            },
            DocOpsParams {
                action: Some(ACTION_PDF_EXTRACT_PAGES.to_string()),
                input: Some(name.clone()),
                output: Some("out.pdf".to_string()),
                pages: Some(vec![1, 1]),
                ..DocOpsParams::default()
            },
            DocOpsParams {
                action: Some(ACTION_PDF_WATERMARK_TEXT.to_string()),
                input: Some(name.clone()),
                output: Some("out.pdf".to_string()),
                ..DocOpsParams::default()
            },
        ];
        for (index, case) in cases.iter().enumerate() {
            let error = DocOpsRunner::new(config.clone())
                .run(case)
                .expect_err(&format!("第 {} 个坏参数必须拒绝", index + 1));
            assert!(
                error.message().starts_with("拒绝文档操作"),
                "拒绝帧前缀必须稳定: {}",
                error.message()
            );
        }
        // 工具面同一真源: 未知字段也拒。
        let tool = DocOpsTool::new(config);
        let call = ToolCall {
            id: "call_doc_ops".into(),
            name: DOC_OPS_TOOL_NAME.into(),
            arguments: json!({"action": ACTION_PDF_INFO, "input": name, "extra": 1}),
        };
        let result = futures::executor::block_on(tool.invoke(&call));
        assert!(!result.is_ok(), "未知字段必须拒绝");
        assert!(
            result.render().contains("拒绝文档操作"),
            "{}",
            result.render()
        );
    }

    /// ⑬ 上限拒: 大小上限 / 页数上限宁拒不截 (含合并总页数)。
    #[test]
    fn size_and_page_caps_refuse_instead_of_truncating() {
        let bytes = fixture_pdf(&[&["a"], &["b"], &["c"]]);
        let root = workspace("caps");
        std::fs::write(root.join("big.pdf"), &bytes).unwrap();
        let tiny = DocOpsConfig::new(&root).with_limits(16, 2, 64);
        let error = DocOpsRunner::new(tiny.clone())
            .run(&params(ACTION_PDF_INFO, "big.pdf"))
            .expect_err("超大小上限必须拒绝");
        assert!(
            matches!(error, DocOpsError::InputTooLarge { .. }),
            "必须是大小上限拒: {error:?}"
        );
        let oversized_pages = DocOpsConfig::new(&root).with_limits(1 << 20, 2, 64);
        let error = DocOpsRunner::new(oversized_pages)
            .run(&params(ACTION_PDF_INFO, "big.pdf"))
            .expect_err("超页数上限必须拒绝");
        assert!(
            matches!(error, DocOpsError::TooManyPages { pages: 3, cap: 2 }),
            "必须是页数上限拒: {error:?}"
        );
    }

    /// ⑭ 原子写: 输出创建不覆盖 (二次同名拒绝且原文件一字不改)。
    #[test]
    fn outputs_are_created_atomically_and_never_clobber() {
        let bytes = fixture_pdf(&[&["alpha"], &["beta"]]);
        let (config, name) = with_input("atomic", "book.pdf", &bytes);
        let root = config.workspace_root.clone();
        let runner = DocOpsRunner::new(config);
        let call = DocOpsParams {
            action: Some(ACTION_PDF_EXTRACT_PAGES.to_string()),
            input: Some(name),
            output: Some("out.pdf".to_string()),
            pages: Some(vec![1]),
            ..DocOpsParams::default()
        };
        runner.run(&call).expect("首写成功");
        let first = std::fs::read(root.join("out.pdf")).unwrap();
        assert!(first.starts_with(b"%PDF-"), "输出必须是完整 PDF");
        let error = runner.run(&call).expect_err("同名再写必须拒绝");
        assert!(
            matches!(error, DocOpsError::OutputExists(_)),
            "必须是只创建不覆盖拒: {error:?}"
        );
        let second = std::fs::read(root.join("out.pdf")).unwrap();
        assert_eq!(first, second, "既有输出必须一字不改");
    }

    /// ⑮ 敏感面拒 + 工作区限域: 凭据面/越界路径拒绝即帧。
    #[test]
    fn sensitive_and_out_of_workspace_paths_are_refused() {
        let bytes = fixture_pdf(&[&["alpha"]]);
        let root = workspace("boundary");
        std::fs::write(root.join("id_rsa.pdf"), &bytes).unwrap();
        std::fs::write(root.parent().unwrap().join("outside.pdf"), &bytes).unwrap();
        let config = DocOpsConfig::new(&root);
        let error = DocOpsRunner::new(config.clone())
            .run(&params(ACTION_PDF_INFO, "id_rsa.pdf"))
            .expect_err("凭据面命名必须拒");
        assert!(
            matches!(error, DocOpsError::SensitiveSurface),
            "必须是敏感面拒: {error:?}"
        );
        assert!(
            error.message().contains("pipeline.pre_deny"),
            "敏感面拒绝必须是 pre_deny 帧: {}",
            error.message()
        );
        let error = DocOpsRunner::new(config.clone())
            .run(&params(
                ACTION_PDF_INFO,
                &root.parent().unwrap().join("outside.pdf").to_string_lossy(),
            ))
            .expect_err("工作区外绝对路径必须拒");
        assert!(
            matches!(error, DocOpsError::OutsideWorkspace(_)),
            "必须是限域拒: {error:?}"
        );
        let error = DocOpsRunner::new(config)
            .run(&params(ACTION_PDF_INFO, "../outside.pdf"))
            .expect_err("相对穿越必须拒");
        assert!(
            matches!(error, DocOpsError::OutsideWorkspace(_)),
            "必须是限域拒: {error:?}"
        );
    }

    /// ⑯ 开关 fail-closed: 只认显式 `"1"`。
    #[test]
    fn the_switch_is_fail_closed_except_the_exact_opt_in() {
        static ENV_LOCK: Mutex<()> = Mutex::new(());
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        std::env::remove_var(DOC_OPS_ENABLE_ENV);
        assert!(!doc_ops_enabled_from_env());
        assert!(DocOpsConfig::from_env(".").is_none());
        for value in ["0", "true", "on", " 1 0 ", ""] {
            std::env::set_var(DOC_OPS_ENABLE_ENV, value);
            assert!(!doc_ops_enabled_from_env(), "必须拒绝 {value:?}");
            assert!(DocOpsConfig::from_env(".").is_none(), "必须拒绝 {value:?}");
        }
        std::env::set_var(DOC_OPS_ENABLE_ENV, "1");
        assert!(doc_ops_enabled_from_env());
        assert!(DocOpsConfig::from_env(".").is_some());
        std::env::set_var(DOC_OPS_ENABLE_ENV, " 1 ");
        assert!(doc_ops_enabled_from_env(), "前后空白容忍");
        std::env::remove_var(DOC_OPS_ENABLE_ENV);
    }

    // ---- 治理面行为钉 ----

    /// ⑰ 审批档: 写面每次人批 (无自动放行档), 读面放行, 词表外 fail-closed。
    #[tokio::test]
    async fn write_actions_stop_at_human_approval_and_reads_pass() {
        fn dispatch(action: &str) -> GovernanceRequest<'static> {
            let capability: &'static CapabilityId = Box::leak(Box::new(
                CapabilityId::new(DOC_OPS_CAPABILITY_ID).expect("capability id"),
            ));
            let arguments: &'static Value = Box::leak(Box::new(json!({ "action": action })));
            GovernanceRequest::new(
                Action::CapabilityDispatch {
                    capability,
                    arguments,
                },
                SessionId::new(),
                TraceId::new(),
                1,
            )
        }
        let hook = DocOpsApprovalHook::new();
        for action in DOC_OPS_WRITE_ACTIONS {
            let decision = hook.evaluate(&dispatch(action)).await;
            let Decision::RequireApproval { reason } = decision else {
                panic!("写面 {action} 必须每次人批: {decision:?}");
            };
            assert!(reason.contains("每次写面都停人工审批"), "{reason}");
        }
        for action in DOC_OPS_READ_ACTIONS {
            let decision = hook.evaluate(&dispatch(action)).await;
            assert!(
                matches!(decision, Decision::Allow),
                "读面 {action} 应放行: {decision:?}"
            );
        }
        let decision = hook.evaluate(&dispatch("rm_rf")).await;
        assert!(
            matches!(decision, Decision::RequireApproval { .. }),
            "词表外必须 fail-closed 停审批: {decision:?}"
        );
    }

    /// ⑱ 边界 Guard 单调只拒不放: 越界/敏感面调用在流水线门禁就被拒。
    #[test]
    fn the_boundary_guard_denies_before_execution() {
        let root = workspace("guard");
        std::fs::write(root.join("id_rsa.pdf"), fixture_pdf(&[&["alpha"]])).unwrap();
        let guard = DocOpsBoundaryGuard::new(DocOpsConfig::new(&root));
        let capability = CapabilityId::new(DOC_OPS_CAPABILITY_ID).expect("capability id");
        let sensitive_args = json!({"action": ACTION_PDF_INFO, "input": "id_rsa.pdf"});
        let request = ToolGuardRequest::new(&capability, DOC_OPS_TOOL_NAME, &sensitive_args);
        let denial = guard.deny(&request);
        assert!(denial.is_some(), "敏感面必须被边界 Guard 拒");
        let missing_args = json!({"action": ACTION_PDF_INFO, "input": "missing.pdf"});
        let request = ToolGuardRequest::new(&capability, DOC_OPS_TOOL_NAME, &missing_args);
        assert!(guard.deny(&request).is_some(), "缺输入必须被拒");
    }

    /// ⑳ 写总闸 (宪法级 `file_write` 唯一写总闸): 关 = 写动作拒绝即帧,
    /// 读面不受影响 (fail-closed, 与写族工具同一把闸)。
    #[test]
    fn the_write_gate_refuses_writes_but_never_reads() {
        let bytes = fixture_pdf(&[&["alpha"]]);
        let (mut config, name) = with_input("write-gate", "memo.pdf", &bytes);
        config = config.with_write_gate(false);
        let runner = DocOpsRunner::new(config);
        // 读面照常。
        runner
            .run(&params(ACTION_PDF_INFO, &name))
            .expect("读面不受影响");
        // 写面拒绝即帧。
        let call = DocOpsParams {
            action: Some(ACTION_PDF_EXTRACT_PAGES.to_string()),
            input: Some(name.clone()),
            output: Some("out.pdf".to_string()),
            pages: Some(vec![1]),
            ..DocOpsParams::default()
        };
        let error = runner.run(&call).expect_err("写总闸关必须拒写");
        assert!(
            matches!(error, DocOpsError::WriteGateClosed(_)),
            "必须是写总闸拒: {error:?}"
        );
        assert!(
            error.message().contains("唯一写总闸"),
            "拒绝帧必须说明写总闸: {}",
            error.message()
        );
    }

    /// ⑲ 自述创作节: 进程内计数真值 (只计数不回路径; 尚无记录显式如实注)。
    #[test]
    fn the_creation_counters_project_truth_without_paths() {
        let bytes = fixture_pdf(&[&["alpha"], &["beta"]]);
        let (config, name) = with_input("counters", "memo.pdf", &bytes);
        let runner = DocOpsRunner::new(config);
        let empty = runner.counters().status();
        assert_eq!(empty.calls, 0);
        assert_eq!(empty.last_op_at_ms, None);
        assert_eq!(empty.note.as_deref(), Some("尚无操作记录"));
        assert_eq!(empty.by_operation.len(), DOC_OPS_ACTIONS.len());
        assert!(empty.by_operation.iter().all(|row| row.count == 0));

        runner.run(&params(ACTION_PDF_INFO, &name)).expect("info");
        runner.run(&params(ACTION_PDF_INFO, &name)).expect("info");
        runner
            .run(&params(ACTION_PDF_EXTRACT_TEXT, &name))
            .expect("extract");
        runner
            .run(&params(ACTION_PDF_INFO, "missing.pdf"))
            .expect_err("缺输入必拒");
        let status = runner.counters().status();
        assert_eq!(status.calls, 4);
        assert_eq!(status.denied, 1);
        assert_eq!(status.pages_touched, 6, "info 两次各 2 页 + extract 2 页");
        assert!(status.last_op_at_ms.is_some(), "成功后必须有真时刻");
        assert_eq!(status.last_action.as_deref(), Some(ACTION_PDF_EXTRACT_TEXT));
        let info_row = status
            .by_operation
            .iter()
            .find(|row| row.operation == ACTION_PDF_INFO)
            .expect("词表行不省");
        assert_eq!(info_row.count, 2);
        let rendered = serde_json::to_value(&status).expect("可序列化");
        let rendered_text = rendered.to_string();
        assert!(
            !rendered_text.contains("memo.pdf") && !rendered_text.contains(".pdf"),
            "创作节零路径零文件名: {rendered_text}"
        );
    }

    // ---- DOCX 写面行为钉 (W2 留缝收口: 生成 / 追加 / 受控面同族) ----

    /// 解压预算 (测试直调口用宽松档)。
    const TEST_ZIP_BUDGET: u64 = 64 * 1024 * 1024;

    /// 读回产物段落 (自家读端解回 = 同源自洽核对)。
    fn read_back_paragraphs(bytes: &[u8]) -> Vec<DocxParagraph> {
        docx_parse_paragraphs(bytes, TEST_ZIP_BUDGET).expect("自家产物可由自家读端解回")
    }

    /// 按名取条目全文 (测试检视用: 缺件即 panic 点名)。
    fn entry_text<'a>(entries: &'a [ZipEntry], name: &str) -> &'a str {
        std::str::from_utf8(
            &entries
                .iter()
                .find(|entry| entry.name == name)
                .unwrap_or_else(|| panic!("{name} 必须在场"))
                .data,
        )
        .expect("条目必须是 UTF-8")
    }

    /// 按固定前缀抽出属性值集合 (`w:pStyle` 引用集 / `w:styleId` 定义集),
    /// 测试用最小扫描: 只认「前缀 + 值 + 闭合引号」这一固定形状。
    fn attribute_values(xml: &str, prefix: &str) -> std::collections::BTreeSet<String> {
        let mut values = std::collections::BTreeSet::new();
        let mut rest = xml;
        while let Some(start) = rest.find(prefix) {
            let after = &rest[start + prefix.len()..];
            let end = after.find('"').expect("属性值必须闭合");
            values.insert(after[..end].to_string());
            rest = &after[end..];
        }
        values
    }

    /// XML 部件良构检查: 开闭标记逐层配对到正文尾 (样式层不在自家读端覆盖面,
    /// 故单验标记成对, 坏标记会整体拒绝而不静默通过)。
    fn assert_well_formed_xml(label: &str, xml: &str) {
        use quick_xml::events::Event;
        let mut reader = quick_xml::Reader::from_str(xml);
        let mut depth = 0usize;
        loop {
            match reader
                .read_event()
                .unwrap_or_else(|error| panic!("{label} 标记损坏: {error}"))
            {
                Event::Start(_) => depth += 1,
                Event::End(_) => {
                    depth = depth
                        .checked_sub(1)
                        .unwrap_or_else(|| panic!("{label} 闭合标记多于开标记"))
                }
                Event::Eof => break,
                _ => {}
            }
        }
        assert_eq!(depth, 0, "{label} 开标记必须全部闭合");
    }

    /// 开写总闸的工作区配置 (生产由组装根注入 `file_write` 同一开关值)。
    fn write_config(tag: &str) -> DocOpsConfig {
        DocOpsConfig::new(workspace(tag)).with_write_gate(true)
    }

    /// `docx_from_markdown` 快捷调用。
    fn from_markdown(
        config: &DocOpsConfig,
        text: &str,
        output: &str,
    ) -> Result<Value, DocOpsError> {
        DocOpsRunner::new(config.clone()).run(&DocOpsParams {
            action: Some(ACTION_DOCX_FROM_MARKDOWN.to_string()),
            text: Some(text.to_string()),
            output: Some(output.to_string()),
            ..DocOpsParams::default()
        })
    }

    /// ㉑ DOCX 生成: Markdown 结构面矩阵 (标题/段落/列表/粗斜体) 生成后用自家
    /// 读端解回 = 同源自洽 (列表往返归一为 `- ` 项, 行内着色不往返且如实)。
    #[test]
    fn docx_from_markdown_covers_the_structure_matrix_and_round_trips() {
        let markdown = "# Title One\n\
            \n\
            Plain paragraph\n\
            ## Sub *i* and **b**\n\
            - bullet one\n\
            * bullet two\n\
            + bullet three\n\
            1. ordered one\n\
            2) ordered two\n\
            Escaped ** unmatched\n\
            2*3*4\n";
        let config = write_config("docx-md");
        let frame = from_markdown(&config, markdown, "out.docx").expect("结构面矩阵可生成");
        assert_eq!(frame["status"], "ok");
        assert_eq!(frame["paragraphs"], 10);
        let bytes = std::fs::read(config.workspace_root.join("out.docx")).expect("产物已落盘");
        let paragraphs = read_back_paragraphs(&bytes);
        let shape: Vec<(&str, bool, &str)> = paragraphs
            .iter()
            .map(|paragraph| {
                (
                    paragraph.style.as_str(),
                    paragraph.numbered,
                    paragraph.text.as_str(),
                )
            })
            .collect();
        assert_eq!(
            shape,
            vec![
                ("Heading1", false, "Title One"),
                ("", false, "Plain paragraph"),
                ("Heading2", false, "Sub i and b"),
                ("", true, "bullet one"),
                ("", true, "bullet two"),
                ("", true, "bullet three"),
                ("", true, "ordered one"),
                ("", true, "ordered two"),
                ("", false, "Escaped ** unmatched"),
                ("", false, "234"),
            ]
        );
        // 往返归一如实: 标题回 `#`, 列表项回 `- ` (编号/项目符号外观不承诺),
        // 行内着色不往返 (读端只取文本)。
        let markdown_out = docx_paragraphs_to_markdown(&paragraphs);
        assert_eq!(
            markdown_out,
            "# Title One\nPlain paragraph\n## Sub i and b\n- bullet one\n- bullet two\n\
             - bullet three\n- ordered one\n- ordered two\nEscaped ** unmatched\n234"
        );
    }

    /// ㉒ DOCX 生成: 一例手工 XML 检视 (六件包逐件 / 结构标记 / 转义 / 样式层与
    /// 编号层同源)。
    #[test]
    fn the_generated_docx_is_inspected_by_hand_at_the_xml_level() {
        let markdown = "# Head\n\nplain a & b < c\n\n- item\n\n**bold** and *ital*\n";
        let config = write_config("docx-xml");
        from_markdown(&config, markdown, "hand.docx").expect("可生成");
        let bytes = std::fs::read(config.workspace_root.join("hand.docx")).expect("产物已落盘");
        // ZIP 词面手工检视: 本地头签名 + 结尾记录签名在场。
        assert!(bytes.starts_with(b"PK\x03\x04"), "本地头签名必须在场");
        assert!(
            bytes
                .windows(4)
                .any(|window| window == [0x50u8, 0x4b, 0x05, 0x06].as_slice()),
            "结尾记录签名必须在场"
        );
        let entries = zip_read_entries(&bytes, TEST_ZIP_BUDGET).expect("写端产物读端可解回");
        let names: Vec<&str> = entries.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "[Content_Types].xml",
                "_rels/.rels",
                "word/_rels/document.xml.rels",
                "word/document.xml",
                "word/styles.xml",
                "word/numbering.xml",
            ],
            "最小包恰六件 (正文 + 样式层 + 编号层, 两个部件三处一起给)"
        );
        // 六件 XML 部件逐件良构 (标记成对; 样式/编号层不在自家读端覆盖面, 单验)。
        for name in [
            "[Content_Types].xml",
            "_rels/.rels",
            "word/_rels/document.xml.rels",
            "word/document.xml",
            "word/styles.xml",
            "word/numbering.xml",
        ] {
            assert_well_formed_xml(name, entry_text(&entries, name));
        }
        // 逐件内容检视: 内容类型 / 根关系 / 正文关系 / 正文 / 样式层 / 编号层。
        let types = entry_text(&entries, "[Content_Types].xml");
        assert!(types.contains("/word/document.xml"), "内容类型登记正文");
        assert!(
            types.contains("/word/styles.xml") && types.contains("wordprocessingml.styles+xml"),
            "内容类型登记样式部件: {types}"
        );
        assert!(
            types.contains("/word/numbering.xml")
                && types.contains("wordprocessingml.numbering+xml"),
            "内容类型登记编号部件: {types}"
        );
        let rels = entry_text(&entries, "_rels/.rels");
        assert!(
            rels.contains("Target=\"word/document.xml\""),
            "根关系指向正文"
        );
        let document_rels = entry_text(&entries, "word/_rels/document.xml.rels");
        assert!(
            document_rels.contains("Target=\"styles.xml\"")
                && document_rels.contains("/relationships/styles"),
            "正文关系指向样式层: {document_rels}"
        );
        assert!(
            document_rels.contains("Id=\"rId2\"")
                && document_rels.contains("Target=\"numbering.xml\"")
                && document_rels.contains("/relationships/numbering"),
            "正文关系指向编号层 (rId2, 标准 numbering 关系): {document_rels}"
        );
        let xml = entry_text(&entries, "word/document.xml");
        assert!(xml.contains("<w:pStyle w:val=\"Heading1\"/>"), "标题样式名");
        assert!(
            xml.contains("<w:numPr><w:numId w:val=\"1\"/></w:numPr>"),
            "列表标记"
        );
        assert!(xml.contains("<w:rPr><w:b/></w:rPr>"), "粗体标记");
        assert!(xml.contains("<w:rPr><w:i/></w:rPr>"), "斜体标记");
        assert!(
            xml.contains("<w:t xml:space=\"preserve\">plain a &amp; b &lt; c</w:t>"),
            "文本转义: {xml}"
        );
        // 编号层最小如实: 一级项目符号定义 + numId=1 指向 abstractNum 0, 正文的
        // numPr numId=1 与之一致 (悬空 numId = 消费端不显示符号)。
        let numbering = entry_text(&entries, "word/numbering.xml");
        assert!(
            numbering.contains("<w:abstractNum w:abstractNumId=\"0\">"),
            "编号层必须有抽象编号定义: {numbering}"
        );
        assert!(
            numbering.contains("<w:lvl w:ilvl=\"0\">")
                && numbering.contains("<w:numFmt w:val=\"bullet\"/>")
                && numbering.contains("<w:lvlText w:val=\"•\"/>"),
            "一级项目符号定义 (bullet + •): {numbering}"
        );
        assert!(
            numbering.contains("<w:lvlJc w:val=\"left\"/>")
                && numbering.contains("<w:ind w:left=\"720\" w:hanging=\"360\"/>"),
            "项目符号对齐与缩进 (左 720 / 悬挂 360): {numbering}"
        );
        assert!(
            numbering.contains("<w:num w:numId=\"1\"><w:abstractNumId w:val=\"0\"/></w:num>"),
            "numId=1 指向抽象编号 0: {numbering}"
        );
        assert!(
            xml.contains("<w:numId w:val=\"1\"/>"),
            "正文列表引用 numId=1 (与编号层对齐)"
        );
        // 样式层最小如实: Normal 为默认段落样式, Heading1 带大纲级别,
        // 列表段落与粗/斜字符样式在场, 无主题/无字体美化 (不写主题部件)。
        let styles = entry_text(&entries, "word/styles.xml");
        assert!(
            styles.contains("<w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\">"),
            "Normal 是默认段落样式: {styles}"
        );
        assert!(
            styles.contains("<w:style w:type=\"paragraph\" w:styleId=\"Heading1\">")
                && styles.contains("<w:outlineLvl w:val=\"0\"/>"),
            "Heading1 带大纲级别 0: {styles}"
        );
        assert!(
            styles.contains("w:styleId=\"ListParagraph\""),
            "列表段落样式在场"
        );
        assert!(styles.contains("w:styleId=\"Strong\""), "粗体字符样式在场");
        assert!(
            styles.contains("w:styleId=\"Emphasis\""),
            "斜体字符样式在场"
        );
        // 正文引用与样式表定义同名同源: 引用必须在定义内 (悬空 = 样式缺失)。
        let references = attribute_values(xml, "<w:pStyle w:val=\"");
        let definitions = attribute_values(styles, "w:styleId=\"");
        assert!(
            references.contains("Heading1"),
            "正文标题引用必须在场: {references:?}"
        );
        assert!(
            references.is_subset(&definitions),
            "正文样式引用悬空: 引用 {references:?} 定义 {definitions:?}"
        );
        assert!(
            !names.iter().any(|name| name.contains("theme")),
            "无主题部件 (样式层不冒充主题)"
        );
    }

    /// ㉒ᵇ DOCX 生成: 六级标题交叉校验 —— 正文 `w:pStyle="HeadingN"` 引用集与
    /// `word/styles.xml` 的 `w:styleId="HeadingN"` 定义集逐级比对 (6 级全覆盖,
    /// 引用不悬空), 且每级带对应大纲级别。
    #[test]
    fn docx_heading_references_are_backed_by_style_definitions() {
        let markdown = "# L1\n\n## L2\n\n### L3\n\n#### L4\n\n##### L5\n\n###### L6\n";
        let config = write_config("docx-headings");
        let frame = from_markdown(&config, markdown, "headings.docx").expect("六级标题可生成");
        assert_eq!(frame["paragraphs"], 6, "六级标题 = 六段");
        let bytes = std::fs::read(config.workspace_root.join("headings.docx")).expect("产物已落盘");
        let entries = zip_read_entries(&bytes, TEST_ZIP_BUDGET).expect("可解回");
        let references = attribute_values(
            entry_text(&entries, "word/document.xml"),
            "<w:pStyle w:val=\"",
        );
        let definitions = attribute_values(entry_text(&entries, "word/styles.xml"), "w:styleId=\"");
        let styles = entry_text(&entries, "word/styles.xml");
        for level in 1..=6u32 {
            let id = format!("Heading{level}");
            assert!(references.contains(&id), "正文缺 {id} 引用: {references:?}");
            assert!(
                definitions.contains(&id),
                "样式表缺 {id} 定义: {definitions:?}"
            );
            assert!(
                styles.contains(&format!("<w:outlineLvl w:val=\"{}\"/>", level - 1)),
                "{id} 缺大纲级别 {}: {styles}",
                level - 1
            );
        }
        assert_eq!(
            references,
            (1..=6u32)
                .map(|level| format!("Heading{level}"))
                .collect::<std::collections::BTreeSet<String>>(),
            "正文引用集必须恰是六级标题 (无多无少)"
        );
        assert!(
            references.is_subset(&definitions),
            "引用必须落在定义内: 悬空 = {:?}",
            references.difference(&definitions).collect::<Vec<_>>()
        );
    }

    /// ㉒ᶜ DOCX 生成: 编号交叉校验 —— 正文出现过的每个 `w:numId` 值都能在
    /// `word/numbering.xml` 找到 `w:num w:numId="…"` 定义 (悬空 numId = 消费端
    /// 不显示符号, 故引用集必须是定义集子集); 同时 `w:num` 指向的抽象编号也必须在场。
    #[test]
    fn docx_list_numbering_references_are_backed_by_numbering_definitions() {
        // 有序项与无序项混排 (当前同渲染为项目符号, 引用同一 numId=1)。
        let markdown = "- bullet a\n\n* bullet b\n\n1. ordered a\n\n2) ordered b\n";
        let config = write_config("docx-numbering");
        let frame = from_markdown(&config, markdown, "lists.docx").expect("列表可生成");
        assert_eq!(frame["paragraphs"], 4, "四个列表项 = 四段");
        let bytes = std::fs::read(config.workspace_root.join("lists.docx")).expect("产物已落盘");
        let entries = zip_read_entries(&bytes, TEST_ZIP_BUDGET).expect("可解回");
        let xml = entry_text(&entries, "word/document.xml");
        let numbering = entry_text(&entries, "word/numbering.xml");
        let references = attribute_values(xml, "<w:numId w:val=\"");
        assert_eq!(
            references.len(),
            1,
            "正文只引用一个编号定义 (一级项目符号): {references:?}"
        );
        let number_definitions = attribute_values(numbering, "<w:num w:numId=\"");
        assert!(
            references.is_subset(&number_definitions),
            "正文 numId 悬空 (消费端不会显示符号): 引用 {references:?} 定义 \
             {number_definitions:?}"
        );
        // 每个 numId 定义必须指向在场的抽象编号 (链条 num → abstractNum → lvl)。
        let abstract_definitions = attribute_values(numbering, "<w:abstractNum w:abstractNumId=\"");
        for definition in &number_definitions {
            let link = format!("<w:num w:numId=\"{definition}\"><w:abstractNumId w:val=\"");
            let targets = attribute_values(numbering, &link);
            assert_eq!(
                targets.len(),
                1,
                "numId={definition} 必须恰指向一个抽象编号: {numbering}"
            );
            assert!(
                targets.is_subset(&abstract_definitions),
                "numId={definition} 指向的抽象编号未定义: 目标 {targets:?} 定义 \
                 {abstract_definitions:?}"
            );
        }
        assert!(
            numbering.contains("<w:lvl w:ilvl=\"0\">")
                && numbering.contains("<w:numFmt w:val=\"bullet\"/>"),
            "抽象编号必须带一级 bullet 级别: {numbering}"
        );
        // 读端只看 document.xml: 编号层在场不影响读回 (段落数/文本口径零改动)。
        let paragraphs = read_back_paragraphs(&bytes);
        let texts: Vec<&str> = paragraphs
            .iter()
            .map(|paragraph| paragraph.text.as_str())
            .collect();
        assert_eq!(
            texts,
            vec!["bullet a", "bullet b", "ordered a", "ordered b"]
        );
        assert!(
            paragraphs.iter().all(|paragraph| paragraph.numbered),
            "读端按 numPr 标记判列表项 (与编号层定义无关)"
        );
    }

    /// ㉓ DOCX 追加: 副本追加段落 (原段不动, 序不乱, 空行忽略), 创建类原子写
    /// (只创建不覆盖, 二次同名拒绝且底稿一字不改), 样式层/编号层条目与底稿逐字节
    /// 一致 (append 只动 `word/document.xml`)。
    #[test]
    fn docx_append_appends_paragraphs_and_never_overwrites() {
        let config = write_config("docx-append");
        from_markdown(&config, "# Base\n\n- item\n", "base.docx").expect("底稿可生成");
        let base_bytes = std::fs::read(config.workspace_root.join("base.docx")).expect("底稿在场");
        let runner = DocOpsRunner::new(config.clone());
        let frame = runner
            .run(&DocOpsParams {
                action: Some(ACTION_DOCX_APPEND.to_string()),
                input: Some("base.docx".to_string()),
                text: Some("appended one\nappended two\n\nappended three\n".to_string()),
                output: Some("appended.docx".to_string()),
                ..DocOpsParams::default()
            })
            .expect("追加可执行");
        assert_eq!(frame["status"], "ok");
        assert_eq!(frame["paragraphs_appended"], 3, "空行忽略 (逐行一段口径)");
        let bytes = std::fs::read(config.workspace_root.join("appended.docx")).expect("产物已落盘");
        let paragraphs = read_back_paragraphs(&bytes);
        let texts: Vec<&str> = paragraphs
            .iter()
            .map(|paragraph| paragraph.text.as_str())
            .collect();
        assert_eq!(
            texts,
            vec![
                "Base",
                "item",
                "appended one",
                "appended two",
                "appended three"
            ]
        );
        assert_eq!(paragraphs[0].style, "Heading1", "原段样式不动");
        assert!(paragraphs[1].numbered, "原列表标记不动");
        // append 只动 `word/document.xml`: 样式层/编号层与正文关系条目内容与底稿
        // 逐字节一致 (编号定义不随追加重写, 引用口径也不变)。
        let base_entries = zip_read_entries(&base_bytes, TEST_ZIP_BUDGET).expect("底稿可解回");
        let out_entries = zip_read_entries(&bytes, TEST_ZIP_BUDGET).expect("副本可解回");
        let entry_bytes = |entries: &[ZipEntry], name: &str| -> Vec<u8> {
            entries
                .iter()
                .find(|entry| entry.name == name)
                .unwrap_or_else(|| panic!("{name} 必须在场"))
                .data
                .clone()
        };
        for name in [
            "[Content_Types].xml",
            "_rels/.rels",
            "word/_rels/document.xml.rels",
            "word/styles.xml",
            "word/numbering.xml",
        ] {
            assert_eq!(
                entry_bytes(&out_entries, name),
                entry_bytes(&base_entries, name),
                "append 只动 document.xml: {name} 必须与底稿逐字节一致"
            );
        }
        assert_ne!(
            entry_bytes(&out_entries, "word/document.xml"),
            entry_bytes(&base_entries, "word/document.xml"),
            "正文必须真的变了 (追加段落)"
        );
        // 只创建不覆盖: 同名输出拒绝即帧, 底稿一字不改。
        let error = runner
            .run(&DocOpsParams {
                action: Some(ACTION_DOCX_APPEND.to_string()),
                input: Some("base.docx".to_string()),
                text: Some("again".to_string()),
                output: Some("appended.docx".to_string()),
                ..DocOpsParams::default()
            })
            .expect_err("同名输出必须拒");
        assert!(matches!(error, DocOpsError::OutputExists(_)), "{error:?}");
        assert_eq!(
            std::fs::read(config.workspace_root.join("base.docx")).expect("底稿在场"),
            base_bytes,
            "底稿原状 (追加是副本面)"
        );
    }

    /// ㉔ DOCX 追加: 非目标条目内容原样搬动; 节属性前让位 (`w:sectPr` 依规恒是
    /// body 尾子)。
    #[test]
    fn docx_append_keeps_other_entries_and_inserts_before_the_section_properties() {
        let document = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
             <w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
             <w:body><w:p><w:r><w:t>first</w:t></w:r></w:p>\
             <w:sectPr><w:pgSz w:w=\"11906\" w:h=\"16838\"/></w:sectPr></w:body></w:document>";
        let custom = b"custom-styles-bytes".to_vec();
        let bytes = zip_stored(&[
            ("word/document.xml", document.as_bytes()),
            ("word/styles.xml", custom.as_slice()),
        ]);
        let (config, name) = with_input("docx-sect", "in.docx", &bytes);
        let frame = DocOpsRunner::new(config.clone())
            .run(&DocOpsParams {
                action: Some(ACTION_DOCX_APPEND.to_string()),
                input: Some(name),
                text: Some("after body".to_string()),
                output: Some("out.docx".to_string()),
                ..DocOpsParams::default()
            })
            .expect("可追加");
        assert_eq!(frame["paragraphs_appended"], 1);
        let out = std::fs::read(config.workspace_root.join("out.docx")).expect("产物已落盘");
        let entries = zip_read_entries(&out, TEST_ZIP_BUDGET).expect("可解回");
        let styles = entries
            .iter()
            .find(|entry| entry.name == "word/styles.xml")
            .expect("条目还在");
        assert_eq!(styles.data, custom, "非目标条目内容零改动");
        let xml = std::str::from_utf8(
            &entries
                .iter()
                .find(|entry| entry.name == "word/document.xml")
                .expect("正文在场")
                .data,
        )
        .expect("UTF-8");
        let inserted = xml.find("after body").expect("追加段落在场");
        let sect = xml.find("<w:sectPr").expect("节属性在场");
        let body_end = xml.find("</w:body>").expect("body 尾标在场");
        assert!(
            inserted < sect && sect < body_end,
            "追加段落必须插在节属性之前: {xml}"
        );
    }

    /// ㉕ DOCX 追加: 坏容器拒绝即帧 (非 ZIP / 缺正文件 / 缺 body 尾标), 真因如实。
    #[test]
    fn docx_append_refuses_broken_containers_with_honest_frames() {
        let (config, _) = with_input("docx-broken", "broken.docx", b"this is not a zip container");
        let runner = DocOpsRunner::new(config.clone());
        let call = |input: &str| DocOpsParams {
            action: Some(ACTION_DOCX_APPEND.to_string()),
            input: Some(input.to_string()),
            text: Some("x".to_string()),
            output: Some("out.docx".to_string()),
            ..DocOpsParams::default()
        };
        let error = runner.run(&call("broken.docx")).expect_err("非 ZIP 必拒");
        assert!(
            error.message().contains("解析失败"),
            "真因必须是解析拒: {}",
            error.message()
        );
        std::fs::write(
            config.workspace_root.join("nodoc.docx"),
            zip_stored(&[("[Content_Types].xml", b"x".as_slice())]),
        )
        .expect("fixture");
        let error = runner.run(&call("nodoc.docx")).expect_err("缺正文件必拒");
        assert!(
            error.message().contains("word/document.xml"),
            "真因必须点名缺件: {}",
            error.message()
        );
        std::fs::write(
            config.workspace_root.join("nobody.docx"),
            zip_stored(&[(
                "word/document.xml",
                b"<w:document><w:body><w:p>hi</w:p>".as_slice(),
            )]),
        )
        .expect("fixture");
        let error = runner
            .run(&call("nobody.docx"))
            .expect_err("缺 body 尾标必拒");
        assert!(
            error.message().contains("w:body"),
            "真因必须点名缺尾标: {}",
            error.message()
        );
    }

    /// ㉖ DOCX 写面源文上限: 字符/段落上限宁拒不截; XML 非法控制字符拒绝不剔除;
    /// 空结构拒绝不生成空文档。
    #[test]
    fn docx_source_caps_refuse_instead_of_truncating() {
        let config = write_config("docx-caps");
        let over_chars = "a".repeat(DOC_OPS_MAX_SOURCE_CHARS + 1);
        let error = from_markdown(&config, &over_chars, "a.docx").expect_err("字符超上限必拒");
        assert!(error.message().contains("字符上限"), "{}", error.message());
        let over_paragraphs = "p\n".repeat(DOC_OPS_MAX_SOURCE_PARAGRAPHS + 1);
        let error = from_markdown(&config, &over_paragraphs, "b.docx").expect_err("段落超上限必拒");
        assert!(error.message().contains("段上限"), "{}", error.message());
        let error =
            from_markdown(&config, "bad\u{0}char", "c.docx").expect_err("XML 非法控制字符必拒");
        assert!(error.message().contains("控制字符"), "{}", error.message());
        let error = from_markdown(&config, "# \n- \n", "d.docx").expect_err("空结构必拒");
        assert!(error.message().contains("无段落"), "{}", error.message());
    }

    /// ㉗ DOCX 写面受控面同族: 写总闸 fail-closed + 每次人批 (无自动放行档) +
    /// 敏感面/越界拒绝即帧。
    #[tokio::test]
    async fn docx_write_actions_share_the_controlled_face() {
        fn dispatch(action: &str) -> GovernanceRequest<'static> {
            let capability: &'static CapabilityId = Box::leak(Box::new(
                CapabilityId::new(DOC_OPS_CAPABILITY_ID).expect("capability id"),
            ));
            let arguments: &'static Value = Box::leak(Box::new(json!({ "action": action })));
            GovernanceRequest::new(
                Action::CapabilityDispatch {
                    capability,
                    arguments,
                },
                SessionId::new(),
                TraceId::new(),
                1,
            )
        }
        let root = workspace("docx-gate");
        std::fs::write(
            root.join("id_rsa.docx"),
            fixture_docx(&[para(None, false, "x")]),
        )
        .expect("fixture");
        let mut append = DocOpsParams {
            action: Some(ACTION_DOCX_APPEND.to_string()),
            input: Some("id_rsa.docx".to_string()),
            text: Some("x".to_string()),
            output: Some("out.docx".to_string()),
            ..DocOpsParams::default()
        };
        // 写总闸关 = 拒写不拒读 (fail-closed, 与写族同一把闸)。
        let closed = DocOpsConfig::new(&root);
        let error = DocOpsRunner::new(closed.clone())
            .run(&append)
            .expect_err("写总闸关必拒写");
        assert!(
            matches!(error, DocOpsError::WriteGateClosed(_)),
            "{error:?}"
        );
        let error = from_markdown(&closed, "x", "out2.docx").expect_err("写总闸关必拒写");
        assert!(
            matches!(error, DocOpsError::WriteGateClosed(_)),
            "{error:?}"
        );
        std::fs::write(
            root.join("plain.docx"),
            fixture_docx(&[para(Some("Heading1"), false, "head")]),
        )
        .expect("fixture");
        DocOpsRunner::new(closed)
            .run(&params(ACTION_DOCX_OUTLINE, "plain.docx"))
            .expect("读面不受写总闸约束 (fail-closed 只挡写)");

        // 敏感面 / 越界拒绝即帧 (限域 + 敏感面同族)。
        let open = DocOpsConfig::new(&root).with_write_gate(true);
        let error = DocOpsRunner::new(open.clone())
            .run(&append)
            .expect_err("凭据面必拒");
        assert!(matches!(error, DocOpsError::SensitiveSurface), "{error:?}");
        append.input = Some("plain.docx".to_string());
        append.output = Some("../out.docx".to_string());
        let error = DocOpsRunner::new(open)
            .run(&append)
            .expect_err("越界输出必拒");
        assert!(
            matches!(error, DocOpsError::OutsideWorkspace(_)),
            "{error:?}"
        );

        // 每次人批 (无自动放行档)。
        let hook = DocOpsApprovalHook::new();
        for action in [ACTION_DOCX_FROM_MARKDOWN, ACTION_DOCX_APPEND] {
            let decision = hook.evaluate(&dispatch(action)).await;
            assert!(
                matches!(decision, Decision::RequireApproval { .. }),
                "{action} 必须每次人批: {decision:?}"
            );
        }
    }

    /// ㉘ ZIP 写端: stored / deflate 两档对称 (写端产物读端解回一字不差;
    /// stored 档字节原样在场, 防写读同模自证)。
    #[test]
    fn zip_write_entries_roundtrips_both_compression_modes() {
        let entries = vec![
            ZipEntry {
                name: "a.txt".to_string(),
                data: b"hello zip".to_vec(),
            },
            ZipEntry {
                name: "dir/b.txt".to_string(),
                data: vec![7u8; 4096],
            },
        ];
        for compression in [ZipCompression::Stored, ZipCompression::Deflate] {
            let bytes = zip_write_entries(&entries, compression).expect("写端可产出");
            assert!(bytes.starts_with(b"PK\x03\x04"), "本地头签名");
            if compression == ZipCompression::Stored {
                assert!(
                    bytes
                        .windows(9)
                        .any(|window| window == b"hello zip".as_slice()),
                    "stored 档字节原样在场"
                );
            }
            let back = zip_read_entries(&bytes, TEST_ZIP_BUDGET).expect("读端可解回");
            assert_eq!(back, entries, "{compression:?} 档一字不差");
        }
    }
}
