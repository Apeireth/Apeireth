//! `calibrate_demo` — 演示 ML 在线校准循环 (round15-01)
//!
//! 模拟 200 条 trace 的伪测量:
//! - 前 50 条 baseline 0.5
//! - 后 150 条整体漂移到 0.3 (dim 0 更剧烈)
//! - 中间手动加 user feedback
//! - 每 100 次 recalibrate, 触发 drift 检测
//!
//! 用法: cargo run -p apeireth-asi --example calibrate_demo

use apeireth_asi::{
    AdaptiveBaseline, DimensionTrace, DriftDetector, LinearCalibration, RecalibrationScheduler,
    UserFeedback, V05_DIMENSION_NAMES, V05_DIM_COUNT, V1136_SUBMEASURE_COUNT,
};

fn synth_trace(seq: usize, dim0_shift: f64) -> DimensionTrace {
    let base = 0.5;
    let drift = if seq > 50 {
        -0.2 * ((seq - 50) as f64 / 100.0)
    } else {
        0.0
    };
    let mut v05 = [base + drift; V05_DIM_COUNT];
    v05[0] = (0.5 + drift + dim0_shift).clamp(0.0, 1.0);
    let v1136 = [base + drift; V1136_SUBMEASURE_COUNT];
    DimensionTrace {
        trace_id: seq as u64,
        sample_id: seq as u64,
        timestamp: 1_700_000_000 + seq as i64,
        v05_dims: v05,
        v1136_subs: v1136,
        hook_overrides: vec![],
    }
}

fn main() {
    println!("=== apeireth-asi ML online calibration demo (round15-01) ===\n");

    let mut baseline = AdaptiveBaseline::with_alpha(0.05);
    let mut detector = DriftDetector::new(2.0, 3);
    let mut scheduler = RecalibrationScheduler::with_every_n(100);
    let calibrator = LinearCalibration::default();

    let total = 200;
    let mut drifts = 0usize;
    let mut recals = 0usize;

    for seq in 1..=total {
        // dim 0 后期极端偏离: 0.85 - 0.95
        let dim0_shift = if seq > 80 {
            0.4 + (seq as f64 * 0.001)
        } else {
            0.0
        };
        let trace = synth_trace(seq, dim0_shift);

        // 用户在 seq=120 加反馈
        if seq == 120 {
            scheduler.add_feedback(UserFeedback::for_dim(
                "thread_continuity",
                0.45, // 当前观测
                0.8,  // 期望
                seq as i64,
            ));
        }

        // 1) drift 检测 (先于 recalibration, 因为 baseline 反映最新)
        let alarms = detector.observe(&trace, &baseline);
        if !alarms.is_empty() {
            drifts += alarms.len();
            if seq <= 5 || seq % 50 == 0 {
                println!(
                    "seq={:>4}  DRIFT x{}: first={}",
                    seq,
                    alarms.len(),
                    alarms[0].name
                );
            }
        }

        // 2) scheduler 触发重新校准
        let report = scheduler.observe(&trace, &mut baseline, &calibrator, seq as i64, 50);
        if let Some(rep) = report {
            recals += 1;
            println!(
                "seq={:>4}  RECAL @ count={} reason={} feedback={} dry_run={}",
                seq, rep.trigger_count, rep.reason, rep.feedback_count, rep.dry_run
            );
        }
    }

    println!("\n--- summary ---");
    println!("total traces:        {}", total);
    println!("drift alarms:        {}", drifts);
    println!("recalibrations:      {}", recals);
    println!("scheduler.history:   {}", scheduler.history.len());
    println!(
        "baseline.dim_mean[0]: {:.4}",
        baseline.dim_mean[V05_DIMENSION_NAMES
            .iter()
            .position(|n| *n == "thread_continuity")
            .unwrap()]
    );
    println!(
        "baseline.dim_std[0]:  {:.4}",
        baseline.dim_std(
            V05_DIMENSION_NAMES
                .iter()
                .position(|n| *n == "thread_continuity")
                .unwrap()
        )
    );
    println!("\nDone.");
}
