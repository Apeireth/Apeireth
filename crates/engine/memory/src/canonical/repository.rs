//! Canonical memory repository contract (M1B1).
//!
//! This trait is persistence-only. Retrieval semantics (ACT-R activation,
//! ranking, semantic search) are intentionally owned by later memory-layer
//! modules and are not part of the repository contract.

use apeireth_core::kernel::Timestamp;
use async_trait::async_trait;

use super::domain::{MemoryId, MemoryItem};
use super::error::MemoryError;

/// Deterministic query filter for canonical memory items.
///
/// `as_of` is required and is the only temporal probe. No hidden wall clock is
/// used. `include_tombstones` defaults to `false`, so normal queries never
/// accidentally surface deleted items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryFilter {
    /// Temporal probe for validity filtering.
    pub as_of: Timestamp,
    /// When `true`, tombstoned items are returned as well.
    pub include_tombstones: bool,
    /// Optional deterministic result cap.
    pub limit: Option<usize>,
}

impl MemoryFilter {
    /// Creates a filter for items effective at `as_of`.
    pub fn new(as_of: Timestamp) -> Self {
        Self {
            as_of,
            include_tombstones: false,
            limit: None,
        }
    }

    /// Sets whether tombstoned items should be included.
    pub fn with_include_tombstones(mut self, include_tombstones: bool) -> Self {
        self.include_tombstones = include_tombstones;
        self
    }

    /// Caps the number of returned items.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Canonical memory repository.
///
/// Implementations persist [`MemoryItem`] values through the canonical storage
/// foundation. Retrieval/ranking is not part of this contract.
#[async_trait]
pub trait MemoryRepository: Send + Sync {
    /// Inserts a new item. Fails with [`MemoryError::Conflict`] when the id
    /// already exists.
    async fn insert(&self, item: MemoryItem) -> Result<(), MemoryError>;

    /// Returns the item with the given id, or `None` when it does not exist
    /// **or** is tombstoned. Tombstoned state can be read through
    /// [`MemoryRepository::query`] with `include_tombstones = true`.
    async fn get(&self, id: &MemoryId) -> Result<Option<MemoryItem>, MemoryError>;

    /// Replaces the stored item with `item`. Fails with
    /// [`MemoryError::NotFound`] when the id does not exist.
    async fn update(&self, item: MemoryItem) -> Result<(), MemoryError>;

    /// Returns items matching `filter`, ordered deterministically by
    /// `created_at` ascending and then `id` ascending.
    async fn query(&self, filter: &MemoryFilter) -> Result<Vec<MemoryItem>, MemoryError>;

    /// Marks the item with the given id as tombstoned.
    ///
    /// Tombstoning an already-tombstoned item succeeds (idempotent). Missing
    /// ids fail with [`MemoryError::NotFound`].
    async fn tombstone(&self, id: &MemoryId) -> Result<(), MemoryError>;
}
