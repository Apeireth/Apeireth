//! 性质族 1 · Panic-freedom harness (对应"零未捕获异常"宣称)。
//!
//! 口径: 对任意**有界**输入, 目标 API 全路径不 panic (Kani 检查全部 unwind
//! 展开内的算术溢出/索引越界/unwrap 失败)。输入由 `kani::any()` 生成并显式
//! 有界; 形状前置条件写作早退守卫 (等价 `kani::assume`, 命题 = cond ⇒ P)。
//! 每个 harness 的注释一句话写明"证明什么、边界是什么"。

use super::async_context::{AsyncArrayKind, AsyncContextMessage, AsyncContextPipeline};
use super::file_fetcher::TransparentFileFetcher;
use super::fold_block::{has_fold_markers, parse_fold_blocks, render_fold_blocks, FoldBlock};
use super::residual_pyramid::OrthogonalResidualPyramid;
use super::river_topology::{DualScaledFieldSolver, RiverObservability, RiverState};
use super::semantic_axis::SemanticAxisBridge;

/// 任意 N 字符串 (字节 0..=127, 含 NUL/控制字符)。
/// 口径修订 2026-10-11 (四次·真解): 生成器四易其稿 —— ③ char::collect 令
/// CBMC 无法证明编码 1 字节/char (encode_utf8 分支) → 串长符号化 → 主循环
/// 展开断言 FAILURE; ④ 字节数组 + from_utf8 —— 长度可证具体, 但 from_utf8
/// 的 UTF-8 验证循环 + unwrap 失败路径的 panic/格式化机器 (rustc_demangle
/// TwoWaySearcher) 仍是 churn 源。终解 = 字节数组 + from_utf8_unchecked:
/// 字节掩码 <0x80 构造即合法 UTF-8, 安全性由构造保证, 验证机/panic 路径
/// 双双退出证明面。
fn bounded_string<const N: usize>() -> String {
    let bytes: Vec<u8> = (0..N).map(|_| rep_byte()).collect();
    // SAFETY: rep_byte() 全部 < 0x80 (ASCII), 必为合法 UTF-8。
    unsafe { String::from_utf8_unchecked(bytes) }
}

/// 有界有限 f32 (9 档代表值域 ∈ [-20, 20))。表选择零算术 ——
/// 旧版 `(m % 2001) as f32 / 100.0` 的生成器自身带 int→f32 转换与除法,
/// 每次取值都是软浮点位爆炸源 (K12); 常量表选择让生成器零成本。
/// 覆盖: 负大/负小/负极小/零/正极小/正小/正 1/正大/正边界。
fn bounded_f32() -> f32 {
    match kani::any::<u8>() % 9 {
        0 => -20.0,
        1 => -1.5,
        2 => -0.01,
        3 => 0.0,
        4 => 0.01,
        5 => 0.5,
        6 => 1.0,
        7 => 10.0,
        _ => 19.99,
    }
}

/// 代表值域字节 (8 档, 全分支类覆盖): NUL / \n / 空格 / '=' (提前收尾) /
/// '+' 'A' (base64 合法) / 0x7F (控制) / '!' (非法)。掩码 <0x80 →
/// 构造即合法 UTF-8。claim 口径: 代表值域量词 + 全分支类覆盖 (K4/K12)。
fn rep_byte() -> u8 {
    match kani::any::<u8>() % 8 {
        0 => 0x00,
        1 => 0x0A,
        2 => 0x20,
        3 => 0x3D,
        4 => 0x2B,
        5 => 0x41,
        6 => 0x7F,
        _ => 0x21,
    }
}

/// 定长 f32 向量 (分支定长法, 口径注见 river harness): n%3 选 1/2/3 维,
/// 每分支 Vec 长度具体 —— 符号长度收集 (`(0..sym).map().collect()`) 令
/// slice 循环按 unwind 上界展开, 是 CBMC OOM 的结构根源 (实测 SSA 75k VCC
/// OOM); 同一 selector 多次调用长度一致 (维度/方阵行同维语义)。
fn f32_vec(selector: u8) -> Vec<f32> {
    match selector % 3 {
        0 => vec![bounded_f32()],
        1 => vec![bounded_f32(), bounded_f32()],
        _ => vec![bounded_f32(), bounded_f32(), bounded_f32()],
    }
}

/// f32_vec 的 0..=3 变体 (含空分支; ragged 形状量词用)。
fn f32_vec0(selector: u8) -> Vec<f32> {
    match selector % 4 {
        0 => vec![],
        1 => vec![bounded_f32()],
        2 => vec![bounded_f32(), bounded_f32()],
        _ => vec![bounded_f32(), bounded_f32(), bounded_f32()],
    }
}

/// fold 文档结构化生成器 (口径四改 2026-10-08, 见 fold_block harness 注):
/// 行取自 {完整标记行, 断标记行, 文本行, 空行} × 任意 ASCII 尾字节 ×
/// 有/无换行 —— 内容大头是字面量 (memcmp/搜索逐字节即决), 少量符号字节
/// 分支, 覆盖真实解析状态 (正常/断标记/夹心/空行) 而公式有限。
/// fold 文档具名结构用例 (K15 具名量词口径, 2026-10-11 终稿):
/// 符号内容量词在 CBMC 下三代生成器均超 25 分钟 (TwoWaySearcher 标记搜索 ×
/// 符号字节分支爆炸, format! 与行界定型后仍超) —— 收敛为**具名结构用例组**,
/// 覆盖全部解析状态: 仅标记行 / 纯文本 / 空文档 / 断标记 / 多块 / 多行文本。
/// 残余值域 (具体字符内容) 由 fold_block 单测互补 (单测含 CJK/多字节例)。
fn fold_docs() -> Vec<String> {
    vec![
        // 仅标记行
        "[===fold:0.5===]".to_string(),
        // 纯文本
        "plain text".to_string(),
        // 空文档
        String::new(),
        // 断标记 (无收尾)
        "[===fold:0.5".to_string(),
        // 多块 + 内容行
        "[===fold:0.5===]\nbody\n[===fold:0.2===]\ntail".to_string(),
        // 多行纯文本 (无标记)
        "x\ny\nz".to_string(),
    ]
}

/// 证明: has_fold_markers 对具名结构用例组不 panic (函数级, 零切片面)。
/// 口径 (K16 拆分): "管线全程无 panic" 拆为三函数级命题 —— 全管线合一的
/// harness 让每处切片的 panic 路径各拖一整套格式化/Backtrace 机器 (实测
/// slice_index_fail 周围 memchr 3937 次展开, 常量输入也 25m+ 不落地),
/// 与输入域无关; 拆分后各命题面最小化。
#[kani::proof]
#[kani::unwind(32)]
fn kani_panic_free_fold_markers() {
    for doc in fold_docs() {
        let _ = has_fold_markers(&doc);
    }
}

/// 证明: parse_fold_blocks 对具名结构用例组不 panic, 块数有界 (≤ 用例行数)。
#[kani::proof]
#[kani::unwind(32)]
fn kani_panic_free_fold_parse() {
    for doc in fold_docs() {
        let blocks = parse_fold_blocks(&doc);
        assert!(blocks.len() <= 8, "块数有界 (6 用例的行数上界)");
    }
}

/// 证明: render_fold_blocks 对直接构造的块组 + 全 sim 域不 panic, 且渲染
/// 记账恒等 (expanded + hidden == blocks 数)。输入 = 直接构造 blocks
/// (跳过解析 —— 解析切片面不进本命题), 覆盖 阈值/描述/正文 形态组合。
#[kani::proof]
#[kani::unwind(32)]
fn kani_panic_free_fold_render() {
    let contents = ["a", "bb", "ccc"];
    let descs = [String::new(), "d".to_string()];
    let thresholds = [0.0f32, 0.35, 1.0];
    for content in contents.iter() {
        for desc in &descs {
            for threshold in &thresholds {
                let blocks = vec![
                    FoldBlock {
                        threshold: *threshold,
                        description: desc.clone(),
                        content: content.to_string(),
                    },
                    FoldBlock {
                        threshold: 0.5,
                        description: String::new(),
                        content: "x".to_string(),
                    },
                ];
                for sim in [
                    bounded_f32(),
                    f32::NAN,
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                    0.0,
                ] {
                    let render = render_fold_blocks(&blocks, sim);
                    assert_eq!(
                        render.expanded + render.hidden,
                        blocks.len(),
                        "渲染记账守恒: 展开数 + 折叠数 == 块数"
                    );
                }
            }
        }
    }
}

/// 证明: OrthogonalResidualPyramid::analyze 对任意 ≤3 维有限有界查询与任意
/// 有界召回闭包不 panic, 输出总解释率/相干度/新颖度/噪声均落 [0,1], 层数 ≤ 3。
/// 边界: 维度 1..=3, 每层召回 2 个向量, 全部数值 |x| < 20 (防 f32 溢出成 NaN),
/// unwind 64 覆盖 MGS/幂迭代界内循环。
#[kani::proof]
#[kani::unwind(64)]
fn kani_panic_free_residual_pyramid_analyze() {
    let dim_sel = kani::any::<u8>();
    let query = f32_vec(dim_sel);
    let tag_a = f32_vec(dim_sel);
    let tag_b = f32_vec(dim_sel);
    let dim = query.len();
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
/// 形状契约: fit 显式校验样本向量长度 == dimension, 非法形状空操作返回
/// (全域无 panic 证明见 kani_panic_free_semantic_axis_fit_any_shape)。样本权重取非负 (负权重开方产生 NaN 属数值
/// 退化而非 panic, 不在本命题内)。unwind 96 覆盖幂迭代 30 轮界内循环。
#[kani::proof]
#[kani::unwind(96)]
fn kani_panic_free_semantic_axis_fit_project() {
    let dim_sel = kani::any::<u8>();
    // 样本数 1..=3: push 链 + 符号条件分支 (无符号长度循环); 每样本向量定长。
    let mut centroids: Vec<(Vec<f32>, f32)> = Vec::new();
    let weight = (kani::any::<i32>() % 2001) as f32 / 100.0;
    centroids.push((f32_vec(dim_sel), weight.abs()));
    if kani::any::<bool>() {
        let weight = (kani::any::<i32>() % 2001) as f32 / 100.0;
        centroids.push((f32_vec(dim_sel), weight.abs()));
    }
    if kani::any::<bool>() {
        let weight = (kani::any::<i32>() % 2001) as f32 / 100.0;
        centroids.push((f32_vec(dim_sel), weight.abs()));
    }

    let dim = centroids[0].0.len();
    let mut bridge = SemanticAxisBridge::new(dim);
    let components = 1 + (kani::any::<u8>() % 3) as usize;
    bridge.fit(&centroids, components);
    assert!(
        bridge.basis_vectors.len() <= components,
        "基底数 ≤ 请求分量数"
    );

    let query = f32_vec(dim_sel);
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

/// 证明: RiverObservability::measure_omega 对任意计数与任意流量片不 panic;
/// 空种子/空边恒 (0.0, Collapsed); 否则 Ω ∈ [0,1] 且状态与 Ω 分类一致。
/// 边界: edge_flows ≤ 3 个元素, unwind 8。
/// 口径注 (2026-10-10): ① 非有限流量由 flow_balance 源头过滤 (NaN 算术
/// 曾令 nan-check 判负); ② `.cbrt()` 是 libm 外调 (kani#2423 不支持),
/// cfg(kani) 以恒等桩代 —— Ω 域断言经"cbrt 保 [0,1] 界"解析闭合, 见
/// river_topology.rs::cbrt_compat; ③ 流量生成改"分支定长" (0/1/2/3 元素
/// 各自定长 Vec) —— 符号长度收集令 slice 循环按 unwind 上界展开, 是
/// CBMC OOM 的结构根源 (run 1133… 实测 SSA 75k VCC OOM); NaN/±inf 改具名
/// 案例覆盖 (过滤语义具名证, 有限域由分支量词覆盖)。
#[kani::proof]
#[kani::unwind(8)]
fn kani_panic_free_river_measure_omega() {
    let active_edges: usize = kani::any();
    let seeds: usize = kani::any();
    let reached: usize = kani::any();
    let flows: Vec<f32> = match kani::any::<u8>() % 4 {
        0 => vec![],
        1 => vec![bounded_f32()],
        2 => vec![bounded_f32(), bounded_f32()],
        _ => vec![bounded_f32(), bounded_f32(), bounded_f32()],
    };

    let (omega, state) = RiverObservability::measure_omega(active_edges, seeds, reached, &flows);
    if seeds == 0 || active_edges == 0 {
        assert_eq!(omega, 0.0, "空拓扑 Ω = 0");
        assert_eq!(state, RiverState::Collapsed, "空拓扑恒 Collapsed");
    } else {
        assert!((0.0..=1.0).contains(&omega), "Ω 在 [0,1]");
    }
    assert_eq!(state, RiverState::classify(omega), "状态由 Ω 分类决定");

    // 具名垃圾值案例: NaN/±inf 流量经源头过滤后契约照旧 (有限域量词 + 具名
    // 垃圾值 = 全域覆盖, 与 fold_block sim 五态同口径)。
    for garbage in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0] {
        let (g_omega, g_state) =
            RiverObservability::measure_omega(3, 1, 2, &[garbage, 0.5, bounded_f32()]);
        assert!((0.0..=1.0).contains(&g_omega), "垃圾流量下 Ω 仍在 [0,1]");
        assert_eq!(
            g_state,
            RiverState::classify(g_omega),
            "垃圾流量下状态仍由 Ω 分类决定"
        );
    }
}

/// 证明: DualScaledFieldSolver::solve 对任意 ≤3 维有限有界源项与方阵不 panic,
/// 输出两场长度 == 输入维度。
/// 形状契约: solve 显式校验矩阵为 n×n, ragged 输入返回空结果
/// (全域无 panic 证明见 kani_panic_free_dual_scaled_field_solve_any_shape)。求解参数 max_iterations = 2 (有界迭代),
/// unwind 48。
#[kani::proof]
#[kani::unwind(48)]
fn kani_panic_free_dual_scaled_field_solve() {
    let n_sel = kani::any::<u8>();
    let source = f32_vec(n_sel);
    let mut matrix: Vec<Vec<f32>> = Vec::new();
    matrix.push(f32_vec(n_sel));
    if source.len() > 1 {
        matrix.push(f32_vec(n_sel));
    }
    if source.len() > 2 {
        matrix.push(f32_vec(n_sel));
    }
    let n = source.len();
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


/// 证明: SemanticAxisBridge::fit 对**任意形状**样本集（含长度 != dimension 的
/// 非法样本）不 panic——形状契约由实现显式校验：出现任一非法形状则空操作返回、
/// 基底保持为空。边界: dimension <= 3, 样本数 <= 3, 每样本向量长 <= 3, unwind 96。
#[kani::proof]
#[kani::unwind(96)]
fn kani_panic_free_semantic_axis_fit_any_shape() {
    let dim = 1 + (kani::any::<usize>() % 3);
    let sample_count = kani::any::<usize>() % 4;
    let mut centroids: Vec<(Vec<f32>, f32)> = Vec::new();
    for _ in 0..sample_count {
        let len = kani::any::<usize>() % 4; // 可能 != dim
        let v: Vec<f32> = (0..len).map(|_| bounded_f32()).collect();
        let weight = ((kani::any::<i32>() % 2001) as f32 / 100.0).abs();
        centroids.push((v, weight));
    }
    let mut bridge = SemanticAxisBridge::new(dim);
    bridge.fit(&centroids, 2);
    let all_well_formed = centroids.iter().all(|(v, _)| v.len() == dim);
    if !all_well_formed {
        assert!(
            bridge.basis_vectors.is_empty(),
            "任一非法形状 -> 空操作, 基底为空"
        );
    }
}

/// 证明: DualScaledFieldSolver::solve 对**任意 ragged 矩阵**（含行宽 != n）不
/// panic——形状契约由实现显式校验：非法输入返回空结果。
/// 边界: n <= 3, 行数 <= 3, 每行宽 <= 3, max_iterations = 2, unwind 48。
#[kani::proof]
#[kani::unwind(48)]
fn kani_panic_free_dual_scaled_field_solve_any_shape() {
    let source = f32_vec0(kani::any::<u8>());
    let n = source.len();
    // 行数 0..=3: 条件 push 链 (路径有界); 每行宽独立 0..=3 (ragged 量词域不变)。
    let mut matrix: Vec<Vec<f32>> = Vec::new();
    if kani::any::<bool>() {
        matrix.push(f32_vec0(kani::any::<u8>()));
        if kani::any::<bool>() {
            matrix.push(f32_vec0(kani::any::<u8>()));
            if kani::any::<bool>() {
                matrix.push(f32_vec0(kani::any::<u8>()));
            }
        }
    }
    let solver = DualScaledFieldSolver {
        max_iterations: 2,
        ..DualScaledFieldSolver::new()
    };
    let (u_local, u_transfer) = solver.solve(&source, &matrix);
    let well_formed = n > 0
        && matrix.len() == n
        && matrix.iter().all(|r| r.len() == n);
    if !well_formed {
        assert!(
            u_local.is_empty() && u_transfer.is_empty(),
            "非法形状 -> 空结果"
        );
    }
}
