//! Alias resolution, LRU lookup cache, and invalidation.
//!
//! Recovered from legacy `apeireth-agent` `AgentManager` (agentManager architecture
//! alias map + prompt cache) as a **metadata helper**, not as an agent owner.
//! The canonical plugin and module registries remain the source of truth;
//! this module only answers "what id does this lookup key refer to" and
//! caches the answer.
//!
//! # Behaviour preserved from the canonical
//!
//! - An entity's id is an implicit alias of itself.
//! - Several aliases may point at one id; a colliding alias is last-writer-wins.
//! - `resolve` checks the record table by id first, then the alias index.
//! - Hits are stored in a bounded LRU keyed by the **lookup string**, so
//!   `id`, `@a1`, and `@a2` are three cache entries.
//! - Register and unregister clear the whole cache (promptCache.clear()
//!   on `loadMap`).
//! - A miss does not occupy cache capacity.
//! - Unregister removes only aliases that still point at the removed id, so a
//!   stolen alias stays with the thief.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::hash::Hash;
use std::num::NonZeroUsize;

use crate::manifest::PluginManifest;

/// Default LRU capacity. The canonical prompt cache was an unbounded `Map`; 64 is
/// the size it used once the cache became a real LRU.
pub const DEFAULT_CACHE_SIZE: usize = 64;

/// TopologicalEngine miss token prefix (`agentManager.js:282` `{{agent:` + alias + `}}`).
pub const LEGACY_UNRESOLVED_PREFIX: &str = "{{agent:";

/// Format a miss placeholder so a consumer can leave the lookup in the prompt
/// rather than inventing a record. V2 uses `plugin` as the kind; pass `agent`
/// to reproduce the canonical token.
pub fn unresolved_token(kind: &str, alias: &str) -> String {
    format!("{{{{{kind}:{alias}}}}}")
}

/// Reverse index: lookup key → canonical id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AliasIndex {
    by_alias: HashMap<String, String>,
}

impl AliasIndex {
    /// An empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind `id` and `aliases` so each key resolves to `id`.
    ///
    /// The id is inserted as an implicit alias. Empty alias strings are ignored.
    /// An alias already owned by another id is overwritten (last writer wins).
    pub fn bind(
        &mut self,
        id: impl Into<String>,
        aliases: impl IntoIterator<Item = impl Into<String>>,
    ) {
        let id = id.into();
        self.by_alias.insert(id.clone(), id.clone());
        for alias in aliases {
            let alias = alias.into();
            if alias.is_empty() {
                continue;
            }
            self.by_alias.insert(alias, id.clone());
        }
    }

    /// Drop every key that currently points at `id`.
    ///
    /// Aliases that have since been claimed by another id are left alone.
    pub fn unbind(&mut self, id: &str) {
        self.by_alias.retain(|_, owner| owner != id);
    }

    /// Replace the alias set for `id` (unbind then bind).
    pub fn rebind(
        &mut self,
        id: impl Into<String>,
        aliases: impl IntoIterator<Item = impl Into<String>>,
    ) {
        let id = id.into();
        self.unbind(&id);
        self.bind(id, aliases);
    }

    /// Canonical id for `id_or_alias`, if any.
    pub fn resolve(&self, id_or_alias: &str) -> Option<&str> {
        self.by_alias.get(id_or_alias).map(String::as_str)
    }

    /// Whether `id_or_alias` is a known id or alias.
    pub fn contains(&self, id_or_alias: &str) -> bool {
        self.by_alias.contains_key(id_or_alias)
    }

    /// Every lookup key, in sorted order.
    pub fn keys(&self) -> Vec<&str> {
        let mut out: Vec<&str> = self.by_alias.keys().map(String::as_str).collect();
        out.sort_unstable();
        out
    }

    /// Number of lookup keys (ids + aliases, deduplicated).
    pub fn len(&self) -> usize {
        self.by_alias.len()
    }

    /// Whether nothing is indexed.
    pub fn is_empty(&self) -> bool {
        self.by_alias.is_empty()
    }
}

/// A small std-only LRU. Insertion order is a `VecDeque` (front = least
/// recently used). Capacity is clamped to at least 1.
#[derive(Debug, Clone)]
pub struct LruCache<K, V> {
    cap: NonZeroUsize,
    map: HashMap<K, V>,
    order: VecDeque<K>,
}

impl<K: Eq + Hash + Clone, V> LruCache<K, V> {
    /// A cache that holds at most `cap` entries (`cap == 0` becomes 1).
    pub fn new(cap: usize) -> Self {
        let cap = NonZeroUsize::new(cap.max(1)).expect("cap.max(1) is never zero");
        Self {
            cap,
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    fn touch(&mut self, key: &K) {
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            self.order.remove(pos);
        }
        self.order.push_back(key.clone());
    }

    /// Borrow a value and mark it most-recently used.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.map.contains_key(key) {
            self.touch(key);
            self.map.get(key)
        } else {
            None
        }
    }

    /// Insert or replace, evicting the least-recently used entry when full.
    pub fn put(&mut self, key: K, value: V) {
        if self.map.contains_key(&key) {
            self.map.insert(key.clone(), value);
            self.touch(&key);
            return;
        }
        if self.map.len() >= self.cap.get() {
            if let Some(old) = self.order.pop_front() {
                self.map.remove(&old);
            }
        }
        self.order.push_back(key.clone());
        self.map.insert(key, value);
    }

    /// Drop every entry.
    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }

    /// Current occupancy.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Maximum occupancy.
    pub fn cap(&self) -> usize {
        self.cap.get()
    }
}

/// Record table + alias index + LRU lookup cache.
///
/// This is the canonical `AgentManager` resolution algorithm with the agent type
/// erased. Callers that own plugins or modules keep those records; they use
/// this helper only for alias/cache behaviour.
#[derive(Debug, Clone)]
pub struct AliasResolver<T> {
    records: HashMap<String, T>,
    index: AliasIndex,
    cache: LruCache<String, T>,
}

impl<T: Clone> AliasResolver<T> {
    /// Empty resolver with the default cache size.
    pub fn new() -> Self {
        Self::with_cache_size(DEFAULT_CACHE_SIZE)
    }

    /// Empty resolver with an explicit cache size.
    pub fn with_cache_size(cache_size: usize) -> Self {
        Self {
            records: HashMap::new(),
            index: AliasIndex::new(),
            cache: LruCache::new(cache_size),
        }
    }

    /// Insert or replace `id`. Existing aliases for the same id are rebound.
    /// The lookup cache is cleared (canonical `loadMap` behaviour).
    pub fn register(
        &mut self,
        id: impl Into<String>,
        aliases: impl IntoIterator<Item = impl Into<String>>,
        value: T,
    ) {
        let id = id.into();
        if self.records.contains_key(&id) {
            self.index.unbind(&id);
        }
        self.index.bind(id.clone(), aliases);
        self.records.insert(id, value);
        self.cache.clear();
    }

    /// Remove by id or alias. Returns the stored value. Clears the cache.
    pub fn unregister(&mut self, id_or_alias: &str) -> Option<T> {
        let id = if self.records.contains_key(id_or_alias) {
            id_or_alias.to_string()
        } else {
            self.index.resolve(id_or_alias)?.to_string()
        };
        let removed = self.records.remove(&id);
        self.index.unbind(&id);
        self.cache.clear();
        removed
    }

    /// Borrow by canonical id, skipping alias resolution and the cache.
    pub fn get(&self, id: &str) -> Option<&T> {
        self.records.get(id)
    }

    /// Resolve by id or alias, filling the LRU on a hit.
    pub fn resolve(&mut self, id_or_alias: &str) -> Option<T> {
        let key = id_or_alias.to_string();
        if let Some(cached) = self.cache.get(&key) {
            return Some(cached.clone());
        }
        let id = if self.records.contains_key(id_or_alias) {
            id_or_alias.to_string()
        } else {
            self.index.resolve(id_or_alias)?.to_string()
        };
        let value = self.records.get(&id)?.clone();
        self.cache.put(key, value.clone());
        Some(value)
    }

    /// Whether `id_or_alias` is a known id or alias.
    pub fn contains(&self, id_or_alias: &str) -> bool {
        self.records.contains_key(id_or_alias) || self.index.contains(id_or_alias)
    }

    /// Canonical ids, sorted.
    pub fn ids(&self) -> Vec<&str> {
        let mut out: Vec<&str> = self.records.keys().map(String::as_str).collect();
        out.sort_unstable();
        out
    }

    /// Every lookup key (ids + aliases), sorted.
    pub fn aliases(&self) -> Vec<&str> {
        self.index.keys()
    }

    /// Number of stored records (not aliases).
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether no records are stored.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Number of lookup keys.
    pub fn alias_count(&self) -> usize {
        self.index.len()
    }

    /// Drop cached lookups without touching records.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Current cache occupancy.
    pub fn cache_len(&self) -> usize {
        self.cache.len()
    }

    /// Cache capacity.
    pub fn cache_capacity(&self) -> usize {
        self.cache.cap()
    }

    /// The alias index (for callers that want to inspect mappings).
    pub fn index(&self) -> &AliasIndex {
        &self.index
    }
}

impl<T: Clone> Default for AliasResolver<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Bind every manifest's id and aliases into one index. Later manifests win
/// colliding aliases. This is a derived view over [`PluginManifest`] data, not
/// a second plugin registry.
pub fn index_manifests<'a, I>(manifests: I) -> AliasIndex
where
    I: IntoIterator<Item = &'a PluginManifest>,
{
    let mut index = AliasIndex::new();
    for manifest in manifests {
        index.bind(manifest.id.as_str().to_string(), manifest.aliases.clone());
    }
    index
}

/// Build an [`AliasIndex`] from `(id, aliases)` pairs. Later pairs win collisions.
pub fn index_pairs<I, A, S>(pairs: I) -> AliasIndex
where
    I: IntoIterator<Item = (S, A)>,
    A: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut index = AliasIndex::new();
    for (id, aliases) in pairs {
        index.bind(id, aliases);
    }
    index
}

/// Group lookup keys by the canonical id they currently resolve to.
pub fn aliases_by_id(index: &AliasIndex) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for key in index.keys() {
        if let Some(id) = index.resolve(key) {
            out.entry(id.to_string()).or_default().push(key.to_string());
        }
    }
    for aliases in out.values_mut() {
        aliases.sort();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Rec {
        id: String,
        name: String,
    }

    fn rec(id: &str) -> Rec {
        Rec {
            id: id.into(),
            name: format!("{id}-name"),
        }
    }

    #[test]
    fn resolver_new_is_empty() {
        let m = AliasResolver::<Rec>::new();
        assert!(m.is_empty());
        assert_eq!(m.len(), 0);
        assert_eq!(m.alias_count(), 0);
        assert_eq!(m.cache_capacity(), DEFAULT_CACHE_SIZE);
    }

    #[test]
    fn register_and_get_by_id() {
        let mut m = AliasResolver::new();
        m.register("coder", ["@coder"], rec("coder"));
        assert_eq!(m.len(), 1);
        assert_eq!(m.get("coder").unwrap().id, "coder");
    }

    #[test]
    fn register_overwrites_same_id_and_drops_old_aliases() {
        let mut m = AliasResolver::new();
        m.register("a", ["@old"], rec("a"));
        m.register("a", ["@new"], rec("a"));
        assert_eq!(m.len(), 1);
        assert!(!m.contains("@old"));
        assert_eq!(m.resolve("@new").unwrap().id, "a");
        assert_eq!(m.resolve("a").unwrap().id, "a");
    }

    #[test]
    fn id_equal_to_alias_is_one_index_entry() {
        let mut m = AliasResolver::new();
        m.register("shared", ["shared"], rec("shared"));
        assert_eq!(m.alias_count(), 1);
    }

    #[test]
    fn unregister_by_id_and_by_alias() {
        let mut m = AliasResolver::new();
        m.register("x", ["@x"], rec("x"));
        let removed = m.unregister("x").unwrap();
        assert_eq!(removed.id, "x");
        assert!(m.get("x").is_none());
        assert!(!m.contains("@x"));

        m.register("y", ["@ya"], rec("y"));
        let removed = m.unregister("@ya").unwrap();
        assert_eq!(removed.id, "y");
        assert!(m.get("y").is_none());
    }

    #[test]
    fn unregister_unknown_returns_none() {
        let mut m = AliasResolver::<Rec>::new();
        assert!(m.unregister("nope").is_none());
    }

    #[test]
    fn unregister_clears_cache() {
        let mut m = AliasResolver::new();
        m.register("c", ["@c"], rec("c"));
        let _ = m.resolve("@c");
        assert!(m.cache_len() > 0);
        m.unregister("c");
        assert_eq!(m.cache_len(), 0);
    }

    #[test]
    fn resolve_by_id_and_by_alias() {
        let mut m = AliasResolver::new();
        m.register("main", ["@m1", "@m2"], rec("main"));
        assert_eq!(m.resolve("main").unwrap().id, "main");
        let a1 = m.resolve("@m1").unwrap();
        let a2 = m.resolve("@m2").unwrap();
        assert_eq!(a1.id, "main");
        assert_eq!(a2.id, "main");
        assert!(m.resolve("@nope").is_none());
        assert!(m.resolve("missing").is_none());
    }

    #[test]
    fn later_registration_steals_a_shared_alias() {
        let mut m = AliasResolver::new();
        m.register("a", ["@shared"], rec("a"));
        m.register("b", ["@shared"], rec("b"));
        assert_eq!(m.resolve("@shared").unwrap().id, "b");
    }

    #[test]
    fn unregister_only_clears_own_aliases() {
        let mut m = AliasResolver::new();
        m.register("a", ["@a1"], rec("a"));
        m.register("b", ["@b1"], rec("b"));
        m.unregister("a");
        assert!(!m.contains("@a1"));
        assert!(m.resolve("@b1").is_some());
    }

    #[test]
    fn stolen_alias_survives_the_original_owner_leaving() {
        let mut m = AliasResolver::new();
        m.register("a", ["@shared"], rec("a"));
        m.register("b", ["@shared"], rec("b"));
        m.unregister("a");
        assert_eq!(m.resolve("@shared").unwrap().id, "b");
    }

    #[test]
    fn contains_id_or_alias() {
        let mut m = AliasResolver::new();
        m.register("x", ["@x1", "@x2"], rec("x"));
        assert!(m.contains("x"));
        assert!(m.contains("@x1"));
        assert!(m.contains("@x2"));
        assert!(!m.contains("nope"));
    }

    #[test]
    fn ids_and_aliases_are_sorted() {
        let mut m = AliasResolver::new();
        m.register("a", ["@a1", "@a2"], rec("a"));
        m.register("b", ["@b1"], rec("b"));
        m.register("c", Vec::<String>::new(), rec("c"));
        assert_eq!(m.ids(), ["a", "b", "c"]);
        let aliases = m.aliases();
        assert_eq!(aliases.len(), 6);
        assert!(aliases.contains(&"a"));
        assert!(aliases.contains(&"@a1"));
        assert!(aliases.contains(&"c"));
    }

    #[test]
    fn cache_hit_after_resolve() {
        let mut m = AliasResolver::new();
        m.register("a", ["@a1"], rec("a"));
        let r1 = m.resolve("@a1").unwrap();
        assert_eq!(m.cache_len(), 1);
        let r2 = m.resolve("@a1").unwrap();
        assert_eq!(r1, r2);
        assert_eq!(m.cache_len(), 1);
    }

    #[test]
    fn register_invalidates_cache() {
        let mut m = AliasResolver::new();
        m.register("a", ["@a1"], rec("a"));
        let _ = m.resolve("@a1");
        assert!(m.cache_len() > 0);
        m.register("b", ["@b1"], rec("b"));
        assert_eq!(m.cache_len(), 0);
    }

    #[test]
    fn lru_evicts_the_least_recent_lookup_key() {
        let mut m = AliasResolver::with_cache_size(2);
        m.register("a", ["@a1"], rec("a"));
        m.register("b", ["@b1"], rec("b"));
        m.register("c", ["@c1"], rec("c"));
        let _ = m.resolve("@a1");
        let _ = m.resolve("@b1");
        assert_eq!(m.cache_len(), 2);
        let _ = m.resolve("@c1");
        assert_eq!(m.cache_len(), 2);
    }

    #[test]
    fn clear_cache_drops_entries_but_keeps_records() {
        let mut m = AliasResolver::new();
        m.register("a", ["@a1"], rec("a"));
        let _ = m.resolve("@a1");
        m.clear_cache();
        assert_eq!(m.cache_len(), 0);
        assert!(m.resolve("@a1").is_some());
    }

    #[test]
    fn miss_does_not_pollute_the_cache() {
        let mut m = AliasResolver::<Rec>::new();
        assert!(m.resolve("@nope").is_none());
        assert_eq!(m.cache_len(), 0);
    }

    #[test]
    fn distinct_lookup_keys_are_distinct_cache_entries() {
        let mut m = AliasResolver::new();
        m.register("a", ["@a1", "@a2"], rec("a"));
        let _ = m.resolve("@a1");
        let _ = m.resolve("@a2");
        let _ = m.resolve("a");
        assert_eq!(m.cache_len(), 3);
    }

    #[test]
    fn id_lookup_beats_a_stolen_alias_of_the_same_string() {
        let mut m = AliasResolver::new();
        m.register("a", Vec::<String>::new(), rec("a"));
        m.register("b", ["a"], rec("b"));
        assert_eq!(
            m.resolve("a").unwrap().id,
            "a",
            "the record table wins over the alias index"
        );
    }

    #[test]
    fn unresolved_token_matches_vcp_shape() {
        assert_eq!(LEGACY_UNRESOLVED_PREFIX, "{{agent:");
        assert_eq!(unresolved_token("agent", "coder"), "{{agent:coder}}");
        assert_eq!(
            unresolved_token("plugin", "builtin.tools"),
            "{{plugin:builtin.tools}}"
        );
    }

    #[test]
    fn index_pairs_and_aliases_by_id() {
        let index = index_pairs([
            ("coder", vec!["@coder", "@chuling"]),
            ("mavis", vec!["@ai"]),
        ]);
        assert_eq!(index.resolve("@chuling"), Some("coder"));
        let grouped = aliases_by_id(&index);
        assert!(grouped["coder"].contains(&"@chuling".to_string()));
        assert!(grouped["mavis"].contains(&"@ai".to_string()));
    }

    #[test]
    fn lru_replace_existing_does_not_grow() {
        let mut cache = LruCache::new(2);
        cache.put("a", 1);
        cache.put("b", 2);
        cache.put("a", 3);
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get(&"a"), Some(&3));
    }

    #[test]
    fn index_manifests_reads_plugin_aliases() {
        use apeireth_core::kernel::PluginId;
        let a = PluginManifest::new(PluginId::new("builtin.coder").unwrap(), "1.0.0", "c")
            .with_aliases(["@coder", "@chuling"]);
        let b = PluginManifest::new(PluginId::new("builtin.mavis").unwrap(), "1.0.0", "m")
            .with_alias("@ai");
        let index = index_manifests([&a, &b]);
        assert_eq!(index.resolve("@chuling"), Some("builtin.coder"));
        assert_eq!(index.resolve("builtin.mavis"), Some("builtin.mavis"));
        assert_eq!(index.resolve("@ai"), Some("builtin.mavis"));
    }
}
