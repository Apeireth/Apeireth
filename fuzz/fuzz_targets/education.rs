#![no_main]

//! fuzz: **换元法校验面（`tool.education` / `DxCheckTool`）** —— 纯函数判据的机器守卫。
//!
//! 被测面（仓内源文件）：`crates/capabilities/tools/src/education.rs`
//! （`DxCheckTool::analyze` / `radical_content` / `classify_radical` / `DxReport`）。
//!
//! ## 为什么值得 fuzz
//!
//! 这是**给模型用的教学/校验面**：它把「模型写的换元推导」判成 PASS/WARN/FAIL。
//! ⇒ ⇒ **判错的代价是「教错」**（一个把 `dx` 与 `dt` 混用说成 PASS 的检查器，
//! 会让模型把错误推导当正确）⇒ ⇒ **判据本身必须被钉死**。
//! 模块头写死的性质：**纯确定性文本规则与符号特征解析**（**0 副作用**）⇒ ⇒
//! ⇒ **同输入恒同输出**（可精确断言）。
//!
//! ## 判据纪律（每条断言都从被测代码自己写明的分支/词表推出来，**不自造期望**）
//!
//! | # | 判据 | 语义出处 |
//! |---|---|---|
//! | O1 | **裁决三档闭合**：`verdict` ∈ {PASS, WARN, FAIL}；`issues` 非空 ⇒ FAIL；`issues` 空但 `tips` 非空 ⇒ WARN；两者皆空 ⇒ PASS | `analyze` 末尾的裁决块 |
//! | O2 | **微分标记分支**（`analyze` 的 ①）：空式子 ⇒ issue「为空」；`dx` + 新微分 ⇒ issue「混用」；`dx` + 有换元 ⇒ issue「忘换 dx」；两者皆无 ⇒ issue「缺少微分标记」；否则 ⇒ check「微分标记: X ✓」 | `analyze` 的 ① 分支链 |
//! | O3 | **闭合微分标记表**：`REPLACED_DIFFS` 恰 6 项（dt/du/ds/dz/dθ/dv）；「新微分」识别**只认这张表** | `REPLACED_DIFFS` 常量 + 注释「允许/识别的换元后微分标记」 |
//! | O4 | **残留原变量**：有换元 + 有新微分 + 式子里仍有 `x` ⇒ **tips**（不是 issue） | `analyze` 的 ② |
//! | O5 | **根式分类闭合**：`classify_radical` 只认四个模式（√(a²−x²) / √(x²−a²) / √(a²+x²) / √(ax+b)），其余 ⇒ `None` | `classify_radical` 的分类链 |
//! | O6 | **纯函数**：同输入 ⇒ 报告**逐字段相同**；`is_pass()` ⇔ `issues` 空 | 模块头「纯确定性」+ `is_pass` |
//! | O7 | **渲染不撒谎**：`render()` 必须与结构化字段一致（裁决词面与 checks/issues 数量对得上） | `DxReport::render` |
//!
//! ## 诚实边界
//!
//! - **不判断「数学上是否真的对」** —— 本面只做**文本规则**判定（这是它的合同）；
//! - 不测 CLI 出口与 `EducationTool` 的工具装配面（另立）；
//! - 不测 LaTeX 解析器本体（`radical_content` 只做**括号配平提取**，非完整解析）。

use libfuzzer_sys::fuzz_target;

use apeireth_tools_canonical::education::{DxCheckTool, REPLACED_DIFFS};

/// 微分标记候选池（含**表外**的形态，用于压「只认闭合表」）。
const DIFF_CANDIDATES: &[&str] = &["dt", "du", "ds", "dz", "dθ", "dv", "dx", "dq", "dw", "dy"];

/// 换元声明候选池。
const SUBSTITUTIONS: &[&str] = &[
    "",
    "t = 1+x^2",
    "u = sin(x)",
    "x = a·sin(θ)",
    "x = a·sec(θ)",
    "x = a·tan(θ)",
    "t = √(ax+b)",
    "y = f(x)",
];

/// 原式候选池（含三种经典根式 + 普通式）。
const PROBLEMS: &[&str] = &[
    "∫ x/(1+x^2) dx",
    "∫ √(a^2-x^2) dx",
    "∫ √(x^2-a^2) dx",
    "∫ √(a^2+x^2) dx",
    "∫ √(2x+3) dx",
    "∫ 1/x dx",
];

/// 换元后式候选池（覆盖 analyze 的 ① 的五条分支）。
const AFTERS: &[&str] = &[
    "",
    "∫ 1/(2t) dt",
    "∫ x dt dx",
    "∫ 1/(2t) dx",
    "∫ f(x) dx",
    "∫ 1/(2t) dt + x",
];

fn pick<T: Copy>(pool: &[T], byte: u8) -> T {
    pool[(byte as usize) % pool.len()]
}

fuzz_target!(|data: &[u8]| {
    let b = |i: usize| -> u8 { data.get(i).copied().unwrap_or(0) };

    let problem = pick(PROBLEMS, b(0));
    let substitution = pick(SUBSTITUTIONS, b(1));
    let after = pick(AFTERS, b(2));
    // 变体：把一个微分标记拼进式子（覆盖「表内/表外」的识别差异）
    let marker = pick(DIFF_CANDIDATES, b(3));
    let after = if b(4) % 2 == 0 {
        format!("{after} {marker}")
    } else {
        after.to_string()
    };

    let report = DxCheckTool::analyze(problem, substitution, &after);

    // ── O1 裁决三档闭合 + 与 issues/tips 一致
    assert!(
        report.verdict == "PASS" || report.verdict == "WARN" || report.verdict == "FAIL",
        "裁决词面必须闭合: {:?}",
        report.verdict
    );
    let expected_verdict = if !report.issues.is_empty() {
        "FAIL"
    } else if !report.tips.is_empty() {
        "WARN"
    } else {
        "PASS"
    };
    assert_eq!(
        report.verdict, expected_verdict,
        "裁决必须与 issues/tips 一致（issues⇒FAIL｜tips⇒WARN｜否则 PASS）"
    );

    // ── O6 is_pass ⇔ issues 空
    assert_eq!(
        report.is_pass(),
        report.issues.is_empty(),
        "is_pass() 必须等价于「issues 为空」"
    );

    // ── O2 微分标记分支（按 analyze 的 ① 分支链逐条重算）
    let after_trimmed = after.trim();
    let has_dx = after.contains("dx");
    let has_replaced = REPLACED_DIFFS.iter().any(|t| after.contains(t));
    let has_subs = !substitution.trim().is_empty();
    let issue_texts: Vec<&str> = report.issues.iter().map(|s| s.as_str()).collect();
    let check_texts: Vec<&str> = report.checks.iter().map(|s| s.as_str()).collect();
    if after_trimmed.is_empty() {
        assert!(
            issue_texts.iter().any(|t| t.contains("为空")),
            "空式子必须报「为空」: {issue_texts:?}"
        );
    } else if has_dx && has_replaced {
        assert!(
            issue_texts.iter().any(|t| t.contains("混用")),
            "dx 与新微分混用必须报「混用」: {issue_texts:?}"
        );
    } else if has_dx && !has_replaced && has_subs {
        assert!(
            issue_texts.iter().any(|t| t.contains("忘换 dx")),
            "声明换元却仍写 dx 必须报「忘换 dx」: {issue_texts:?}"
        );
    } else if !has_dx && !has_replaced {
        assert!(
            issue_texts.iter().any(|t| t.contains("缺少微分标记")),
            "两种微分都没有必须报「缺少微分标记」: {issue_texts:?}"
        );
    } else {
        assert!(
            check_texts.iter().any(|t| t.starts_with("微分标记:")),
            "微分标记正确必须进 checks（不是 issues）: checks={check_texts:?} issues={issue_texts:?}"
        );
    }

    // ── O3 闭合微分标记表
    assert_eq!(
        REPLACED_DIFFS.len(),
        6,
        "换元后微分标记表是闭合的（6 项）"
    );
    assert!(
        REPLACED_DIFFS.contains(&"dt")
            && REPLACED_DIFFS.contains(&"dθ")
            && !REPLACED_DIFFS.contains(&"dx"),
        "标记表必须含 dt/dθ 且**不含 dx**（dx 是原微分）"
    );

    // ── O4 残留原变量 ⇒ tips（不是 issue）
    if has_subs && has_replaced && after.contains('x') {
        assert!(
            report.tips.iter().any(|t| t.contains('x')),
            "残留原变量 x 必须给 tips: tips={:?}",
            report.tips
        );
    }

    // ── O5 根式分类闭合
    if let Some(content) = DxCheckTool::radical_content(problem) {
        if let Some((pattern, _tip)) = DxCheckTool::classify_radical(&content) {
            assert!(
                matches!(
                    pattern,
                    "√(a²−x²)" | "√(x²−a²)" | "√(a²+x²)" | "√(ax+b)"
                ),
                "根式分类必须落在四个经典模式内: {pattern:?}"
            );
        }
    }

    // ── O6 纯函数：同输入恒同输出
    let again = DxCheckTool::analyze(problem, substitution, &after);
    assert_eq!(
        report, again,
        "纯函数：同输入必须恒同输出（模块头「纯确定性文本规则」）"
    );

    // ── O7 渲染不撒谎
    let rendered = report.render();
    assert!(
        rendered.contains(&report.verdict),
        "渲染文本必须含裁决词面: {rendered:?}"
    );
    for check in &report.checks {
        assert!(
            rendered.contains(check.as_str()),
            "渲染文本必须含每条 check: check={check:?}"
        );
    }
    for issue in &report.issues {
        assert!(
            rendered.contains(issue.as_str()),
            "渲染文本必须含每条 issue: issue={issue:?}"
        );
    }
});
