//! ⭐ **资源句柄与加载状态** —— 台账 L1-G7 的第一片。
//!
//! ## 为什么归引擎（而策略归宿主）
//!
//! 图片/字体加载**本身**是宿主的事（网络、缓存、鉴权、打包命名都属 L3）
//! ⇒ ⇒ 但「**资源还没准备好**」这个状态必须能被引擎表达，否则：
//!
//! - **字体未就绪 ⇒ 文本度量会变** ⇒ 按旧度量算出来的窗口/命中/滚动位置全错
//!   ⇒ ⇒ 用户看到**布局跳动**（CLS），而且**只在首次加载/弱网时复现**；
//! - 图片未就绪 ⇒ 占位尺寸与真实尺寸不一致 ⇒ 页面在图片到达时**整块跳**。
//!
//! ⇒ ⇒ 所以引擎需要三件东西：**资源句柄**（稳定身份）、**加载状态机**（Pending/Ready/Failed）、
//! 以及**「度量未定」**这个可判定的状态（[`MetricsStatus`]）。
//!
//! ## 边界（0 装纪律）
//!
//! - 本模块**不发起请求**、**不缓存**、**不做预加载清单**（那些属 L3）；
//! - 本模块**不决定**「未就绪时怎么画」（视觉策略属 L2/L3）；
//! - 本模块只保证：**状态可查询、可被引擎消费**，且「未知」**不会被当成「已知」**。

use std::collections::BTreeMap;

/// 资源句柄（稳定身份；由 [`ResourceRegistry::register`] 发出，**只增不复用**）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(u64);

impl ResourceId {
    /// 原始编号（判据/日志用）。
    pub fn raw(self) -> u64 {
        self.0
    }
}

/// 资源加载状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceState {
    /// 已登记、尚未完成（**默认**状态；不得当作"可用"）。
    Pending,
    /// 已就绪（可被绘制/度量）。
    Ready,
    /// 加载失败（**保留原因**，不得静默降级成 Pending 或 Ready）。
    Failed {
        /// 失败原因（宿主提供；引擎只透传）。
        reason: String,
    },
}

impl ResourceState {
    /// 是否可用（只有 `Ready` 可用）。
    pub fn is_ready(&self) -> bool {
        matches!(self, ResourceState::Ready)
    }

    /// 是否仍在等待。
    pub fn is_pending(&self) -> bool {
        matches!(self, ResourceState::Pending)
    }

    /// 是否失败。
    pub fn is_failed(&self) -> bool {
        matches!(self, ResourceState::Failed { .. })
    }
}

/// ⭐ **度量状态**：文本/字体度量是否已经确定。
///
/// 「未知」是一个**独立状态**，不是「已知但值为 0」
/// ⇒ ⇒ 引擎在未知时**必须拒绝**基于度量的计算（否则会用错误度量算出错误结果，
/// 而且错误会被缓存下来）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MetricsStatus {
    /// ⭐ **度量未定**（字体未加载完 / 度量尚未测量）。
    #[default]
    Unknown,
    /// 度量已确定（可以用它做布局/窗口/命中计算）。
    Ready,
}

impl MetricsStatus {
    /// 是否已确定。
    pub fn is_ready(self) -> bool {
        matches!(self, MetricsStatus::Ready)
    }
}

/// 资源登记表（**只增不删**：句柄一旦发出就必须一直能查询到状态，避免悬空句柄）。
#[derive(Debug, Default)]
pub struct ResourceRegistry {
    next_id: u64,
    states: BTreeMap<ResourceId, ResourceState>,
}

impl ResourceRegistry {
    /// 建一个空登记表。
    pub fn new() -> Self {
        Self {
            next_id: 0,
            states: BTreeMap::new(),
        }
    }

    /// 登记一个资源（返回句柄；初始状态 = `Pending`）。
    pub fn register(&mut self) -> ResourceId {
        let id = ResourceId(self.next_id);
        self.next_id += 1;
        self.states.insert(id, ResourceState::Pending);
        id
    }

    /// 查询状态（未知句柄 ⇒ `None`，**不**默认返回 Pending）。
    pub fn state(&self, id: ResourceId) -> Option<&ResourceState> {
        self.states.get(&id)
    }

    /// 标记就绪（未知句柄 ⇒ `Err`）。
    pub fn mark_ready(&mut self, id: ResourceId) -> Result<(), &'static str> {
        let state = self
            .states
            .get_mut(&id)
            .ok_or("resource: 未知句柄（未登记）")?;
        *state = ResourceState::Ready;
        Ok(())
    }

    /// 标记失败（**保留原因**；未知句柄 ⇒ `Err`）。
    pub fn mark_failed(
        &mut self,
        id: ResourceId,
        reason: impl Into<String>,
    ) -> Result<(), &'static str> {
        let state = self
            .states
            .get_mut(&id)
            .ok_or("resource: 未知句柄（未登记）")?;
        *state = ResourceState::Failed {
            reason: reason.into(),
        };
        Ok(())
    }

    /// 仍在等待的资源数。
    pub fn pending_count(&self) -> usize {
        self.states
            .values()
            .filter(|state| state.is_pending())
            .count()
    }

    /// 已就绪的资源数。
    pub fn ready_count(&self) -> usize {
        self.states
            .values()
            .filter(|state| state.is_ready())
            .count()
    }

    /// 是否**全部**就绪（空表 ⇒ `true`：没有待加载项就不必等）。
    pub fn all_ready(&self) -> bool {
        self.pending_count() == 0
    }
}

/// ⭐ **解码后的图像数据**（**宿主/上层提供**：引擎不做解码）。
///
/// ## 为什么引擎不解码
///
/// 解码 PNG/JPEG/WebP 需要第三方库 ⇒ ⇒ 与 crates-ui 的 **0 外部依赖**纪律冲突
/// ⇒ ⇒ 而解码结果（RGBA 字节）是**平台无关**的 ⇒ ⇒ 由宿主解好再交给引擎
/// ⇒ ⇒ 引擎负责的是**校验 + 上传 + 状态收敛**这三件与 GPU/渲染帧耦合的事。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageData {
    /// 宽（像素）。
    pub width: u32,
    /// 高（像素）。
    pub height: u32,
    /// RGBA8 字节（长度必须 = `width × height × 4`）。
    pub rgba: Vec<u8>,
}

impl ImageData {
    /// 构造（不做校验；校验在 [`upload_decoded`] 里统一做）。
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        Self {
            width,
            height,
            rgba,
        }
    }

    /// 期望的字节数（`width × height × 4`）。
    pub fn expected_len(&self) -> usize {
        (self.width as usize)
            .saturating_mul(self.height as usize)
            .saturating_mul(4)
    }

    /// 校验：尺寸非 0 且字节数匹配。
    pub fn validate(&self) -> Result<(), String> {
        if self.width == 0 || self.height == 0 {
            return Err(String::from("image: 尺寸为 0"));
        }
        if self.rgba.len() != self.expected_len() {
            return Err(format!(
                "image: 字节数不匹配（期望 {}，实际 {}）",
                self.expected_len(),
                self.rgba.len()
            ));
        }
        Ok(())
    }
}

/// ⭐ **纹理上传器**（由 GPU 层实现；引擎只定义接口）。
///
/// 只暴露「把已解码的 RGBA 传上去」这一件事 ⇒ ⇒ 引擎不关心纹理格式/采样参数
/// （那些属渲染层策略），也不关心缓存策略（属 L3）。
pub trait TextureUploader {
    /// 上传到 `id` 对应的纹理槽；失败返回原因（**不得**静默忽略）。
    fn upload(&mut self, id: ResourceId, image: &ImageData) -> Result<(), String>;
}

/// ⭐⭐ **上传已解码资源**：校验 → 上传 → 状态收敛（成功 ⇒ `Ready`，失败 ⇒ `Failed{原因}`）。
///
/// ## 不变量（这是本函数存在的理由）
///
/// **`Ready` 必须意味着「真的上传成功了」**
/// ⇒ ⇒ 校验失败或上传失败时**绝不能**把资源标成 `Ready`
/// （否则渲染层会拿一个空纹理去画 ⇒ 画面黑块/花屏，且**看不出是加载失败**）。
pub fn upload_decoded(
    registry: &mut ResourceRegistry,
    uploader: &mut dyn TextureUploader,
    id: ResourceId,
    image: ImageData,
) -> Result<(), String> {
    if let Err(reason) = image.validate() {
        let _ = registry.mark_failed(id, reason.clone());
        return Err(reason);
    }
    if let Err(reason) = uploader.upload(id, &image) {
        let _ = registry.mark_failed(id, reason.clone());
        return Err(reason);
    }
    registry.mark_ready(id).map_err(|error| error.to_owned())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 新登记的句柄状态必须是 `Pending`（**不能**默认 Ready）。
    #[test]
    fn registered_resource_starts_pending() {
        let mut registry = ResourceRegistry::new();
        let id = registry.register();
        assert_eq!(registry.state(id), Some(&ResourceState::Pending));
        assert_eq!(registry.pending_count(), 1);
        assert_eq!(registry.ready_count(), 0);
        assert!(!registry.all_ready(), "还有待加载项 ⇒ 不是全就绪");
    }

    /// 就绪/失败的状态迁移，以及「失败原因必须保留」。
    #[test]
    fn state_transitions_keep_the_failure_reason() {
        let mut registry = ResourceRegistry::new();
        let ready = registry.register();
        registry.mark_ready(ready).expect("标记就绪必须成功");
        assert!(registry.state(ready).unwrap().is_ready());

        let failed = registry.register();
        registry
            .mark_failed(failed, "404 字体文件缺失")
            .expect("标记失败必须成功");
        match registry.state(failed) {
            Some(ResourceState::Failed { reason }) => {
                assert!(reason.contains("404"), "失败原因必须保留：{reason}");
            }
            other => panic!("应为 Failed，实际 {other:?}"),
        }
        assert!(
            !registry.state(failed).unwrap().is_ready(),
            "失败不得被当成可用"
        );
    }

    /// 未知句柄 ⇒ `None` / `Err`（**不得**默认成 Pending 蒙混）。
    #[test]
    fn unknown_handle_is_rejected_not_defaulted() {
        let mut registry = ResourceRegistry::new();
        let unknown = ResourceId(999);
        assert_eq!(registry.state(unknown), None);
        assert!(registry.mark_ready(unknown).is_err());
        assert!(registry.mark_failed(unknown, "x").is_err());
    }

    /// 空表 ⇒ 全部就绪（没有待加载项就不该等）。
    #[test]
    fn empty_registry_is_all_ready() {
        let registry = ResourceRegistry::new();
        assert!(registry.all_ready());
    }

    struct RecordingUploader {
        calls: usize,
        fail: bool,
    }

    impl TextureUploader for RecordingUploader {
        fn upload(&mut self, _id: ResourceId, _image: &ImageData) -> Result<(), String> {
            self.calls += 1;
            if self.fail {
                return Err(String::from("GPU 上传失败（模拟）"));
            }
            Ok(())
        }
    }

    /// ⭐ **上传成功 ⇒ Ready**；**上传失败 ⇒ Failed 且保留原因**（不得标 Ready）。
    #[test]
    fn upload_success_marks_ready_and_failure_keeps_reason() {
        let mut registry = ResourceRegistry::new();
        let ok_id = registry.register();
        let mut uploader = RecordingUploader {
            calls: 0,
            fail: false,
        };
        let image = ImageData::new(2, 2, vec![0u8; 2 * 2 * 4]);
        upload_decoded(&mut registry, &mut uploader, ok_id, image).expect("应上传成功");
        assert!(registry.state(ok_id).unwrap().is_ready());
        assert_eq!(uploader.calls, 1);

        let bad_id = registry.register();
        let mut failing = RecordingUploader {
            calls: 0,
            fail: true,
        };
        let image = ImageData::new(1, 1, vec![7u8; 4]);
        let error = upload_decoded(&mut registry, &mut failing, bad_id, image).unwrap_err();
        assert!(
            !registry.state(bad_id).unwrap().is_ready(),
            "失败绝不能被标成 Ready"
        );
        match registry.state(bad_id) {
            Some(ResourceState::Failed { reason }) => {
                assert!(reason.contains("上传失败"), "失败原因必须保留：{reason}");
            }
            other => panic!("应为 Failed，实际 {other:?}"),
        }
        assert!(error.contains("上传失败"));
    }

    /// ⭐ **尺寸/字节数不合法 ⇒ 拒绝上传**（不得把坏数据传给 GPU）。
    #[test]
    fn invalid_image_is_rejected_before_upload() {
        let mut registry = ResourceRegistry::new();
        let id = registry.register();
        let mut uploader = RecordingUploader {
            calls: 0,
            fail: false,
        };
        let image = ImageData::new(2, 2, vec![0u8; 3]); // 字节数不足
        assert!(upload_decoded(&mut registry, &mut uploader, id, image).is_err());
        assert_eq!(uploader.calls, 0, "校验失败时**不得**调用上传器");
        assert!(!registry.state(id).unwrap().is_ready());

        let zero = ImageData::new(0, 4, vec![]);
        assert!(zero.validate().is_err());
    }

    /// ⭐ **度量状态默认是「未定」** —— 未知不能被当成「已知为 0」。
    #[test]
    fn metrics_default_to_unknown_and_must_be_marked_ready() {
        let status = MetricsStatus::default();
        assert!(
            !status.is_ready(),
            "默认必须是 Unknown（否则会用错度量算布局）"
        );
        assert!(MetricsStatus::Ready.is_ready());
    }
}
