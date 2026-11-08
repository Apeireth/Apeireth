//! 配额深度维 (调用树深度): 真计量 + 超限拒绝。
//!
//! 深度维 = 推理深度 / 思考量 = **调用树深度跟踪**。这里钉死的行为:
//!
//! 1. 深度在上限内 ⇒ 不出超限事实, 读数是真数字;
//! 2. 深度**恰好等于**上限 ⇒ 耗尽 (与 token / 步数 / 花费同读法:
//!    `consumed >= limit`);
//! 3. 再往下一层 ⇒ **拒** (既有配额拒绝语义的事实, 维度名 `depth`;
//!    被拒的进入不改调用树);
//! 4. 深度为 0 ⇒ 短路 (顶层回合不到上限, 出栈不下溢);
//! 5. 嵌套调用 ⇒ 深度逐层累计, 峰值单调 (出栈不洗白);
//! 6. **未启用**深度维 ⇒ 旧路径逐字节等 (同一事实、同一顺序、同一维度名);
//! 7. **对抗**: "深度很大但步数很小" 的输入**不得**绕过上限; 出栈 / 报更小
//!    的深度都洗不白已经越限的调用树。

use apeireth_orchestration::CognitiveQuota;
use apeireth_runtime::canonical::quota::{
    parse_quota_max_depth, DepthQuota, DepthQuotaReading, QuotaDimension, QuotaExceeded,
    SessionQuotaLimits, MAX_QUOTA_DEPTH,
};
use apeireth_runtime::canonical::RuntimeError;

/// 深度上限的默认构造 (上限 3 层)。
fn capped(limit: u64) -> DepthQuota {
    DepthQuota::with_max_depth(Some(limit))
}

/// ① 正常: 深度在限内 ⇒ 不出超限事实。
#[test]
fn depth_inside_the_ceiling_does_not_bind() {
    let mut depth = capped(3);
    assert_eq!(depth.enter(), Ok(1));
    assert_eq!(depth.enter(), Ok(2));
    assert_eq!(depth.check(), None, "深度 2 < 上限 3: 不出事实");
    assert_eq!(depth.current_depth(), 2);
    assert_eq!(depth.peak_depth(), 2);
    assert_eq!(depth.consumed(), 2);
    let reading = depth.reading();
    assert_eq!(reading.limit, Some(3));
    assert_eq!(reading.consumed(), 2);
}

/// ② 边界: 深度恰好等于上限 ⇒ 耗尽 (口径与其它三维一致)。
#[test]
fn depth_exactly_at_the_ceiling_is_exhausted() {
    let mut depth = capped(3);
    for expected in 1..=3u32 {
        assert_eq!(depth.enter(), Ok(expected));
    }
    assert_eq!(depth.peak_depth(), 3);
    assert_eq!(
        depth.check(),
        Some(QuotaExceeded {
            dimension: QuotaDimension::Depth,
            limit: 3,
            consumed: 3,
        }),
        "恰好等于上限即耗尽 (consumed >= limit, 口径唯一)"
    );
}

/// ③ 超限拒绝: 再往下一层被拒, 且拒绝**不改**调用树。
#[test]
fn going_deeper_than_the_ceiling_is_refused() {
    let mut depth = capped(3);
    for _ in 0..3 {
        depth.enter().expect("限内的嵌套层必须放行");
    }
    let refused = depth.enter().expect_err("第四层必须被拒");
    assert_eq!(refused.dimension, QuotaDimension::Depth);
    assert_eq!(refused.limit, 3);
    assert_eq!(refused.consumed, 3, "事实带真数字: 上限 / 已耗");
    assert_eq!(
        depth.current_depth(),
        3,
        "被拒的进入不改调用树 (不是先放进去再报错)"
    );
    assert_eq!(depth.peak_depth(), 3);
}

/// ④ 对抗: "深度很大但步数很小" 的输入**不得**绕过上限。
///
/// 步数只有 1、token 为 0, 但调用树很深 ⇒ 深度维独立出事实 (深度维不
/// 寄生在步数维上, 也不因步数小而被跳过)。
#[test]
fn a_deep_call_tree_with_almost_no_steps_cannot_bypass_the_ceiling() {
    let limits = SessionQuotaLimits {
        max_tokens: None,
        max_tool_steps: Some(1_000),
        max_cost_micros: None,
    };
    let mut meter = CognitiveQuota::default();
    meter.consume_tool_step();
    assert_eq!(meter.consumed_tool_steps, 1, "步数很小");

    let mut depth = capped(4);
    for _ in 0..4 {
        depth.enter().expect("限内的嵌套层放行");
    }
    // 深度到顶, 其余三维都没到顶 ⇒ 深度维自己绑上限。
    let exceeded = limits
        .check_with_depth(&mut meter, &depth)
        .expect("深度维必须出超限事实");
    assert_eq!(exceeded.dimension, QuotaDimension::Depth);
    assert_eq!(exceeded.limit, 4);
    assert_eq!(exceeded.consumed, 4);
    assert_eq!(meter.consumed_tool_steps, 1, "步数维没耗尽, 深度维照绑");
}

/// ⑤ 对抗: 出栈 / 报更小的深度都**洗不白**已越限的调用树。
#[test]
fn unwinding_or_reporting_a_smaller_depth_cannot_launder_the_overrun() {
    let mut depth = capped(2);
    depth.enter().expect("第一层");
    depth.enter().expect("第二层 (到顶)");
    assert_eq!(depth.exit(), 1);
    assert_eq!(depth.exit(), 0);
    assert_eq!(depth.current_depth(), 0);
    assert_eq!(depth.peak_depth(), 2, "峰值单调: 出栈不洗白");
    assert!(
        depth.check().is_some(),
        "已经到顶的调用树仍然到顶 (不能靠回卷绕过上限)"
    );
    // 观测口报一个更小的深度同样洗不白 (峰值只升不降)。
    assert_eq!(depth.observe_depth(0), 2);
    assert_eq!(depth.peak_depth(), 2);
    assert!(depth.check().is_some());
    // 越限后再进一层仍然被拒。
    assert!(depth.enter().is_err(), "回卷不能换来继续嵌套的权利");
}

/// ⑥ 深度为 0 的短路: 顶层回合不到上限, 出栈不下溢。
#[test]
fn depth_zero_short_circuits() {
    let mut depth = DepthQuota::unlimited();
    assert_eq!(depth.check(), None, "不设限: 顶层不触上限");
    assert_eq!(depth.enter(), Ok(1));
    assert_eq!(depth.exit(), 0);
    assert_eq!(depth.exit(), 0, "出栈不下溢: 深度不为负");
    assert_eq!(depth.current_depth(), 0);
    assert_eq!(depth.peak_depth(), 1);
    assert_eq!(depth.check(), None, "不设限: 任何深度都不出事实");

    // 设了上限时, 深度 0 (顶层回合) 也不到顶。
    let mut capped_zero = capped(1);
    assert_eq!(capped_zero.check(), None, "深度 0 < 上限 1: 短路, 不出事实");
    assert_eq!(capped_zero.enter(), Ok(1));
}

/// ⑦ 嵌套调用: 深度逐层累计, 峰值单调。
#[test]
fn nested_calls_accumulate_the_depth() {
    let mut depth = DepthQuota::unlimited();
    for expected in 1..=5u32 {
        assert_eq!(depth.enter(), Ok(expected));
    }
    assert_eq!(depth.current_depth(), 5);
    assert_eq!(depth.peak_depth(), 5);
    assert_eq!(depth.consumed(), 5);
    for expected in (0..5u32).rev() {
        assert_eq!(depth.exit(), expected);
    }
    assert_eq!(depth.current_depth(), 0);
    assert_eq!(depth.peak_depth(), 5, "出栈不降峰值 (已耗单调)");
}

/// ⑧ 既有调用树的深度读数喂进计量器 (不新造第二棵调用树)。
#[test]
fn the_existing_call_tree_readout_feeds_the_meter() {
    let mut depth = capped(2);
    assert_eq!(depth.observe_depth(1), 1, "侧车调用 = 第一层");
    assert_eq!(depth.check(), None);
    assert_eq!(depth.observe_depth(2), 2, "第二层 = 到顶");
    assert_eq!(
        depth.check(),
        Some(QuotaExceeded {
            dimension: QuotaDimension::Depth,
            limit: 2,
            consumed: 2,
        })
    );
}

/// ⑨ 零变化: 未启用深度维时, 旧路径**逐字节等**。
#[test]
fn an_unconfigured_depth_dimension_leaves_the_old_path_byte_identical() {
    let limits = SessionQuotaLimits {
        max_tokens: Some(10),
        max_tool_steps: Some(2),
        max_cost_micros: Some(1_000),
    };
    let depth = DepthQuota::unlimited();

    // 预算内。
    let mut meter = CognitiveQuota::default();
    meter.consume_tokens(5);
    meter.consume_tool_step();
    assert_eq!(limits.check_with_depth(&mut meter, &depth), None);
    assert_eq!(
        limits.check_with_depth(&mut meter, &depth),
        limits.check(&mut meter),
        "未启用深度维: 四维检查与旧三维检查逐字节等"
    );

    // token 维耗尽 (维度顺序不动: token 先于其余三维)。
    meter.consume_tokens(5);
    assert_eq!(
        limits.check_with_depth(&mut meter, &depth),
        limits.check(&mut meter)
    );
    assert_eq!(
        limits
            .check_with_depth(&mut meter, &depth)
            .map(|fact| fact.dimension),
        Some(QuotaDimension::Tokens)
    );

    // 步数维耗尽。
    let mut steps = CognitiveQuota::default();
    steps.consume_tool_step();
    steps.consume_tool_step();
    assert_eq!(
        limits.check_with_depth(&mut steps, &depth),
        limits.check(&mut steps)
    );
    assert_eq!(
        limits
            .check_with_depth(&mut steps, &depth)
            .map(|fact| fact.dimension),
        Some(QuotaDimension::ToolSteps)
    );

    // 花费维耗尽。
    let mut cost = CognitiveQuota::default();
    cost.consume_cost_micros(1_000);
    assert_eq!(
        limits.check_with_depth(&mut cost, &depth),
        limits.check(&mut cost)
    );
    assert_eq!(
        limits
            .check_with_depth(&mut cost, &depth)
            .map(|fact| fact.dimension),
        Some(QuotaDimension::CostMicros)
    );
}

/// ⑩ 深度维接既有拒绝语义: 同一份事实直接进既有预算耗尽帧 (不新增治理档位)。
#[test]
fn the_depth_fact_enters_the_existing_budget_exhausted_frame() {
    let mut depth = capped(2);
    depth.enter().expect("第一层");
    depth.enter().expect("第二层 (到顶)");
    let exceeded = depth.check().expect("到顶出事实");
    let frame = RuntimeError::BudgetExhausted {
        dimension: exceeded.dimension,
        limit: exceeded.limit,
        consumed: exceeded.consumed,
    };
    let text = frame.to_string();
    assert!(text.contains("depth"), "帧里带维度名: {text}");
    assert!(
        text.contains("3") || text.contains("2"),
        "帧里带真数字: {text}"
    );
}

/// ⑪ 深度维名稳定 (帧与自述面共用同一个名字)。
#[test]
fn the_depth_dimension_name_is_stable() {
    assert_eq!(QuotaDimension::Depth.as_str(), "depth");
    assert_eq!(QuotaDimension::Depth.to_string(), "depth");
}

/// ⑫ 自述读数是真数字 (不再是 null / 未接线标记)。
#[test]
fn the_depth_reading_is_a_real_number_even_without_a_ceiling() {
    let mut depth = DepthQuota::unlimited();
    for _ in 0..4 {
        depth.enter().expect("不设限: 放行");
    }
    let reading: DepthQuotaReading = depth.reading();
    assert_eq!(reading.limit, None, "不设限 = 上限侧 None (显示 '—')");
    assert_eq!(reading.consumed(), 4, "已耗是真数字 (峰值深度), 不是 null");
    assert_eq!(reading.current_depth, 4);
    assert_eq!(reading.peak_depth, 4);
}

/// ⑬ 解析纪律与配额家族一致 (正整数直通 / 0 与非法 = 不设限 / 越界钳制)。
#[test]
fn the_depth_ceiling_parses_like_the_rest_of_the_quota_family() {
    assert_eq!(parse_quota_max_depth(Some("4")), Some(4));
    assert_eq!(parse_quota_max_depth(Some("  8  ")), Some(8));
    for raw in [
        None,
        Some(""),
        Some("   "),
        Some("0"),
        Some("-2"),
        Some("abc"),
        Some("1.5"),
    ] {
        assert_eq!(
            parse_quota_max_depth(raw),
            None,
            "深度上限必须读 {raw:?} 为不设限"
        );
    }
    assert_eq!(
        parse_quota_max_depth(Some("99999999999999")),
        Some(MAX_QUOTA_DEPTH),
        "越界钳到家族锚点, 不发明更深的预算"
    );
}
