//! R25 D-3: trace_visualize example
//!
//! **目的**: 演示 3 个 advisor 协作任务 + reasoning trace 打印
//!
//! **per v2.0 strategy §2B 验收硬指标**:
//! - "3 个 advisor 协作完成任务的 demo" — 3 advisor (architect / security_reviewer / product_manager)
//! - "reasoning trace 可视化" — 走 `TraceReport::to_pretty_print()` + `to_json()` + `to_step_jsonl()` 3 格式
//!
//! **0 漂移**:
//! - 0 改 R10 既有
//! - 0 引入 I/O / 网络
//! - 0 触碰 workspace.version

use apeireth_council::advisor::StanceKind;
use apeireth_council::{
    CouncilMember, DebateMode, HierarchicalMode, PlannerExecutor, SubTask, TraceReport, Voter,
    VotingMode, VotingStrategy,
};

fn main() {
    println!("=================================================");
    println!("R25 D-3: 3 advisor 协作任务 + reasoning trace 可视化");
    println!("=================================================\n");

    // 1. 准备 3 个 advisor (per AutoGen 借鉴)
    let members = vec![
        CouncilMember::new("architect", "设计稳的架构", "10 年 Rust", "claude_code"),
        CouncilMember::new("security_reviewer", "找安全漏洞", "5 年安全", "codex"),
        CouncilMember::new("product_manager", "用户价值", "5 年产品", "gemini_cli"),
    ];

    let query = apeireth_council::deliberation::CouncilQuery::new(
        "trace-demo-q1",
        "deploy the new auth system to production",
        0,
    );

    // ====================================================================
    // Demo 1: Planner+Executor 模式
    // ====================================================================
    println!("--- Demo 1: Planner+Executor 模式 ---\n");
    let mut pe = PlannerExecutor::new("architect");
    let plan: Vec<SubTask> = pe.plan(&query);
    println!("Planner 拆解: {} 步", plan.len());
    for st in &plan {
        println!("  - Step {}: {}", st.step + 1, st.role);
    }
    let pe_verdict = pe.run(&query);
    let pe_trace = TraceReport::from_verdict(&pe_verdict).with_query(query.description.clone());
    println!(
        "\n[Planner+Executor Trace - Pretty]\n{}",
        pe_trace.to_pretty_print()
    );

    // ====================================================================
    // Demo 2: Debate 模式 (复用 R33-4-1)
    // ====================================================================
    println!("--- Demo 2: Debate 模式 (复用 R33-4-1) ---\n");
    let mut dm = DebateMode::new(members.clone());
    let d_verdict = dm.run(&query);
    let d_trace = TraceReport::from_verdict(&d_verdict).with_query(query.description.clone());
    println!("[Debate Trace - Pretty]\n{}", d_trace.to_pretty_print());

    // ====================================================================
    // Demo 3: Voting 模式
    // ====================================================================
    println!("--- Demo 3: Voting 模式 ---\n");
    let voters = vec![
        Voter::new(
            "v1",
            "architect",
            StanceKind::Approve,
            0.8,
            "design is sound",
        ),
        Voter::new(
            "v2",
            "security_reviewer",
            StanceKind::Approve,
            0.7,
            "no critical issues",
        ),
        Voter::new(
            "v3",
            "product_manager",
            StanceKind::Approve,
            0.9,
            "user value high",
        ),
    ];
    let mut vm = VotingMode::new(voters).with_strategy(VotingStrategy::WeightedMajority);
    let v_verdict = vm.run(&query);
    let v_trace = TraceReport::from_verdict(&v_verdict).with_query(query.description.clone());
    println!("[Voting Trace - Pretty]\n{}", v_trace.to_pretty_print());

    // ====================================================================
    // Demo 4: Hierarchical 模式
    // ====================================================================
    println!("--- Demo 4: Hierarchical 模式 ---\n");
    let mut hm = HierarchicalMode::new("cto");
    let h_verdict = hm.run(&query);
    let h_trace = TraceReport::from_verdict(&h_verdict).with_query(query.description.clone());
    println!(
        "[Hierarchical Trace - Pretty]\n{}",
        h_trace.to_pretty_print()
    );

    // ====================================================================
    // JSON 格式输出 (机器可读)
    // ====================================================================
    println!("--- JSON 格式 (整张 trace) ---\n");
    println!("{}", pe_trace.to_json());

    // ====================================================================
    // JSONL 格式输出 (claude_code trace 风格)
    // ====================================================================
    println!("--- JSONL 格式 (claude_code trace 风格) ---\n");
    println!("{}", pe_trace.to_step_jsonl());

    println!("=================================================");
    println!("Demo 完成: 4 模式 + 3 trace 格式");
    println!("=================================================");
}
