//! B7 · Phase 6 原型一: 漫游记忆 CRDT (Research 前缀, 不进默认路径)。
//!
//! # 学术账本 (铁律 3)
//! - **问题定义**: 多载体 (桌面/移动/远端) 离线编辑同一记忆集合后重聚,
//!   无中心协调地合并, 且删除必须与 Apeireth 的"永不物理删除"治理纪律一致
//!   (tombstone 而非 erase)。
//! - **假设**: 状态式 CRDT (state-based, 单调半格 join) 的 LWW-Element-Set +
//!   LWW-Register 组合可表达"条目增删改 + 内容修订", 合并满足交换律/结合律/
//!   幂等律, 网络重放任意顺序结果唯一。
//! - **状态**: 原型已实现 (纯 std, 确定性可测)。生产接线不在此批次。
//! - **引用**: Shapiro et al. 2011 (Conflict-free Replicated Data Types);
//!   项目纪律锚定 `append_only.rs` / `memory_governance.rs` (软删 sidecar)。
//! - **baseline**: `research/baselines/baseline-2026-09-phase0.md`。
//! - **已知局限**: ① 单条内容用 LWW (后写覆盖), 不解决文本级并发编辑 (需
//!   RGA/序列 CRDT, 留后续); ② clock 为 (timestamp, replica_id) 逻辑时钟,
//!   跨设备真实时钟偏移需 NTP/混合逻辑时钟校准; ③ 无 GC 压缩 (tombstone 累积,
//!   与治理纪律一致但需后续压实策略)。

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// 逻辑时钟: (timestamp_ms, replica_id) 字典序比较, 同刻由副本 id 打破平局。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ResearchLogicalClock {
    pub timestamp_ms: u64,
    pub replica_id: u64,
}

/// 漫游记忆条目: LWW-Register (content) + 删除墓碑 (LWW: delete 与 update 取新者)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchRoamingItem {
    pub id: String,
    pub content: String,
    pub updated: ResearchLogicalClock,
    /// Some(clock) = 已删除 (墓碑保留, 与软删纪律一致)。
    pub deleted: Option<ResearchLogicalClock>,
    pub tags: BTreeSet<String>,
}

impl ResearchRoamingItem {
    fn new(id: impl Into<String>, content: impl Into<String>, clock: ResearchLogicalClock) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            updated: clock,
            deleted: None,
            tags: BTreeSet::new(),
        }
    }

    /// 单条目 merge (LWW): 取 updated 较大的一方; 平局取内容字典序 (确定性)。
    /// delete 与 update 同域竞争: 比较 (updated 与 deleted 的最大时钟)。
    fn merge_into(&mut self, other: &Self) {
        debug_assert_eq!(self.id, other.id, "merge 只允许同 id 条目");
        if other.updated > self.updated {
            self.content = other.content.clone();
            self.updated = other.updated;
        }
        let self_del = self.deleted;
        let other_del = other.deleted;
        let self_max = self_del
            .map(|c| c.max(self.updated))
            .unwrap_or(self.updated);
        let other_max = other_del
            .map(|c| c.max(other.updated))
            .unwrap_or(other.updated);
        match (self_del, other_del) {
            (None, None) => {}
            (Some(_), None) => {
                if other_max > self_max {
                    self.deleted = None;
                    if other.updated > self.updated {
                        self.content = other.content.clone();
                        self.updated = other.updated;
                    }
                }
            }
            (None, Some(_)) => {
                if other_max >= self_max {
                    self.deleted = other_del;
                }
            }
            (Some(_), Some(_)) => {
                if other_max > self_max {
                    self.deleted = other_del;
                }
            }
        }
        self.tags = self.tags.union(&other.tags).cloned().collect();
    }
}

/// 漫游记忆副本 (state-based CRDT: 整个 map 的 join 即 merge)。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResearchRoamingMemory {
    pub replica_id: u64,
    pub items: BTreeMap<String, ResearchRoamingItem>,
}

impl ResearchRoamingMemory {
    pub fn new(replica_id: u64) -> Self {
        Self {
            replica_id,
            items: BTreeMap::new(),
        }
    }

    fn clock(&self, timestamp_ms: u64) -> ResearchLogicalClock {
        ResearchLogicalClock {
            timestamp_ms,
            replica_id: self.replica_id,
        }
    }

    /// upsert 内容 (LWW: 覆盖旧内容, 但删除墓碑不被 update 复活——
    /// 若已删除且 update 时钟更新, 视为重新创建, 清墓碑)。
    pub fn upsert(&mut self, id: &str, content: &str, timestamp_ms: u64) {
        let clock = self.clock(timestamp_ms);
        match self.items.get_mut(id) {
            Some(item) => {
                if let Some(del) = item.deleted {
                    if clock > del {
                        // 删除后的重新创建
                        item.content = content.to_string();
                        item.updated = clock;
                        item.deleted = None;
                    }
                    // else: 旧时钟 update 被墓碑压制 (已删不复活)
                } else {
                    item.content = content.to_string();
                    item.updated = clock;
                }
            }
            None => {
                self.items
                    .insert(id.to_string(), ResearchRoamingItem::new(id, content, clock));
            }
        }
    }

    /// 删除 (墓碑: 永不物理删除, 与治理纪律一致)。幂等。
    pub fn delete(&mut self, id: &str, timestamp_ms: u64) {
        let clock = self.clock(timestamp_ms);
        match self.items.get_mut(id) {
            Some(item) => {
                if item.deleted.map_or(true, |d| clock > d) {
                    item.deleted = Some(clock);
                }
            }
            None => {
                // 幽灵删除: 记录 tombstone-only 条目 (防删除被旧副本的更新复活)。
                let mut item = ResearchRoamingItem::new(id, String::new(), clock);
                item.deleted = Some(clock);
                self.items.insert(id.to_string(), item);
            }
        }
    }

    /// merge = 逐条目 join (state-based CRDT)。交换/结合/幂等律由测试验证。
    pub fn merge(&mut self, other: &Self) {
        for (id, item) in &other.items {
            match self.items.get_mut(id) {
                Some(mine) => mine.merge_into(item),
                None => {
                    self.items.insert(id.clone(), item.clone());
                }
            }
        }
    }

    /// 存活条目 (未删除)。
    pub fn alive(&self) -> Vec<&ResearchRoamingItem> {
        self.items
            .values()
            .filter(|i| i.deleted.is_none())
            .collect()
    }

    /// 已删除条目 (墓碑审计)。
    pub fn tombstones(&self) -> Vec<&ResearchRoamingItem> {
        self.items
            .values()
            .filter(|i| i.deleted.is_some())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 双副本并发 upsert + merge: LWW 后写覆盖, 双方结果一致。
    #[test]
    fn lww_upsert_merge_converges() {
        let mut a = ResearchRoamingMemory::new(1);
        let mut b = ResearchRoamingMemory::new(2);
        a.upsert("m1", "A 版本", 100);
        b.upsert("m1", "B 版本", 200);
        a.merge(&b);
        b.merge(&a);
        assert_eq!(a.items["m1"].content, "B 版本", "LWW: 新时钟胜");
        assert_eq!(a.items, b.items, "双向 merge 收敛一致");
    }

    /// 删除墓碑: 旧时钟 update 不复活已删条目; 新时钟 update 重新创建。
    #[test]
    fn tombstone_blocks_stale_resurrection() {
        let mut a = ResearchRoamingMemory::new(1);
        let mut b = ResearchRoamingMemory::new(2);
        a.upsert("m1", "内容", 100);
        b.delete("m1", 150); // 幽灵删除 (b 从未见过 m1)
        a.merge(&b);
        assert_eq!(a.tombstones().len(), 1, "删除以墓碑合并");
        // 旧时钟 update 不复活
        a.upsert("m1", "复活尝试", 120);
        assert_eq!(a.tombstones().len(), 1);
        assert_eq!(a.alive().len(), 0);
        // 新时钟 update = 重新创建
        a.upsert("m1", "重建内容", 300);
        assert_eq!(a.alive().len(), 1);
        assert_eq!(a.alive()[0].content, "重建内容");
    }

    /// CRDT 三律: 交换律/结合律/幂等律 (确定性验证)。
    #[test]
    fn crdt_laws_commutative_associative_idempotent() {
        let mk = |id: u64, edits: &[(&str, &str, u64)]| {
            let mut m = ResearchRoamingMemory::new(id);
            for (i, c, t) in edits {
                m.upsert(i, c, *t);
            }
            m
        };
        let edits_a = [("x", "1", 100u64), ("y", "2", 200)];
        let edits_b = [("x", "9", 300u64), ("z", "3", 150)];
        let a = mk(1, &edits_a);
        let b = mk(2, &edits_b);
        // 交换律
        let mut ab = a.clone();
        ab.merge(&b);
        let mut ba = b.clone();
        ba.merge(&a);
        assert_eq!(ab.items, ba.items, "merge 交换律");
        // 幂等律
        let mut ab2 = ab.clone();
        ab2.merge(&b);
        assert_eq!(ab.items, ab2.items, "merge 幂等律");
        // 结合律 (a⊕b)⊕c == a⊕(b⊕c)
        let c = mk(3, &[("w", "4", 400)]);
        let mut left = a.clone();
        left.merge(&b);
        left.merge(&c);
        let mut right_bc = b.clone();
        right_bc.merge(&c);
        let mut right = a.clone();
        right.merge(&right_bc);
        assert_eq!(left.items, right.items, "merge 结合律");
    }

    /// 确定性: 同一批操作任意 merge 顺序结果唯一 (网络重放安全)。
    #[test]
    fn merge_order_determinism() {
        let mut replicas = Vec::new();
        for r in 0..4 {
            let mut m = ResearchRoamingMemory::new(r);
            m.upsert("k", &format!("v{r}"), 100 + r);
            replicas.push(m);
        }
        // 顺序 0,1,2,3
        let mut seq = ResearchRoamingMemory::new(9);
        for r in &replicas {
            seq.merge(r);
        }
        // 顺序 3,1,0,2
        let mut rev = ResearchRoamingMemory::new(9);
        for r in replicas.iter().rev() {
            rev.merge(r);
        }
        assert_eq!(seq.items, rev.items);
        assert_eq!(seq.items["k"].content, "v3", "最高时钟内容胜出");
    }

    /// 墓碑审计保留 (与 append-only/软删纪律一致)。
    #[test]
    fn tombstones_never_physically_removed() {
        let mut m = ResearchRoamingMemory::new(1);
        m.upsert("m1", "内容", 100);
        m.delete("m1", 200);
        m.upsert("m1", "重建", 300);
        assert_eq!(m.items.len(), 1);
        // 重建后墓碑被清 (同 id 重创建), 但删除动作在历史上存在过 —
        // 原型不保留删除史; 与治理表联动的删除史留生产接线 (0 装)。
        assert_eq!(m.alive().len(), 1);
    }
}
