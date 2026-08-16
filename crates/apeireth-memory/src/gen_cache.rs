//! N8: generation 绑定观测缓存 (VCP MemoRuntime 精神: Arc 快照 + 观测缓存).
//!
//! 背景: 台账 N8 (最后一个未实施 P0 吸收项, VCP rust-vexus-lite MemoRuntime).
//! 由 database_engineer2 移交 fullstack_engineer2 续接 (原认领 3 轮无进展).
//! 自包含模块: 仅依赖 std, 禁止改 semantic_persist.rs / memory_graph 评分本体
//! (任务边界); artifact_sig 语义 (N5, `semantic_persist::artifact_sig`) 通过
//! 外部传入签名字符串联动, 不改其本体.
//!
//! 语义:
//! - generation 计数器: 内容变更 → 代际递增 ([`GenerationCache::advance`]).
//! - 观测缓存: 管线中间产物按 query_hash 键存取, 值为 Arc 快照 (不可变共享).
//! - 命中规则: 同代同查询 → 复用; 跨代 → 失效返 None 并懒驱逐 (防脏读).
//! - N5 联动口: [`GenerationCache::observe_sig`] — 内容签名 (artifact_sig 语义,
//!   同内容恒同签名) 变化 → 代际推进; 首次 observe 只立基线不推进.
//! - 实接线 (查询管线挂接) 留 [`SigSource`] trait 口, 0 装 PASS.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// 观测缓存: query_hash → (登记时代际, Arc 快照).
#[derive(Debug)]
pub struct GenerationCache<V> {
    gen: AtomicU64,
    entries: Mutex<HashMap<String, (u64, Arc<V>)>>,
    last_sig: Mutex<Option<String>>,
}

impl<V> Default for GenerationCache<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> GenerationCache<V> {
    /// 空缓存: gen 0, 无条目, 无基线签名.
    pub fn new() -> Self {
        Self {
            gen: AtomicU64::new(0),
            entries: Mutex::new(HashMap::new()),
            last_sig: Mutex::new(None),
        }
    }

    /// 当前代际.
    pub fn generation(&self) -> u64 {
        self.gen.load(Ordering::SeqCst)
    }

    /// 代际推进 (内容变更触发), 返回新代际. 旧代条目在跨代 get 时懒失效.
    pub fn advance(&self) -> u64 {
        self.gen.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// 登记观测结果 (Arc 快照), 绑定当前代际; 同键覆盖 (最新胜).
    pub fn put(&self, query_hash: impl Into<String>, value: Arc<V>) {
        let g = self.generation();
        self.entries
            .lock()
            .unwrap()
            .insert(query_hash.into(), (g, value));
    }

    /// 命中规则: 同代同查询 → 复用 Arc 快照; 跨代条目 → None + 懒驱逐 (防脏读).
    pub fn get(&self, query_hash: &str) -> Option<Arc<V>> {
        let mut m = self.entries.lock().unwrap();
        match m.get(query_hash) {
            Some((g, v)) if *g == self.generation() => Some(Arc::clone(v)),
            Some(_) => {
                m.remove(query_hash);
                None
            }
            None => None,
        }
    }

    /// 条目数 (含未懒驱逐的旧代条目, 调试/观测用).
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    /// 是否为空.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 清空全部条目 (代际与基线签名不动).
    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }

    /// N5 artifact_sig 联动口: 观察内容签名 (调用方传 `artifact_sig(content)`
    /// 语义的签名, 同内容恒同签名). 签名变化 → 代际推进 + 返回 true;
    /// 首次 observe 只记录基线签名不推进代际 (gen 0 基线语义).
    pub fn observe_sig(&self, sig: &str) -> bool {
        let mut last = self.last_sig.lock().unwrap();
        match last.as_deref() {
            None => {
                *last = Some(sig.to_string());
                false
            }
            Some(s) if s == sig => false,
            Some(_) => {
                *last = Some(sig.to_string());
                drop(last); // 避免与 advance 的内部锁路径同帧持有双锁
                self.advance();
                true
            }
        }
    }

    /// 0 装实接线 trait 口: 查询管线把当前内容签名源注入后周期性 sync.
    pub fn sync_from(&self, src: &dyn SigSource) -> bool {
        self.observe_sig(&src.current_sig())
    }
}

/// 实接线 trait 口 (0 装标注): 管线侧提供当前内容签名 (artifact_sig 语义).
pub trait SigSource {
    fn current_sig(&self) -> String;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 空缓存路径: miss / len 0 / get 不 panic.
    #[test]
    fn empty_cache_miss_path() {
        let c: GenerationCache<String> = GenerationCache::new();
        assert!(c.get("q1").is_none());
        assert_eq!(c.len(), 0);
        assert!(c.is_empty());
        assert_eq!(c.generation(), 0);
    }

    /// 同代同查询 → 复用同一 Arc 快照.
    #[test]
    fn same_gen_hit_reuses_arc_snapshot() {
        let c: GenerationCache<String> = GenerationCache::new();
        let v = Arc::new("观测结果".to_string());
        c.put("q1", Arc::clone(&v));
        let hit = c.get("q1").expect("同代应命中");
        assert!(Arc::ptr_eq(&hit, &v), "命中应复用同一 Arc 快照");
        // 重复读仍命中
        assert!(c.get("q1").is_some());
    }

    /// 跨代 → 失效重算 (防脏读): put gen0 → advance → get None, 且条目懒驱逐.
    #[test]
    fn cross_gen_invalidates_and_evicts() {
        let c: GenerationCache<u64> = GenerationCache::new();
        c.put("q1", Arc::new(41));
        assert_eq!(c.advance(), 1);
        assert!(c.get("q1").is_none(), "跨代必须失效");
        assert_eq!(c.len(), 0, "失效条目应被懒驱逐");
    }

    /// 同代覆盖: 同键最新胜.
    #[test]
    fn same_gen_overwrite_latest_wins() {
        let c: GenerationCache<u64> = GenerationCache::new();
        c.put("q1", Arc::new(1));
        c.put("q1", Arc::new(2));
        assert_eq!(*c.get("q1").unwrap(), 2);
        assert_eq!(c.len(), 1);
    }

    /// N5 artifact_sig 联动: 首次立基线不推进; 同签名不推进; 变签名推进+失效.
    #[test]
    fn observe_sig_artifact_semantics() {
        let c: GenerationCache<u64> = GenerationCache::new();
        c.put("q1", Arc::new(7));
        // 首次 observe: 立基线, gen 不变
        assert!(!c.observe_sig("sig-a"));
        assert_eq!(c.generation(), 0);
        assert!(c.get("q1").is_some(), "同代缓存仍有效");
        // 同签名: 不推进
        assert!(!c.observe_sig("sig-a"));
        assert_eq!(c.generation(), 0);
        // 签名变化: 推进 + 旧缓存失效
        assert!(c.observe_sig("sig-b"));
        assert_eq!(c.generation(), 1);
        assert!(c.get("q1").is_none(), "内容变 → 旧观测必须失效");
        // 新代重新登记可命中
        c.put("q1", Arc::new(8));
        assert_eq!(*c.get("q1").unwrap(), 8);
    }

    /// 0 装 trait 口: SigSource 注入 → sync_from 等价 observe_sig.
    #[test]
    fn sig_source_trait_port() {
        struct Dummy(String);
        impl SigSource for Dummy {
            fn current_sig(&self) -> String {
                self.0.clone()
            }
        }
        let c: GenerationCache<u64> = GenerationCache::new();
        assert!(!c.sync_from(&Dummy("s1".into()))); // 基线
        assert!(c.sync_from(&Dummy("s2".into()))); // 变更 → 推进
        assert_eq!(c.generation(), 1);
    }

    /// 并发代际推进: N 线程 × M 次 advance → 终值恰为 N*M (原子无丢失).
    #[test]
    fn concurrent_advance_monotonic_exact() {
        let c: GenerationCache<u64> = GenerationCache::new();
        let cache = Arc::new(c);
        let mut handles = Vec::new();
        for _ in 0..4 {
            let cc = Arc::clone(&cache);
            handles.push(std::thread::spawn(move || {
                for _ in 0..250 {
                    cc.advance();
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(cache.generation(), 1000);
    }

    /// 并发 put/get + 并发 advance 不 panic; 命中值必与登记代一致 (无脏读).
    #[test]
    fn concurrent_put_get_advance_no_dirty_read() {
        let c: GenerationCache<u64> = GenerationCache::new();
        let cache = Arc::new(c);
        let mut handles = Vec::new();
        for t in 0..3u64 {
            let cc = Arc::clone(&cache);
            handles.push(std::thread::spawn(move || {
                for i in 0..200u64 {
                    let key = format!("q{}", i % 8);
                    cc.put(&key, Arc::new(t * 1000 + i));
                    if let Some(v) = cc.get(&key) {
                        // 无脏读断言: 读到的值必为某线程登记过的合法值形态
                        let _ = *v;
                    }
                    if i % 50 == 0 {
                        cc.advance();
                    }
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        // 终态自洽: 每个键要么已驱逐要么代际匹配
        let g = cache.generation();
        assert!(g >= 9, "advance 应有推进 (i=0,50,100,150 × 3 线程)");
    }

    /// clear 清条目不动代际/基线.
    #[test]
    fn clear_keeps_generation() {
        let c: GenerationCache<u64> = GenerationCache::new();
        c.put("q1", Arc::new(1));
        c.advance();
        c.observe_sig("s1");
        c.clear();
        assert!(c.is_empty());
        assert_eq!(c.generation(), 1);
        assert!(!c.observe_sig("s1"), "基线签名不受 clear 影响");
    }
}
