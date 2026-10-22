//! Application-layer envelope encryption face (安全加固批 B6, 隐患 #6).
//!
//! 会话/记忆 SQLite 静态数据的**应用层信封加密**: 不锁死 SQLite 引擎
//! (拒整库引擎级加密: 与创世标尺 §3「可插拔存储、引擎自研」冲突), 不锁死
//! 机器 (拒平台机器绑定加密: 与 T1 加密漫游冲突) —— 加密发生在**字段层**,
//! 可装饰任意存储后端 ([`layer::SealingLayer`]), 密钥主权在用户。
//!
//! ## 密钥层级 (信封加密)
//!
//! ```text
//! 用户口令 --KDF--> KEK  ─┐
//! 恢复件 (裸 KEK 等价物) ─┼─ 包裹 ─> DEK 信封 (KeyWrapFile, 槽位制)
//! 平台凭据库 (KekAnchor) ─┘                │
//!                                          v
//!                        PartitionKey (分区级 DEK, §5 单主体×N分区)
//!                                          │
//!                                          v
//!                        SealedField (AES-256-GCM, AAD 绑分区 id)
//! ```
//!
//! - **分区级 DEK** (创世标尺 §5 分区隔离墙, 跨分区披露 R4 零容忍): 每个
//!   记忆分区一把 [`PartitionKey`], AAD 绑分区 id —— 跨分区挪用密文
//!   **拒解**, 分区内透明语义不受损 (密钥在手 = 可读可导可清);
//! - **KEK 不随备份走** ([`backup`]): 备份 = 数据 + DEK 信封, 恢复需
//!   用户口令或恢复件; 密钥丢失 = 显式「不可恢复」边界 (见
//!   [`backup::KEY_LOSS_BOUNDARY`]), 不假装能找回。
//!
//! ## 命名承诺 (与既有「信封」职责隔离)
//!
//! `foundation/core::stored_doc` 的 StoredDoc 是**格式信封** (描述符字段 +
//! 主体, 版本契约), 与本模块的**加密包裹**职责不同。本模块刻意不复用
//! 「信封」一词作类型名: 密文单元叫 [`sealed::SealedField`] (密封字段),
//! 密钥包裹叫 [`wrap::KeyWrapFile`] (包裹件), 备份包叫 [`backup::SealedBackup`].
//!
//! ## 依赖面 (0 引新外部包)
//!
//! 复用锁内既有包: `aes-gcm` (AES-256-GCM 原语, workspace 既有加密族) /
//! `sha2` (HMAC-SHA-256 + PBKDF2 构造) / `rand` (OsRng nonce 随机源) /
//! `serde`+`serde_json` (包裹件与清单 JSON) / `zeroize`+`hex` (密钥材料
//! 零化与编码, 与 credentials 层同款)。KDF 为标准 PBKDF2-HMAC-SHA-256
//! 构造的仓内实现, 已钉标准测试向量 (见 [`kdf`]); 默认迭代数已拍板 600k
//! (2026-10-21 调参, 性能未实测, 见 [`kdf::DEFAULT_PBKDF2_ITERATIONS`])。
//!
//! ## 列级定级 (哪些列进加密, 单一真源)
//!
//! 定级表 + 理由 + 残余见 [`classification`] (代码 + 测试逐行钉):
//! 内容列与**内容指纹哈希列**进 P0 加密; 向量/聚合元数据/检索键保明;
//! 未勘列 fail-closed 显式待定。列级接缝 (真实表上的封缄读写示范) 见
//! [`sqlite_sink`]。指纹列的查重/溯源检索走**盲索引** ([`blind_index`]):
//! 索引面与内容面分离, 检索键单向、查询不触原文。
//!
//! ## 0 装诚实边界
//!
//! - 平台凭据库/桌面侧车 keychain 的 [`anchor::KekAnchor`] 具体实现**未接线**
//!   (归口装配面/桌面侧车线), engine 层只定义锚定接口;
//! - **列级接入示范已交** (IMPLEMENTED): [`sqlite_sink::SqliteFieldSink`] 在
//!   storage 真实表上端到端封缄读写 (往返/重启/篡改拒读有测试); 各存储
//!   owner (memory 等) 真读写路径的逐列改造**未接线**, 归口各 owner;
//! - KDF 默认迭代 600k 的解锁开销**未实测** (拍板值已由测试钉字面)。

pub mod anchor;
pub mod backup;
pub mod blind_index;
pub mod classification;
pub mod error;
pub mod kdf;
pub mod layer;
pub mod partition;
pub mod sealed;
pub mod sqlite_sink;
pub mod wrap;

pub use anchor::KekAnchor;
pub use backup::{
    export_sealed_backup, restore_sealed_backup, verify_sealed_backup, BackupEntry,
    SealedBackupManifest, SealedRestoreReport, KEY_LOSS_BOUNDARY,
};
pub use blind_index::{BlindIndex, BlindIndexKey, BlindIndexSink, IndexTarget};
pub use classification::{classify_column, FieldClass};
pub use error::CryptoError;
pub use kdf::{
    derive_kek_from_passphrase, pbkdf2_hmac_sha256, KdfParams, DEFAULT_PBKDF2_ITERATIONS,
};
pub use layer::{FieldSink, SealingLayer};
pub use partition::{PartitionId, PartitionKey};
pub use sealed::SealedField;
pub use sqlite_sink::{SealedTable, SqliteFieldSink};
pub use wrap::{KekSource, KeyWrapFile, KeyWrapSlot, RecoveryKey, UnlockKey};
