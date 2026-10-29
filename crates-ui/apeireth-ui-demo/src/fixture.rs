//! 固定数据（**两侧同源**）。
//!
//! 设计文档 §9.3 步骤 3：「两端喂**同一份** fixture（不连真实后端）⇒ 渲染的
//! 是同一批内容 ⇒ 差异只可能来自引擎」。
//!
//! 真源 = `harness/fixtures/pilot-request.json`（JSON）；Rust 侧由
//! `harness/scripts/extract-assets.mjs` **构建期生成**（`fixture_generated.rs`）
//! ⇒ 不手抄、不漂移（抽取脚本的 `--check` 模式可复核漂移）。

pub use super::fixture_generated::pilot_item;

#[cfg(test)]
mod tests {
    //! fixture 的形状断言（数据本身由生成物提供）。

    use super::*;

    #[test]
    fn pilot_item_carries_every_field_the_reference_implementation_reads() {
        let item = pilot_item();
        assert!(item.title.is_some(), "标题");
        assert!(item.command_text.is_some(), "命令文本");
        assert!(item.arguments_summary.is_some(), "参数摘要");
        assert!(item.reason.is_some(), "审批原因");
        assert!(item.created_at.is_some(), "创建时刻");
        assert!(item.sandbox.is_some(), "沙箱摘要");
        assert!(item.cwd.is_some(), "工作目录");
        assert!(item.isolation.is_some(), "隔离态摘要");
    }

    #[test]
    fn pilot_item_uses_the_sandboxed_variant() {
        let item = pilot_item();
        assert!(
            !item
                .sandbox
                .as_deref()
                .unwrap_or_default()
                .contains("未沙箱"),
            "对照 fixture 用「有墙」形态（青盾）"
        );
    }
}
