//! 性质族 1 · Panic-freedom harness (对应"零未捕获异常"宣称)。
//!
//! 口径: 对任意**有界**输入, 目标 API 全路径不 panic (Kani 检查全部 unwind
//! 展开内的算术溢出/索引越界/unwrap 失败)。输入由 `kani::any()` 生成并显式
//! 有界; 形状前置条件写作早退守卫 (等价 `kani::assume`, 命题 = cond ⇒ P)。
//! 每个 harness 的注释一句话写明"证明什么、边界是什么"。

use super::async_context::{AsyncArrayKind, AsyncContextMessage, AsyncContextPipeline};
use super::file_fetcher::TransparentFileFetcher;
use super::fold_block::{has_fold_markers, parse_fold_blocks, render_fold_blocks};
use super::residual_pyramid::OrthogonalResidualPyramid;
use super::river_topology::{DualScaledFieldSolver, RiverObservability, RiverState};
use super::semantic_axis::SemanticAxisBridge;

/// 任意 N 字节串 (含非法 UTF-8 → lossy 替换; 含 NUL/控制字符均可)。
fn bounded_string<const N: usize>() -> String {
    let mut bytes: Vec<u8> = Vec::with_capacity(N);
    for _ in 0..N {
        bytes.push(kani::any::<u8>());
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// 有界有限 f32 ∈ (-20, 20) (经整数取模构造, 不会产生 NaN/inf)。
fn bounded_f32() -> f32 {
    let m: i32 = kani::any();
    (m % 2001) as f32 / 100.0
}

/// 证明: 任意 ≤8 字节串经 has_fold_markers/parse_fold_blocks/render_fold_blocks
/// 全程无 panic, 且渲染记账恒等 (expanded + hidden == blocks 数)。
/// 边界: 输入 8 字节 (行数 ≤ 8), similarity 为任意 f32 (含 NaN, 依契约视作 0.0),
/// unwind 64 覆盖逐行解析与 join。
#[kani::proof]
#[kani::unwind(64)]
fn kani_panic_free_fold_block_string_pipeline() {
    let s = bounded_string::<8>();
    let _markers = has_fold_markers(&s);
    let blocks = parse_fold_blocks(&s);
    let sim: f32 = kani::any();
    let render = render_fold_blocks(&blocks, sim);
    assert_eq!(
        render.expanded + render.hidden,
        blocks.len(),
        "渲染记账守恒: 展开数 + 折叠数 == 块数"
    );
}

/// 证明: OrthogonalResidualPyramid::analyze 对任意 ≤3 维有限有界查询与任意
/// 有界召回闭包不 panic, 输出总解释率/相干度/新颖度/噪声均落 [0,1], 层数 ≤ 3。
/// 边界: 维度 1..=3, 每层召回 2 个向量, 全部数值 |x| < 20 (防 f32 溢出成 NaN),
/// unwind 64 覆盖 MGS/幂迭代界内循环。
#[kani::proof]
#[kani::unwind(64)]
fn kani_panic_free_residual_pyramid_analyze() {
    let dim = 1 + (kani::any::<usize>() % 3);
    let query: Vec<f32> = (0..dim).map(|_| bounded_f32()).collect();
    let tag_a: Vec<f32> = (0..dim).map(|_| bounded_f32()).collect();
    let tag_b: Vec<f32> = (0..dim).map(|_| bounded_f32()).collect();
    let id_a: u64 = kani::any();
    let id_b: u64 = kani::any();

    let pyramid = OrthogonalResidualPyramid {
        dimension: dim,
        max_levels: 3,
        min_energy_ratio: 0.10,
    };
    let analysis = pyramid.analyze(&query, |_residual: &[f32], _top_k: usize| {
        vec![(id_a, tag_a.clone()), (id_b, tag_b.clone())]
    });

    assert!(analysis.levels.len() <= 3, "层数不得超过 max_levels");
    assert!(
        (0.0..=1.0).contains(&analysis.total_explained_ratio),
        "总解释率在 [0,1]"
    );
    assert!((0.0..=1.0).contains(&analysis.coherence), "相干度在 [0,1]");
    assert!(
        (0.0..=1.0).contains(&analysis.novelty_signal),
        "新颖度在 [0,1]"
    );
    assert!((0.0..=1.0).contains(&analysis.noise_signal), "噪声在 [0,1]");
}

/// 证明: SemanticAxisBridge::{fit, project} 对任意 ≤3 维有限有界样本不 panic,
/// 投影输出熵/逻辑深度落 [0,1], 主轴数 ≤ 请求分量数; 长度不匹配的查询向量
/// 走早退分支不 panic。
/// 边界 (API 形状契约): fit 要求每个样本向量长度 == dimension —— 实现按
/// 0..dimension 索引 vector[i], 短向量会索引越界 panic (见 README 缺陷记录);
/// 本 harness 在形状契约内证明。样本权重取非负 (负权重开方产生 NaN 属数值
/// 退化而非 panic, 不在本命题内)。unwind 96 覆盖幂迭代 30 轮界内循环。
#[kani::proof]
#[kani::unwind(96)]
fn kani_panic_free_semantic_axis_fit_project() {
    let dim = 1 + (kani::any::<usize>() % 3);
    let sample_count = 1 + (kani::any::<usize>() % 3);
    let mut centroids: Vec<(Vec<f32>, f32)> = Vec::new();
    for _ in 0..sample_count {
        let v: Vec<f32> = (0..dim).map(|_| bounded_f32()).collect();
        let weight = (kani::any::<i32>() % 2001) as f32 / 100.0; // [0,20) 取绝对值如下
        centroids.push((v, weight.abs()));
    }

    let mut bridge = SemanticAxisBridge::new(dim);
    let components = 1 + (kani::any::<usize>() % 3);
    bridge.fit(&centroids, components);
    assert!(
        bridge.basis_vectors.len() <= components,
        "基底数 ≤ 请求分量数"
    );

    let query: Vec<f32> = (0..dim).map(|_| bounded_f32()).collect();
    let projection = bridge.project(&query);
    assert!(
        (0.0..=1.0).contains(&projection.normalized_entropy),
        "归一化熵在 [0,1]"
    );
    assert!(
        (0.0..=1.0).contains(&projection.logic_depth),
        "逻辑深度在 [0,1]"
    );
    assert!(projection.resonance_score >= 0.0, "共振强度非负");

    // 长度不匹配的查询: 实现早退返回默认值, 不 panic。
    let _ = bridge.project(&[]);
}

/// 证明: RiverObservability::measure_omega 对任意计数与任意 f32 流量片
/// (含 NaN/±inf) 不 panic; 空种子/空边恒 (0.0, Collapsed);
/// 否则 Ω ∈ [0,1] 且状态与 Ω 分类一致。
/// 边界: edge_flows ≤ 3 个元素, unwind 32。
#[kani::proof]
#[kani::unwind(32)]
fn kani_panic_free_river_measure_omega() {
    let active_edges: usize = kani::any();
    let seeds: usize = kani::any();
    let reached: usize = kani::any();
    let flow_count = kani::any::<usize>() % 4;
    let flows: Vec<f32> = (0..flow_count).map(|_| kani::any::<f32>()).collect();

    let (omega, state) = RiverObservability::measure_omega(active_edges, seeds, reached, &flows);
    if seeds == 0 || active_edges == 0 {
        assert_eq!(omega, 0.0, "空拓扑 Ω = 0");
        assert_eq!(state, RiverState::Collapsed, "空拓扑恒 Collapsed");
    } else {
        assert!((0.0..=1.0).contains(&omega), "Ω 在 [0,1]");
    }
    assert_eq!(state, RiverState::classify(omega), "状态由 Ω 分类决定");
}

/// 证明: DualScaledFieldSolver::solve 对任意 ≤3 维有限有界源项与方阵不 panic,
/// 输出两场长度 == 输入维度。
/// 边界 (API 形状契约): solve 仅检查 adjacency_matrix.len() == n, 行短于 n 时
/// relax 内 adjacency_matrix[j][i] 索引越界 panic (见 README 缺陷记录);
/// 本 harness 在方阵形状契约内证明。求解参数 max_iterations = 2 (有界迭代),
/// unwind 48。
#[kani::proof]
#[kani::unwind(48)]
fn kani_panic_free_dual_scaled_field_solve() {
    let n = 1 + (kani::any::<usize>() % 3);
    let source: Vec<f32> = (0..n).map(|_| bounded_f32()).collect();
    let mut matrix: Vec<Vec<f32>> = Vec::new();
    for _ in 0..n {
        matrix.push((0..n).map(|_| bounded_f32()).collect());
    }
    let solver = DualScaledFieldSolver {
        alpha_local: bounded_f32(),
        alpha_transfer: bounded_f32(),
        max_iterations: 2,
        tolerance: 0.0,
    };
    let (u_local, u_transfer) = solver.solve(&source, &matrix);
    assert_eq!(u_local.len(), n, "局域场长度 == 维度");
    assert_eq!(u_transfer.len(), n, "迁移场长度 == 维度");
}

/// 证明: TransparentFileFetcher::compute_cache_key 对任意 ≤2 字节 URL 不 panic,
/// 输出恰为 64 个小写十六进制字符 (SHA-256 摘要格式健全)。
/// 边界: URL 2 字节 (单 SHA-256 压缩块符号执行), sha2 用 force-soft 纯软件后端,
/// unwind 128 覆盖压缩函数 64 轮循环与 64 字符格式化循环。
#[kani::proof]
#[kani::unwind(128)]
fn kani_panic_free_compute_cache_key() {
    let s = bounded_string::<2>();
    let key = TransparentFileFetcher::compute_cache_key(&s);
    assert_eq!(key.len(), 64, "SHA-256 十六进制摘要恰 64 字符");
    assert!(
        key.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
        "摘要只含小写十六进制"
    );
}

/// 证明: AsyncContextPipeline::{push_message, assemble_prompt_context,
/// post_inference_cleanup} 对任意 ≤3 条有界消息不 panic, 且装配守恒:
/// 组装条数 == 管线总条数; 清理 (clear_hud=true) 后只剩持久事实与摘要层。
/// 边界: 消息文本 ≤4 字节, 推入 3 条 (有界队列上界由 MAX_* 常量另测), unwind 48。
#[kani::proof]
#[kani::unwind(48)]
fn kani_panic_free_async_context_pipeline() {
    let max_hud = 1 + (kani::any::<usize>() % 3);
    let mut pipeline = AsyncContextPipeline::new(max_hud);

    let push_count = kani::any::<usize>() % 4;
    for _ in 0..push_count {
        let kind = match kani::any::<u8>() % 4 {
            0 => AsyncArrayKind::EphemeralAsyncUser,
            1 => AsyncArrayKind::DurableSyncUser,
            2 => AsyncArrayKind::SummaryStatusUser,
            _ => AsyncArrayKind::NotificationHUDUser,
        };
        let msg = AsyncContextMessage::new(
            bounded_string::<4>(),
            kind,
            bounded_string::<4>(),
            bounded_string::<4>(),
            kani::any::<u64>(),
        );
        pipeline.push_message(msg);
    }

    let assembled = pipeline.assemble_prompt_context();
    assert_eq!(
        assembled.len(),
        pipeline.total_messages_count(),
        "装配守恒: 组装输出条数 == 管线总条数"
    );

    let _cleared = pipeline.post_inference_cleanup(true);
    assert_eq!(
        pipeline.total_messages_count(),
        pipeline.export_durable_facts().len() + pipeline.export_summary_records().len(),
        "清理后仅剩持久事实层与摘要层"
    );
}
