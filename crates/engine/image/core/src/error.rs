// Copyright (c) 2026 The Apeireth Authors.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! 接口层错误面：把下层引擎家族的失败统一成一个错误枚举。

use thiserror::Error;

/// 图像核心接口层的错误。
#[derive(Debug, Error)]
pub enum ImageError {
    /// 编解码失败（格式识别/解析/写出）。
    #[error("image codec error: {0}")]
    Codec(#[from] apeireth_image_codecs::CodecError),
    /// 分层文档（PSD/PSB 面）失败。
    #[error("layered document error: {0}")]
    Document(#[from] apeireth_image_psd::PsdError),
    /// 像素面分配失败（引擎面的可失败分配被拒绝）。
    #[error("pixel surface allocation failed: {0}")]
    Allocation(String),
    /// 门面层第一批未覆盖的输入面（如实拒绝，不静默降级）。
    #[error("not covered by the facade layer yet: {0}")]
    Unsupported(String),
    /// 参数不合法。
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}
