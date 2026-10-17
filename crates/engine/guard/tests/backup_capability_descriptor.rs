//! G5 备份恢复: `tool.backup_export` 的规范描述符真值断言 (敏感面=凭据排除断言)。
//!
//! 描述符如实标双面: 读记忆/账本正文 + 写备份 bundle (持久写, 非毁灭、非外联);
//! **凭据面 `may_access_credentials = false`** —— 密钥/令牌面显式排除
//! (凭据不随行, 排除表 + 打包内零密钥测试钉死), 描述符不得谎报有凭据入口。

use apeireth_guard::{
    descriptor_for_capability, DataSensitivity, DescriptorSource, BUILTIN_CANONICAL_CAPABILITY_IDS,
};

#[test]
fn backup_export_descriptor_is_honest_about_its_faces() {
    let descriptor = descriptor_for_capability("tool.backup_export", &serde_json::json!({}));
    assert_eq!(descriptor.source, DescriptorSource::Canonical);
    assert!(descriptor.known);
    assert!(
        descriptor.persistent_effect,
        "打包落盘是持久写面, 必须如实标 persistent"
    );
    assert!(!descriptor.destructive, "无删除入口, 不得谎报毁灭面");
    assert!(!descriptor.requires_network, "备份不出网");
    assert!(!descriptor.external_effect);
    assert!(
        !descriptor.may_access_credentials,
        "凭据不随行: 密钥/令牌面显式排除, 描述符不得谎报凭据入口"
    );
    assert_eq!(
        descriptor.data_sensitivity,
        DataSensitivity::MemoryPrivate,
        "打包的是记忆正文, 敏感档如实标 MemoryPrivate"
    );
}

#[test]
fn backup_export_is_in_the_builtin_canonical_roster() {
    assert!(
        BUILTIN_CANONICAL_CAPABILITY_IDS.contains(&"tool.backup_export"),
        "新工具必须入规范名册 (描述符覆盖率 100% 同一口径)"
    );
    let coverage = apeireth_guard::canonical_descriptor_coverage();
    assert_eq!(coverage.coverage_pct, 100.0);
    assert_eq!(coverage.fallback_pct, 0.0);
}
