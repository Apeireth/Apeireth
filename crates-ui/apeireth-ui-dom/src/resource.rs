//! ⭐ **布局度量状态** —— 台账 L1-G7 的**布局面**一片。
//!
//! ## 为什么留在 `ui-dom`
//!
//! 「度量是否已定」是**布局面**的事：字体未就绪 => 文本度量会变 => 按旧度量算出
//! 的窗口/命中/滚动位置全错 => 布局跳动（CLS）。由 DOM 渲染器消费
//! （`renderer.rs` 的 `metrics_status` 字段）=> 归 `ui-dom`。
//!
//! ⚠️ **资源句柄 / 加载状态 / 纹理上传契约已搬到 `apeireth_ui_gpu::resource`**
//! （2026-11-06）—— 那些属**像素投影**，且 `ui-dom` 内部 0 消费；
//! 搬走是为了让 `TextureUploader` 的实现能与契约**同 crate**。

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

#[cfg(test)]
mod tests {
    use super::*;

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
