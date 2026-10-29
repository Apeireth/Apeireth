//! `apeireth-memory::offline_embed` — **纯本地**的 embedder（无 API key、无网络）。
//!
//! # 为什么
//!
//! 仓里唯一的 `EmbeddingProvider` 实现需要 **API key + 网络**
//! ⇒ **离线环境跑不了语义召回** ⇒ 而「语义召回」是内部卷召回瓶颈的
//! **两大候选解法之一**（另一是「题面增信号」）。
//!
//! # ⚠️ 诚实的定位
//!
//! 本 embedder 是 **字符 n-gram 哈希投影** ⇒ 它**不是**语义模型
//! ⇒ 它**能**回答的是：「**词面饱和时，字符级信号还有没有用**」
//! ⇒ **不能**回答「语义召回行不行」（那需要真模型）。
//!
//! # 三条纪律
//!
//! 1. **确定性**：同文本 ⇒ 同向量（**不许**用随机数 / 不稳定的 hasher）；
//! 2. **不依赖网络**（离线可用）；
//! 3. **维度固定**（维度不匹配 ⇒ 余弦返回 0 ⇒ 静默失效 ⇒ 宁可显式错）。
//!
//! # 为什么不用 `std::collections::hash_map::DefaultHasher`
//!
//! `DefaultHasher` 的**输出不保证跨 Rust 版本稳定**（标准库明确不承诺）
//! ⇒ 换一次工具链，同一文本的向量就变 ⇒ **历史分数不可比**
//! ⇒ 本模块**自己实现** FNV-1a + murmur3 finalizer（两者都是**定死的常量**）
//! ⇒ 向量只取决于**文本本身**，与工具链版本无关。
//!
//! # 口径（钉死，便于复核）
//!
//! * **归一化输入**：只保留**字母数字**字符（丢掉空白与标点）⇒ 标点不参与成词
//!   （否则「，我」这种跨标点边界的碎片会进桶）；ASCII 字母折叠成小写。
//! * **n-gram 阶数**：[`NGRAM_SIZES`] = 2 / 3（字符级 ⇒ 不依赖分词词典）。
//! * **桶**：n-gram 的哈希 ⇒ 固定维度的一个桶；同桶**累加**计数
//!   （经典的 hashing vectorizer ⇒ 固定维度必然有碰撞，这是**已知代价**）。
//! * **权重**：**次线性 TF** `1 + ln(tf)`（长文里的高频片段不会线性压倒一切）。
//! * **归一**：L2 归一 ⇒ 余弦 = 点积。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-27）
//!
//! 四级口径（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：类型 + 算法 + 测试齐备（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：**生产 0 调用点**（只有 example 与测试在用）；
//! - **DEFAULT ENABLED** ✘：没有开关位；
//! - **HARDWARE VALIDATED** ✘：**未**在真实卷子上验证过「能否提 recall」
//!   （`examples/semantic_ab.rs` 是**人工在用**的对照实验，不是生产路径）。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。

use apeireth_orchestration::Embedder;

/// 默认向量维度。
///
/// 为什么是 256：足够让「常见字二元组」散开，又小到**每次查询全量打分也很快**
/// （对照实验要拿它给**整份语料**打分）。
pub const DEFAULT_DIMENSION: usize = 256;

/// 字符 n-gram 的阶数（2 / 3 字）。
///
/// 为什么是 2 / 3：2-gram 抓「词内相邻」，3-gram 抓「三字词」；
/// 再往上（如 4）会让**模板套话**（「我们当时记了什么」）的权重更重。
const NGRAM_SIZES: [usize; 2] = [2, 3];

/// 桶号缓冲区上限（`1` 字节阶数前缀 + 每个字符最多 4 字节 UTF-8）。
///
/// 现在的最大阶数是 3 ⇒ 最多 `1 + 3 * 4 = 13` 字节 ⇒ 16 足够。
const GRAM_BUFFER_BYTES: usize = 16;

/// FNV-1a 64 位的**初始偏移**（定死的常量 ⇒ 跨版本稳定）。
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;

/// FNV-1a 64 位的**质数**（定死的常量 ⇒ 跨版本稳定）。
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// 一个**纯本地**的 embedder。
///
/// # 定位（别误读）
///
/// 它是**词袋的推广**（字符级），**不是**语义模型
/// ⇒ 相似 ⇒ **字符面相似**，不代表「意思相近」。
#[derive(Debug, Clone)]
pub struct OfflineEmbedder {
    /// 向量维度。
    pub dimension: usize,
}

impl OfflineEmbedder {
    /// 构造（`dimension = 0` ⇒ **直接拒绝**（零维向量会让余弦恒为 0 ⇒ 静默失效））。
    ///
    /// # 为什么拒绝零维而不是「照做」
    ///
    /// 零维向量**看起来**能用（返回空 Vec）⇒ 但余弦恒为 0 ⇒ 检索**全部并列**
    /// ⇒ 结果**看着**是「召回为 0」，实则是**维度错了** ⇒ 静默失效最伤人
    /// ⇒ 宁可**构造时就报错**。
    pub fn new(dimension: usize) -> Result<Self, String> {
        if dimension == 0 {
            return Err("向量维度必须 > 0（零维向量会让余弦恒为 0 ⇒ 静默失效）".to_string());
        }
        Ok(Self { dimension })
    }

    /// **固有方法**版的 embed（与 `Embedder` trait 的实现**同口径**）。
    ///
    /// # 为什么**同时**提供固有方法
    ///
    /// 实现 `Embedder` trait 后，调用方必须 `use …::Embedder;` 才能调 `.embed()`
    /// ⇒ **忘了 import 就编译不过**（本仓真实踩过：测试里 `no method named embed`）。
    /// ⇒ 这里给一份**固有方法** ⇒ **两种调用方式都成立**
    /// ⇒ 且**两者必然一致**（固有方法就是 trait 实现的本体）。
    pub fn embed(&self, text: &str) -> Vec<f32> {
        embed_text(text, self.dimension)
    }

    /// 默认维度（**256**）。
    pub fn default_embedder() -> Self {
        Self {
            dimension: DEFAULT_DIMENSION,
        }
    }
}

impl Default for OfflineEmbedder {
    fn default() -> Self {
        Self::default_embedder()
    }
}

impl Embedder for OfflineEmbedder {
    fn embed(&self, text: &str) -> Vec<f32> {
        // ⚠️ **委托给固有方法**（单一实现源 ⇒ trait 版与固有版**不可能漂移**）。
        OfflineEmbedder::embed(self, text)
    }
}

/// 纯函数版 embed（便于测试与复核）。
///
/// # 口径
///
/// * `dimension = 0` ⇒ 返回**空 Vec**（不 panic；调用方拿到的余弦会是 0）；
/// * 文本**归一后为空**（全是空白/标点）或**太短**（不足 2 个字）
///   ⇒ 返回**全 0 向量**（L2 范数 0 ⇒ 余弦恒为 0 ⇒ 与既有 `cosine` 语义一致）；
/// * 其余 ⇒ **次线性 TF 加权 + L2 归一**（范数 ≈ 1）。
pub fn embed_text(text: &str, dimension: usize) -> Vec<f32> {
    if dimension == 0 {
        return Vec::new();
    }
    let counts = bucket_counts(text, dimension);
    let mut weights: Vec<f32> = counts
        .iter()
        .map(|&count| {
            if count == 0 {
                0.0
            } else {
                // 次线性 TF：长文里出现 50 次的片段不该压过出现 1 次的片段 50 倍。
                1.0 + (count as f32).ln()
            }
        })
        .collect();
    // L2 归一 ⇒ 余弦 = 点积（零向量保持零 ⇒ 余弦仍是 0，不产生 NaN）。
    let norm = weights
        .iter()
        .map(|weight| weight * weight)
        .sum::<f32>()
        .sqrt();
    if norm > 0.0 {
        for weight in weights.iter_mut() {
            *weight /= norm;
        }
    }
    weights
}

/// 归一化后的字符序列（只保留字母数字；ASCII 折小写）。
///
/// # 为什么丢掉标点与空白
///
/// 否则「，我」「了。」这种**跨标点边界**的碎片会成词 ⇒ 它们在语料里高频
/// ⇒ 桶被模板碎片占满 ⇒ **真信号被稀释**。
fn normalized_chars(text: &str) -> Vec<char> {
    text.chars()
        .filter(|ch| ch.is_alphanumeric())
        .map(|ch| {
            // `to_lowercase` 对绝大多数字符产出 1 个字符；取首个即可（非 ASCII 大写可能变多）。
            let mut lowered = ch.to_lowercase();
            lowered.next().unwrap_or(ch)
        })
        .collect()
}

/// 统计每个桶的 n-gram 出现次数。
fn bucket_counts(text: &str, dimension: usize) -> Vec<u32> {
    let chars = normalized_chars(text);
    let mut counts = vec![0u32; dimension];
    for &size in &NGRAM_SIZES {
        if chars.len() < size {
            continue;
        }
        for window in chars.windows(size) {
            let bucket = bucket_of(size, window, dimension);
            counts[bucket] += 1;
        }
    }
    counts
}

/// 一个 n-gram ⇒ 桶号。
///
/// # 为什么哈希里要带 `size` 前缀
///
/// 不带的话，2-gram 与 3-gram 的**公共前缀**会撞进同一个桶
/// ⇒ 不同阶数的信号**混在一起**，分不开。带前缀 ⇒ 两阶各占各的桶。
fn bucket_of(size: usize, window: &[char], dimension: usize) -> usize {
    let mut buffer = [0u8; GRAM_BUFFER_BYTES];
    let mut len = 0usize;
    buffer[len] = size as u8;
    len += 1;
    for ch in window {
        let mut encoded = [0u8; 4];
        let bytes = ch.encode_utf8(&mut encoded).as_bytes();
        let end = len + bytes.len();
        if end > GRAM_BUFFER_BYTES {
            // 不该发生（最大 3 字 ⇒ 13 字节 < 16）⇒ 保守截断，不 panic。
            break;
        }
        buffer[len..end].copy_from_slice(bytes);
        len = end;
    }
    let mixed = mix64(fnv1a(&buffer[..len]));
    (mixed % dimension as u64) as usize
}

/// FNV-1a 64 位（**定死的常量** ⇒ 跨 Rust 版本稳定）。
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// murmur3 的**收尾混合**（finalizer）：把哈希的高低位搅匀。
///
/// # 为什么要有它
///
/// FNV-1a 的**低位**比特在短字符串上分布不佳 ⇒ 直接 `% dimension`
/// 会让相邻桶的负载严重不均 ⇒ 碰撞不是「随机」而是「有结构的」
/// ⇒ 再加一层 avalanche（murmur3 finalizer，常数定死）⇒ 桶负载均衡。
fn mix64(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
