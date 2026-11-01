//! ⭐ **纹理上传（GPU 侧）** —— 台账 L1-G7 的最后一段。
//!
//! ## 为什么需要单独一层「行对齐」逻辑
//!
//! `wgpu` 要求 `Queue::write_texture` 的 `bytes_per_row` 是
//! [`COPY_BYTES_PER_ROW_ALIGNMENT`]（**256 字节**）的整数倍
//! ⇒ ⇒ 而我们的 RGBA 数据是 `width × 4` 字节一行，几乎**永远不是** 256 的倍数
//! （`width = 100` ⇒ 400 字节，不是 256 的倍数）
//! ⇒ ⇒ 直接把原数据传上去会**panic 或校验失败**（表现为"图片传不上去"，
//! 且不同宽度下表现不同 —— 典型的隐藏缺陷）。
//!
//! ⇒ ⇒ 所以本模块把「行对齐」做成**纯函数**（可 native 测试），上传时才用它。
//!
//! ## 边界（0 装纪律）
//!
//! - 本模块**不做解码**（RGBA 由宿主提供，见 `apeireth_ui_dom::resource::ImageData`）；
//! - 本模块**不决定**采样参数/纹理格式策略（由渲染层定，这里固定 `Rgba8UnormSrgb` 并写明）；
//! - 上传需要真实 `wgpu::Device`/`Queue` ⇒ ⇒ **GPU 路径本身无法在 native 单测**，
//!   本模块的 native 测试只覆盖**纯逻辑**（对齐、行拷贝、非法输入）；GPU 路径靠浏览器证据。

use std::collections::BTreeMap;

/// `wgpu` 的行对齐常量（256 字节）。
///
/// 直接写字面量而不是引用 `wgpu::COPY_BYTES_PER_ROW_ALIGNMENT`：
/// 该常量在部分版本里位于 `wgpu::util`，引用它会把本模块的**纯逻辑**绑死在 wgpu 上
/// ⇒ ⇒ 纯函数要能在**不启用 GPU 特性**时也能编译与测试。
pub const COPY_BYTES_PER_ROW_ALIGNMENT: u32 = 256;

/// 每像素字节数（RGBA8）。
pub const BYTES_PER_PIXEL: u32 = 4;

/// 一次上传的**行布局**（纯计算结果）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowLayout {
    /// 一行真实字节数（`width × 4`）。
    pub row_bytes: u32,
    /// 对齐后的一行字节数（256 的倍数）。
    pub padded_row_bytes: u32,
    /// 对齐后的总字节数（`padded_row_bytes × height`）。
    pub padded_len: usize,
}

impl RowLayout {
    /// 需要的行数（= 高度；保留这个访问器是为了让调用点不必自己持有 height）。
    pub fn rows(&self) -> usize {
        if self.padded_row_bytes == 0 {
            0
        } else {
            self.padded_len / self.padded_row_bytes as usize
        }
    }

    /// 每行尾部填充的字节数。
    pub fn padding_per_row(&self) -> u32 {
        self.padded_row_bytes - self.row_bytes
    }
}

/// 计算行布局；尺寸为 0 或溢出 ⇒ `Err`（**不**静默返回空布局）。
pub fn row_layout(width: u32, height: u32) -> Result<RowLayout, String> {
    if width == 0 || height == 0 {
        return Err(String::from("texture: 尺寸为 0（拒绝上传）"));
    }
    let row_bytes = width
        .checked_mul(BYTES_PER_PIXEL)
        .ok_or_else(|| String::from("texture: 行字节数溢出"))?;
    let padded_row_bytes =
        row_bytes.div_ceil(COPY_BYTES_PER_ROW_ALIGNMENT) * COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_len = (padded_row_bytes as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| String::from("texture: 总字节数溢出"))?;
    Ok(RowLayout {
        row_bytes,
        padded_row_bytes,
        padded_len,
    })
}

/// 把紧凑的 RGBA 数据**按行**拷进对齐缓冲（尾部填 0）。
///
/// ⚠️ 用「逐行拷贝」而不是「整块 memcpy」：整块拷贝会把第 2 行开始的数据**错位**
/// （每行少 padding 个字节）⇒ ⇒ 画面出现**斜向撕裂**，而且只在宽度非 256 倍数时出现。
pub fn pad_rows(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let layout = row_layout(width, height)?;
    let expected = layout.row_bytes as usize * height as usize;
    if rgba.len() != expected {
        return Err(format!(
            "texture: 字节数不匹配（期望 {expected}，实际 {}）",
            rgba.len()
        ));
    }
    let mut padded = vec![0u8; layout.padded_len];
    let src_row = layout.row_bytes as usize;
    let dst_row = layout.padded_row_bytes as usize;
    for row in 0..height as usize {
        let src_start = row * src_row;
        let dst_start = row * dst_row;
        padded[dst_start..dst_start + src_row]
            .copy_from_slice(&rgba[src_start..src_start + src_row]);
    }
    Ok(padded)
}

/// 一张已上传的纹理（只保留视图：渲染层用它建 bind group）。
#[derive(Debug)]
pub struct UploadedTexture {
    /// 纹理视图（渲染层采样用）。
    pub view: wgpu::TextureView,
    /// 宽（像素）。
    pub width: u32,
    /// 高（像素）。
    pub height: u32,
}

/// ⭐ **纹理仓库**：按资源编号保存纹理视图。
///
/// ⚠️ 与 `apeireth_ui_dom::resource::TextureUploader` 的**对接**需要一个适配器
/// （`ui-demo`/`ui-kit` 这类同时依赖两边的 crate 来做）
/// ⇒ ⇒ 因为让 **ui-gpu 依赖 ui-dom** 会把分层方向倒过来（GPU 是更底层）
/// ⇒ ⇒ 这条内部依赖边若确实需要，应走 ADR，不在本批擅自添加。
#[derive(Debug, Default)]
pub struct TextureStore {
    textures: BTreeMap<u64, UploadedTexture>,
}

impl TextureStore {
    /// 建空仓库。
    pub fn new() -> Self {
        Self {
            textures: BTreeMap::new(),
        }
    }

    /// 上传一张 RGBA 纹理（失败返回原因，**不得**静默忽略）。
    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        id: u64,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), String> {
        let layout = row_layout(width, height)?;
        let padded = pad_rows(rgba, width, height)?;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("apeireth-ui texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &padded,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(layout.padded_row_bytes),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.textures.insert(
            id,
            UploadedTexture {
                view,
                width,
                height,
            },
        );
        Ok(())
    }

    /// 取已上传的纹理。
    pub fn get(&self, id: u64) -> Option<&UploadedTexture> {
        self.textures.get(&id)
    }

    /// 已上传数量。
    pub fn len(&self) -> usize {
        self.textures.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 宽度 100（400 字节/行）⇒ 对齐到 512。
    #[test]
    fn row_layout_pads_to_256_alignment() {
        let layout = row_layout(100, 3).expect("合法尺寸");
        assert_eq!(layout.row_bytes, 400);
        assert_eq!(layout.padded_row_bytes, 512);
        assert_eq!(layout.padding_per_row(), 112);
        assert_eq!(layout.padded_len, 512 * 3);
        assert_eq!(layout.rows(), 3);
    }

    /// 恰好对齐的宽度不应多填（256/4 = 64 像素一行正好 256 字节）。
    #[test]
    fn exact_multiple_needs_no_padding() {
        let layout = row_layout(64, 2).expect("合法尺寸");
        assert_eq!(layout.row_bytes, 256);
        assert_eq!(layout.padded_row_bytes, 256);
        assert_eq!(layout.padding_per_row(), 0);
        assert_eq!(layout.padded_len, 512);
    }

    /// 1 像素宽 = 4 字节 ⇒ 对齐到 256。
    #[test]
    fn single_pixel_row_is_heavily_padded() {
        let layout = row_layout(1, 4).expect("合法尺寸");
        assert_eq!(layout.row_bytes, 4);
        assert_eq!(layout.padded_row_bytes, 256);
        assert_eq!(layout.padded_len, 1024);
    }

    /// 尺寸为 0 / 溢出 ⇒ 显式 `Err`（不静默）。
    #[test]
    fn illegal_dimensions_are_rejected() {
        assert!(row_layout(0, 10).is_err());
        assert!(row_layout(10, 0).is_err());
        assert!(row_layout(u32::MAX, u32::MAX).is_err());
    }

    /// ⭐ **逐行拷贝**：每行的数据必须落在对齐行起点（否则画面斜向撕裂）。
    #[test]
    fn rows_are_copied_to_aligned_starts() {
        // 2 像素宽 ⇒ row_bytes = 8，padded = 256
        let rgba = vec![
            1, 2, 3, 4, 5, 6, 7, 8, // 第 0 行
            9, 10, 11, 12, 13, 14, 15, 16, // 第 1 行
        ];
        let padded = pad_rows(&rgba, 2, 2).expect("合法输入");
        assert_eq!(padded.len(), 512);
        assert_eq!(&padded[0..8], &rgba[0..8], "第 0 行必须在起点");
        assert_eq!(&padded[256..264], &rgba[8..16], "第 1 行必须在 256 偏移处");
        assert!(padded[8..256].iter().all(|byte| *byte == 0), "填充必须是 0");
    }

    /// 字节数不匹配 ⇒ 拒绝（不把坏数据传进 GPU）。
    #[test]
    fn mismatched_length_is_rejected() {
        // 2×2 RGBA ⇒ 期望 16 字节
        assert!(pad_rows(&[0u8; 15], 2, 2).is_err());
        assert!(pad_rows(&[0u8; 16], 2, 2).is_ok());
    }
}
