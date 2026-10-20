//! KEK 锚定接口 (主密钥主权在用户).
//!
//! KEK (key-encryption key) 的**存放/取回**由装配面实现:
//! - 平台凭据库 / 桌面侧车 keychain (基础件在桌面侧车线, 引擎不绑平台);
//! - 用户口令经 KDF 派生 (走 [`crate::crypto::kdf`], 不经本接口);
//! - 恢复件 (裸 KEK 等价物, 用户保管, 走 [`crate::crypto::wrap::RecoveryKey`])。
//!
//! engine 层只定义**锚定 trait** (本文件), 不实现任何具体锚 ——
//! 锁死平台的加密 (机器绑定/平台 API 直连) 与创世标尺 T1 加密漫游冲突,
//! 锚定实现归口装配面与桌面侧车线 (0 装: 现无任何实现挂载)。

use zeroize::Zeroizing;

use crate::crypto::error::CryptoError;
use crate::crypto::partition::PartitionId;

/// KEK 锚定接口: 把分区 KEK 锚进用户可控的凭据库。
///
/// 契约:
/// - [`KekAnchor::anchor_id`] 写进包裹槽位, 恢复时按 id 选锚;
/// - [`KekAnchor::load_kek`] 无锚定记录 = `Ok(None)` (如实报缺, 不编值);
/// - 实现方保证 KEK 落盘/落库形态由锚自身负责加密或权限保护。
pub trait KekAnchor: Send + Sync {
    /// 锚定身份 (稳定字符串, 如平台凭据库条目名; 不含密钥材料)。
    fn anchor_id(&self) -> &str;

    /// 取回该分区的 KEK (32 字节); 无锚定记录 = `Ok(None)`。
    fn load_kek(&self, partition: &PartitionId)
        -> Result<Option<Zeroizing<[u8; 32]>>, CryptoError>;

    /// 锚定该分区的 KEK (写入锚定存储)。
    fn store_kek(&self, partition: &PartitionId, kek: &[u8; 32]) -> Result<(), CryptoError>;
}
