//! 创作工具面 W1: 图像操作 (`tool.image_ops`) —— 受控文件写族 + 受控读面.
//!
//! # 面向谁
//! 设计第一律「AI 先用」: 本面的第一用户是 agent 自己 (它能编辑图片); 同一内核
//! 将来外接人类界面做第二出口 (删掉人类界面后本面仍完整可用)。
//!
//! # 操作词表 (闭合)
//! `resize` / `crop` / `rotate` / `flip` / `format_convert` / `brightness` /
//! `contrast` / `grayscale` / `blur` / `sharpen` / `watermark_text` —— 十一项
//! 枚举即全集 ([`OPERATION_VOCABULARY`]), 表外操作名与逐操作多余/缺失参数一律
//! **拒绝即帧** ([`ImageOpsError::ParamRejected`]), 无静默忽略、无半应用。
//!
//! # 受控面
//! - **读面**: 输入 = 工作区限域读 (守门词法层 + canonicalize 现实层 + 敏感面拒,
//!   与本地读工具同一套解析口径), 读事件把版本记入共享 [`ObservedGate`] (观测门禁
//!   同款)。
//! - **写面**: 输出 = 工作区限域写, 同一解析口径 + 同一敏感面拒; 落盘走**文件写入
//!   族**归档 (见下) 的读前观测门禁 (`gated_write_atomic`: 未读不得覆盖写) +
//!   `storage_atomic` 原子替换 (临时文件 + 换名, 失败清理, 不留半成品)。
//! - **上限 (防炸弹图)**: 输入字节上限 [`MAX_INPUT_BYTES`] (按实读判定, 与文件
//!   增长竞态无关) + 解码期维度/分配上限 (解码器 Limits) + 解码后像素上限
//!   [`MAX_IMAGE_PIXELS`], 三层各自拒绝即帧。
//! - **格式白名单**: 输入按内容嗅探、输出按目标格式, 双侧都只认
//!   png / jpeg / webp / bmp 四种 ([`ImageFormatKind`]); 白名单外拒绝即帧。
//!
//! # 归档裁定 (读码定归档: 图像写入族 or 文件写入族 → **文件写入族**)
//! 治理层族词表 `apeireth_governance::approval_family` 是冻结面 (新族必须显式
//! 入表并补测试), 现有写族 [`WriteContent`] = 「文件写入族」覆盖「改内容 / 追加 /
//! 覆盖输出 / 原地截断」; 图像输出写 = 写文件, 不新造「图像写入族」—— 唯一写总闸
//! (`APEIRETH_ENABLE_FILE_WRITE`) 的宪法语义是「文件写入只有唯一总闸」, 另立图像
//! 族会把写权力劈成两闸。故本面输出写**归档文件写入族**, 注册即闸 =
//! `APEIRETH_ENABLE_IMAGE_OPS=1` + `APEIRETH_ENABLE_FILE_WRITE=1` + 工作区根
//! (子依赖主 + 写总闸, fail-closed), 与 `tool.apply_patch` / `tool.worktree_apply`
//! 同受一闸一观测表。
//!
//! # 批量与原子写
//! 一次调用 = 一条操作管道 ([`ImageOperation`] 序列, 上限 [`MAX_PIPELINE_STEPS`]),
//! 顺序应用、中途任一步拒绝即帧则**全无产出** (目标原状, 无临时文件残留);
//! 编码整幅在内存完成后才落盘, 落盘 = 临时文件 + 换名 (原子替换), 失败清理。
//!
//! # 脱敏口径 (路径不出帧)
//! - 自产帧与结果帧里的路径**只回工作区相对拼写**; 工作区根的绝对拼写 (含
//!   canonical / 斜杠归一形态) 一律不出帧。
//! - 越界 / 敏感面 / 词法攻击面的拒绝帧**不回显路径** (只报事实类别与稳定 code);
//!   凭据面沿用读工具同一帧 ([`credential_surface_refusal`])。
//! - 自述创作节只回计数, 零路径零文件名。
//!
//! # 开关
//! 注册即闸 [`IMAGE_OPS_ENABLE_ENV`] (`APEIRETH_ENABLE_IMAGE_OPS`), 默认关
//! fail-closed (只认显式 `"1"`)。
//!
//! # 外部依赖报备 (本批唯一新引入)
//! 图像编解码用纯 Rust 图像库 `image` (MIT OR Apache-2.0, 与本仓 Apache-2.0 OR
//! MIT 双协议兼容, 任选其一、互不传染), 特性面只开 png/jpeg/webp/bmp 四种白名单
//! 格式 (default-features 关闭, 不引入其余编解码器与并行后端)。选它而自实现
//! 编解码的理由: 四格式编解码自研 = 造轮子反模式 (PNG 的 zlib 流 / JPEG 的 DCT /
//! WebP 的熵编码合计数万行, 自写反而扩大攻击面)。webp 输出面如实标注: 该库
//! 的 WebP 编码器只支持**无损**编码 (无损即无损, 不冒充有损)。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use async_trait::async_trait;
use image::codecs::bmp::BmpEncoder;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::codecs::webp::WebPEncoder;
use image::imageops::{self, FilterType};
use image::{ImageEncoder, ImageError, ImageFormat, ImageReader, Limits, Rgba, RgbaImage};
use serde::Deserialize;

use crate::observed_gate::{
    gated_write_atomic, FileVersion, FsVersionProbe, GatedWriteError, ObservedGate, VersionProbe,
    WriteKind, WriteRequest,
};
use crate::self_status::{ImageOpsOpCount, ImageOpsStatus};
use crate::sensitive_path::{credential_surface_refusal, is_sensitive_path};

/// 工具能力 id (`tool.image_ops`)。
pub const IMAGE_OPS_CAPABILITY_ID: &str = "tool.image_ops";

/// 工具声明名 (模型可见)。
pub const IMAGE_OPS_TOOL_NAME: &str = "image_ops";

/// 图像操作面总开关 (`APEIRETH_ENABLE_IMAGE_OPS`): 注册即闸, 默认关 fail-closed。
pub const IMAGE_OPS_ENABLE_ENV: &str = "APEIRETH_ENABLE_IMAGE_OPS";

/// 输入文件字节上限 (32 MiB): 按**实读**字节判定 (open + take), 与文件增长竞态无关。
pub const MAX_INPUT_BYTES: u64 = 32 * 1024 * 1024;

/// 单边维度上限 (像素)。
pub const MAX_IMAGE_DIM: u32 = 8192;

/// 总像素上限 (2^25 ≈ 3355 万像素): 解码期分配上限与解码后复核同值, 防炸弹图。
pub const MAX_IMAGE_PIXELS: u64 = 33_554_432;

/// 一次调用的操作步数上限 (防超长管道拖垮单次调用)。
pub const MAX_PIPELINE_STEPS: usize = 32;

/// 水印文字长度上限 (字符)。
pub const MAX_WATERMARK_TEXT_CHARS: usize = 64;

/// 水印字模放大倍数上限 (每字模像素 = scale × scale 方块)。
pub const MAX_WATERMARK_SCALE: u32 = 8;

/// jpeg 编码质量 (该库 jpeg 编码器是整数 DCT 有损编码; 质量固定 90, 不开放旋钮)。
const JPEG_QUALITY: u8 = 90;

/// 操作词表 (闭合): 枚举即全集, 顺序 = 自述创作节逐操作计数的行序。
pub const OPERATION_VOCABULARY: [&str; 11] = [
    "resize",
    "crop",
    "rotate",
    "flip",
    "format_convert",
    "brightness",
    "contrast",
    "grayscale",
    "blur",
    "sharpen",
    "watermark_text",
];

/// 格式白名单 (闭合): 输入内容嗅探与输出目标格式双侧只认这四种。
pub const SUPPORTED_FORMATS: [&str; 4] = ["png", "jpeg", "webp", "bmp"];

/// 自述创作节计数口径注记 (进程内计数, 不落账不造账)。
pub const IMAGE_OPS_COUNT_SCOPE_NOTE: &str =
    "进程内计数: 本进程生命周期内的最近操作计数 (重启清零), 不落账不造账";

/// 格式白名单成员。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormatKind {
    /// png (无损, 含 alpha)。
    Png,
    /// jpeg (有损, 无 alpha —— 编码前白底合成)。
    Jpeg,
    /// webp (该库编码器只有**无损**档)。
    Webp,
    /// bmp (无损)。
    Bmp,
}

impl ImageFormatKind {
    /// 词面名 (白名单拼写)。
    pub const fn name(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpeg",
            Self::Webp => "webp",
            Self::Bmp => "bmp",
        }
    }

    /// 名字 → 白名单成员 (含 `jpg` 常见拼写); 表外名回 `None`。
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "png" => Some(Self::Png),
            "jpeg" | "jpg" => Some(Self::Jpeg),
            "webp" => Some(Self::Webp),
            "bmp" => Some(Self::Bmp),
            _ => None,
        }
    }

    /// 扩展名 → 白名单成员; 表外扩展名回 `None`。
    pub fn from_extension(extension: &str) -> Option<Self> {
        Self::from_name(extension)
    }

    /// 内容嗅探格式 → 白名单成员; 白名单外回 `None`。
    pub fn from_image_format(format: ImageFormat) -> Option<Self> {
        match format {
            ImageFormat::Png => Some(Self::Png),
            ImageFormat::Jpeg => Some(Self::Jpeg),
            ImageFormat::WebP => Some(Self::Webp),
            ImageFormat::Bmp => Some(Self::Bmp),
            _ => None,
        }
    }

    /// 解码器/编码器格式轴。
    pub const fn as_image_format(self) -> ImageFormat {
        match self {
            Self::Png => ImageFormat::Png,
            Self::Jpeg => ImageFormat::Jpeg,
            Self::Webp => ImageFormat::WebP,
            Self::Bmp => ImageFormat::Bmp,
        }
    }
}

/// 翻转轴词表 (闭合)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlipAxis {
    /// 水平翻转 (左右镜像)。
    Horizontal,
    /// 垂直翻转 (上下镜像)。
    Vertical,
}

/// 闭合操作词表的十一种操作 (每种参数在校验期逐项核对, 拒绝即帧)。
#[derive(Debug, Clone, PartialEq)]
pub enum ImageOperation {
    /// 缩放到精确目标尺寸 (width / height, 双边 1..=MAX_IMAGE_DIM)。
    Resize {
        /// 目标宽。
        width: u32,
        /// 目标高。
        height: u32,
    },
    /// 裁剪矩形 (x / y / width / height; 矩形必须整框落在图内, 应用期校验)。
    Crop {
        /// 左上角 x。
        x: u32,
        /// 左上角 y。
        y: u32,
        /// 裁剪宽。
        width: u32,
        /// 裁剪高。
        height: u32,
    },
    /// 旋转 (只认 90 / 180 / 270 整倍数 —— 像素级精确, 不做任意角插值)。
    Rotate {
        /// 角度 (度), 闭合集 {90, 180, 270}。
        angle_deg: u16,
    },
    /// 翻转 (horizontal / vertical)。
    Flip {
        /// 翻转轴。
        axis: FlipAxis,
    },
    /// 目标编码格式 (不改像素; 也是输出格式的显式指定, 一次调用只认最后一个)。
    FormatConvert {
        /// 目标格式 (白名单)。
        format: ImageFormatKind,
    },
    /// 亮度 (通道加减, -255..=255, alpha 不动)。
    Brightness {
        /// 增量 (负值变暗)。
        amount: i32,
    },
    /// 对比度 (以 128 为轴的线性拉伸, factor 0.0..=3.0, 1.0 = 不变, alpha 不动)。
    Contrast {
        /// 对比度因子。
        factor: f64,
    },
    /// 灰度 (整数亮度公式 (77r + 150g + 29b + 128) >> 8, alpha 不动)。
    Grayscale,
    /// 高斯模糊 (sigma 0<σ<=32)。
    Blur {
        /// 模糊半径。
        sigma: f32,
    },
    /// 反锐化 (sigma 0<σ<=32; threshold 0..=255)。
    Sharpen {
        /// 锐化半径。
        sigma: f32,
        /// 锐化阈值。
        threshold: i32,
    },
    /// 文字水印 (内置 5×7 点阵字模, 白色不透明像素; text / x / y / scale)。
    WatermarkText {
        /// 水印文字 (可打印 ASCII 0x20..=0x7E, 1..=64 字符)。
        text: String,
        /// 字框左上角 x。
        x: u32,
        /// 字框左上角 y。
        y: u32,
        /// 字模放大倍数 (1..=8)。
        scale: u32,
    },
}

impl ImageOperation {
    /// 词表名 (闭合词表的成员名)。
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Resize { .. } => "resize",
            Self::Crop { .. } => "crop",
            Self::Rotate { .. } => "rotate",
            Self::Flip { .. } => "flip",
            Self::FormatConvert { .. } => "format_convert",
            Self::Brightness { .. } => "brightness",
            Self::Contrast { .. } => "contrast",
            Self::Grayscale => "grayscale",
            Self::Blur { .. } => "blur",
            Self::Sharpen { .. } => "sharpen",
            Self::WatermarkText { .. } => "watermark_text",
        }
    }

    /// 从调用面参数解析 + 逐参数校验 (词表外名 / 缺参 / 多余参 / 越界值一律
    /// 拒绝即帧; 校验先于任何 IO 与像素处理)。
    pub fn parse(spec: &OperationSpec) -> Result<Self, ImageOpsError> {
        let name = spec.op.trim().to_ascii_lowercase();
        let reject = |reason: String| Err(ImageOpsError::ParamRejected(reason));
        match name.as_str() {
            "resize" => {
                spec.require_exact("resize", &["width", "height"])?;
                let width = spec.int("resize", "width")?;
                let height = spec.int("resize", "height")?;
                Ok(Self::Resize {
                    width: dim("resize", "width", width)?,
                    height: dim("resize", "height", height)?,
                })
            }
            "crop" => {
                spec.require_exact("crop", &["x", "y", "width", "height"])?;
                let x = spec.int("crop", "x")?;
                let y = spec.int("crop", "y")?;
                let width = spec.int("crop", "width")?;
                let height = spec.int("crop", "height")?;
                Ok(Self::Crop {
                    x: coord("crop", "x", x)?,
                    y: coord("crop", "y", y)?,
                    width: dim("crop", "width", width)?,
                    height: dim("crop", "height", height)?,
                })
            }
            "rotate" => {
                spec.require_exact("rotate", &["angle_deg"])?;
                let angle = spec.int("rotate", "angle_deg")?;
                match angle {
                    90 | 180 | 270 => Ok(Self::Rotate {
                        angle_deg: angle as u16,
                    }),
                    other => reject(format!(
                        "rotate: angle_deg 只认 90/180/270 (整倍数像素级精确), 得到 {other}"
                    )),
                }
            }
            "flip" => {
                spec.require_exact("flip", &["axis"])?;
                let axis = spec.text("flip", "axis")?;
                match axis.as_str() {
                    "horizontal" => Ok(Self::Flip {
                        axis: FlipAxis::Horizontal,
                    }),
                    "vertical" => Ok(Self::Flip {
                        axis: FlipAxis::Vertical,
                    }),
                    other => reject(format!(
                        "flip: axis 只认 horizontal/vertical, 得到 {other:?}"
                    )),
                }
            }
            "format_convert" => {
                spec.require_exact("format_convert", &["format"])?;
                let format = spec.text("format_convert", "format")?;
                match ImageFormatKind::from_name(&format) {
                    Some(format) => Ok(Self::FormatConvert { format }),
                    None => reject(format!(
                        "format_convert: format 只认白名单 {:?}, 得到 {format:?}",
                        SUPPORTED_FORMATS
                    )),
                }
            }
            "brightness" => {
                spec.require_exact("brightness", &["amount"])?;
                let amount = spec.int("brightness", "amount")?;
                if !(-255..=255).contains(&amount) {
                    return reject(format!(
                        "brightness: amount 必须在 -255..=255, 得到 {amount}"
                    ));
                }
                Ok(Self::Brightness {
                    amount: amount as i32,
                })
            }
            "contrast" => {
                spec.require_exact("contrast", &["factor"])?;
                let factor = spec.float("contrast", "factor")?;
                if !factor.is_finite() || !(0.0..=3.0).contains(&factor) {
                    return reject(format!(
                        "contrast: factor 必须是 0.0..=3.0 的有限数, 得到 {factor}"
                    ));
                }
                Ok(Self::Contrast { factor })
            }
            "grayscale" => {
                spec.require_exact("grayscale", &[])?;
                Ok(Self::Grayscale)
            }
            "blur" => {
                spec.require_exact("blur", &["sigma"])?;
                let sigma = spec.float("blur", "sigma")?;
                let sigma = sigma_in("blur", sigma)?;
                Ok(Self::Blur { sigma })
            }
            "sharpen" => {
                spec.require_exact("sharpen", &["sigma", "threshold"])?;
                let sigma = spec.float("sharpen", "sigma")?;
                let sigma = sigma_in("sharpen", sigma)?;
                let threshold = spec.int("sharpen", "threshold")?;
                if !(0..=255).contains(&threshold) {
                    return reject(format!(
                        "sharpen: threshold 必须在 0..=255, 得到 {threshold}"
                    ));
                }
                Ok(Self::Sharpen {
                    sigma,
                    threshold: threshold as i32,
                })
            }
            "watermark_text" => {
                spec.require_exact("watermark_text", &["text", "x", "y", "scale"])?;
                let text = spec.text("watermark_text", "text")?;
                let chars = text.chars().count();
                if chars == 0 || chars > MAX_WATERMARK_TEXT_CHARS {
                    return reject(format!(
                        "watermark_text: text 字符数必须在 1..={MAX_WATERMARK_TEXT_CHARS}, 得到 {chars}"
                    ));
                }
                if !text.chars().all(is_supported_glyph_char) {
                    return reject(
                        "watermark_text: text 只收可打印 ASCII (0x20..=0x7E), 表外字符拒绝"
                            .to_string(),
                    );
                }
                let x = spec.int("watermark_text", "x")?;
                let y = spec.int("watermark_text", "y")?;
                let scale = spec.int("watermark_text", "scale")?;
                if !(1..=i64::from(MAX_WATERMARK_SCALE)).contains(&scale) {
                    return reject(format!(
                        "watermark_text: scale 必须在 1..={MAX_WATERMARK_SCALE}, 得到 {scale}"
                    ));
                }
                Ok(Self::WatermarkText {
                    text,
                    x: coord("watermark_text", "x", x)?,
                    y: coord("watermark_text", "y", y)?,
                    scale: scale as u32,
                })
            }
            other => reject(format!(
                "操作名 {other:?} 不在闭合词表 {:?} (表外操作拒绝即帧)",
                OPERATION_VOCABULARY
            )),
        }
    }
}

/// 边长类参数 (1..=MAX_IMAGE_DIM)。
fn dim(op: &str, field: &str, value: i64) -> Result<u32, ImageOpsError> {
    if (1..=i64::from(MAX_IMAGE_DIM)).contains(&value) {
        Ok(value as u32)
    } else {
        Err(ImageOpsError::ParamRejected(format!(
            "{op}: {field} 必须在 1..={MAX_IMAGE_DIM}, 得到 {value}"
        )))
    }
}

/// 坐标类参数 (0..=MAX_IMAGE_DIM, 图幅上限内的偏移; 有界即无截断)。
fn coord(op: &str, field: &str, value: i64) -> Result<u32, ImageOpsError> {
    if (0..=i64::from(MAX_IMAGE_DIM)).contains(&value) {
        Ok(value as u32)
    } else {
        Err(ImageOpsError::ParamRejected(format!(
            "{op}: {field} 必须在 0..={MAX_IMAGE_DIM}, 得到 {value}"
        )))
    }
}

/// sigma 类参数 (有限, 0<σ<=32)。
fn sigma_in(op: &str, value: f64) -> Result<f32, ImageOpsError> {
    if value.is_finite() && value > 0.0 && value <= 32.0 {
        Ok(value as f32)
    } else {
        Err(ImageOpsError::ParamRejected(format!(
            "{op}: sigma 必须是 0<σ<=32 的有限数, 得到 {value}"
        )))
    }
}

/// 调用面一次操作的原始参数 (serde 面, 数字统一 i64/f64 承接, 语义校验在
/// [`ImageOperation::parse`] —— 拒绝理由逐字段点名, 无 serde 式模糊帧)。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationSpec {
    /// 操作名 (闭合词表)。
    pub op: String,
    /// resize / crop 边长。
    pub width: Option<i64>,
    /// resize / crop 边长。
    pub height: Option<i64>,
    /// crop / watermark_text 起点。
    pub x: Option<i64>,
    /// crop / watermark_text 起点。
    pub y: Option<i64>,
    /// rotate 角度。
    pub angle_deg: Option<i64>,
    /// flip 轴。
    pub axis: Option<String>,
    /// format_convert 目标格式。
    pub format: Option<String>,
    /// brightness 增量。
    pub amount: Option<i64>,
    /// contrast 因子。
    pub factor: Option<f64>,
    /// blur / sharpen 半径。
    pub sigma: Option<f64>,
    /// sharpen 阈值。
    pub threshold: Option<i64>,
    /// watermark_text 文字。
    pub text: Option<String>,
    /// watermark_text 放大倍数。
    pub scale: Option<i64>,
}

impl OperationSpec {
    /// 出现过的字段名 (用于「该操作只认自己的参数」的闭合校验)。
    fn present_fields(&self) -> Vec<&'static str> {
        let mut present = Vec::new();
        if self.width.is_some() {
            present.push("width");
        }
        if self.height.is_some() {
            present.push("height");
        }
        if self.x.is_some() {
            present.push("x");
        }
        if self.y.is_some() {
            present.push("y");
        }
        if self.angle_deg.is_some() {
            present.push("angle_deg");
        }
        if self.axis.is_some() {
            present.push("axis");
        }
        if self.format.is_some() {
            present.push("format");
        }
        if self.amount.is_some() {
            present.push("amount");
        }
        if self.factor.is_some() {
            present.push("factor");
        }
        if self.sigma.is_some() {
            present.push("sigma");
        }
        if self.threshold.is_some() {
            present.push("threshold");
        }
        if self.text.is_some() {
            present.push("text");
        }
        if self.scale.is_some() {
            present.push("scale");
        }
        present
    }

    /// 逐操作参数集合校验: 恰好给出 `allowed` 的字段, 缺一/多一都拒绝即帧。
    fn require_exact(&self, op: &str, allowed: &[&str]) -> Result<(), ImageOpsError> {
        let present = self.present_fields();
        let unexpected: Vec<&str> = present
            .iter()
            .copied()
            .filter(|field| !allowed.contains(field))
            .collect();
        if !unexpected.is_empty() {
            return Err(ImageOpsError::ParamRejected(format!(
                "{op}: 不接受参数 {unexpected:?} (该操作只认 {allowed:?})"
            )));
        }
        let missing: Vec<&str> = allowed
            .iter()
            .copied()
            .filter(|field| !present.contains(field))
            .collect();
        if !missing.is_empty() {
            return Err(ImageOpsError::ParamRejected(format!(
                "{op}: 缺参数 {missing:?}"
            )));
        }
        Ok(())
    }

    fn int(&self, op: &str, field: &str) -> Result<i64, ImageOpsError> {
        match field {
            "width" => self.width,
            "height" => self.height,
            "x" => self.x,
            "y" => self.y,
            "angle_deg" => self.angle_deg,
            "amount" => self.amount,
            "threshold" => self.threshold,
            "scale" => self.scale,
            _ => None,
        }
        .ok_or_else(|| ImageOpsError::ParamRejected(format!("{op}: {field} 缺失")))
    }

    fn float(&self, op: &str, field: &str) -> Result<f64, ImageOpsError> {
        match field {
            "factor" => self.factor,
            "sigma" => self.sigma,
            _ => None,
        }
        .ok_or_else(|| ImageOpsError::ParamRejected(format!("{op}: {field} 缺失")))
    }

    fn text(&self, op: &str, field: &str) -> Result<String, ImageOpsError> {
        match field {
            "axis" => &self.axis,
            "format" => &self.format,
            "text" => &self.text,
            _ => &None,
        }
        .clone()
        .ok_or_else(|| ImageOpsError::ParamRejected(format!("{op}: {field} 缺失")))
    }
}

/// 图像操作面失败分类 (拒绝即帧: 稳定 code + 真因, 无静默降级)。
///
/// 帧 = `image_ops.<kind>: <reason>`; 敏感面沿用读工具的凭据帧 (pre_deny 语义)。
/// 脱敏口径: 路径不入帧 (越界/敏感/词法面不回显路径), 共享件回传文案经
/// `redact_workspace_paths` 剥根前缀后才入帧。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageOpsError {
    /// 参数校验拒绝 (词表外名 / 缺参 / 多余参 / 越界值 / 几何越界)。
    ParamRejected(String),
    /// 格式白名单拒绝 (输入内容 / 输出目标 / 扩展名冲突)。
    FormatRejected(String),
    /// 大小/维度/像素上限拒绝 (防炸弹图)。
    LimitExceeded(String),
    /// 词法攻击面 / 敏感系统路径 (不回显路径)。
    PathRejected,
    /// 目标越出工作区 (不回显路径)。
    OutsideWorkspace,
    /// 凭据面 (沿用读工具同一帧)。
    SensitiveSurface,
    /// 解码失败 (坏字节流 / 截断; 无路径)。
    DecodeFailed(String),
    /// 编码失败 (无路径)。
    EncodeFailed(String),
    /// 输入读取失败 (相对拼写 + 错误类别)。
    ReadFailed(String),
    /// 读前观测门禁拒绝 (未读不得覆盖写; 文案已剥根前缀)。
    WriteDenied(String),
    /// 原子写失败 (文案已剥根前缀)。
    WriteFailed(String),
}

impl ImageOpsError {
    /// 帧的稳定 code。
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ParamRejected(_) => "image_ops.param_rejected",
            Self::FormatRejected(_) => "image_ops.format_rejected",
            Self::LimitExceeded(_) => "image_ops.limit_exceeded",
            Self::PathRejected => "image_ops.path_rejected",
            Self::OutsideWorkspace => "image_ops.outside_workspace",
            Self::SensitiveSurface => "pipeline.pre_deny",
            Self::DecodeFailed(_) => "image_ops.decode_failed",
            Self::EncodeFailed(_) => "image_ops.encode_failed",
            Self::ReadFailed(_) => "image_ops.read_failed",
            Self::WriteDenied(_) => "image_ops.write_denied",
            Self::WriteFailed(_) => "image_ops.write_failed",
        }
    }

    /// 帧的真因 (越界/敏感/词法面不回显路径)。
    pub fn reason(&self) -> String {
        match self {
            Self::ParamRejected(detail)
            | Self::FormatRejected(detail)
            | Self::LimitExceeded(detail)
            | Self::DecodeFailed(detail)
            | Self::EncodeFailed(detail)
            | Self::ReadFailed(detail)
            | Self::WriteDenied(detail)
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

/// 调用面参数 (一次调用 = 输入 → 操作管道 → 输出)。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageOpsParams {
    /// 输入图 (工作区相对拼写或其内绝对拼写)。
    pub input: String,
    /// 输出图 (工作区相对拼写或其内绝对拼写)。
    pub output: String,
    /// 操作管道 (1..=MAX_PIPELINE_STEPS 步, 顺序应用)。
    pub operations: Vec<OperationSpec>,
}

/// 一次成功调用的产出汇报 (路径只回工作区相对拼写 —— 脱敏口径)。
#[derive(Debug, Clone, PartialEq)]
pub struct ImageOpsReport {
    /// 输入的相对拼写。
    pub input: String,
    /// 输出的相对拼写。
    pub output: String,
    /// 输入格式 (白名单)。
    pub input_format: ImageFormatKind,
    /// 输出格式 (白名单)。
    pub output_format: ImageFormatKind,
    /// 输入尺寸。
    pub input_size: (u32, u32),
    /// 输出尺寸。
    pub output_size: (u32, u32),
    /// 应用了的操作 (词表名, 顺序)。
    pub operations: Vec<String>,
    /// 是否覆盖了既有目标 (true = 覆盖写, 走读前观测门禁)。
    pub overwrote: bool,
    /// 落盘字节数。
    pub bytes_written: u64,
}

impl ImageOpsReport {
    /// 投影成结构化 JSON (字段合同的唯一实现)。
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "input": self.input,
            "output": self.output,
            "input_format": self.input_format.name(),
            "output_format": self.output_format.name(),
            "input_size": { "width": self.input_size.0, "height": self.input_size.1 },
            "output_size": { "width": self.output_size.0, "height": self.output_size.1 },
            "operations": self.operations,
            "overwrote": self.overwrote,
            "bytes_written": self.bytes_written,
        })
    }
}

/// 创作节计数器 (进程内): 与工具面同一实例 —— 自述创作节与工具同源同账。
///
/// 计数口径 ([`IMAGE_OPS_COUNT_SCOPE_NOTE`]): 本进程生命周期内的最近操作计数,
/// 重启清零, 不落账不造账 (0 装诚实: 不拿磁盘账冒充)。
#[derive(Debug, Default)]
pub struct ImageOpsActivity {
    counts: Mutex<ActivityCounts>,
}

#[derive(Debug, Default)]
struct ActivityCounts {
    calls: u64,
    ops_applied: u64,
    denied: u64,
    by_operation: [u64; OPERATION_VOCABULARY.len()],
    last_op_at_ms: Option<i64>,
    last_call_ops: Option<String>,
}

impl ImageOpsActivity {
    /// 空计数器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 记一次调用 (成败都记)。
    pub fn note_call(&self) {
        self.lock().calls += 1;
    }

    /// 记一次成功应用 (逐操作计数 + 最近一调用的操作序列)。
    pub fn record_applied(&self, operations: &[String]) {
        let mut counts = self.lock();
        for name in operations {
            counts.ops_applied += 1;
            if let Some(index) = OPERATION_VOCABULARY.iter().position(|entry| entry == name) {
                counts.by_operation[index] += 1;
            }
        }
        counts.last_op_at_ms = Some(now_epoch_ms());
        counts.last_call_ops = Some(operations.join(" -> "));
    }

    /// 记一次拒绝即帧 (错误即帧也计入创作节, 不藏拒绝次数)。
    pub fn record_denied(&self) {
        self.lock().denied += 1;
    }

    /// 自述创作节真值 (最近操作计数, 只计数不回路径)。
    pub fn status(&self) -> ImageOpsStatus {
        let counts = self.lock();
        let by_operation = OPERATION_VOCABULARY
            .iter()
            .enumerate()
            .map(|(index, name)| ImageOpsOpCount {
                operation: (*name).to_string(),
                count: counts.by_operation[index],
            })
            .collect();
        let note = if counts.calls == 0 {
            format!("尚无操作记录; {IMAGE_OPS_COUNT_SCOPE_NOTE}")
        } else {
            IMAGE_OPS_COUNT_SCOPE_NOTE.to_string()
        };
        ImageOpsStatus {
            calls: counts.calls,
            ops_applied: counts.ops_applied,
            denied: counts.denied,
            by_operation,
            last_op_at_ms: counts.last_op_at_ms,
            last_call_ops: counts.last_call_ops.clone(),
            note: Some(note),
            reason: None,
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, ActivityCounts> {
        self.counts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// 图像操作工具 (`tool.image_ops`, 声明名 [`IMAGE_OPS_TOOL_NAME`])。
pub struct ImageOpsTool {
    id: CapabilityId,
    root: PathBuf,
    gate: Arc<ObservedGate>,
    activity: Arc<ImageOpsActivity>,
    max_input_bytes: u64,
}

impl ImageOpsTool {
    /// 绑定工作区根 + 共享读前观测门禁 + 创作节计数器。
    pub fn new(
        root: impl Into<PathBuf>,
        gate: Arc<ObservedGate>,
        activity: Arc<ImageOpsActivity>,
    ) -> Self {
        Self {
            id: CapabilityId::new(IMAGE_OPS_CAPABILITY_ID).expect("capability id"),
            root: root.into(),
            gate,
            activity,
            max_input_bytes: MAX_INPUT_BYTES,
        }
    }

    /// 覆写输入字节上限 (测试/诊断用)。
    #[must_use]
    pub fn with_max_input_bytes(mut self, max_input_bytes: u64) -> Self {
        self.max_input_bytes = max_input_bytes;
        self
    }

    /// 创作节计数器 (与自述创作节同一实例)。
    pub fn activity(&self) -> &Arc<ImageOpsActivity> {
        &self.activity
    }

    /// 工作区根 (canonical)。
    fn canonical_root(&self) -> Result<PathBuf, ImageOpsError> {
        std::fs::canonicalize(&self.root).map_err(|_| {
            ImageOpsError::ReadFailed(
                "workspace root is not accessible; 工作区根不可访问".to_string(),
            )
        })
    }

    /// 工作区限域解析 (读写双面同口径): 守门词法层 → canonicalize 现实层 →
    /// root 包含 → 敏感面拒。`must_exist` = 读面 (目标须在); 写面解析父目录。
    fn resolve_contained(
        &self,
        requested: &str,
        must_exist: bool,
    ) -> Result<PathBuf, ImageOpsError> {
        if requested.trim().is_empty() {
            return Err(ImageOpsError::ParamRejected(
                "path must not be empty; 路径不得为空".to_string(),
            ));
        }
        let root = self.canonical_root()?;
        let candidate = if Path::new(requested).is_absolute() {
            PathBuf::from(requested)
        } else {
            root.join(requested)
        };

        crate::guardrail::ToolGuardrail::verify_path_access(&root, &candidate).map_err(
            |error| match error {
                crate::guardrail::PreCallGuardError::OutsideWorkspace(_) => {
                    ImageOpsError::OutsideWorkspace
                }
                crate::guardrail::PreCallGuardError::PathTraversal(_) => {
                    ImageOpsError::PathRejected
                }
                crate::guardrail::PreCallGuardError::DangerousCommandInjection(_) => {
                    ImageOpsError::PathRejected
                }
            },
        )?;

        if must_exist {
            let canonical =
                std::fs::canonicalize(&candidate).map_err(|error| match error.kind() {
                    std::io::ErrorKind::NotFound => {
                        ImageOpsError::ReadFailed("input image not found; 输入图不存在".to_string())
                    }
                    _ => ImageOpsError::ReadFailed(format!(
                        "input image unreadable ({})",
                        error.kind()
                    )),
                })?;
            if !canonical.starts_with(&root) {
                return Err(ImageOpsError::OutsideWorkspace);
            }
            if is_sensitive_path(&root, &candidate) || is_sensitive_path(&root, &canonical) {
                return Err(ImageOpsError::SensitiveSurface);
            }
            return Ok(canonical);
        }

        // 写面: 目标可以尚不存在 —— 对父目录做现实层包含检查, 目标名走同一敏感面拒。
        let parent = candidate.parent().unwrap_or(Path::new("")).to_path_buf();
        let canonical_parent = std::fs::canonicalize(&parent).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                ImageOpsError::WriteFailed(
                    "output directory does not exist (directories are not auto-created); \
                     输出目录不存在 (不自动建目录)"
                        .to_string(),
                )
            } else {
                ImageOpsError::WriteFailed(format!(
                    "output directory is not accessible ({}); 输出目录不可访问",
                    error.kind()
                ))
            }
        })?;
        if !canonical_parent.starts_with(&root) {
            return Err(ImageOpsError::OutsideWorkspace);
        }
        if is_sensitive_path(&root, &candidate) || is_sensitive_path(&root, &canonical_parent) {
            return Err(ImageOpsError::SensitiveSurface);
        }
        let file_name = match candidate.file_name() {
            Some(name) => name.to_owned(),
            None => {
                return Err(ImageOpsError::ParamRejected(
                    "output must name a file; 输出必须是文件".to_string(),
                ))
            }
        };
        Ok(canonical_parent.join(file_name))
    }

    /// 相对拼写 (脱敏口径: 报告与帧只回相对路径)。
    fn relative_display(&self, canonical: &Path, root: &Path) -> String {
        match canonical.strip_prefix(root) {
            Ok(relative) if relative.as_os_str().is_empty() => ".".to_string(),
            Ok(relative) => relative.to_string_lossy().replace('\\', "/"),
            Err(_) => "<path>".to_string(),
        }
    }

    /// 请求拼写的第二种观测键 (与读工具同口径: 跨工具拼写差异不丢观测)。
    fn observed_spelling(&self, requested: &str) -> PathBuf {
        let candidate = Path::new(requested);
        if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            self.root.join(candidate)
        }
    }

    /// 脱敏: 把工作区根的绝对拼写 (原形 / canonical / 斜杠归一) 剥成 `<workspace>`。
    fn redact_paths(&self, text: &str) -> String {
        let mut redacted = text.to_string();
        let spellings = [
            self.root.to_string_lossy().to_string(),
            std::fs::canonicalize(&self.root)
                .map(|root| root.to_string_lossy().to_string())
                .unwrap_or_default(),
        ];
        for spelling in spellings {
            if spelling.is_empty() {
                continue;
            }
            for variant in [
                spelling.clone(),
                spelling.replace('\\', "/"),
                spelling.to_lowercase(),
                spelling.replace('\\', "/").to_lowercase(),
            ] {
                if variant.is_empty() {
                    continue;
                }
                redacted = redacted.replace(&variant, "<workspace>");
            }
        }
        redacted
    }

    /// 读面: 工作区限域读 + 观测记录 (观测门禁同款) + 大小上限 + 内容嗅探白名单。
    fn read_input(
        &self,
        requested: &str,
    ) -> Result<(PathBuf, Vec<u8>, ImageFormatKind), ImageOpsError> {
        let request_path = self.observed_spelling(requested);
        let canonical = match self.resolve_contained(requested, true) {
            Ok(path) => path,
            Err(error) => {
                if matches!(error, ImageOpsError::ReadFailed(_)) {
                    // 读到「目标不存在」也是观测事件 (创建类写入凭它过门禁)。
                    self.gate.observe_missing(&request_path);
                }
                return Err(error);
            }
        };
        let mut bytes = Vec::new();
        let observed_version;
        match std::fs::File::open(&canonical) {
            Ok(file) => {
                observed_version = file
                    .metadata()
                    .ok()
                    .map(|meta| FileVersion::from_metadata(&meta));
                let mut limited = file.take(self.max_input_bytes.saturating_add(1));
                limited.read_to_end(&mut bytes).map_err(|error| {
                    ImageOpsError::ReadFailed(format!("input read failed ({})", error.kind()))
                })?;
            }
            Err(error) => {
                return Err(ImageOpsError::ReadFailed(format!(
                    "input open failed ({})",
                    error.kind()
                )))
            }
        }
        if bytes.len() as u64 > self.max_input_bytes {
            return Err(ImageOpsError::LimitExceeded(format!(
                "input exceeds the {} byte limit (按实读判定); 输入字节超上限",
                self.max_input_bytes
            )));
        }
        // 读事件记录观测 (canonical + 请求拼写都记, 与读工具同口径)。
        if let Some(version) = observed_version {
            self.gate.observe_present(&canonical, version);
            self.gate.observe_present(&request_path, version);
        }
        let guessed = image::guess_format(&bytes).map_err(|_| {
            ImageOpsError::FormatRejected(
                "input format cannot be sniffed from content; 输入内容不是可识别图像".to_string(),
            )
        })?;
        let format = ImageFormatKind::from_image_format(guessed).ok_or_else(|| {
            ImageOpsError::FormatRejected(format!(
                "input format {:?} is outside the whitelist {:?}; 白名单外格式拒绝",
                guessed, SUPPORTED_FORMATS
            ))
        })?;
        Ok((canonical, bytes, format))
    }

    /// 解码: 解码器 Limits (维度 + 分配上限) + 解码后像素复核 (防炸弹图三层)。
    fn decode(&self, bytes: &[u8]) -> Result<RgbaImage, ImageOpsError> {
        let mut reader = ImageReader::new(std::io::Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|error| {
                ImageOpsError::DecodeFailed(format!("input format sniff failed ({})", error.kind()))
            })?;
        let mut limits = Limits::default();
        limits.max_image_width = Some(MAX_IMAGE_DIM);
        limits.max_image_height = Some(MAX_IMAGE_DIM);
        limits.max_alloc = Some(MAX_IMAGE_PIXELS.saturating_mul(4));
        reader.limits(limits);
        let decoded = reader.decode().map_err(|error| match error {
            ImageError::Limits(detail) => ImageOpsError::LimitExceeded(format!(
                "decoder limit refused the image ({detail}); 解码期上限拒绝 (防炸弹图)"
            )),
            other => ImageOpsError::DecodeFailed(format!("image decode failed ({other})")),
        })?;
        let (width, height) = (decoded.width(), decoded.height());
        let pixels = u64::from(width).saturating_mul(u64::from(height));
        if pixels > MAX_IMAGE_PIXELS {
            return Err(ImageOpsError::LimitExceeded(format!(
                "decoded image has {pixels} pixels > {MAX_IMAGE_PIXELS} cap; 像素超上限 (防炸弹图)"
            )));
        }
        Ok(decoded.to_rgba8())
    }

    /// 输出格式裁定: `format_convert` 最后一个显式指定 > 输出扩展名 > 输入格式;
    /// 输出扩展名若映射到已知图像格式却与目标不符 = 拒绝即帧 (不写驴唇马嘴扩展名)。
    fn resolve_output_format(
        &self,
        operations: &[ImageOperation],
        output: &str,
        input_format: ImageFormatKind,
    ) -> Result<ImageFormatKind, ImageOpsError> {
        let explicit = operations
            .iter()
            .rev()
            .find_map(|operation| match operation {
                ImageOperation::FormatConvert { format } => Some(*format),
                _ => None,
            });
        let extension = Path::new(output)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase);
        if let Some(extension) = &extension {
            match ImageFormat::from_path(Path::new(output)) {
                Ok(path_format) => match ImageFormatKind::from_image_format(path_format) {
                    Some(extension_format) => {
                        let target = explicit.unwrap_or(extension_format);
                        if target != extension_format {
                            return Err(ImageOpsError::ParamRejected(format!(
                                "output extension implies {} but target format is {}; \
                                 扩展名与目标格式不符",
                                extension_format.name(),
                                target.name()
                            )));
                        }
                        return Ok(target);
                    }
                    None => {
                        return Err(ImageOpsError::FormatRejected(format!(
                            "output extension .{extension} maps to a non-whitelisted format \
                             (whitelist {:?}); 白名单外扩展名拒绝",
                            SUPPORTED_FORMATS
                        )))
                    }
                },
                Err(_) => {
                    // 未知扩展名 (非图像后缀): 目标格式由显式指定或输入格式决定。
                }
            }
        }
        Ok(explicit.unwrap_or(input_format))
    }

    /// 编码整幅在内存完成 (编码失败 = 拒绝即帧, 不落盘不留半成品)。
    fn encode(&self, image: &RgbaImage, format: ImageFormatKind) -> Result<Vec<u8>, ImageOpsError> {
        let (width, height) = image.dimensions();
        let mut out = Vec::new();
        let result = match format {
            ImageFormatKind::Png => PngEncoder::new(&mut out)
                .write_image(
                    image.as_raw(),
                    width,
                    height,
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|error| error.to_string()),
            ImageFormatKind::Jpeg => {
                let rgb = composite_on_white(image);
                JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
                    .write_image(&rgb, width, height, image::ExtendedColorType::Rgb8)
                    .map_err(|error| error.to_string())
            }
            ImageFormatKind::Webp => WebPEncoder::new_lossless(&mut out)
                .write_image(
                    image.as_raw(),
                    width,
                    height,
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|error| error.to_string()),
            ImageFormatKind::Bmp => BmpEncoder::new(&mut out)
                .write_image(
                    image.as_raw(),
                    width,
                    height,
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|error| error.to_string()),
        };
        result.map_err(|detail| {
            ImageOpsError::EncodeFailed(format!("encode to {} failed ({detail})", format.name()))
        })?;
        Ok(out)
    }

    /// 一次调用的完整执行面: 校验 → 读 → 解码 → 管道 → 编码 → 受控原子写。
    pub fn run(&self, params: &ImageOpsParams) -> Result<ImageOpsReport, ImageOpsError> {
        // ① 参数校验先于任何 IO: 词表/参数集合/上限逐项核对, 拒绝即帧。
        if params.input.trim().is_empty() || params.output.trim().is_empty() {
            return Err(ImageOpsError::ParamRejected(
                "input/output must not be empty; 输入与输出路径都必填".to_string(),
            ));
        }
        if params.operations.is_empty() {
            return Err(ImageOpsError::ParamRejected(
                "operations must not be empty (至少一步, 词表闭合); 空管道拒绝".to_string(),
            ));
        }
        if params.operations.len() > MAX_PIPELINE_STEPS {
            return Err(ImageOpsError::ParamRejected(format!(
                "operations exceeds the {MAX_PIPELINE_STEPS} step cap; 管道步数超上限"
            )));
        }
        let operations: Vec<ImageOperation> = params
            .operations
            .iter()
            .map(ImageOperation::parse)
            .collect::<Result<_, _>>()?;

        // ② 读面 (工作区限域 + 观测记录 + 大小上限 + 格式白名单)。
        let (canonical_input, bytes, input_format) = self.read_input(&params.input)?;

        // ③ 解码 (炸弹图三层上限)。
        let mut image = self.decode(&bytes)?;
        let input_size = image.dimensions();

        // ④ 操作管道: 顺序应用, 中途拒绝 = 全无产出 (目标原状)。
        for operation in &operations {
            apply_operation(&mut image, operation)?;
        }

        // ⑤ 输出格式裁定 (显式 > 扩展名 > 输入)。
        let output_format =
            self.resolve_output_format(&operations, &params.output, input_format)?;

        // ⑥ 写面 (工作区限域 + 敏感面拒) + 编码 + 受控原子写。
        let canonical_output = self.resolve_contained(&params.output, false)?;
        let encoded = self.encode(&image, output_format)?;
        let root = self.canonical_root()?;
        let output_rel = self.relative_display(&canonical_output, &root);

        let current = FsVersionProbe.version(&canonical_output);
        let overwrote = current.is_some();
        let request = WriteRequest {
            path: canonical_output.clone(),
            kind: if overwrote {
                WriteKind::Replace
            } else {
                WriteKind::Create
            },
            replace_if_version: None,
            create_if_absent: true,
        };
        gated_write_atomic(
            &self.gate,
            &FsVersionProbe,
            &request,
            &encoded,
            apeireth_core::storage_atomic::DEFAULT_FILE_MODE,
        )
        .map_err(|error| match error {
            GatedWriteError::Denied(denial) => {
                ImageOpsError::WriteDenied(self.redact_paths(&denial.message()))
            }
            GatedWriteError::Io(detail) => ImageOpsError::WriteFailed(self.redact_paths(&detail)),
        })?;

        let input_rel = self.relative_display(&canonical_input, &root);
        Ok(ImageOpsReport {
            input: input_rel,
            output: output_rel,
            input_format,
            output_format,
            input_size,
            output_size: image.dimensions(),
            operations: operations
                .iter()
                .map(|operation| operation.name().to_string())
                .collect(),
            overwrote,
            bytes_written: encoded.len() as u64,
        })
    }

    fn frame_result(&self, call: &ToolCall, error: &ImageOpsError) -> ToolResult {
        ToolResult::permanent_error(&call.id, self.redact_paths(&error.frame()))
    }
}

#[async_trait]
impl ToolCapability for ImageOpsTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let parameters = serde_json::json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Input image path (workspace-relative, or absolute inside the workspace)"
                },
                "output": {
                    "type": "string",
                    "description": "Output image path (workspace-relative, or absolute inside the workspace)"
                },
                "operations": {
                    "type": "array",
                    "description": "Operation pipeline applied in order (max 32 steps). Closed vocabulary: resize, crop, rotate, flip, format_convert, brightness, contrast, grayscale, blur, sharpen, watermark_text",
                    "items": {
                        "type": "object",
                        "properties": {
                            "op": {
                                "type": "string",
                                "enum": [
                                    "resize", "crop", "rotate", "flip", "format_convert",
                                    "brightness", "contrast", "grayscale", "blur", "sharpen",
                                    "watermark_text"
                                ]
                            },
                            "width": { "type": "integer" },
                            "height": { "type": "integer" },
                            "x": { "type": "integer" },
                            "y": { "type": "integer" },
                            "angle_deg": { "type": "integer", "enum": [90, 180, 270] },
                            "axis": { "type": "string", "enum": ["horizontal", "vertical"] },
                            "format": { "type": "string", "enum": ["png", "jpeg", "webp", "bmp"] },
                            "amount": { "type": "integer", "minimum": -255, "maximum": 255 },
                            "factor": { "type": "number", "minimum": 0.0, "maximum": 3.0 },
                            "sigma": { "type": "number", "exclusiveMinimum": 0.0, "maximum": 32.0 },
                            "threshold": { "type": "integer", "minimum": 0, "maximum": 255 },
                            "text": { "type": "string", "minLength": 1, "maxLength": 64 },
                            "scale": { "type": "integer", "minimum": 1, "maximum": 8 }
                        },
                        "required": ["op"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["input", "output", "operations"],
            "additionalProperties": false
        });
        let mut params = apeireth_protocol::canonical::ToolParameters::new();
        params.extend(parameters.as_object().cloned().unwrap_or_default());

        NormalizedTool::new(IMAGE_OPS_TOOL_NAME)
            .with_description(
                "Controlled image editing for the agent: read one workspace image, apply an \
                 ordered operation pipeline (resize, crop, rotate, flip, format_convert, \
                 brightness, contrast, grayscale, blur, sharpen, watermark_text), and write the \
                 result back into the workspace as png/jpeg/webp/bmp. Everything is \
                 workspace-scoped, credential/key paths are refused, oversized or bomb-like \
                 images are rejected, and the write is atomic (temp file + rename) with \
                 read-before-overwrite observation gating.",
            )
            .with_parameters(params)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.activity.note_call();
        let params: ImageOpsParams = match serde_json::from_value(call.arguments.clone()) {
            Ok(params) => params,
            Err(error) => {
                self.activity.record_denied();
                return self.frame_result(
                    call,
                    &ImageOpsError::ParamRejected(format!("invalid image_ops parameters: {error}")),
                );
            }
        };
        match self.run(&params) {
            Ok(report) => {
                self.activity.record_applied(&report.operations);
                ToolResult::ok(&call.id, report.to_json()).with_name(IMAGE_OPS_TOOL_NAME)
            }
            Err(error) => {
                self.activity.record_denied();
                self.frame_result(call, &error)
            }
        }
    }
}

/// 逐操作像素应用 (顺序语义: 前一步的产物是后一步的输入; 几何类越界拒绝即帧)。
fn apply_operation(image: &mut RgbaImage, operation: &ImageOperation) -> Result<(), ImageOpsError> {
    match operation {
        ImageOperation::Resize { width, height } => {
            *image = imageops::resize(image, *width, *height, FilterType::Triangle);
        }
        ImageOperation::Crop {
            x,
            y,
            width,
            height,
        } => {
            let (image_width, image_height) = image.dimensions();
            if *x >= image_width
                || *y >= image_height
                || x.saturating_add(*width) > image_width
                || y.saturating_add(*height) > image_height
            {
                return Err(ImageOpsError::ParamRejected(format!(
                    "crop: 矩形 ({}x{} @ {}, {}) 超出图幅 {}x{}; 裁剪越界拒绝",
                    width, height, x, y, image_width, image_height
                )));
            }
            *image = imageops::crop_imm(image, *x, *y, *width, *height).to_image();
        }
        ImageOperation::Rotate { angle_deg } => {
            *image = match angle_deg {
                90 => imageops::rotate90(image),
                180 => imageops::rotate180(image),
                _ => imageops::rotate270(image),
            };
        }
        ImageOperation::Flip { axis } => {
            *image = match axis {
                FlipAxis::Horizontal => imageops::flip_horizontal(image),
                FlipAxis::Vertical => imageops::flip_vertical(image),
            };
        }
        ImageOperation::FormatConvert { .. } => {
            // 只定目标编码格式, 不动像素 (编码在收尾一次完成)。
        }
        ImageOperation::Brightness { amount } => {
            let amount = *amount;
            for pixel in image.pixels_mut() {
                let [r, g, b, a] = pixel.0;
                pixel.0 = [
                    clamp_channel(r, amount),
                    clamp_channel(g, amount),
                    clamp_channel(b, amount),
                    a,
                ];
            }
        }
        ImageOperation::Contrast { factor } => {
            let factor = *factor;
            for pixel in image.pixels_mut() {
                let [r, g, b, a] = pixel.0;
                pixel.0 = [
                    stretch_channel(r, factor),
                    stretch_channel(g, factor),
                    stretch_channel(b, factor),
                    a,
                ];
            }
        }
        ImageOperation::Grayscale => {
            for pixel in image.pixels_mut() {
                let [r, g, b, a] = pixel.0;
                let luma = luma(r, g, b);
                pixel.0 = [luma, luma, luma, a];
            }
        }
        ImageOperation::Blur { sigma } => {
            *image = imageops::blur(image, *sigma);
        }
        ImageOperation::Sharpen { sigma, threshold } => {
            *image = imageops::unsharpen(image, *sigma, *threshold);
        }
        ImageOperation::WatermarkText { text, x, y, scale } => {
            draw_watermark(image, text, *x, *y, *scale)?;
        }
    }
    Ok(())
}

/// 亮度通道算术: 饱和加 (alpha 不动)。
fn clamp_channel(value: u8, amount: i32) -> u8 {
    i32::from(value)
        .saturating_add(amount)
        .clamp(0, 255)
        .try_into()
        .unwrap_or(255)
}

/// 对比度通道算术: 以 128 为轴线性拉伸 (四舍五入, alpha 不动)。
fn stretch_channel(value: u8, factor: f64) -> u8 {
    let stretched = (f64::from(value) - 128.0).mul_add(factor, 128.0).round();
    stretched.clamp(0.0, 255.0) as u8
}

/// 灰度整数亮度公式: (77r + 150g + 29b + 128) >> 8 (≈ 0.299/0.587/0.114)。
fn luma(r: u8, g: u8, b: u8) -> u8 {
    let weighted = 77 * u32::from(r) + 150 * u32::from(g) + 29 * u32::from(b) + 128;
    (weighted >> 8).min(255) as u8
}

/// jpeg 无 alpha: 白底合成 (透明像素变白, 不留黑底)。
fn composite_on_white(image: &RgbaImage) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(image.as_raw().len() / 4 * 3);
    for Rgba([r, g, b, a]) in image.pixels() {
        let alpha = f64::from(*a) / 255.0;
        let blend =
            |channel: u8| (f64::from(channel) * alpha + 255.0 * (1.0 - alpha)).round() as u8;
        rgb.push(blend(*r));
        rgb.push(blend(*g));
        rgb.push(blend(*b));
    }
    rgb
}

/// 文字水印: 内置 5×7 点阵字模, 白色不透明像素按 scale×scale 方块落笔;
/// 字模框必须整框落在图内 (越界拒绝即帧), 框外像素一个不动。
fn draw_watermark(
    image: &mut RgbaImage,
    text: &str,
    x: u32,
    y: u32,
    scale: u32,
) -> Result<(), ImageOpsError> {
    let glyph_count = text.chars().count() as u32;
    let box_width = glyph_count.saturating_mul(6).saturating_mul(scale);
    let box_height = 7u32.saturating_mul(scale);
    let (image_width, image_height) = image.dimensions();
    if x.saturating_add(box_width) > image_width || y.saturating_add(box_height) > image_height {
        return Err(ImageOpsError::ParamRejected(format!(
            "watermark_text: 字框 {}x{} @ {}, {} 超出图幅 {}x{}; 字框越界拒绝",
            box_width, box_height, x, y, image_width, image_height
        )));
    }
    let white = Rgba([255u8, 255, 255, 255]);
    for (index, ch) in text.chars().enumerate() {
        let glyph_x = x + index as u32 * 6 * scale;
        for (row, row_pixels) in glyph_rows(ch).iter().enumerate() {
            for (column, marker) in row_pixels.chars().enumerate() {
                if marker != '#' {
                    continue;
                }
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = glyph_x + column as u32 * scale + dx;
                        let py = y + row as u32 * scale + dy;
                        if px < image_width && py < image_height {
                            image.put_pixel(px, py, white);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// 字模支持面: 可打印 ASCII 0x20..=0x7E (覆盖 = 字形表全集, 由单元测试钉死)。
fn is_supported_glyph_char(ch: char) -> bool {
    (0x20..=0x7E).contains(&(ch as u32))
}

/// 取字模 (7 行 × 5 列, `#` = 落笔); 表内每字形都由覆盖测试钉形状合法。
fn glyph_rows(ch: char) -> [&'static str; 7] {
    for (glyph, rows) in FONT_5X7 {
        if *glyph == ch {
            let mut out = [""; 7];
            for (slot, row) in rows.split('/').enumerate() {
                if slot < 7 {
                    out[slot] = row;
                }
            }
            return out;
        }
    }
    ["....."; 7]
}

/// 内置 5×7 点阵字模表: 每行 = (字符, 7 行 × 5 列, `/` 分行, `#` 落笔 `.` 空)。
/// 覆盖可打印 ASCII 0x20..=0x7E (覆盖完整性与形状合法性由单元测试钉死);
/// 字形是本仓库自带的点阵描绘, 不引任何字体文件或字体渲染依赖。
#[rustfmt::skip]
const FONT_5X7: &[(char, &str)] = &[
    (' ', "...../...../...../...../...../...../....."),
    ('!', "..#../..#../..#../..#../..#../...../..#.."),
    ('"', ".#.#./.#.#./...../...../...../...../....."),
    ('#', ".#.#./#####/.#.#./#####/.#.#./...../....."),
    ('$', "..#../.####/#.#../.###./..#.#/####./..#.."),
    ('%', "##..#/##..#/...#./..#../.#.../#..##/#..##"),
    ('&', ".##../#..#./#.#../.#.../#.#.#/#..#./.##.#"),
    ('\'', "..#../..#../...../...../...../...../....."),
    ('(', "...#./..#../.#.../.#.../.#.../..#../...#."),
    (')', ".#.../..#../...#./...#./...#./..#../.#..."),
    ('*', "...../#.#.#/.###./#####/.###./#.#.#/....."),
    ('+', "...../..#../..#../#####/..#../..#../....."),
    (',', "...../...../...../...../..##./..#../.#..."),
    ('-', "...../...../...../#####/...../...../....."),
    ('.', "...../...../...../...../...../.##../.##.."),
    ('/', "....#/...#./...#./..#../.#.../.#.../#...."),
    ('0', ".###./#...#/##..#/#.#.#/#..##/#...#/.###."),
    ('1', "..#../.##../..#../..#../..#../..#../.###."),
    ('2', ".###./#...#/....#/...#./..#../.#.../#####"),
    ('3', "#####/...#./..#../...#./....#/#...#/.###."),
    ('4', "...#./..##./.#.#./#..#./#####/...#./...#."),
    ('5', "#####/#..../####./....#/....#/#...#/.###."),
    ('6', "..##./.#.../#..../####./#...#/#...#/.###."),
    ('7', "#####/....#/...#./..#../.#.../.#.../.#..."),
    ('8', ".###./#...#/#...#/.###./#...#/#...#/.###."),
    ('9', ".###./#...#/#...#/.####/....#/...#./.##.."),
    (':', "...../.##../.##../...../.##../.##../....."),
    (';', "...../.##../.##../...../.##../..#../.#..."),
    ('<', "...#./..#../.#.../#..../.#.../..#../...#."),
    ('=', "...../...../#####/...../#####/...../....."),
    ('>', ".#.../..#../...#./....#/...#./..#../.#..."),
    ('?', ".###./#...#/....#/...#./..#../...../..#.."),
    ('@', ".###./#...#/#.###/#.#.#/#.###/#..../.###."),
    ('A', ".###./#...#/#...#/#####/#...#/#...#/#...#"),
    ('B', "####./#...#/#...#/####./#...#/#...#/####."),
    ('C', ".###./#...#/#..../#..../#..../#...#/.###."),
    ('D', "####./#...#/#...#/#...#/#...#/#...#/####."),
    ('E', "#####/#..../#..../####./#..../#..../#####"),
    ('F', "#####/#..../#..../####./#..../#..../#...."),
    ('G', ".###./#...#/#..../#..##/#...#/#...#/.####"),
    ('H', "#...#/#...#/#...#/#####/#...#/#...#/#...#"),
    ('I', ".###./..#../..#../..#../..#../..#../.###."),
    ('J', "....#/....#/....#/....#/#...#/#...#/.###."),
    ('K', "#...#/#..#./#.#../##.../#.#../#..#./#...#"),
    ('L', "#..../#..../#..../#..../#..../#..../#####"),
    ('M', "#...#/##.##/#.#.#/#.#.#/#...#/#...#/#...#"),
    ('N', "#...#/##..#/#.#.#/#..##/#...#/#...#/#...#"),
    ('O', ".###./#...#/#...#/#...#/#...#/#...#/.###."),
    ('P', "####./#...#/#...#/####./#..../#..../#...."),
    ('Q', ".###./#...#/#...#/#...#/#.#.#/#..#./.##.#"),
    ('R', "####./#...#/#...#/####./#.#../#..#./#...#"),
    ('S', ".####/#..../#..../.###./....#/....#/####."),
    ('T', "#####/..#../..#../..#../..#../..#../..#.."),
    ('U', "#...#/#...#/#...#/#...#/#...#/#...#/.###."),
    ('V', "#...#/#...#/#...#/#...#/#...#/.#.#./..#.."),
    ('W', "#...#/#...#/#...#/#.#.#/#.#.#/##.##/#...#"),
    ('X', "#...#/#...#/.#.#./..#../.#.#./#...#/#...#"),
    ('Y', "#...#/#...#/.#.#./..#../..#../..#../..#.."),
    ('Z', "#####/....#/...#./..#../.#.../#..../#####"),
    ('[', ".###./.#.../.#.../.#.../.#.../.#.../.###."),
    ('\\', "#..../.#.../.#.../..#../...#./...#./....#"),
    (']', ".###./...#./...#./...#./...#./...#./.###."),
    ('^', "..#../.#.#./#...#/...../...../...../....."),
    ('_', "...../...../...../...../...../...../#####"),
    ('`', ".#.../..#../...../...../...../...../....."),
    ('a', "...../...../.###./....#/.####/#...#/.####"),
    ('b', "#..../#..../####./#...#/#...#/#...#/####."),
    ('c', "...../...../.###./#..../#..../#...#/.###."),
    ('d', "....#/....#/.####/#...#/#...#/#...#/.####"),
    ('e', "...../...../.###./#...#/#####/#..../.###."),
    ('f', "..##./.#..#/.#.../####./.#.../.#.../.#..."),
    ('g', "...../.####/#...#/#...#/.####/....#/.###."),
    ('h', "#..../#..../####./#...#/#...#/#...#/#...#"),
    ('i', "..#../...../.##../..#../..#../..#../.###."),
    ('j', "...#./...../...##/...#./...#./#..#./.##.."),
    ('k', "#..../#..../#..#./#.#../##.../#.#../#..#."),
    ('l', ".##../..#../..#../..#../..#../..#../.###."),
    ('m', "...../...../##.#./#.#.#/#.#.#/#...#/#...#"),
    ('n', "...../...../####./#...#/#...#/#...#/#...#"),
    ('o', "...../...../.###./#...#/#...#/#...#/.###."),
    ('p', "...../####./#...#/#...#/####./#..../#...."),
    ('q', "...../.####/#...#/#...#/.####/....#/....#"),
    ('r', "...../...../#.##./##..#/#..../#..../#...."),
    ('s', "...../...../.####/#..../.###./....#/####."),
    ('t', ".#.../.#.../####./.#.../.#.../.#..#/..##."),
    ('u', "...../...../#...#/#...#/#...#/#..##/.##.#"),
    ('v', "...../...../#...#/#...#/#...#/.#.#./..#.."),
    ('w', "...../...../#...#/#...#/#.#.#/#.#.#/.#.#."),
    ('x', "...../...../#...#/.#.#./..#../.#.#./#...#"),
    ('y', "...../#...#/#...#/#...#/.####/....#/.###."),
    ('z', "...../...../#####/...#./..#../.#.../#####"),
    ('{', "...##/..#../..#../.#.../..#../..#../...##"),
    ('|', "..#../..#../..#../..#../..#../..#../..#.."),
    ('}', "##.../..#../..#../...#./..#../..#../##..."),
    ('~', "...../...../.#.../#.#.#/...#./...../....."),
];

/// 图像操作面开关 (`APEIRETH_ENABLE_IMAGE_OPS`) 实况: 只认显式 `"1"`
/// (前后空白容忍), 其余任何取值 = 关 (默认关 fail-closed)。
pub fn image_ops_enabled_from_env() -> bool {
    std::env::var(IMAGE_OPS_ENABLE_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(op: &str) -> OperationSpec {
        OperationSpec {
            op: op.to_string(),
            ..OperationSpec::default()
        }
    }

    /// 开关 fail-closed: 只认显式 "1", 其余取值一律关。
    #[test]
    fn image_ops_switch_fails_closed_on_anything_but_one() {
        std::env::remove_var(IMAGE_OPS_ENABLE_ENV);
        assert!(!image_ops_enabled_from_env(), "未设 = 关");
        for value in ["0", "true", "yes", "on", " 2 ", "11", "1x"] {
            std::env::set_var(IMAGE_OPS_ENABLE_ENV, value);
            assert!(!image_ops_enabled_from_env(), "must refuse {value:?}");
        }
        std::env::set_var(IMAGE_OPS_ENABLE_ENV, "1");
        assert!(image_ops_enabled_from_env());
        std::env::set_var(IMAGE_OPS_ENABLE_ENV, " 1 ");
        assert!(image_ops_enabled_from_env(), "前后空白容忍");
        std::env::remove_var(IMAGE_OPS_ENABLE_ENV);
    }

    /// 词表闭合: 十一名成员逐一可解析 (缺参即拒, 无参者合法), 表外名拒绝即帧。
    #[test]
    fn operation_vocabulary_is_closed_and_parses() {
        assert_eq!(
            OPERATION_VOCABULARY.len(),
            11,
            "词表冻结为十一项, 增删须显式改表并补测试"
        );
        for name in OPERATION_VOCABULARY {
            let bare = ImageOperation::parse(&spec(name));
            if name == "grayscale" {
                // 唯一无参操作: 空参即合法。
                assert_eq!(bare, Ok(ImageOperation::Grayscale), "grayscale 无参合法");
            } else {
                assert!(
                    bare.is_err(),
                    "{name} 缺参数必须被拒 (参数校验先于解析成功)"
                );
            }
        }
        let mut unknown = spec("rotate_all");
        unknown.angle_deg = Some(90);
        let error = ImageOperation::parse(&unknown).unwrap_err();
        assert!(
            matches!(error, ImageOpsError::ParamRejected(_)),
            "表外操作名必须拒绝即帧"
        );
    }

    /// 字形表覆盖可打印 ASCII 全集, 且每个字形恰为 7 行 × 5 列。
    #[test]
    fn font_table_covers_printable_ascii_with_well_formed_glyphs() {
        for code in 0x20u32..=0x7E {
            let ch = char::from_u32(code).expect("printable ascii");
            let mut found = false;
            for (glyph, rows) in FONT_5X7 {
                if *glyph != ch {
                    continue;
                }
                found = true;
                let parts: Vec<&str> = rows.split('/').collect();
                assert_eq!(parts.len(), 7, "{ch:?} 必须 7 行");
                for row in parts {
                    assert_eq!(row.len(), 5, "{ch:?} 每行必须 5 列");
                    assert!(
                        row.chars().all(|c| c == '#' || c == '.'),
                        "{ch:?} 只收 #/. 描绘"
                    );
                }
            }
            assert!(found, "{ch:?} (0x{code:02X}) 缺字形");
        }
        assert_eq!(
            FONT_5X7.len(),
            0x7F - 0x20,
            "字形表恰好覆盖 95 个可打印字符"
        );
    }

    /// 操作参数矩阵: 每操作的缺参 / 多余参 / 越界值拒绝即帧, 理由点名字段。
    #[test]
    fn parameter_validation_matrix_rejects_every_malformed_operation() {
        // 缺参。
        let mut resize_missing = spec("resize");
        resize_missing.width = Some(2);
        let error = ImageOperation::parse(&resize_missing).unwrap_err();
        assert!(error.frame().contains("height"), "{}", error.frame());

        // 多余参。
        let mut grayscale_extra = spec("grayscale");
        grayscale_extra.amount = Some(10);
        let error = ImageOperation::parse(&grayscale_extra).unwrap_err();
        assert!(error.frame().contains("amount"), "{}", error.frame());

        // 越界值矩阵。
        let mut rotate = spec("rotate");
        rotate.angle_deg = Some(45);
        assert!(ImageOperation::parse(&rotate).is_err());
        let mut flip = spec("flip");
        flip.axis = Some("diagonal".to_string());
        assert!(ImageOperation::parse(&flip).is_err());
        let mut format = spec("format_convert");
        format.format = Some("gif".to_string());
        assert!(ImageOperation::parse(&format).is_err());
        let mut brightness = spec("brightness");
        brightness.amount = Some(256);
        assert!(ImageOperation::parse(&brightness).is_err());
        let mut contrast = spec("contrast");
        contrast.factor = Some(3.5);
        assert!(ImageOperation::parse(&contrast).is_err());
        let mut blur = spec("blur");
        blur.sigma = Some(0.0);
        assert!(ImageOperation::parse(&blur).is_err());
        let mut sharpen = spec("sharpen");
        sharpen.sigma = Some(1.0);
        sharpen.threshold = Some(300);
        assert!(ImageOperation::parse(&sharpen).is_err());
        let mut resize = spec("resize");
        resize.width = Some(0);
        resize.height = Some(2);
        assert!(ImageOperation::parse(&resize).is_err());
        let mut watermark = spec("watermark_text");
        watermark.text = Some("ok".to_string());
        watermark.x = Some(0);
        watermark.y = Some(0);
        watermark.scale = Some(9);
        assert!(ImageOperation::parse(&watermark).is_err());
    }
}
