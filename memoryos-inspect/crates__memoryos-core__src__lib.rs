**Source**: https://raw.githubusercontent.com/TelivANT/memoryos-rust/main/crates/memoryos-core/src/lib.rs

---

pub mod config;
pub mod error;
pub mod faq;
pub mod health;
pub mod history;
pub mod identity;
pub mod llm;
pub mod memory;
pub mod optimization;
pub mod rbac;
pub mod retry;
pub mod security;
pub mod tenant;
pub mod wiki;

pub use config::{AppConfig, ConfigManager};
pub use error::AppError;
pub use faq::{
    AutoPromoter, AutoPromotionConfig, ExportResult, ExportTarget, FaqClassification, HeatConfig,
    HeatStats, HeatTracker, LlmClassifierConfig, PromotionRecord, PromotionResult, PromotionStats,
    WikiExportBackend, WikiExportConfig, WikiExporter,
};
pub use health::{DependencyState, HealthMode, HealthStatus};
pub use history::{HistoryEventType, MemoryHistoryEntry};
pub use identity::PrincipalContext;
pub use memory::*;
pub use optimization::{
    BatchEmbedder, BloomFilter, EmbeddingCache, HeatBuffer, IncrementalSummarizer,
    OptimizedFaqMatcher, OptimizedRetriever, SimilarityFilter,
};
pub use rbac::{Permission, RbacManager, Role, UserRecord};
pub use security::{
    AuditConfig, AuditEvent, AuditEventType, AuditLogger, AuditOutcome, DataEncryptor,
    EncryptedPayload, EncryptionConfig, GdprManager,
};
pub use tenant::{Tenant, TenantContext, TenantManager};