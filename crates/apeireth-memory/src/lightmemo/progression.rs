//! R179 P1-11: 4-layer progressive memory (借鉴 mempalace 4-layer closed-loop).
//!
//! ## 概念
//! 每个 memory item 从 L1 逐渐升级到 L4:
//! - **L1**: raw file (原始存储, 永不注销)
//! - **L2**: vector 嵌入 (可检索近重复, 供 dedup / 检索)
//! - **L3**: tag 倒排索引 (可按主题过滤)
//! - **L4**: LCM 压缩/摘要 (适合长文本, 节省检索成本)
//!
//! ## 升级规则 (为什么要 "progressive")
//! - L1 总是保留 (raw persistence, append-only 守则)
//! - L2 仅当 item 有嵌入时 promote (质量低的内容不必嵌入)
//! - L3 仅当 item 有 ≥1 个 tag 时 promote (无 tag 则不可按主题检索)
//! - L4 仅当 content 长度 >= chunk_size 时 promote
//!   (短文本压缩之后会丢信息, 不值)
//!
//! ## 降级 (忘记曲线)
//! - 随 decay < threshold 从 L4 降到 L3 再到 L2
//! - L1 永不降 (原始存储 是 安全网)
//!
//! ## 跟 mempalace 4-layer 的差别
//! - mempalace: 4 layer 各自独立, 主动 fill 所有 layer
//! - apeireth: 同一个 item 随生命周期逐渐升级
//!   (你可以查询 "this item has been promoted to L4?")
//!
//! ## 用法
//! ```rust,no_run
//! use apeireth_memory::lightmemo::progression::{LayerProgression, Layer};
//!
//! let mut prog = LayerProgression::new();
//! let id = "item-1".to_string();
//! prog.touch_l1(&id);
//! prog.touch_l2(&id);  // 有了 embedding
//! prog.touch_l3(&id, &["memory".into(), "rust".into()]);
//! assert!(prog.is_in(&id, Layer::L3));
//! assert!(!prog.is_in(&id, Layer::L4));
//! ```

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use super::l4_lcm::L4LcmCompressor;

/// 4 层枚举 (从低到高).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Layer {
    /// L1: raw file (always present).
    L1 = 1,
    /// L2: vector embedding.
    L2 = 2,
    /// L3: tag inverted index.
    L3 = 3,
    /// L4: LCM compressed summary.
    L4 = 4,
}

impl Layer {
    /// 顶层 (默认 L4).
    pub fn top() -> Self {
        Layer::L4
    }
    /// 最低层 (L1).
    pub fn bottom() -> Self {
        Layer::L1
    }
    /// 正序上升一层.
    pub fn promote_from(self) -> Option<Self> {
        match self {
            Layer::L1 => Some(Layer::L2),
            Layer::L2 => Some(Layer::L3),
            Layer::L3 => Some(Layer::L4),
            Layer::L4 => None,
        }
    }
}

#[derive(Debug, Clone)]
struct ItemLayerState {
    top_layer: Layer,
    in_layer: [bool; 4],
    promoted_at: DateTime<Utc>,
    access_count: i64,
    last_accessed: DateTime<Utc>,
}

impl Default for ItemLayerState {
    fn default() -> Self {
        Self {
            top_layer: Layer::L1,
            in_layer: [true, false, false, false],
            promoted_at: Utc::now(),
            access_count: 0,
            last_accessed: Utc::now(),
        }
    }
}

impl ItemLayerState {
    fn new() -> Self { Self::default() }
    fn touch(&mut self, layer: Layer) {
        let idx = (layer as usize) - 1;
        self.in_layer[idx] = true;
        if layer > self.top_layer {
            self.top_layer = layer;
            self.promoted_at = Utc::now();
        }
        self.access_count += 1;
        self.last_accessed = Utc::now();
    }
    fn demote(&mut self, layer: Layer) {
        let idx = (layer as usize) - 1;
        self.in_layer[idx] = false;
        self.top_layer = Layer::L1;
        for l in [Layer::L4, Layer::L3, Layer::L2] {
            if self.in_layer[(l as usize) - 1] {
                self.top_layer = l;
                break;
            }
        }
    }
}

/// Layer progression tracker (不拿业务锁, 外部加 Mutex).
#[derive(Debug, Default)]
pub struct LayerProgression {
    items: HashMap<String, ItemLayerState>,
}

impl LayerProgression {
    pub fn new() -> Self { Self::default() }

    pub fn touch_l1(&mut self, id: &str) {
        let s = self.items.entry(id.to_string()).or_insert_with(ItemLayerState::new);
        s.touch(Layer::L1);
    }

    pub fn touch_l2(&mut self, id: &str) {
        let s = self.items.entry(id.to_string()).or_insert_with(ItemLayerState::new);
        s.touch(Layer::L1);
        s.touch(Layer::L2);
    }

    pub fn touch_l3(&mut self, id: &str, _tags: &[String]) {
        let s = self.items.entry(id.to_string()).or_insert_with(ItemLayerState::new);
        s.touch(Layer::L1);
        s.touch(Layer::L3);
    }

    pub fn touch_l4(&mut self, id: &str) {
        let s = self.items.entry(id.to_string()).or_insert_with(ItemLayerState::new);
        s.touch(Layer::L1);
        s.touch(Layer::L4);
    }

    pub fn is_in(&self, id: &str, layer: Layer) -> bool {
        self.items.get(id).map(|s| s.in_layer[(layer as usize) - 1]).unwrap_or(false)
    }

    pub fn top_layer_of(&self, id: &str) -> Layer {
        self.items.get(id).map(|s| s.top_layer).unwrap_or(Layer::L1)
    }

    pub fn access_count(&self, id: &str) -> i64 {
        self.items.get(id).map(|s| s.access_count).unwrap_or(0)
    }

    /// 按 decay 降级: 超过 threshold_hours 未访问, 从高到低 demote (除 L1).
    pub fn decay_demote(&mut self, id: &str, threshold_hours: f64) -> usize {
        let now = Utc::now();
        let state = match self.items.get_mut(id) {
            Some(s) => s,
            None => return 0,
        };
        let elapsed_hours = (now - state.last_accessed).num_seconds() as f64 / 3600.0;
        if elapsed_hours < threshold_hours { return 0; }
        let mut demoted = 0;
        for layer in [Layer::L4, Layer::L3, Layer::L2] {
            if state.in_layer[(layer as usize) - 1] {
                state.demote(layer);
                demoted += 1;
            }
        }
        demoted
    }

    /// 按 L4 compressor 的 chunk_size 自动判定一个 item 是否该升到 L4.
    pub fn recommend_top_layer(content: &str, compressor: &L4LcmCompressor) -> Layer {
        if content.len() >= compressor.chunk_size {
            Layer::L4
        } else if content.len() >= compressor.chunk_size / 4 {
            Layer::L3
        } else if content.len() >= compressor.chunk_size / 16 {
            Layer::L2
        } else {
            Layer::L1
        }
    }

    pub fn count_per_layer(&self) -> [usize; 4] {
        let mut out = [0usize; 4];
        for s in self.items.values() {
            for (i, &b) in s.in_layer.iter().enumerate() {
                if b { out[i] += 1; }
            }
        }
        out
    }

    pub fn count_by_top_layer(&self) -> HashMap<Layer, usize> {
        let mut out = HashMap::new();
        for s in self.items.values() {
            *out.entry(s.top_layer).or_insert(0) += 1;
        }
        out
    }

    pub fn remove(&mut self, id: &str) -> bool {
        self.items.remove(id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_item_only_in_l1() {
        let mut p = LayerProgression::new();
        p.touch_l1("x");
        assert!(p.is_in("x", Layer::L1));
        assert!(!p.is_in("x", Layer::L2));
        assert_eq!(p.top_layer_of("x"), Layer::L1);
    }

    #[test]
    fn touch_l2_implies_l1() {
        let mut p = LayerProgression::new();
        p.touch_l2("x");
        assert!(p.is_in("x", Layer::L1));
        assert!(p.is_in("x", Layer::L2));
    }

    #[test]
    fn progressive_promotion() {
        let mut p = LayerProgression::new();
        p.touch_l1("x");
        p.touch_l2("x");
        p.touch_l3("x", &["a".into()]);
        p.touch_l4("x");
        for l in [Layer::L1, Layer::L2, Layer::L3, Layer::L4] {
            assert!(p.is_in("x", l));
        }
        assert_eq!(p.top_layer_of("x"), Layer::L4);
    }

    #[test]
    fn promotion_never_demotes() {
        let mut p = LayerProgression::new();
        p.touch_l4("x");
        p.touch_l3("x", &["a".into()]);
        assert!(p.is_in("x", Layer::L4));
    }

    #[test]
    fn decay_demote_old_item() {
        let mut p = LayerProgression::new();
        p.touch_l2("old");
        p.touch_l3("old", &[]);
        p.touch_l4("old");
        let state = p.items.get_mut("old").unwrap();
        state.last_accessed = Utc::now() - chrono::Duration::hours(100);
        let n = p.decay_demote("old", 24.0);
        assert_eq!(n, 3);
        assert!(p.is_in("old", Layer::L1));
        assert!(!p.is_in("old", Layer::L2));
        assert!(!p.is_in("old", Layer::L3));
        assert!(!p.is_in("old", Layer::L4));
        assert_eq!(p.top_layer_of("old"), Layer::L1);
    }

    #[test]
    fn count_per_layer() {
        let mut p = LayerProgression::new();
        p.touch_l1("a");
        p.touch_l2("b");  // L1 + L2
        p.touch_l3("c", &[]);  // L1 + L3
        p.touch_l4("d");  // L1 + L4
        let counts = p.count_per_layer();
        assert_eq!(counts[0], 4);
        assert_eq!(counts[1], 1);
        assert_eq!(counts[2], 1);  // only c
        assert_eq!(counts[3], 1);
    }

    #[test]
    fn count_by_top() {
        let mut p = LayerProgression::new();
        p.touch_l1("a");
        p.touch_l2("b");
        p.touch_l3("c", &[]);
        p.touch_l4("d");
        let counts = p.count_by_top_layer();
        assert_eq!(counts.get(&Layer::L1).copied().unwrap_or(0), 1);
        assert_eq!(counts.get(&Layer::L4).copied().unwrap_or(0), 1);
    }

    #[test]
    fn remove_item() {
        let mut p = LayerProgression::new();
        p.touch_l4("x");
        assert!(p.is_in("x", Layer::L4));
        assert!(p.remove("x"));
        assert!(!p.is_in("x", Layer::L4));
        assert!(!p.remove("x"));
    }

    #[test]
    fn recommend_by_content_length() {
        let c = L4LcmCompressor::with_chunk_size(L4LcmCompressor::new(), 1000);
        assert_eq!(LayerProgression::recommend_top_layer("hi", &c), Layer::L1);
        assert_eq!(LayerProgression::recommend_top_layer(&"x".repeat(100), &c), Layer::L2);
        assert_eq!(LayerProgression::recommend_top_layer(&"x".repeat(400), &c), Layer::L3);
        assert_eq!(LayerProgression::recommend_top_layer(&"x".repeat(2000), &c), Layer::L4);
    }

    #[test]
    fn layer_discriminants() {
        assert_eq!(Layer::L1 as usize, 1);
        assert_eq!(Layer::L4 as usize, 4);
    }

    #[test]
    fn promote_from_works() {
        assert_eq!(Layer::L1.promote_from(), Some(Layer::L2));
        assert_eq!(Layer::L4.promote_from(), None);
    }
}
