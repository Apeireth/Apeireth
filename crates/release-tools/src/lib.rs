// ============================================================================
// release-tools — TP20-S5 塞缝批
// ----------------------------------------------------------------------------
// 0 运行时代码。仅供:
//   1. `cargo vet --manifest-path crates/release-tools/Cargo.toml` 验证
//      本 crate 自身依赖审计配置被读取
//   2. `.cargo/vet.toml` 的 publisher 配置 reference
//   3. CI release.yml 的 audit-stage 引用 `release_tools::VERSION` 常量
//      做编译期版本硬编码 (与 APEIRETH_VERSION 环境变量对账)
//
// 哲学:
//   - 机制而非补丁: 把「发布期供应链验证」做成 first-class crate,
//     而不是散落的 shell 脚本 (脚本在 scripts/ 但底座在这)
//   - 集成而非分立: vet/audit/deny/SBOM 共享同一个 workspace = true
//     version, 不会因为切到子目录就跑出 workspace 之外的解析
//   - 安全底线: 任何 release 走 vet + audit + deny + sbom, 0 装可走
//     (cargo-vet 缺时降级为 vet=skipped, 文档在 scripts/vet.sh 头部)
// ============================================================================

//! release-tools: 发布期供应链验证 (vet + audit + deny + CycloneDX SBOM) 的
//! 编译期/文档期锚点。零运行时逻辑。
//!
//! 完整使用文档见 [`crates/release-tools/README.md`](https://github.com/apeireth/-/blob/main/crates/release-tools/README.md)。
//! 本文件仅保留编译期断言与公开常量。

/// 当前 crate 编译期版本 (== workspace version, 与 APEIRETH_VERSION 环境变量对账)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// CycloneDX 1.5 规范 (与 cargo-cyclonedx 输出格式对齐)
pub const CYCLONEDX_SPEC_VERSION: &str = "1.5";

/// 默认 SBOM 输出文件名 (CI release.yml 与 scripts/sbom.sh 共用)
pub const SBOM_FILENAME: &str = "cyclonedx-sbom.json";

/// vet/audit/deny/sbom 任一失败时 release 必须阻塞的硬约定。
/// 真值在 scripts/vet.sh + .github/workflows/release.yml 的 `if: failure()`
/// 双闸门处体现, 本常量仅供文档引用。
pub const SUPPLY_CHAIN_BLOCK_ON_FAIL: bool = true;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        // 编译期硬编码: workspace version 必须存在 (与 APEIRETH_VERSION 对账)
        // 若失败 = 有人把 version 设成空, 触发 release 阻断
        // ponytail: workspace version 实际值 (1.2.0) 不在这里钉死, 而是在 .github/workflows/release.yml
        // 的 APEIRETH_VERSION env 与 CI gate 同步, 避免本测试与 release gate 双源真相漂移
        assert!(
            !VERSION.is_empty(),
            "VERSION must be non-empty (workspace version)"
        );
        assert!(
            VERSION.chars().next().unwrap().is_ascii_digit(),
            "VERSION must start with a digit (semver-shaped)"
        );
    }

    #[test]
    fn cyclonedx_target_is_1_5() {
        // 钉死: 只接受 CycloneDX 1.5, 不接受 SPDX/2.x
        assert_eq!(CYCLONEDX_SPEC_VERSION, "1.5");
    }

    #[test]
    fn sbom_filename_is_cyclonedx_sbom_json() {
        // 与 scripts/sbom.sh + release.yml artifacts.name 对齐
        assert_eq!(SBOM_FILENAME, "cyclonedx-sbom.json");
    }
}
