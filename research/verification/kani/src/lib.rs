//! Kani 验证 mirror crate (零复制)。
//!
//! 通过 `#[path]` 直接包含 canonical 源码 —— 与生产同源同文件,
//! 不存在拷贝漂移问题:
//!   - crates/engine/runtime/src/canonical/research_approval_sm.rs
//!   - crates/engine/runtime-assembly/src/canonical/causal_world_model.rs
//!   - crates/foundation/orchestration/src/context_fold/fold_block.rs
//!   - crates/foundation/orchestration/src/async_context.rs
//!   - crates/foundation/orchestration/src/cognitive_quota_scheduler.rs
//!   - crates/engine/memory/src/{residual_pyramid,semantic_axis,river_topology,bitemporal_graph}.rs
//!   - crates/adapters/gateway/src/file_fetcher.rs
//!   - crates/capabilities/tools/src/sensitive_path.rs
//!   - crates/foundation/governance/src/{risk,intent}.rs
//!
//! 为什么存在: workspace 声明 rustc 1.97, Kani 0.67 (crates.io 最新) 自带
//! nightly 1.93, `cargo kani -p apeireth-runtime` 被 cargo rust-version 检查拒绝。
//! 本 crate 独立于 workspace (root Cargo.toml 已 exclude research/),
//! 只编译经 #[path] 引入的 canonical 文件 + 少量依赖, 1.93 可编译。
//!
//! 验证目标 = 两处 `#[cfg(kani)]` 门控 harness:
//!   1. research_approval_sm.rs 内 `mod kani_proofs` (3 个 #[kani::proof], 既有);
//!   2. 本 crate `src/harness_*.rs` 六个性质族 harness (assertion 即命题):
//!      - harness_panic_freedom.rs       性质族 1  panic-freedom (零未捕获异常)
//!      - harness_memory_conservation.rs 性质族 2  记忆守恒 (protect/forget 语义纯模型)
//!      - harness_governance.rs          性质族 3  治理单调性 (默认拒绝 / fail-closed)
//!      - harness_quota_scheduler.rs     性质族 4  配额非负与调度安全 (含 PIP)
//!      - harness_path_sandbox.rs        性质族 5  路径沙箱不逃逸
//!      - harness_saga_rollback.rs       性质族 6  SAGA/CoW 回滚精确性
//!   另: file_fetcher.rs 内 `mod kani_base64_proofs` 覆盖模块私有 base64_decode。
//!
//! harness 输入全部由 `kani::any()` 生成并显式有界; 前置形状约束写作早退守卫
//! (与 `kani::assume` 语义等价: 命题为 cond ⇒ P), 每个 harness 的注释写明
//! "证明什么、边界是什么"。生产构建 cfg(kani) 关闭, 验证代码零参与。
//!
//! 运行 (CI: .github/workflows/kani.yml):
//!   cargo kani --manifest-path research/verification/kani/Cargo.toml --harness <name>
//! 本地 Windows 无 Kani; 本地可跑的类型检查/冒烟桩在 ../kani-typecheck/。

// 非 kani 构建下 harness 被 cfg 掉, canonical 条目呈 dead_code —— 静音;
// unused_imports 同 workspace lints 口径 (canonical 文件按整 crate 编译时
// 部分导入仅测试路径使用)。
#![allow(dead_code, unused_imports)]

#[path = "../../../../crates/engine/runtime/src/canonical/research_approval_sm.rs"]
pub mod research_approval_sm;

#[path = "../../../../crates/engine/runtime-assembly/src/canonical/causal_world_model.rs"]
pub mod causal_world_model;

#[path = "../../../../crates/foundation/orchestration/src/context_fold/fold_block.rs"]
pub mod fold_block;

#[path = "../../../../crates/foundation/orchestration/src/async_context.rs"]
pub mod async_context;

#[path = "../../../../crates/foundation/orchestration/src/cognitive_quota_scheduler.rs"]
pub mod cognitive_quota_scheduler;

#[path = "../../../../crates/engine/memory/src/residual_pyramid.rs"]
pub mod residual_pyramid;

#[path = "../../../../crates/engine/memory/src/semantic_axis.rs"]
pub mod semantic_axis;

#[path = "../../../../crates/engine/memory/src/river_topology.rs"]
pub mod river_topology;

#[path = "../../../../crates/engine/memory/src/bitemporal_graph.rs"]
pub mod bitemporal_graph;

#[path = "../../../../crates/adapters/gateway/src/file_fetcher.rs"]
pub mod file_fetcher;

// ===== 编译面 shim (证明面外, 2026-10-03) =====
// sensitive_path.rs 的两个 crate 内外围引用面:
//   - crate::mcp_bridge::config::is_secret_key
//   - crate::exec_pipeline::PipelineFailure::PreDenied (+ code()/message())
// 真模块链 (mcp_bridge / exec_pipeline) 依赖 apeireth-{core,plugin,protocol,
// governance} 整链 —— #[path] 整模块纳入会把 CBMC 翻译面扩大到与命题无关的
// 大片代码 (与 sha2 force-soft 同一理由)。harness 只引用 is_sensitive_path,
// 下列 shim 只为 canonical 文件编译通过, 不在任何 harness 的证明面上。
// 漂移防线: 生产侧测试锁死真值 (sensitive_path.rs::
// credential_surface_refusal_is_a_pre_deny_frame / mcp_bridge::config tests);
// 改真值时同步此处。若 sensitive_path.rs 未来扩大 crate 内依赖面,
// 优先零复制纳入真模块, 不扩 shim。
pub mod mcp_bridge {
    pub mod config {
        /// 与 crates/capabilities/tools/src/mcp_bridge/config.rs::is_secret_key 同义。
        pub fn is_secret_key(key: &str) -> bool {
            let lowered = key.trim_start_matches('-').to_ascii_lowercase();
            const SECRET_FRAGMENTS: &[&str] = &[
                "token",
                "secret",
                "key",
                "password",
                "passwd",
                "pwd",
                "auth",
                "credential",
                "signature",
                "bearer",
            ];
            SECRET_FRAGMENTS
                .iter()
                .any(|fragment| lowered.contains(fragment))
        }
    }
}

pub mod exec_pipeline {
    /// 与 crates/capabilities/tools/src/exec_pipeline/mod.rs::PipelineFailure 同义;
    /// 只声明 sensitive_path.rs 实际构造的 PreDenied 变体。
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum PipelineFailure {
        PreDenied { source: String, reason: String },
    }

    impl PipelineFailure {
        pub const fn code(&self) -> &'static str {
            match self {
                Self::PreDenied { .. } => "pipeline.pre_deny",
            }
        }

        pub fn message(&self) -> String {
            match self {
                Self::PreDenied { source, reason } => {
                    format!("pre-execute deny from {source}: {reason}")
                }
            }
        }
    }
}

// ===== kani_collections: cfg(kani) 证明面 drop-in 线性 HashMap =====
//
// 病灶 (2026-10-07 日志考古): hashbrown RawTable 桶探测循环对新鲜分配内存
// 展开到 unwind 上界 (raw.rs max-iter=31), SipHash write 循环符号展开 ——
// HashMap 对 CBMC 就是路径爆炸源 (research_approval_sm 三 proof 25m/25m/45m
// 跑不完的根因; 换线性表后 4.6s/7.3s/5.6s 证完)。本模块给其余被镜像
// canonical 文件同一药方: 各文件以 `#[cfg(kani)] use super::kani_collections::
// HashMap` 换掉 std HashMap —— 真 crate 里该 use 行编译期剔除, 生产零参与。
// 语义口径: 与 std HashMap 等价 (insert 返回旧值 / remove 返回被删值 /
// retain/entry 同语义 / PartialEq 为 set 语义与插入序无关)。有界性:
// 线性扫描 = 条目数上界, 无哈希/无桶探测/无符号内存读。
#[cfg(kani)]
pub mod kani_collections {
    use std::borrow::Borrow;

    #[derive(Debug, Clone)]
    pub struct HashMap<K, V> {
        entries: Vec<(K, V)>,
    }

    impl<K: PartialEq, V: PartialEq> PartialEq for HashMap<K, V> {
        fn eq(&self, other: &Self) -> bool {
            self.entries.len() == other.entries.len()
                && self
                    .entries
                    .iter()
                    .all(|(k, v)| other.entries.iter().any(|(k2, v2)| k == k2 && v == v2))
        }
    }

    impl<K: PartialEq + Eq, V: Eq> Eq for HashMap<K, V> {}

    impl<K, V> Default for HashMap<K, V> {
        fn default() -> Self {
            Self {
                entries: Vec::new(),
            }
        }
    }

    impl<K, V> HashMap<K, V> {
        pub fn new() -> Self {
            Self::default()
        }
    }

    impl<K: PartialEq, V> HashMap<K, V> {
        pub fn insert(&mut self, key: K, value: V) -> Option<V> {
            for slot in self.entries.iter_mut() {
                if slot.0 == key {
                    return Some(std::mem::replace(&mut slot.1, value));
                }
            }
            self.entries.push((key, value));
            None
        }

        pub fn get<Q: ?Sized + PartialEq>(&self, key: &Q) -> Option<&V>
        where
            K: Borrow<Q>,
        {
            self.entries
                .iter()
                .find(|(k, _)| k.borrow() == key)
                .map(|(_, v)| v)
        }

        pub fn get_mut<Q: ?Sized + PartialEq>(&mut self, key: &Q) -> Option<&mut V>
        where
            K: Borrow<Q>,
        {
            self.entries
                .iter_mut()
                .find(|(k, _)| k.borrow() == key)
                .map(|(_, v)| v)
        }

        pub fn remove<Q: ?Sized + PartialEq>(&mut self, key: &Q) -> Option<V>
        where
            K: Borrow<Q>,
        {
            let idx = self
                .entries
                .iter()
                .position(|(k, _)| k.borrow() == key)?;
            Some(self.entries.swap_remove(idx).1)
        }

        pub fn contains_key<Q: ?Sized + PartialEq>(&self, key: &Q) -> bool
        where
            K: Borrow<Q>,
        {
            self.get(key).is_some()
        }

        pub fn keys(&self) -> impl Iterator<Item = &K> {
            self.entries.iter().map(|(k, _)| k)
        }

        pub fn values(&self) -> impl Iterator<Item = &V> {
            self.entries.iter().map(|(_, v)| v)
        }

        pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
            self.entries.iter_mut().map(|(_, v)| v)
        }

        pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
            self.entries.iter().map(|(k, v)| (k, v))
        }

        pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
            self.entries.iter_mut().map(|(k, v)| (&*k, v))
        }

        pub fn len(&self) -> usize {
            self.entries.len()
        }

        pub fn is_empty(&self) -> bool {
            self.entries.is_empty()
        }

        pub fn clear(&mut self) {
            self.entries.clear();
        }

        pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, mut f: F) {
            self.entries.retain_mut(|(k, v)| f(&*k, v));
        }

        pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
            match self.entries.iter().position(|(k, _)| *k == key) {
                Some(idx) => Entry::Occupied(OccupiedEntry { map: self, idx }),
                None => Entry::Vacant(VacantEntry { map: self, key }),
            }
        }
    }

    pub enum Entry<'a, K, V> {
        Occupied(OccupiedEntry<'a, K, V>),
        Vacant(VacantEntry<'a, K, V>),
    }

    pub struct OccupiedEntry<'a, K, V> {
        map: &'a mut HashMap<K, V>,
        idx: usize,
    }

    pub struct VacantEntry<'a, K, V> {
        map: &'a mut HashMap<K, V>,
        key: K,
    }

    impl<'a, K: PartialEq, V> Entry<'a, K, V> {
        pub fn or_insert(self, default: V) -> &'a mut V {
            match self {
                Entry::Occupied(entry) => &mut entry.map.entries[entry.idx].1,
                Entry::Vacant(entry) => {
                    entry.map.entries.push((entry.key, default));
                    &mut entry.map.entries.last_mut().unwrap().1
                }
            }
        }

        pub fn or_insert_with<F: FnOnce() -> V>(self, f: F) -> &'a mut V {
            match self {
                Entry::Occupied(entry) => &mut entry.map.entries[entry.idx].1,
                Entry::Vacant(entry) => {
                    entry.map.entries.push((entry.key, f()));
                    &mut entry.map.entries.last_mut().unwrap().1
                }
            }
        }

        pub fn or_default(self) -> &'a mut V
        where
            V: Default,
        {
            self.or_insert_with(V::default)
        }

        pub fn and_modify<F: FnOnce(&mut V)>(self, f: F) -> Self {
            match self {
                Entry::Occupied(mut entry) => {
                    f(&mut entry.map.entries[entry.idx].1);
                    Entry::Occupied(entry)
                }
                vacant => vacant,
            }
        }
    }

    impl<K, V> IntoIterator for HashMap<K, V> {
        type Item = (K, V);
        type IntoIter = std::vec::IntoIter<(K, V)>;
        fn into_iter(self) -> Self::IntoIter {
            self.entries.into_iter()
        }
    }

    impl<'a, K, V> IntoIterator for &'a HashMap<K, V> {
        type Item = (&'a K, &'a V);
        type IntoIter = std::iter::Map<
            std::slice::Iter<'a, (K, V)>,
            fn(&(K, V)) -> (&K, &V),
        >;
        fn into_iter(self) -> Self::IntoIter {
            fn pair<K, V>(slot: &(K, V)) -> (&K, &V) {
                (&slot.0, &slot.1)
            }
            self.entries.iter().map(pair as fn(&(K, V)) -> (&K, &V))
        }
    }

    impl<'a, K, V> IntoIterator for &'a mut HashMap<K, V> {
        type Item = (&'a K, &'a mut V);
        type IntoIter = std::iter::Map<
            std::slice::IterMut<'a, (K, V)>,
            fn(&mut (K, V)) -> (&K, &mut V),
        >;
        fn into_iter(self) -> Self::IntoIter {
            fn pair<K, V>(slot: &mut (K, V)) -> (&K, &mut V) {
                (&slot.0, &mut slot.1)
            }
            self.entries
                .iter_mut()
                .map(pair as fn(&mut (K, V)) -> (&K, &mut V))
        }
    }

    impl<K: PartialEq, V, Q: ?Sized + PartialEq> std::ops::Index<&Q> for HashMap<K, V>
    where
        K: Borrow<Q>,
    {
        type Output = V;
        fn index(&self, key: &Q) -> &V {
            self.get(key).expect("map index on missing key")
        }
    }

    impl<K: PartialEq, V, Q: ?Sized + PartialEq> std::ops::IndexMut<&Q> for HashMap<K, V>
    where
        K: Borrow<Q>,
    {
        fn index_mut(&mut self, key: &Q) -> &mut V {
            self.get_mut(key).expect("map index on missing key")
        }
    }

    impl<K: serde::Serialize, V: serde::Serialize> serde::Serialize for HashMap<K, V> {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            use serde::ser::SerializeMap;
            let mut map = serializer.serialize_map(Some(self.entries.len()))?;
            for (key, value) in &self.entries {
                map.serialize_entry(key, value)?;
            }
            map.end()
        }
    }

    impl<'de, K, V> serde::Deserialize<'de> for HashMap<K, V>
    where
        K: serde::Deserialize<'de> + PartialEq,
        V: serde::Deserialize<'de>,
    {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct MapVisitor<K, V>(std::marker::PhantomData<(K, V)>);
            impl<'de, K, V> serde::de::Visitor<'de> for MapVisitor<K, V>
            where
                K: serde::Deserialize<'de> + PartialEq,
                V: serde::Deserialize<'de>,
            {
                type Value = HashMap<K, V>;
                fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "a map")
                }
                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    mut access: A,
                ) -> Result<Self::Value, A::Error> {
                    let mut map = HashMap::new();
                    while let Some((key, value)) = access.next_entry()? {
                        map.insert(key, value);
                    }
                    Ok(map)
                }
            }
            deserializer.deserialize_map(MapVisitor(std::marker::PhantomData))
        }
    }
}

#[path = "../../../../crates/capabilities/tools/src/sensitive_path.rs"]
pub mod sensitive_path;

#[path = "../../../../crates/foundation/governance/src/risk.rs"]
pub mod risk;

#[path = "../../../../crates/foundation/governance/src/intent.rs"]
pub mod intent;

// ===== 性质族 harness (cfg(kani) 门控, 生产零参与) =====

#[cfg(kani)]
#[path = "harness_panic_freedom.rs"]
mod harness_panic_freedom;

#[cfg(kani)]
#[path = "harness_memory_conservation.rs"]
mod harness_memory_conservation;

#[cfg(kani)]
#[path = "harness_governance.rs"]
mod harness_governance;

#[cfg(kani)]
#[path = "harness_quota_scheduler.rs"]
mod harness_quota_scheduler;

#[cfg(kani)]
#[path = "harness_path_sandbox.rs"]
mod harness_path_sandbox;

#[cfg(kani)]
#[path = "harness_saga_rollback.rs"]
mod harness_saga_rollback;
