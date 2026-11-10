//! `apeireth-memory::offline_embed` 的测试（**性质断言**，不钉死数值）。
//!
//! # 为什么测试**不**断言具体数值
//!
//! 本 embedder 的哈希实现（FNV-1a + murmur3 finalizer）是**实现细节**
//! ⇒ 一旦改动它，所有向量都会变 ⇒ 把数值写进测试 ⇒ **改实现就得改测试**
//! ⇒ 数值断言会把「实现」与「契约」焊死。
//!
//! ⇒ 本文件**只断言性质**：
//!
//! 1. **确定性**（同文本 ⇒ 逐字相等）；
//! 2. **维度固定**（输出长度 == `dimension`）；
//! 3. **相似文本更相近**（核心断言）；
//! 4. **归一化**（L2 范数 ≈ 1）；
//! 5. **边界不 panic**（空文本 / 零维 / 异长向量）。
//!
//! ⚠️ **刻意不依赖 `DefaultHasher`**（跨版本不稳定 ⇒ 数值不可复核）。

use apeireth_memory::offline_embed::{embed_text, OfflineEmbedder, DEFAULT_DIMENSION};
use apeireth_orchestration::context_fold::semantic::{cosine, Embedder};

/// 两段**高度相似**的文本（只差一个字）。
const SIMILAR_A: &str = "红楼梦里贾宝玉喜欢林黛玉";
/// 与 [`SIMILAR_A`] 高度相似。
const SIMILAR_B: &str = "红楼梦中贾宝玉喜欢林黛玉";
/// 与上面两段**无关**的文本。
const UNRELATED_A: &str = "水浒传里宋江在梁山泊聚义";
/// 与 [`UNRELATED_A`] 无关。
const UNRELATED_B: &str = "现代天文学测量恒星的光谱曲线";

/// L2 范数。
fn l2_norm(vector: &[f32]) -> f32 {
    vector.iter().map(|value| value * value).sum::<f32>().sqrt()
}

/// 测试 1：**确定性** —— 同文本两次 ⇒ 逐字相等。
#[test]
fn deterministic_same_text_same_vector() {
    let first = embed_text("关于黛玉，我们当时记了什么？", DEFAULT_DIMENSION);
    let second = embed_text("关于黛玉，我们当时记了什么？", DEFAULT_DIMENSION);
    assert_eq!(first, second, "同文本必须产出逐字相等的向量");
}

/// 测试 2：**确定性** —— 换调用顺序（中间夹别的文本）⇒ 仍逐字相等。
///
/// # 为什么要有这条
///
/// 若实现里藏了**全局可变状态**（例如「上次调用留下的统计量」）
/// ⇒ 顺序一变结果就变 ⇒ 卷子分数不可复现 ⇒ 这条钉死它。
#[test]
fn deterministic_independent_of_call_order() {
    let baseline = embed_text("賈母與寶玉", DEFAULT_DIMENSION);
    let _noise = embed_text("完全无关的另一段文字内容", DEFAULT_DIMENSION);
    let again = embed_text("賈母與寶玉", DEFAULT_DIMENSION);
    assert_eq!(baseline, again, "向量不得依赖调用历史");
}

/// 测试 3：**维度固定** —— 输出长度 == `dimension`。
#[test]
fn output_length_matches_dimension() {
    for dimension in [1usize, 8, 64, 256, 1024] {
        let vector = embed_text("宝黛故事", dimension);
        assert_eq!(vector.len(), dimension, "维度必须固定为 {}", dimension);
    }
}

/// 测试 4：**归一化** —— L2 范数 ≈ 1（容差 1e-5）。
#[test]
fn vector_is_l2_normalized() {
    let vector = embed_text("宝黛故事：黛玉葬花，宝玉痴情", DEFAULT_DIMENSION);
    let norm = l2_norm(&vector);
    assert!(
        (norm - 1.0).abs() < 1e-5,
        "非空文本的向量必须 L2 归一（实际范数 {norm}）"
    );
}

/// 测试 5：**相似文本更相近**（**核心断言**）。
///
/// # 为什么是核心
///
/// 这是「**它到底有没有信号**」的唯一判据：若相似文本的余弦**不高于**
/// 无关文本 ⇒ 这个 embedder **没有判别力** ⇒ 后面所有召回实验都白跑。
#[test]
fn similar_texts_are_closer_than_unrelated_texts() {
    let similar = cosine(
        &embed_text(SIMILAR_A, DEFAULT_DIMENSION),
        &embed_text(SIMILAR_B, DEFAULT_DIMENSION),
    );
    let unrelated = cosine(
        &embed_text(UNRELATED_A, DEFAULT_DIMENSION),
        &embed_text(UNRELATED_B, DEFAULT_DIMENSION),
    );
    assert!(
        similar > unrelated,
        "相似文本的余弦 ({similar:.6}) 必须高于无关文本的余弦 ({unrelated:.6})"
    );
}

/// 测试 6：**相似文本更相近**（更大维度下也要成立 ⇒ 排除「桶碰撞偶然」）。
#[test]
fn similar_texts_are_closer_at_large_dimension() {
    let dimension = 4096;
    let similar = cosine(
        &embed_text(SIMILAR_A, dimension),
        &embed_text(SIMILAR_B, dimension),
    );
    let unrelated = cosine(
        &embed_text(UNRELATED_A, dimension),
        &embed_text(UNRELATED_B, dimension),
    );
    assert!(
        similar > unrelated,
        "大维度下相似文本的余弦 ({similar:.6}) 仍须高于无关文本 ({unrelated:.6})"
    );
}

/// 测试 7：**自相似 = 1**（同一段文本与自己的余弦应为 1）。
#[test]
fn self_cosine_is_one() {
    let vector = embed_text("林黛玉焚稿断痴情", DEFAULT_DIMENSION);
    let score = cosine(&vector, &vector);
    assert!(
        (score - 1.0).abs() < 1e-5,
        "同一向量的余弦应为 1（实际 {score}）"
    );
}

/// 测试 8：**空文本** ⇒ 不 panic，返回全 0 向量。
///
/// # 口径
///
/// 空文本 ⇒ 无 n-gram ⇒ 无桶被点亮 ⇒ 全 0 ⇒ L2 范数 0
/// ⇒ 与既有 `cosine` 的「零向量 ⇒ 0.0」语义一致（**如实**，不是 NaN）。
#[test]
fn empty_text_returns_zero_vector_without_panic() {
    let vector = embed_text("", DEFAULT_DIMENSION);
    assert_eq!(vector.len(), DEFAULT_DIMENSION);
    assert!(vector.iter().all(|value| *value == 0.0), "空文本应返回全 0");
    assert_eq!(l2_norm(&vector), 0.0);
    // 与既有 `cosine` 语义一致：零向量 ⇒ 余弦 0（不是 NaN）。
    assert_eq!(cosine(&vector, &vector), 0.0);
}

/// 测试 9：**纯标点 / 纯空白** ⇒ 同样不 panic。
#[test]
fn punctuation_only_text_is_safe() {
    let vector = embed_text("，。！？  \t\n", DEFAULT_DIMENSION);
    assert_eq!(vector.len(), DEFAULT_DIMENSION);
    assert!(vector.iter().all(|value| *value == 0.0));
}

/// 测试 10：**零维** ⇒ `new` 直接拒绝；纯函数版返回空 Vec（不 panic）。
#[test]
fn zero_dimension_is_rejected() {
    assert!(
        OfflineEmbedder::new(0).is_err(),
        "零维必须被拒绝（零维向量会让余弦恒为 0 ⇒ 静默失效）"
    );
    let vector = embed_text("宝黛", 0);
    assert!(vector.is_empty(), "零维纯函数版应返回空 Vec");
}

/// 测试 11：**不同维度** ⇒ 余弦为 0（既有 `cosine` 的语义）。
#[test]
fn mismatched_dimensions_give_zero_cosine() {
    let left = embed_text("宝黛", 64);
    let right = embed_text("宝黛", 128);
    assert_eq!(cosine(&left, &right), 0.0, "维度不匹配 ⇒ 余弦必须是 0");
}

/// 测试 12：`OfflineEmbedder` 的 `embed` 与纯函数 `embed_text` **逐字相等**。
///
/// # 为什么要有这条
///
/// 两条入口**必须**是同一条算法 ⇒ 否则「测的是纯函数、生产用的是 trait 实现」
/// ⇒ 两边可能悄悄漂移 ⇒ 这条钉死它们。
#[test]
fn trait_embed_matches_pure_function() {
    let embedder = OfflineEmbedder::new(DEFAULT_DIMENSION).expect("维度合法");
    assert_eq!(
        embedder.embed("賈雨村與林黛玉"),
        embed_text("賈雨村與林黛玉", DEFAULT_DIMENSION)
    );
}

/// 测试 13：**默认 embedder** 的维度是 256，且 trait 生效。
#[test]
fn default_embedder_uses_default_dimension() {
    let embedder = OfflineEmbedder::default_embedder();
    assert_eq!(embedder.dimension, DEFAULT_DIMENSION);
    assert_eq!(embedder.embed("宝黛").len(), DEFAULT_DIMENSION);
}

/// 测试 14：**标点与空白不影响结果**（归一化输入的口径）。
///
/// # 为什么
///
/// 题面里有「，」「？」⇒ 若标点参与成词 ⇒ 同一主体的两个题面会得到
/// 不同的向量 ⇒ 判别力被无谓地削弱 ⇒ 这条钉死「标点不参与」。
#[test]
fn punctuation_and_whitespace_are_ignored() {
    let with_punctuation = embed_text("贾宝玉，林黛玉。", DEFAULT_DIMENSION);
    let without = embed_text("贾宝玉林黛玉", DEFAULT_DIMENSION);
    assert_eq!(
        with_punctuation, without,
        "标点与空白必须在归一化阶段被丢弃"
    );
}

/// 测试 15：**大小写无关**（ASCII 折小写）。
#[test]
fn ascii_case_is_folded() {
    assert_eq!(
        embed_text("Baoyu Daiyu", DEFAULT_DIMENSION),
        embed_text("baoyu daiyu", DEFAULT_DIMENSION)
    );
}

/// 测试 16：**重复文本的向量不等于单次文本**（TF 真的被计进去了）。
///
/// # 为什么
///
/// 若实现把 TF 抹平（只用「出现过没有」）⇒ 向量退化成**集合** ⇒
/// 文档长度信息全丢 ⇒ 长文档的权重被严重低估 ⇒ 这条钉死 TF 参与。
#[test]
fn term_frequency_matters() {
    let once = embed_text("宝黛", DEFAULT_DIMENSION);
    let repeated = embed_text("宝黛宝黛宝黛宝黛宝黛宝黛宝黛宝黛宝黛", DEFAULT_DIMENSION);
    assert_ne!(
        once, repeated,
        "TF 必须真的参与计算（否则退化成集合，文档长度信息丢失）"
    );
}
