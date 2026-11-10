//! `apeireth_core::hash_chain` — 哈希链原语（纯密码学，无策略）。
//!
//! # 为什么在 core（而不是 governance）
//!
//! 设计稿 `docs/01-architecture/memory-knowledge-depth-system-spec.md` 原则 P3
//! 要求「权威层带哈希链 + 能定位到具体记录」。哈希链本身是**纯密码学布局**：
//! 谁来写、写多少条、要不要落盘、失败怎么办，全是**策略**，不属于任何领域。
//!
//! - 放在 `apeireth-governance` ⇒ 想用它的记忆层（`apeireth-memory`）得欠一条
//!   **跨领域依赖**（记忆层不依赖治理层，治理层在依赖图的上游/平行位置）。
//! - 放在 `apeireth-core` ⇒ 两个上层 crate 都只向下依赖 core，依赖方向不乱。
//!
//! 所以：**原语下沉，策略留在上层**。治理审计账本（`apeireth_governance::audit`）
//! 复用这里的布局与摘要算法，自己保留「序列号连续 / 记录内容 / 错误类型」等
//! 治理语义。
//!
//! # 字节布局（**冻结**）
//!
//! ```text
//! 摘要 = SHA-256( 片段₁ ‖ 片段₂ ‖ … ‖ LP(previous_hash) )
//! LP(bytes) = u64 大端字节长度 ‖ bytes   （变长字段用）
//! Raw(bytes) = bytes                    （定长字段用，原样拼接）
//! ```
//!
//! 十六进制**小写**、无分隔符。治理审计账本的历史摘要值就是按这套布局算出来的
//! ⇒ 本模块对同一输入必须给出**逐字节相同**的摘要（见本文件的常量断言测试：
//! 三个常量取自治理旧实现，不是现算的）。
//!
//! # 依赖面
//!
//! 本模块**零新增依赖**：SHA-256（FIPS 180-4 公开标准）在本文件内实现，
//! 输出与 `sha2` crate **逐字节一致**（见 `sha256_matches_fips_vectors`）。
//! 之所以不引 `sha2`：`apeireth-core` 的依赖面是本仓的硬约束，而 `sha2` 目前
//! 只挂在治理 crate 上；把算法留在 core 内部可以不让 core 欠一条新依赖。
//!
//! # 边界（不假装）
//!
//! 本模块**只管摘要与链接**：不判业务合法性、不判时间顺序、不落盘、不做密钥管理。
//! 「哪些内容该进链」「链断了怎么办」由上层决定。

use thiserror::Error;

/// 创世记录的固定前驱值（全零摘要）。
///
/// 原样搬自 `apeireth_governance::audit::GENESIS_PREVIOUS_HASH`：治理审计账本的
/// 链头指向这个值，**不可改**（改了等于换掉整条账本的起点）。
///
/// 全零不假装证明链外的任何事，只表示「这是链上的第一条」。
pub const GENESIS_PREVIOUS_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// 参与摘要的字段片段及其编码方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chunk<'a> {
    /// 原样拼接：给**定长**字段用（如 u64 序列号、i64 时间戳）。
    ///
    /// 定长字段的长度由类型本身固定，不需要长度前缀；用 `Raw` 是为了把
    /// 「这一段不加前缀」写进类型，避免调用方随手换编码。
    Raw(&'a [u8]),
    /// 先写 `u64` 大端长度再拼接：给**变长**字段用。
    ///
    /// 为什么变长字段必须带长度前缀：`["ab", "c"]` 与 `["a", "bc"]` 直接拼接
    /// 得到的字节**完全相同** ⇒ 摘要相同 ⇒ 链无法区分两种切分（拼接歧义）。
    /// 带上长度后，两种切分的前像不同 ⇒ 摘要必不同。
    LenPrefixed(&'a [u8]),
}

/// 链上的一条记录（通用：内容是字节，摘要是 hex）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainRecord {
    /// 本条记录的摘要（hex，小写）。
    pub current_hash: String,
    /// 上一条的摘要（hex，小写）；创世记录用 [`GENESIS_PREVIOUS_HASH`]。
    pub previous_hash: String,
}

/// 链校验错误。
///
/// 语义与治理侧 `AuditChainError` 的链接类变体**对齐**（同一件事、同一批索引），
/// 但治理侧**不**直接用这个类型：治理链还要报序列号断裂与内容被改，这两件事
/// 本原语看不到（它没有内容、也没有序列号）。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HashChainError {
    /// 第 0 条记录的前驱不是创世值 ⇒ 链的起点被换过。
    #[error("hash chain record {index} has an invalid genesis previous_hash")]
    GenesisHashMismatch { index: usize },
    /// 某条记录的前驱不是上一条的摘要 ⇒ 链被断开、重排或删改过。
    #[error("hash chain record {index} previous_hash does not point to the previous current_hash")]
    PreviousHashMismatch { index: usize },
}

/// 算一条记录的摘要：内容片段 + 前驱摘要。
///
/// 字节布局见模块文档。这是本原语唯一的摘要入口：上层把领域字段翻译成
/// [`Chunk`]，本函数负责拼前像、算 SHA-256、出小写 hex。
pub fn hash_linked(previous_hash: &str, chunks: &[Chunk<'_>]) -> String {
    let mut preimage = Vec::new();
    for chunk in chunks {
        match chunk {
            Chunk::Raw(bytes) => preimage.extend_from_slice(bytes),
            Chunk::LenPrefixed(bytes) => {
                preimage.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
                preimage.extend_from_slice(bytes);
            }
        }
    }
    // 前驱摘要作为**最后一个**长度前缀字段：链接关系进摘要 ⇒ 改链头即改摘要。
    preimage.extend_from_slice(&(previous_hash.len() as u64).to_be_bytes());
    preimage.extend_from_slice(previous_hash.as_bytes());
    hex(&sha256(&preimage))
}

/// 哈希链构造器：只负责按规则算摘要与链接。
#[derive(Debug, Clone)]
pub struct HashChain {
    last_hash: String,
    count: usize,
}

impl Default for HashChain {
    /// 与 [`HashChain::new`] 等价：空链的链头就是创世前驱值。
    ///
    /// **为什么不用 derive(Default)**：派生版会把链头留成空字符串 ⇒ 第一条记录
    /// 的前驱既不是创世值、也不是任何摘要 ⇒ 空链与「链从空摘要起步」两种语义
    /// 混在一起，调用方分不清。
    fn default() -> Self {
        Self::new()
    }
}

impl HashChain {
    /// 空链（链头 = 创世前驱值）。
    pub fn new() -> Self {
        Self {
            last_hash: GENESIS_PREVIOUS_HASH.to_string(),
            count: 0,
        }
    }

    /// 追加一条内容，返回该条记录（摘要 + 前驱摘要）。
    pub fn append_record(&mut self, content: &[u8]) -> ChainRecord {
        let previous_hash = self.last_hash.clone();
        let current_hash = hash_linked(&previous_hash, &[Chunk::LenPrefixed(content)]);
        self.last_hash = current_hash.clone();
        self.count += 1;
        ChainRecord {
            current_hash,
            previous_hash,
        }
    }

    /// 追加一条内容，返回该条的摘要（并推进链头）。
    ///
    /// 长度前缀哈希（length-prefixed）⇒ 避免拼接歧义（见 [`Chunk::LenPrefixed`]）。
    pub fn append(&mut self, content: &[u8]) -> String {
        self.append_record(content).current_hash
    }

    /// 当前链头摘要（空链时 = 创世前驱值）。
    pub fn head(&self) -> &str {
        &self.last_hash
    }

    /// 已追加条数。
    pub fn len(&self) -> usize {
        self.count
    }

    /// 是否一条都没追加。
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// 校验一串记录是否首尾相连。
    ///
    /// 只看**链接关系**：第 0 条的前驱必须是创世值，第 `i` 条的前驱必须是第
    /// `i-1` 条的摘要。空链校验通过（没有可断的地方）。
    ///
    /// **为什么参数是记录而不是一串裸摘要**：链接关系住在 `previous_hash` 里，
    /// 只拿到一串摘要无法判断首尾是否相连（谁是谁的前驱不在摘要里）。
    ///
    /// **看不到什么**：本函数**不**重算摘要（没有内容就没法重算），所以它能证明
    /// 「链接没断」，不能单独证明「内容没被改」。要那一条保证，上层得自己拿内容
    /// 重算并比对。
    pub fn verify_chain(records: &[ChainRecord]) -> Result<(), HashChainError> {
        for (index, record) in records.iter().enumerate() {
            let expected = if index == 0 {
                GENESIS_PREVIOUS_HASH
            } else {
                records[index - 1].current_hash.as_str()
            };
            if record.previous_hash != expected {
                return Err(if index == 0 {
                    HashChainError::GenesisHashMismatch { index }
                } else {
                    HashChainError::PreviousHashMismatch { index }
                });
            }
        }
        Ok(())
    }
}

/// 十六进制编码（小写、无分隔符）——与治理审计账本旧实现的输出格式一致。
/// SHA-256 摘要（走 `sha2` crate —— **不自研密码代码**）。
///
/// 仓里已有 `sha2`（workspace 依赖），自研实现是**不必要且危险**的重复
/// （密码原语一旦自己写错，审计链的可信度就归零）。R5 下沉时的临时自研实现
/// 已在此替换为 crate 版；**摘要值逐字节不变**（由固定历史常量断言钉住）。
fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    let out = hasher.finalize();
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&out);
    digest
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[usize::from(byte >> 4)] as char);
        out.push(DIGITS[usize::from(byte & 0x0f)] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 治理审计账本的历史摘要值（**取自治理旧实现**，不是本实现现算的）：
    /// 第 0 条 `boot_system`/`system`，第 1 条 `governance.evaluated`/`tool.shell`，
    /// 第 2 条 `capability.dispatched`/`tool.calculator`。
    ///
    /// 这三个常量是「行为逐字节不变」的锚：core 的布局一旦漂移，它们立刻红。
    const LEGACY_RECORD_0: &str =
        "0edff970aed2df8f22b0ba07e6ad4fce9087d5ffa5e60a4bc631d04ba7697d9e";
    const LEGACY_RECORD_1: &str =
        "f70b452d981613ba29faa08f1f286e889ceef81108c5f1ad8baece184db7b137";
    const LEGACY_RECORD_2: &str =
        "8c69db0599fe9bc59a0c0173f21373e24fa057b7c16d6a49b430b6d2398d94f9";

    /// 按治理审计记录的字段顺序算摘要（序列号、时间戳、事件、主体）。
    fn audit_digest(
        sequence: u64,
        timestamp: i64,
        event_kind: &str,
        subject: &str,
        previous_hash: &str,
    ) -> String {
        let sequence_bytes = sequence.to_be_bytes();
        let timestamp_bytes = timestamp.to_be_bytes();
        hash_linked(
            previous_hash,
            &[
                Chunk::Raw(&sequence_bytes),
                Chunk::Raw(&timestamp_bytes),
                Chunk::LenPrefixed(event_kind.as_bytes()),
                Chunk::LenPrefixed(subject.as_bytes()),
            ],
        )
    }

    #[test]
    fn sha256_matches_fips_vectors() {
        assert_eq!(
            hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex(&sha256(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn genesis_previous_hash_is_all_zeroes() {
        assert_eq!(GENESIS_PREVIOUS_HASH.len(), 64);
        assert!(GENESIS_PREVIOUS_HASH.bytes().all(|byte| byte == b'0'));
    }

    #[test]
    fn governance_layout_digests_match_legacy_constants() {
        // 三组常量断言：同一批内容 ⇒ 同一批摘要，与治理旧实现逐字节相同。
        let hash_0 = audit_digest(
            0,
            1_700_000_000_000,
            "boot_system",
            "system",
            GENESIS_PREVIOUS_HASH,
        );
        assert_eq!(hash_0, LEGACY_RECORD_0);

        let hash_1 = audit_digest(
            1,
            1_700_000_000_001,
            "governance.evaluated",
            "tool.shell",
            &hash_0,
        );
        assert_eq!(hash_1, LEGACY_RECORD_1);

        let hash_2 = audit_digest(
            2,
            1_700_000_000_002,
            "capability.dispatched",
            "tool.calculator",
            &hash_1,
        );
        assert_eq!(hash_2, LEGACY_RECORD_2);
    }

    #[test]
    fn chain_links_records_head_to_head() {
        let mut chain = HashChain::new();
        let first = chain.append_record(b"first");
        let second = chain.append_record(b"second");
        let third = chain.append_record(b"third");

        assert_eq!(first.previous_hash, GENESIS_PREVIOUS_HASH);
        assert_eq!(second.previous_hash, first.current_hash);
        assert_eq!(third.previous_hash, second.current_hash);
        assert_eq!(chain.head(), third.current_hash);
        assert_eq!(chain.len(), 3);
        assert!(!chain.is_empty());
    }

    #[test]
    fn empty_chain_starts_at_genesis() {
        let chain = HashChain::new();
        assert_eq!(chain.head(), GENESIS_PREVIOUS_HASH);
        assert_eq!(chain.len(), 0);
        assert!(chain.is_empty());
        assert_eq!(HashChain::default().head(), GENESIS_PREVIOUS_HASH);
    }

    #[test]
    fn verify_chain_accepts_intact_and_empty_chains() {
        let mut chain = HashChain::new();
        let mut records = Vec::new();
        for content in [b"one".as_slice(), b"two", b"three"] {
            records.push(chain.append_record(content));
        }
        assert!(HashChain::verify_chain(&records).is_ok());
        assert!(HashChain::verify_chain(&[]).is_ok());
    }

    #[test]
    fn verify_chain_rejects_tampered_middle_record() {
        let mut chain = HashChain::new();
        let mut records = Vec::new();
        for content in [b"one".as_slice(), b"two", b"three"] {
            records.push(chain.append_record(content));
        }
        records[1].previous_hash = "deadbeef".to_string();
        assert!(matches!(
            HashChain::verify_chain(&records),
            Err(HashChainError::PreviousHashMismatch { index: 1 })
        ));
    }

    #[test]
    fn verify_chain_rejects_wrong_genesis() {
        let mut chain = HashChain::new();
        let mut records = vec![chain.append_record(b"one")];
        records[0].previous_hash = "deadbeef".to_string();
        assert!(matches!(
            HashChain::verify_chain(&records),
            Err(HashChainError::GenesisHashMismatch { index: 0 })
        ));
    }

    #[test]
    fn length_prefix_prevents_concatenation_ambiguity() {
        // 同样的字节切分不同 ⇒ 摘要必须不同（否则链分不清两条记录还是一条）。
        let mut left = HashChain::new();
        left.append(b"ab");
        let left_digest = left.append(b"c");

        let mut right = HashChain::new();
        right.append(b"a");
        let right_digest = right.append(b"bc");

        assert_ne!(left_digest, right_digest);
        assert_ne!(left.head(), right.head());
    }
}
