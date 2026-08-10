//! # report — e2e 报告 (E2eReport + 4 格式化函数)
//!
//! **职责**: 汇总 60+ 测试结果, 输出人类可读 / JSON / 守门断言 3 种形式.
//!
//! **5 函数** (per 派活单 §7):
//! 1. `generate_report`            — 从 `&[TestResult]` 聚合出 `E2eReport`
//! 2. `format_human_readable`      — 表格输出, 控制台友好
//! 3. `format_json`                — JSON 输出, 机器友好
//! 4. `assert_all_passed`          — 守门, 全过返 Ok, 有 fail 返 E2EError
//! 5. `TestResult::ok` / `TestResult::fail` — 测试结果构造
//!
//! **3 层分类** (per 派活单):
//! - Workspace — 5 测试
//! - API       — 19+2 测试
//! - TUI       — 14+1 测试
//!
//! **8 不修改承诺**: 跟 lib.rs / error.rs / harness.rs / api_e2e / tui_e2e / workspace_e2e 一致

use std::collections::HashMap;
use std::fmt;

use crate::error::{E2EError, E2EResult};

// =====================================================================
// TestResult — 单个测试结果
// =====================================================================

/// 单个测试结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestResult {
    /// 测试名 (e.g. "test_api_metrics_endpoint_returns_prometheus")
    pub name: String,
    /// 所属层
    pub layer: E2eLayer,
    /// 是否通过
    pub passed: bool,
    /// 跳过原因 (None = 没跳过)
    pub skip_reason: Option<String>,
    /// 失败原因 (None = 没失败)
    pub failure_reason: Option<String>,
    /// 耗时 (毫秒)
    pub elapsed_ms: u64,
}

impl TestResult {
    /// 构造一个通过的测试结果
    pub fn ok(name: impl Into<String>, layer: E2eLayer, elapsed_ms: u64) -> Self {
        Self {
            name: name.into(),
            layer,
            passed: true,
            skip_reason: None,
            failure_reason: None,
            elapsed_ms,
        }
    }

    /// 构造一个失败的测试结果
    pub fn fail(
        name: impl Into<String>,
        layer: E2eLayer,
        reason: impl Into<String>,
        elapsed_ms: u64,
    ) -> Self {
        Self {
            name: name.into(),
            layer,
            passed: false,
            skip_reason: None,
            failure_reason: Some(reason.into()),
            elapsed_ms,
        }
    }

    /// 构造一个跳过的测试结果
    pub fn skip(
        name: impl Into<String>,
        layer: E2eLayer,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            layer,
            passed: false,
            skip_reason: Some(reason.into()),
            failure_reason: None,
            elapsed_ms: 0,
        }
    }
}

// =====================================================================
// E2eLayer — 测试层枚举
// =====================================================================

/// 3 层 e2e (workspace / api / tui)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum E2eLayer {
    /// Workspace 层 (主仓状态) — default
    #[default]
    Workspace,
    /// API 层 (HTTP 端点)
    Api,
    /// TUI 层 (终端渲染)
    Tui,
}

impl E2eLayer {
    /// 中文 label
    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Workspace => "主仓",
            Self::Api => "API",
            Self::Tui => "TUI",
        }
    }

    /// 全部 3 层
    pub const ALL: [Self; 3] = [Self::Workspace, Self::Api, Self::Tui];
}

impl fmt::Display for E2eLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label_zh())
    }
}

// =====================================================================
// E2eLayerReport — 单层报告
// =====================================================================

/// 单层报告
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct E2eLayerReport {
    /// 层
    pub layer: E2eLayer,
    /// 总数
    pub total: u32,
    /// 通过
    pub passed: u32,
    /// 失败
    pub failed: u32,
    /// 跳过
    pub skipped: u32,
    /// 失败列表 (测试名 + 原因)
    pub failures: Vec<(String, String)>,
}

// =====================================================================
// E2eReport — 总报告
// =====================================================================

/// 总报告
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct E2eReport {
    /// 总测试数
    pub total_tests: u32,
    /// 总通过
    pub passed: u32,
    /// 总失败
    pub failed: u32,
    /// 总跳过
    pub skipped: u32,
    /// 总耗时 (毫秒)
    pub total_elapsed_ms: u64,
    /// 按层分类
    pub by_layer: HashMap<E2eLayer, E2eLayerReport>,
}

impl E2eReport {
    /// 是否全部通过 (跳过的也算不失败)
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }

    /// 通过率 (0.0 - 1.0, 不含跳过)
    pub fn pass_rate(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            self.passed as f64 / self.total_tests as f64
        }
    }
}

// =====================================================================
// 4 格式化函数 (per 派活单 §7)
// =====================================================================

/// 1. generate_report — 从 `&[TestResult]` 聚合
pub fn generate_report(results: &[TestResult]) -> E2eReport {
    let mut report = E2eReport::default();
    report.total_tests = results.len() as u32;
    report.total_elapsed_ms = results.iter().map(|r| r.elapsed_ms).sum();
    for r in results {
        let layer_report = report
            .by_layer
            .entry(r.layer)
            .or_insert_with(|| E2eLayerReport {
                layer: r.layer,
                ..Default::default()
            });
        layer_report.total += 1;
        if r.skip_reason.is_some() {
            layer_report.skipped += 1;
            report.skipped += 1;
        } else if r.passed {
            layer_report.passed += 1;
            report.passed += 1;
        } else {
            layer_report.failed += 1;
            report.failed += 1;
            let reason = r
                .failure_reason
                .clone()
                .unwrap_or_else(|| "unknown failure".into());
            layer_report.failures.push((r.name.clone(), reason));
        }
    }
    report
}

/// 2. format_human_readable — 表格输出
pub fn format_human_readable(report: &E2eReport) -> String {
    let mut s = String::new();
    s.push_str("=== Apeireth 集成测试 e2e 报告 ===\n");
    s.push_str(&format!(
        "总测试: {} | 通过: {} | 失败: {} | 跳过: {} | 耗时: {} ms\n",
        report.total_tests,
        report.passed,
        report.failed,
        report.skipped,
        report.total_elapsed_ms
    ));
    s.push_str(&format!(
        "通过率: {:.1}%\n",
        report.pass_rate() * 100.0
    ));
    s.push_str("\n--- 按层 ---\n");
    for layer in E2eLayer::ALL.iter() {
        let lr = report.by_layer.get(layer).cloned().unwrap_or_default();
        s.push_str(&format!(
            "{layer}: 总 {total} | 通过 {passed} | 失败 {failed} | 跳过 {skipped}\n",
            layer = layer.label_zh(),
            total = lr.total,
            passed = lr.passed,
            failed = lr.failed,
            skipped = lr.skipped,
        ));
    }
    let failure_count = report
        .by_layer
        .values()
        .flat_map(|lr| lr.failures.iter())
        .count();
    if failure_count > 0 {
        s.push_str("\n--- 失败列表 ---\n");
        for (name, reason) in report.by_layer.values().flat_map(|lr| lr.failures.iter()) {
            s.push_str(&format!("  ✗ {name}: {reason}\n"));
        }
    }
    s.push_str(&format!("\n结果: {}\n", if report.all_passed() { "✓ 全部通过" } else { "✗ 有失败" }));
    s
}

/// 3. format_json — JSON 输出
pub fn format_json(report: &E2eReport) -> String {
    let mut by_layer_json = serde_json::Map::new();
    for (k, v) in report.by_layer.iter() {
        by_layer_json.insert(
            k.label_zh().to_string(),
            serde_json::json!({
                "total": v.total,
                "passed": v.passed,
                "failed": v.failed,
                "skipped": v.skipped,
                "failures": v.failures.iter().map(|(n, r)| serde_json::json!({
                    "name": n, "reason": r
                })).collect::<Vec<_>>()
            }),
        );
    }
    serde_json::json!({
        "total_tests": report.total_tests,
        "passed": report.passed,
        "failed": report.failed,
        "skipped": report.skipped,
        "total_elapsed_ms": report.total_elapsed_ms,
        "pass_rate": report.pass_rate(),
        "all_passed": report.all_passed(),
        "by_layer": by_layer_json
    })
    .to_string()
}

/// 4. assert_all_passed — 守门
pub fn assert_all_passed(report: &E2eReport) -> E2EResult<()> {
    if report.all_passed() {
        Ok(())
    } else {
        let mut msg = format!(
            "e2e {} 测试中 {} 失败: ",
            report.total_tests, report.failed
        );
        for (i, (name, reason)) in report
            .by_layer
            .values()
            .flat_map(|lr| lr.failures.iter())
            .enumerate()
        {
            if i > 0 {
                msg.push_str("; ");
            }
            msg.push_str(&format!("{name} ({reason})"));
        }
        Err(E2EError::TuiAssert {
            context: "assert_all_passed".into(),
            expected: "all 60+ tests passed".into(),
            actual: msg,
        })
    }
}

// =====================================================================
// 单元测试 (5+)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_ok() {
        let r = TestResult::ok("test_x", E2eLayer::Api, 10);
        assert!(r.passed);
        assert!(r.failure_reason.is_none());
        assert_eq!(r.elapsed_ms, 10);
    }

    #[test]
    fn test_result_fail() {
        let r = TestResult::fail("test_y", E2eLayer::Tui, "buffer not found", 5);
        assert!(!r.passed);
        assert_eq!(r.failure_reason.as_deref(), Some("buffer not found"));
    }

    #[test]
    fn test_result_skip() {
        let r = TestResult::skip("test_z", E2eLayer::Workspace, "需要 cargo");
        assert!(!r.passed);
        assert_eq!(r.skip_reason.as_deref(), Some("需要 cargo"));
    }

    #[test]
    fn e2e_layer_all_3() {
        assert_eq!(E2eLayer::ALL.len(), 3);
    }

    #[test]
    fn e2e_layer_label_zh_unique() {
        let labels: Vec<&str> = E2eLayer::ALL.iter().map(|l| l.label_zh()).collect();
        let unique: std::collections::HashSet<&str> = labels.iter().copied().collect();
        assert_eq!(unique.len(), 3);
    }

    #[test]
    fn generate_report_empty() {
        let r = generate_report(&[]);
        assert_eq!(r.total_tests, 0);
        assert_eq!(r.passed, 0);
        assert!(r.all_passed());
    }

    #[test]
    fn generate_report_mixed() {
        let results = vec![
            TestResult::ok("test_1", E2eLayer::Api, 5),
            TestResult::ok("test_2", E2eLayer::Api, 10),
            TestResult::fail("test_3", E2eLayer::Tui, "buffer", 8),
            TestResult::skip("test_4", E2eLayer::Workspace, "需要 cargo"),
        ];
        let r = generate_report(&results);
        assert_eq!(r.total_tests, 4);
        assert_eq!(r.passed, 2);
        assert_eq!(r.failed, 1);
        assert_eq!(r.skipped, 1);
        assert!(!r.all_passed());
    }

    #[test]
    fn generate_report_by_layer() {
        let results = vec![
            TestResult::ok("a1", E2eLayer::Api, 5),
            TestResult::ok("a2", E2eLayer::Api, 5),
            TestResult::ok("t1", E2eLayer::Tui, 5),
        ];
        let r = generate_report(&results);
        let api = r.by_layer.get(&E2eLayer::Api).unwrap();
        assert_eq!(api.total, 2);
        let tui = r.by_layer.get(&E2eLayer::Tui).unwrap();
        assert_eq!(tui.total, 1);
    }

    #[test]
    fn format_human_readable_contains_summary() {
        let r = generate_report(&[TestResult::ok("t1", E2eLayer::Api, 5)]);
        let s = format_human_readable(&r);
        assert!(s.contains("总测试"));
        assert!(s.contains("通过"));
        assert!(s.contains("API"));
    }

    #[test]
    fn format_json_valid_json() {
        let r = generate_report(&[TestResult::ok("t1", E2eLayer::Api, 5)]);
        let s = format_json(&r);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["total_tests"], 1);
        assert_eq!(v["passed"], 1);
        assert_eq!(v["all_passed"], true);
    }

    #[test]
    fn assert_all_passed_when_passed() {
        let r = generate_report(&[TestResult::ok("t1", E2eLayer::Api, 5)]);
        assert_all_passed(&r).unwrap();
    }

    #[test]
    fn assert_all_passed_when_failed() {
        let r = generate_report(&[TestResult::fail("t1", E2eLayer::Api, "x", 5)]);
        let result = assert_all_passed(&r);
        assert!(result.is_err());
    }

    #[test]
    fn pass_rate_calculation() {
        let r = generate_report(&[
            TestResult::ok("t1", E2eLayer::Api, 5),
            TestResult::ok("t2", E2eLayer::Api, 5),
            TestResult::fail("t3", E2eLayer::Api, "x", 5),
        ]);
        // 3 total, 2 passed, 1 failed, 0 skipped → 2/3 ≈ 0.667
        assert!((r.pass_rate() - 2.0 / 3.0).abs() < 0.001);
    }

    #[test]
    fn pass_rate_zero_when_empty() {
        let r = generate_report(&[]);
        assert_eq!(r.pass_rate(), 0.0);
    }

    #[test]
    fn report_default_zero() {
        let r = E2eReport::default();
        assert_eq!(r.total_tests, 0);
        assert!(r.all_passed());
        assert_eq!(r.pass_rate(), 0.0);
    }
}
