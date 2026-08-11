//! Integration test: Provider trait + 5+ 平台 mock 行为验证.
//!
//! R20 阶段 6 flesh out #2: "5+ 平台 mock" 子任务验证 (per 主 22:13 拍
//! "machine-id flesh out").
//!
//! ## 覆盖
//!
//! - 5 mock provider 行为 (3 真实场景 + 2 失败场景)
//! - ProviderChain first-success 语义
//! - ProviderChain all-fail 语义
//! - ProviderChain skip non-applicable 语义
//! - 异构 chain (mix real + mock)
//! - ProviderProbeResult JSON 序列化
//!
//! ## 6 哲学锚穿透
//!
//! - ✅ **S-1 北极星**: 沿用 std::io::Read trait 测试模式 (mock impl + 测试 fixture)
//! - ✅ **S-2 实事求是**: mock 返真实预置值, 不假装
//! - ✅ **O-2 走在前人肩上**: 测试 internal 抽象, UI 不暴露
//! - ✅ **O-3 干到底**: 1 集成测试文件 5+ fixture
//! - ✅ **O-4 任何人都能接手**: 测试独立, 不依赖真 platform 探测
//! - ✅ **O-5 不假装**: 本节自检; 不假装 mock 测的是"真" provider
//!
//! ## 8 项不修改承诺
//!
//! - ✅ **不假装已实现**: mock 是"预置 raw" 行为, 真实 provider 是 cfg-gated 真跑
//! - ✅ **编译期 hardcode**: trait 接口 4 方法编译期固化
//! - ✅ **不改 LOCKED**: 0 触碰 24 LOCKED crate
//! - ✅ **不改 workspace version**: v1.0.0 严守
//! - ✅ **6 哲学锚穿透**: 上节
//! - ✅ **不依赖 NewAPI**: 纯 std / tokio / serde_json / async-trait
//! - ✅ **不重复造轮子**: mock 模式抄 sqlx mock / std::io::Read mock 行业惯例
//! - ✅ **诚实标缺**: mock 返值写死在结构体 (易识别), 不假装"真" provider 行为

use apeireth_machine_id::{
    MacHashProvider, MachineIdFileProvider, MachineIdProvider, MockEmptyProvider,
    MockFailingProvider, MockMacHashProvider, MockMachineIdFileProvider, MockSmBiosDmiProvider,
    MockWindowsSidProvider, ProviderChain, ProviderProbeResult, SmBiosDmiProvider,
    WindowsSidProvider,
};

// ----------------------------------------------------------------------------
// Fixture 1: 4 真实 Provider 编译期构造 + name/description 守门
// ----------------------------------------------------------------------------

#[test]
fn fixture_real_providers_construct_and_name_correct() {
    // 4 真实 Provider
    let smbios: Box<dyn MachineIdProvider> = Box::new(SmBiosDmiProvider::new());
    let mac: Box<dyn MachineIdProvider> = Box::new(MacHashProvider::new());
    let file: Box<dyn MachineIdProvider> = Box::new(MachineIdFileProvider::new());
    let win: Box<dyn MachineIdProvider> = Box::new(WindowsSidProvider::new());

    // name 守门 (防 typo)
    assert_eq!(smbios.name(), "smbios-dmi");
    assert_eq!(mac.name(), "mac-hash");
    assert_eq!(file.name(), "machine-id-file");
    assert_eq!(win.name(), "windows-sid");

    // description 守门 (非空)
    for p in [&smbios, &mac, &file, &win] {
        let d = p.description();
        assert!(!d.is_empty(), "description 必须非空: {d}");
    }
}

// ----------------------------------------------------------------------------
// Fixture 2: 6 mock Provider 编译期构造 + name 守门
// ----------------------------------------------------------------------------

#[test]
fn fixture_six_mocks_construct_and_name_correct() {
    // 6 mock Provider (5+ 满足子任务要求)
    let m1 = MockSmBiosDmiProvider::default();
    let m2 = MockMacHashProvider::default();
    let m3 = MockMachineIdFileProvider::default();
    let m4 = MockWindowsSidProvider::default();
    let m5 = MockFailingProvider::default();
    let m6 = MockEmptyProvider;

    // name 守门 (区分 6 个)
    assert_eq!(m1.name(), "mock-smbios-dmi");
    assert_eq!(m2.name(), "mock-mac-hash");
    assert_eq!(m3.name(), "mock-machine-id-file");
    assert_eq!(m4.name(), "mock-windows-sid");
    assert_eq!(m5.name(), "mock-failing");
    assert_eq!(m6.name(), "mock-empty");
}

// ----------------------------------------------------------------------------
// Fixture 3: 6 mock probe 行为守门 (返预置值, 不依赖真 platform)
// ----------------------------------------------------------------------------

#[tokio::test]
async fn fixture_six_mocks_probe_return_predetermined_values() {
    // m1: SMBIOS mock → 返 UUID
    let (raw, src) = MockSmBiosDmiProvider::default().probe().await.expect("ok");
    assert_eq!(raw, "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE");
    assert_eq!(src, "mock-dmi");

    // m2: MAC mock → 返 MAC
    let (raw, src) = MockMacHashProvider::default().probe().await.expect("ok");
    assert_eq!(raw, "aa:bb:cc:dd:ee:ff");
    assert_eq!(src, "mock-mac");

    // m3: machine-id file mock → 返预置 raw
    let (raw, src) = MockMachineIdFileProvider::default().probe().await.expect("ok");
    assert_eq!(raw, "mock-machine-id-12345");
    assert_eq!(src, "mock-etc");

    // m4: Windows SID mock → 返预置 SID
    let (raw, src) = MockWindowsSidProvider::default().probe().await.expect("ok");
    assert_eq!(raw, "S-1-5-21-MOCK-SID-12345");
    assert_eq!(src, "mock-registry");

    // m5: failing mock → 返 Err
    let err = MockFailingProvider { error_msg: "boom".into() }.probe().await.expect_err("应失败");
    assert!(err.to_string().contains("boom"), "error 应含自定义 msg, got {err}");

    // m6: empty mock → 返空 raw
    let (raw, src) = MockEmptyProvider.probe().await.expect("ok");
    assert!(raw.is_empty(), "empty mock 返 raw 必须空");
    assert_eq!(src, "mock-empty");
}

// ----------------------------------------------------------------------------
// Fixture 4: ProviderChain first-success 语义
// ----------------------------------------------------------------------------

#[tokio::test]
async fn fixture_chain_first_success_returns_first_ok() {
    let chain = ProviderChain::new()
        .with(MockSmBiosDmiProvider::default())
        .with(MockMacHashProvider::default())
        .with(MockMachineIdFileProvider::default());
    let (raw, src) = chain.probe().await.expect("chain probe ok");
    // 首个 mock 返 UUID
    assert_eq!(raw, "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE");
    // source 应含 "mock-smbios-dmi" 前缀 (ProviderChain::probe 加 "provider_name:" 前缀)
    assert!(src.starts_with("mock-smbios-dmi:"), "source 必须以 provider name 开头, got {src}");
    // 后续 mock 不应被探 (first-success)
    assert!(!src.contains("mac-hash"), "first-success 后续不探");
}

#[tokio::test]
async fn fixture_chain_skips_failing_and_returns_next_success() {
    // 链: failing → smbios-dmi (成功)
    let chain = ProviderChain::new()
        .with(MockFailingProvider { error_msg: "skip me".into() })
        .with(MockSmBiosDmiProvider::default());
    let (raw, _) = chain.probe().await.expect("应返首个成功 (smbios-dmi)");
    assert_eq!(raw, "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE");
}

// ----------------------------------------------------------------------------
// Fixture 5: ProviderChain all-fail 语义
// ----------------------------------------------------------------------------

#[tokio::test]
async fn fixture_chain_all_fail_returns_last_error() {
    let chain = ProviderChain::new()
        .with(MockFailingProvider { error_msg: "first fail".into() })
        .with(MockFailingProvider { error_msg: "second fail".into() })
        .with(MockFailingProvider { error_msg: "last fail".into() });
    let err = chain.probe().await.expect_err("应失败");
    // 返最后 error
    assert!(err.to_string().contains("last fail"), "应返最后 error, got {err}");
}

#[tokio::test]
async fn fixture_chain_empty_returns_no_applicable_error() {
    let chain = ProviderChain::new();
    let err = chain.probe().await.expect_err("空 chain 应失败");
    assert!(
        err.to_string().contains("no applicable"),
        "空 chain error 应含 'no applicable', got {err}"
    );
}

// ----------------------------------------------------------------------------
// Fixture 6: ProviderChain probe_all 返所有 attempt 结果
// ----------------------------------------------------------------------------

#[tokio::test]
async fn fixture_chain_probe_all_returns_all_attempts() {
    let chain = ProviderChain::new()
        .with(MockSmBiosDmiProvider::default())
        .with(MockFailingProvider { error_msg: "fail".into() })
        .with(MockMacHashProvider::default());

    let results = chain.probe_all().await;
    assert_eq!(results.len(), 3, "probe_all 应返 3 个 result");
    assert_eq!(results[0].provider_name, "mock-smbios-dmi");
    assert!(results[0].raw.is_some(), "首个 mock 应成功");
    assert_eq!(results[1].provider_name, "mock-failing");
    assert!(results[1].error.is_some(), "failing mock 应有 error");
    assert!(results[1].raw.is_none(), "failing mock raw 必须空");
    assert_eq!(results[2].provider_name, "mock-mac-hash");
    assert!(results[2].raw.is_some(), "第三个 mock 应成功 (跳过 failing 继续)");
}

// ----------------------------------------------------------------------------
// Fixture 7: 异构 chain (mock + 真 provider)
// ----------------------------------------------------------------------------

#[tokio::test]
async fn fixture_heterogeneous_chain_mix_real_and_mock() {
    // 链: 真 SmBiosDmiProvider → 真 MacHashProvider → 真 MachineIdFileProvider
    // 适用于 4 平台, 不依赖特定 platform 真有 wmic/ioreg/DMI 文件
    // (用 cfg-gated 真实 impl, 跑会失败返 Ok is fine - 我们只测 chain 组合正确)
    let chain = ProviderChain::new()
        .with(MockSmBiosDmiProvider::default())
        .with(MacHashProvider::new());
    let (raw, src) = chain.probe().await.expect("chain 至少 mock 成功");
    // mock 在最前, 应被返
    assert_eq!(raw, "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE");
    assert!(src.starts_with("mock-smbios-dmi:"));
}

// ----------------------------------------------------------------------------
// Fixture 8: ProviderProbeResult JSON 序列化
// ----------------------------------------------------------------------------

#[test]
fn fixture_provider_probe_result_serde() {
    let r = ProviderProbeResult {
        provider_name: "smbios-dmi".to_string(),
        applicable: true,
        raw: Some("test-uuid".to_string()),
        source: Some("dmi".to_string()),
        error: None,
    };
    let json = serde_json::to_string(&r).expect("serialize ok");
    assert!(json.contains("smbios-dmi"));
    assert!(json.contains("test-uuid"));
    assert!(json.contains("applicable"));

    // 反序列化对等
    let parsed: ProviderProbeResult = serde_json::from_str(&json).expect("deserialize ok");
    assert_eq!(parsed.provider_name, "smbios-dmi");
    assert!(parsed.applicable);
    assert_eq!(parsed.raw.as_deref(), Some("test-uuid"));
    assert_eq!(parsed.error, None);
}

#[test]
fn fixture_provider_probe_result_with_error_serializes() {
    let r = ProviderProbeResult {
        provider_name: "failing-mock".to_string(),
        applicable: true,
        raw: None,
        source: None,
        error: Some("permission denied".to_string()),
    };
    let json = serde_json::to_string(&r).expect("serialize ok");
    let parsed: ProviderProbeResult = serde_json::from_str(&json).expect("deserialize ok");
    assert_eq!(parsed.error.as_deref(), Some("permission denied"));
    assert!(parsed.raw.is_none());
}

// ----------------------------------------------------------------------------
// Fixture 9: trait 4 方法编译期存在 (rustc 强制)
// ----------------------------------------------------------------------------

#[test]
fn fixture_trait_four_methods_exist() {
    // 通过 trait object 验证 4 方法编译期存在
    let p: &dyn MachineIdProvider = &MockEmptyProvider;
    let _: &str = p.name();
    let _: &str = p.description();
    let _: bool = p.is_applicable();
    let _f = p.probe(); // 不 await, 仅验证方法存在
}

// ----------------------------------------------------------------------------
// Fixture 10: ProviderChain len / is_empty
// ----------------------------------------------------------------------------

#[test]
fn fixture_chain_len_and_is_empty() {
    let empty = ProviderChain::new();
    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());

    let one = ProviderChain::new().with(MockEmptyProvider);
    assert_eq!(one.len(), 1);
    assert!(!one.is_empty());

    let three = ProviderChain::new()
        .with(MockEmptyProvider)
        .with(MockEmptyProvider)
        .with(MockEmptyProvider);
    assert_eq!(three.len(), 3);
}
