//! `apeireth-web` ASI 24 维测量可视化模块 (R18 sub-agent #4)
//!
//! 端到端真接通 apeireth-asi 后端:
//! - `GET /asi`              → 24 维 SVG 雷达图 (V0.5)
//! - `GET /asi/calibration`  → ML 校准状态 (EMA + 漂移检测 + 重校准次数)
//!
//! 数据生成 (web 没有真测量源, demo 模式):
//! 每次请求跑 200 次合成测量, 用 `DimensionRegistry::compute_all_dims(&sample)`
//! 真调 apeireth-asi 24 个 measure_dim_* 函数, 喂入 AdaptiveBaseline + DriftDetector
//! + RecalibrationScheduler (calibrate_demo 同款模式).
//!
//! 24 维名称 = V05_DIMENSION_NAMES, 真值来源 `DimensionTrace.v05_dims`.
//!
//! 不引入新依赖: 沿用 axum / serde / apeireth-asi (workspace crate).

use std::collections::HashMap;

use apeireth_asi::{
    AdaptiveBaseline, DimensionRegistry, DimensionTrace, DriftDetector, LinearCalibration,
    MeasurementSample, RecalibrationScheduler, V05_DIMENSION_NAMES, V05_DIM_COUNT,
    V1136_SUBMEASURE_COUNT,
};
use axum::response::{Html, IntoResponse};

use crate::templates::{html_escape, render_error_page};

// ============================================================
// 常量
// ============================================================

/// 每次请求跑的合成测量次数 (和 calibrate_demo 对齐).
const DEMO_TOTAL: usize = 200;
/// 调度器重校准间隔 (同 calibrate_demo).
const SCHEDULER_EVERY_N: usize = 100;
/// scheduler observe 时用的历史窗口.
const HISTORY_WINDOW: usize = 50;
/// 哲学守门维度在 V05_DIMENSION_NAMES 中的位置范围 [start, end) — 单点用 `..`.
/// idx 15..=19 (v1/v2/v3/cone_of_truth/action_guard) 用 `philosophy_gate_trials` 字段,
/// 其余 19 个维度用 `successes/attempts/qualities` 字段.
/// compute_dim() 内部 (15..=19).contains(&dim_idx) 分支会优先读 philosophy_gate_trials.
const PHILOSOPHY_GATE_NAMES: &[&str] = &[
    "v1_pass_rate",
    "v2_pass_rate",
    "v3_pass_rate",
    "cone_of_truth_rate",
    "action_guard_rate",
];

/// 当前 epoch 起点, 模拟数据时间戳从这里累加.
const EPOCH_BASE: i64 = 1_700_000_000;

// ============================================================
// Demo 数据生成 (真调 apeireth-asi 24 维测量函数)
// ============================================================

/// 跑 N 次合成测量的累积状态 — 200 次 trace + 校准组件.
struct AsiDemoState {
    traces: Vec<DimensionTrace>,
    baseline: AdaptiveBaseline,
    detector: DriftDetector,
    scheduler: RecalibrationScheduler,
    drift_alarms: Vec<apeireth_asi::DriftAlarm>,
}

/// 跑一次完整的 demo 循环: 生成 sample → compute_all_dims → 喂 baseline/detector/scheduler.
/// 返回最终累积状态, handler 拿这个渲染页面.
fn run_demo_loop() -> AsiDemoState {
    let registry = DimensionRegistry::new();
    let mut baseline = AdaptiveBaseline::with_alpha(0.05);
    let mut detector = DriftDetector::new(2.0, 3);
    let mut scheduler = RecalibrationScheduler::with_every_n(SCHEDULER_EVERY_N);
    let calibrator = LinearCalibration::default();
    let mut traces: Vec<DimensionTrace> = Vec::with_capacity(DEMO_TOTAL);
    let mut drift_alarms: Vec<apeireth_asi::DriftAlarm> = Vec::new();

    for seq in 1..=DEMO_TOTAL {
        let trace = synth_and_measure(seq, &registry);
        // detector 先于 scheduler, 避免 baseline 更新后 z-score 失真
        let alarms = detector.observe(&trace, &baseline);
        for a in alarms {
            // 只收集首条以避免太长; 实际每个 alarm 都存
            drift_alarms.push(a);
        }
        scheduler.observe(
            &trace,
            &mut baseline,
            &calibrator,
            EPOCH_BASE + seq as i64,
            HISTORY_WINDOW,
        );
        traces.push(trace);
    }

    AsiDemoState {
        traces,
        baseline,
        detector,
        scheduler,
        drift_alarms,
    }
}

/// 合成一个 MeasurementSample, 然后真调 `DimensionRegistry::compute_all_dims`.
/// 模式: 前 50 次 base 0.5, 之后整体漂移到 0.3; dim_01 (thread_continuity) 后期极端偏离.
fn synth_and_measure(seq: usize, registry: &DimensionRegistry) -> DimensionTrace {
    let base = 0.5_f64;
    let drift = if seq > 50 {
        -0.2 * ((seq - 50) as f64 / 100.0)
    } else {
        0.0
    };
    let dim0_shift = if seq > 80 {
        0.4 + (seq as f64 * 0.001)
    } else {
        0.0
    };
    let base_f = (base + drift).clamp(0.0, 1.0);
    let dim0 = (0.5 + drift + dim0_shift).clamp(0.0, 1.0);

    let mut sample = MeasurementSample::default();
    // 19 个非哲学守门维度 (idx 0..=14 + 20..=23)
    for (i, name) in V05_DIMENSION_NAMES.iter().enumerate() {
        let v = if i == 0 { dim0 } else { base_f };
        // success / attempt 比 = v, attempts=100 → success = round(v*100)
        let attempts = 100u32;
        let success = (v * f64::from(attempts)).round() as u32;
        sample.successes.insert((*name).to_string(), success);
        sample.attempts.insert((*name).to_string(), attempts);
        sample.qualities.insert((*name).to_string(), 1.0);
    }
    // 5 个哲学守门维度 (idx 15..=19) — compute_dim 会优先读 philosophy_gate_trials
    for name in PHILOSOPHY_GATE_NAMES {
        let attempts = 100u32;
        let passed = (base_f * f64::from(attempts)).round() as u32;
        sample
            .philosophy_gate_trials
            .insert((*name).to_string(), (passed, attempts));
    }

    let v05_dims = registry.compute_all_dims(&sample);
    // sub 测度本 demo 不展开 (Mavis 任务主线是 24 维, sub 留给 calibrate_demo)
    let v1136_subs = [0.0_f64; V1136_SUBMEASURE_COUNT];

    DimensionTrace {
        trace_id: seq as u64,
        sample_id: seq as u64,
        timestamp: EPOCH_BASE + seq as i64,
        v05_dims,
        v1136_subs,
        hook_overrides: vec![],
    }
}

// ============================================================
// Handlers
// ============================================================

/// `GET /asi` — 24 维 V0.5 雷达图 (最新一次 trace + 历史均值).
pub async fn asi_page_handler() -> impl IntoResponse {
    let state = run_demo_loop();
    let latest = state
        .traces
        .last()
        .cloned()
        .unwrap_or_else(|| DimensionTrace {
            trace_id: 0,
            sample_id: 0,
            timestamp: 0,
            v05_dims: [0.0; V05_DIM_COUNT],
            v1136_subs: [0.0; V1136_SUBMEASURE_COUNT],
            hook_overrides: vec![],
        });
    let mean_24 = latest.mean_v05();

    // dim name → value 映射, 给 render_asi_radar 用
    let mut measurements: HashMap<String, f64> = HashMap::with_capacity(V05_DIM_COUNT);
    for (i, name) in V05_DIMENSION_NAMES.iter().enumerate() {
        measurements.insert((*name).to_string(), latest.v05_dims[i]);
    }

    Html(render_asi_radar_page(
        &measurements,
        mean_24,
        state.traces.len(),
    ))
}

/// `GET /asi/calibration` — ML 校准状态: EMA + 漂移检测 + 重校准次数.
pub async fn asi_calibration_handler() -> impl IntoResponse {
    let state = run_demo_loop();
    Html(render_asi_calibration_page(&state))
}

/// `POST /asi/calibrate` — 手动触发一次重校准 (force_run), 然后 302 → /asi/calibration.
///
/// 用 `Result<Redirect, Html<String>>` 让 axum 统一处理成功 (302) 和错误 (200 错误页).
pub async fn asi_calibrate_handler() -> Result<axum::response::Redirect, Html<String>> {
    let mut state = run_demo_loop();
    let calibrator = LinearCalibration::default();
    // 手动 force_run 一次: 跑 dry_run=false 写入新系数
    let now = state
        .traces
        .last()
        .map(|t| t.timestamp + 1)
        .unwrap_or(EPOCH_BASE);
    let _report =
        state
            .scheduler
            .force_run(&state.baseline, &calibrator, now, HISTORY_WINDOW, false);
    Ok(axum::response::Redirect::to("/asi/calibration"))
}

// ============================================================
// HTML 渲染
// ============================================================

/// 雷达图布局常量.
const RADAR_VIEWBOX_SIZE: f64 = 400.0;
const RADAR_CENTER: f64 = 200.0;
const RADAR_RADIUS: f64 = 150.0;

/// 24 维 SVG 雷达图 (主页面). 嵌入完整 HTML 页面.
fn render_asi_radar_page(
    measurements: &HashMap<String, f64>,
    mean_24: f64,
    total_traces: usize,
) -> String {
    // 准备 24 个 (x, y) 极坐标点: i / 24 圈, 角度 = -90° 起 (12 点钟)
    // 每个点半径 = value * RADAR_RADIUS (clamp 0..=1)
    let mut polygon_points = String::new();
    let mut axis_lines = String::new();
    let mut labels = String::new();
    // 同时计算一个值表 (HTML 表格)
    let mut table_rows = String::new();

    for (i, name) in V05_DIMENSION_NAMES.iter().enumerate() {
        let value = measurements.get(*name).copied().unwrap_or(0.0);
        let v = value.clamp(0.0, 1.0);
        let angle = (i as f64) * 2.0 * std::f64::consts::PI / (V05_DIM_COUNT as f64)
            - std::f64::consts::PI / 2.0;
        let r = v * RADAR_RADIUS;
        let x = RADAR_CENTER + r * angle.cos();
        let y = RADAR_CENTER + r * angle.sin();

        if i > 0 {
            polygon_points.push(' ');
        }
        polygon_points.push_str(&format!("{:.2},{:.2}", x, y));

        // 24 条轴线 (从中心到 1.0 圈)
        let x_outer = RADAR_CENTER + RADAR_RADIUS * angle.cos();
        let y_outer = RADAR_CENTER + RADAR_RADIUS * angle.sin();
        axis_lines.push_str(&format!(
            r##"<line x1="{cx}" y1="{cy}" x2="{x2:.2}" y2="{y2:.2}" stroke="#e0e0e0" stroke-width="0.5"/>"##,
            cx = RADAR_CENTER,
            cy = RADAR_CENTER,
            x2 = x_outer,
            y2 = y_outer
        ));
        axis_lines.push('\n');

        // 24 个轴 label (在 1.15 半径处), 短名字截断
        let short = short_label(name);
        let x_label = RADAR_CENTER + (RADAR_RADIUS + 18.0) * angle.cos();
        let y_label = RADAR_CENTER + (RADAR_RADIUS + 18.0) * angle.sin();
        let anchor = if angle.cos() > 0.3 {
            "start"
        } else if angle.cos() < -0.3 {
            "end"
        } else {
            "middle"
        };
        labels.push_str(&format!(
            r##"<text x="{:.2}" y="{:.2}" font-size="9" fill="#333" text-anchor="{}">{}</text>"##,
            x_label, y_label, anchor, short
        ));
        labels.push('\n');

        // 表格行
        table_rows.push_str(&format!(
            r##"<tr><td class="dim-idx">{}</td><td class="dim-name">{}</td><td class="dim-full">{}</td><td class="dim-bar"><div class="bar" style="width: {:.0}%"></div></td><td class="dim-val">{:.3}</td></tr>"##,
            i + 1,
            short,
            html_escape(name),
            v * 100.0,
            value
        ));
    }

    // 5 个同心参考圈 (0.2 / 0.4 / 0.6 / 0.8 / 1.0)
    let mut reference_circles = String::new();
    for frac in &[0.2_f64, 0.4, 0.6, 0.8, 1.0] {
        let r = frac * RADAR_RADIUS;
        reference_circles.push_str(&format!(
            r##"<circle cx="{cx}" cy="{cy}" r="{r:.2}" fill="none" stroke="#d0d0d0" stroke-width="0.5" stroke-dasharray="2 2"/>"##,
            cx = RADAR_CENTER,
            cy = RADAR_CENTER,
            r = r
        ));
        reference_circles.push('\n');
    }

    format!(
        r##"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <link id="leptos" rel="stylesheet" href="/style/main.css" />
    <title>Apeireth ASI 24 维雷达图 (V0.5)</title>
    <meta name="description" content="Apeireth ASI V0.5 24 维真实测量 (apeireth-asi 后端), SVG 雷达图可视化" />
    <style>
        .asi-radar-wrap {{ display: flex; gap: 24px; flex-wrap: wrap; align-items: flex-start; }}
        .asi-radar-svg {{ background: #fafafa; border: 1px solid #ddd; border-radius: 8px; padding: 12px; }}
        .asi-radar-info {{ flex: 1 1 320px; min-width: 280px; }}
        .asi-radar-info h2 {{ margin: 0 0 8px 0; font-size: 1.1em; }}
        .asi-radar-table {{ width: 100%; border-collapse: collapse; font-size: 0.85em; }}
        .asi-radar-table th, .asi-radar-table td {{ border-bottom: 1px solid #eee; padding: 4px 8px; text-align: left; }}
        .asi-radar-table th {{ background: #f0f0f0; font-weight: 600; }}
        .asi-radar-table .dim-idx {{ color: #888; font-family: monospace; width: 28px; }}
        .asi-radar-table .dim-name {{ font-family: monospace; color: #555; }}
        .asi-radar-table .dim-full {{ color: #333; }}
        .asi-radar-table .dim-bar {{ width: 100px; padding: 0; }}
        .asi-radar-table .bar {{ display: block; height: 8px; background: linear-gradient(90deg, #4287f5, #42c5f5); border-radius: 4px; }}
        .asi-radar-table .dim-val {{ font-family: monospace; text-align: right; width: 50px; }}
        .asi-summary {{ background: #f6f8fa; border-left: 3px solid #4287f5; padding: 12px 16px; margin: 16px 0; border-radius: 4px; }}
        .asi-summary code {{ background: #e1e4e8; padding: 1px 6px; border-radius: 3px; font-size: 0.9em; }}
    </style>
</head>
<body>
    <main class="apeireth-app">
        <header class="apeireth-header">
            <h1>Apeireth ASI · 24 维 V0.5 雷达图</h1>
            <p class="apeireth-tagline">R18 sub-agent #4 · 端到端真接通 apeireth-asi · DimensionRegistry::compute_all_dims</p>
        </header>

        <div class="asi-summary">
            <p>📊 <strong>本次测量</strong>: 跑 <code>{total}</code> 次合成测量后, 取最新一次 trace (trace_id = <code>{trace_id}</code>) 的 24 维值。</p>
            <p>📐 <strong>24 维均值</strong>: <code>{mean:.4}</code> (clamp [0, 1])</p>
            <p>🧮 <strong>公式</strong>: 19 维 = <code>successes / attempts × quality × latency_factor</code>, 5 维哲学守门 (v1/v2/v3/cone_of_truth/action_guard) = <code>passed / total</code> (clamp [0, 1]).</p>
            <p>🎯 <strong>trade-off</strong>: 当前为 demo 模式 (200 次合成 sample), 真生产环境应从 R17 Memory/Episode 抽取观察喂入 <code>MeasurementSample</code>。</p>
        </div>

        <div class="asi-radar-wrap">
            <svg class="asi-radar-svg" viewBox="0 0 {vb} {vb}" width="420" height="420" xmlns="http://www.w3.org/2000/svg">
                {ref_circles}
                {axis_lines}
                <polygon points="{poly}" fill="rgba(66,135,245,0.35)" stroke="#4287f5" stroke-width="1.5" />
                {labels}
            </svg>

            <div class="asi-radar-info">
                <h2>24 维详细数值 (V0.5 公式)</h2>
                <table class="asi-radar-table">
                    <thead>
                        <tr><th>#</th><th>短名</th><th>全名</th><th>条</th><th>值</th></tr>
                    </thead>
                    <tbody>
                        {table}
                    </tbody>
                </table>
            </div>
        </div>

        <div class="apeireth-actions">
            <a class="apeireth-button-link" href="/asi/calibration">📈 查看 ML 校准状态 (EMA + 漂移)</a>
            <a class="apeireth-button-link" href="/">← 返回首页</a>
        </div>
    </main>
</body>
</html>"##,
        total = total_traces,
        trace_id = total_traces,
        mean = mean_24,
        vb = RADAR_VIEWBOX_SIZE as i32,
        ref_circles = reference_circles,
        axis_lines = axis_lines,
        poly = polygon_points,
        labels = labels,
        table = table_rows,
    )
}

/// 把 24 维名压成 2 字母短名 (绕圈排列, 24 个字 label 全部可见但 12 象限内侧容易挤压).
/// 用 24 个 dim 名字前 4 字符的稳定映射; 避免长名重叠.
fn short_label(name: &str) -> String {
    // 短名: 维度的语义化中文 + 后缀
    let short = match name {
        "thread_continuity" => "线程连",
        "fact_recall" => "事实记",
        "context_window" => "上下文",
        "session_recovery" => "会话恢",
        "identity_persistence" => "身份持",
        "importance_score" => "重要性",
        "novelty_score" => "新颖度",
        "actionability_score" => "可执行",
        "confidence_score" => "置信度",
        "temporal_relevance" => "时序性",
        "core_values_consistency" => "核心值",
        "voice_consistency" => "语气一",
        "behavioral_patterns" => "行为模",
        "role_adherence" => "角色遵",
        "philosophy_alignment" => "哲学对",
        "v1_pass_rate" => "V1通过",
        "v2_pass_rate" => "V2通过",
        "v3_pass_rate" => "V3通过",
        "cone_of_truth_rate" => "真锥率",
        "action_guard_rate" => "动作守",
        "cross_domain_generalization" => "跨域泛",
        "abstraction_level" => "抽象层",
        "analogy_quality" => "类比质",
        "tool_reuse" => "工具复",
        _ => name,
    };
    short.to_string()
}

/// ML 校准状态页面: EMA 24 维均值 / 漂移 streak / 重校准次数。
fn render_asi_calibration_page(state: &AsiDemoState) -> String {
    let total_traces = state.traces.len();
    let recal_count = state.scheduler.history.len();
    let seen = state.baseline.seen;
    let alpha = state.baseline.alpha;
    let z_threshold = state.detector.z_threshold;
    let window_threshold = state.detector.window_threshold;

    // EMA 24 维均值表
    let mut ema_rows = String::new();
    for (i, name) in V05_DIMENSION_NAMES.iter().enumerate() {
        let m = state.baseline.dim_mean[i];
        let v = state.baseline.dim_var[i];
        ema_rows.push_str(&format!(
            r#"<tr><td class="dim-idx">{}</td><td class="dim-name">{}</td><td class="dim-val">{:.4}</td><td class="dim-val">{:.6}</td></tr>"#,
            i + 1,
            short_label(name),
            m,
            v
        ));
    }

    // 漂移 streak 表 (per-dim)
    let dim_streaks = state.detector.dim_streaks();
    let mut drift_rows = String::new();
    let mut active_drift_count = 0usize;
    for (i, name) in V05_DIMENSION_NAMES.iter().enumerate() {
        let s = dim_streaks[i];
        if s >= window_threshold {
            active_drift_count += 1;
        }
        let row_class = if s >= window_threshold {
            "drift-active"
        } else {
            ""
        };
        drift_rows.push_str(&format!(
            r#"<tr class="{cls}"><td class="dim-idx">{}</td><td class="dim-name">{}</td><td class="dim-val">{}</td><td>{}</td></tr>"#,
            i + 1,
            short_label(name),
            s,
            if s >= window_threshold { "🚨 ALARM" } else { "ok" },
            cls = row_class
        ));
    }

    // 漂移告警样本 (最多列前 5 条)
    let mut alarm_html = String::new();
    let alarms_to_show = state.drift_alarms.len().min(5);
    for a in state.drift_alarms.iter().take(alarms_to_show) {
        alarm_html.push_str(&format!(
            r#"<li><code>{name}</code>  z={z:+.2}σ  current={cur:.3}  mean={mean:.3}  std={std:.3}  streak={streak}</li>"#,
            name = html_escape(&a.name),
            z = a.z_score,
            cur = a.current,
            mean = a.mean,
            std = a.std,
            streak = a.streak
        ));
    }
    if alarm_html.is_empty() {
        alarm_html.push_str("<li>(无 — 200 次合成 trace 中 24 维均未连续 3 次超 2σ 阈值)</li>");
    }

    // 重校准历史 (每次 M=100 触发)
    let mut recal_rows = String::new();
    for (i, c) in state.scheduler.history.iter().enumerate() {
        let dim_scales: Vec<String> = (0..V05_DIM_COUNT)
            .map(|j| format!("{:.2}", c.dims[j].scale))
            .collect();
        let scales_preview = if dim_scales.len() > 5 {
            format!("{}, … (+{})", dim_scales[..5].join(","), V05_DIM_COUNT - 5)
        } else {
            dim_scales.join(",")
        };
        recal_rows.push_str(&format!(
            r#"<tr><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td></tr>"#,
            i + 1,
            c.sample_count,
            c.calibrated_at,
            scales_preview
        ));
    }
    if recal_rows.is_empty() {
        recal_rows.push_str(r#"<tr><td colspan="4">(无 — 200 次未触发 M=100 阈值)</td></tr>"#);
    }

    format!(
        r##"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <link id="leptos" rel="stylesheet" href="/style/main.css" />
    <title>Apeireth ASI · ML 校准状态</title>
    <meta name="description" content="Apeireth ASI ML 校准 (EMA + 漂移检测 + 重校准次数), 端到端真接通 apeireth-asi" />
    <style>
        .calib-grid {{ display: grid; grid-template-columns: 1fr; gap: 24px; margin: 16px 0; }}
        .calib-card {{ background: #fafafa; border: 1px solid #ddd; border-radius: 8px; padding: 16px; }}
        .calib-card h2 {{ margin: 0 0 12px 0; font-size: 1.1em; }}
        .calib-table {{ width: 100%; border-collapse: collapse; font-size: 0.85em; }}
        .calib-table th, .calib-table td {{ border-bottom: 1px solid #eee; padding: 4px 8px; text-align: left; }}
        .calib-table th {{ background: #f0f0f0; font-weight: 600; }}
        .calib-table .dim-idx {{ color: #888; font-family: monospace; width: 28px; }}
        .calib-table .dim-name {{ font-family: monospace; color: #555; }}
        .calib-table .dim-val {{ font-family: monospace; text-align: right; width: 80px; }}
        .calib-table tr.drift-active {{ background: #fff3f3; }}
        .calib-summary {{ background: #f6f8fa; border-left: 3px solid #4287f5; padding: 12px 16px; margin: 16px 0; border-radius: 4px; }}
        .calib-summary code {{ background: #e1e4e8; padding: 1px 6px; border-radius: 3px; font-size: 0.9em; }}
        .calib-stats {{ display: flex; gap: 16px; flex-wrap: wrap; }}
        .calib-stat {{ background: #fff; border: 1px solid #ddd; border-radius: 4px; padding: 8px 16px; }}
        .calib-stat .label {{ color: #888; font-size: 0.8em; }}
        .calib-stat .value {{ font-size: 1.4em; font-weight: 600; color: #333; font-family: monospace; }}
        ul.calib-alarm-list {{ margin: 0; padding-left: 20px; font-family: monospace; font-size: 0.85em; }}
    </style>
</head>
<body>
    <main class="apeireth-app">
        <header class="apeireth-header">
            <h1>Apeireth ASI · ML 校准状态</h1>
            <p class="apeireth-tagline">R18 sub-agent #4 · 端到端真接通 apeireth-asi · AdaptiveBaseline + DriftDetector + RecalibrationScheduler</p>
        </header>

        <div class="calib-summary">
            <p>🔄 <strong>校准循环</strong>: 跑 <code>{total}</code> 次合成 trace → AdaptiveBaseline (EMA) + DriftDetector (z≥{zt}σ 连续 {wt} 次) + RecalibrationScheduler (每 M={m} 次)</p>
            <div class="calib-stats">
                <div class="calib-stat"><div class="label">trace 数</div><div class="value">{total}</div></div>
                <div class="calib-stat"><div class="label">baseline.seen</div><div class="value">{seen}</div></div>
                <div class="calib-stat"><div class="label">alpha (EMA)</div><div class="value">{alpha}</div></div>
                <div class="calib-stat"><div class="label">重校准次数</div><div class="value">{recal}</div></div>
                <div class="calib-stat"><div class="label">活跃 drift 维度</div><div class="value">{active_drifts}</div></div>
            </div>
        </div>

        <div class="calib-grid">
            <div class="calib-card">
                <h2>📊 AdaptiveBaseline 24 维 EMA 均值 (滚动更新)</h2>
                <p>公式: <code>mean_new = α·x + (1-α)·mean_prev</code>, 默认 α={alpha}, seen={seen}</p>
                <table class="calib-table">
                    <thead><tr><th>#</th><th>维度</th><th>EMA 均值</th><th>EMA 方差</th></tr></thead>
                    <tbody>{ema_table}</tbody>
                </table>
            </div>

            <div class="calib-card">
                <h2>🚨 DriftDetector streak (z≥{zt}σ 连续 {wt} 次触发 ALARM)</h2>
                <p>当前活跃 ALARM 数: <strong>{active_drifts}</strong> / 24 维 (高亮行 = 已超阈值)</p>
                <table class="calib-table">
                    <thead><tr><th>#</th><th>维度</th><th>streak</th><th>状态</th></tr></thead>
                    <tbody>{drift_table}</tbody>
                </table>
                <h3 style="margin-top: 16px; font-size: 1em;">最近告警 (前 {alarms_n} 条)</h3>
                <ul class="calib-alarm-list">{alarm_list}</ul>
            </div>

            <div class="calib-card">
                <h2>🔧 RecalibrationScheduler history (M={m})</h2>
                <p>共 <strong>{recal}</strong> 次重校准 (history.len)。每行 = 一次触发的 <code>CalibrationCoefficients</code> (24 维 scale, 前 5 维预览)。</p>
                <table class="calib-table">
                    <thead><tr><th>#</th><th>sample_count</th><th>calibrated_at</th><th>scale [0..4]</th></tr></thead>
                    <tbody>{recal_table}</tbody>
                </table>
            </div>
        </div>

        <div class="apeireth-actions">
            <a class="apeireth-button-link" href="/asi">📊 查看 24 维雷达图</a>
            <a class="apeireth-button-link" href="/">← 返回首页</a>
        </div>
    </main>
</body>
</html>"##,
        total = total_traces,
        zt = z_threshold as i32,
        wt = window_threshold,
        m = SCHEDULER_EVERY_N,
        seen = seen,
        alpha = alpha,
        recal = recal_count,
        active_drifts = active_drift_count,
        ema_table = ema_rows,
        drift_table = drift_rows,
        alarms_n = alarms_to_show,
        alarm_list = alarm_html,
        recal_table = recal_rows,
    )
}

// ============================================================
// 错误渲染 (回退路径)
// ============================================================

/// 渲染 fallback 错误页 (demo 内部一般不会触发, 但作为防御性 API).
#[allow(dead_code)]
pub fn render_asi_error(msg: &str) -> String {
    render_error_page(&format!("ASI 模块错误: {msg}"))
}
