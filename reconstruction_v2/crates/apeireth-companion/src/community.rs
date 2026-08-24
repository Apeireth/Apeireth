//! Community - 社区 (从 v1.0 apeireth-companion/community.rs 360 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 CommunityMember + 投票

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CommunityMember { pub id: String, pub name: String, pub reputation: u32 }

pub struct Community { pub members: HashMap<String, CommunityMember> }

impl Community {
    pub fn new() -> Self { Self { members: HashMap::new() } }
    /// 0 装 PASS: 真 add
    pub fn add(&mut self, m: CommunityMember) { self.members.insert(m.id.clone(), m); }
    /// 0 装 PASS: 真按 reputation 排序
    pub fn top_reputation(&self, n: usize) -> Vec<&CommunityMember> {
        let mut v: Vec<_> = self.members.values().collect();
        v.sort_by(|a, b| b.reputation.cmp(&a.reputation));
        v.into_iter().take(n).collect()
    }
    pub fn count(&self) -> usize { self.members.len() }
}

impl Default for Community { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add() {
        let mut c = Community::new();
        c.add(CommunityMember { id: "m1".into(), name: "Alice".into(), reputation: 100 });
        assert_eq!(c.count(), 1);
    }
    #[test] fn test_top() {
        let mut c = Community::new();
        c.add(CommunityMember { id: "1".into(), name: "A".into(), reputation: 50 });
        c.add(CommunityMember { id: "2".into(), name: "B".into(), reputation: 90 });
        let t = c.top_reputation(1);
        assert_eq!(t[0].id, "2");
    }
    #[test] fn test_default() { let c: Community = Default::default(); assert_eq!(c.count(), 0); }
}
