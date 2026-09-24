//! P2P Bluetooth / LAN mesh packet format and peer registry.
//!
//! # 诚实边界 (O-5 · 2026-09-24 审计 C1)
//!
//! 本模块是**明文 mesh 报文编解码 + 对端注册表**, **不是**加密协议:
//! - 无 Noise / Noise_XX 握手, 无私钥交换, 无洋葱多层封装;
//! - `payload_hex` 是 payload 的**十六进制明文** (hex decode 即得原文);
//! - `payload_len_hex` 只是负载长度 (两个不同内容的同长度负载值相同),
//!   **不是** SHA-256 完整性校验;
//! - `packet_id` 是 `(node_id, len)` 派生的人类可读标签, **不保证唯一**
//!   (同长度负载即相同), 不可用于去重/防重放;
//! - `process_roaming_delta` 只做输入健全性检查 (非空 source + root 是
//!   合法 hex), **不验证** Merkle 树/bitemporal 一致性。
//!
//! 原版本模块 doc 宣称 "Noise_XX / 洋葱路由 / 防中间节点窃听 / Merkle 验证",
//! 实现与字段命名 (`ephemeral_pubkey_hex` 实为长期公钥、`checksum_sha256`
//! 实为长度) 均与实现不符 —— 已按"0 装 PASS"纪律全部更正。
//!
//! `TODO(v2.1)`: 接入真加密 (X25519 + ChaCha20-Poly1305 会话密钥, 或
//! `apeireth-credentials` 的密钥环) 后再恢复加密语义命名。
//!
//! Pure Safe Rust (`#![deny(unsafe_code)]`)。

use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

/// Mesh transport kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeshTransportKind {
    BluetoothLE,
    LocalUdpBroadcast,
    TcpDirect,
}

/// Mesh node descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshNodeDescriptor {
    pub node_id: String,
    pub device_name: String,
    pub transport: MeshTransportKind,
    /// 节点长期公钥 (hex)。**不是**每包临时的 ephemeral key —— 无握手协议。
    pub public_key_hex: String,
    pub last_seen_secs: u64,
}

/// Mesh packet envelope (明文, 无加密 —— 见模块 doc 诚实边界)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshPacket {
    /// 人类可读标签 `pkt_<node>_<len>`; **不唯一**, 不可用于去重/防重放。
    pub packet_id: String,
    /// 逻辑跳数; 当前实现恒为 0 (单跳 stub, 无洋葱多层)。
    pub hop_count: u8,
    /// 配置的最大跳数 (当前未被实施)。
    pub max_hops: u8,
    pub target_recipient_node_id: String,
    /// 发送方长期公钥 (hex); 非 ephemeral。
    pub sender_public_key_hex: String,
    /// payload 字节数 (十进制 hex 化) —— 长度提示, **非密码学校验和**。
    pub payload_len_hex: String,
    /// payload 的十六进制明文 (**未加密**)。
    pub payload_hex: String,
}

/// Differential memory synchronization delta (未验证, 见 process_roaming_delta)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryRoamingDelta {
    pub source_node_id: String,
    /// 调用方声称的 Merkle 树根 (hex); 本模块**不验证**其真实性。
    pub merkle_root_hex: String,
    pub new_fact_ids: Vec<String>,
    pub timestamp_secs: u64,
}

/// 对端注册表容量上限 (防对端泛洪耗尽内存)。
pub const MAX_DISCOVERED_PEERS: usize = 256;

/// P2P mesh controller: peer registry + 明文 packet 编解码。
#[derive(Debug, Clone)]
pub struct P2pMeshController {
    pub local_node_id: String,
    pub local_device_name: String,
    /// 本节点长期公钥 (hex)。
    pub local_public_key_hex: String,
    discovered_peers: HashMap<String, MeshNodeDescriptor>,
    /// 注册顺序 (FIFO 淘汰用)。
    peer_order: VecDeque<String>,
}

impl P2pMeshController {
    pub fn new(local_node_id: &str, device_name: &str, public_key_hex: &str) -> Self {
        Self {
            local_node_id: local_node_id.to_string(),
            local_device_name: device_name.to_string(),
            local_public_key_hex: public_key_hex.to_string(),
            discovered_peers: HashMap::new(),
            peer_order: VecDeque::new(),
        }
    }

    /// Registers a newly discovered peer node on BLE/LAN.
    ///
    /// 有界: 超过 [`MAX_DISCOVERED_PEERS`] 时按注册顺序淘汰最旧对端
    /// (2026-09-24 审计 L2: 原实现纯 insert 无上限无老化)。
    pub fn register_peer(&mut self, peer: MeshNodeDescriptor) {
        if !self.discovered_peers.contains_key(&peer.node_id) {
            while self.discovered_peers.len() >= MAX_DISCOVERED_PEERS {
                match self.peer_order.pop_front() {
                    Some(oldest) => {
                        self.discovered_peers.remove(&oldest);
                    }
                    None => break,
                }
            }
            self.peer_order.push_back(peer.node_id.clone());
        }
        self.discovered_peers.insert(peer.node_id.clone(), peer);
    }

    /// Lists all active discovered peer nodes.
    pub fn list_peers(&self) -> Vec<&MeshNodeDescriptor> {
        self.discovered_peers.values().collect()
    }

    /// Encapsulates a payload into a mesh packet (**明文 hex, 无加密/无洋葱** ——
    /// 见模块 doc 诚实边界)。
    pub fn wrap_mesh_packet(
        &self,
        target_node_id: &str,
        payload_bytes: &[u8],
        max_hops: u8,
    ) -> MeshPacket {
        let payload_hex = payload_bytes
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join("");

        MeshPacket {
            packet_id: format!("pkt_{}_{}", self.local_node_id, payload_bytes.len()),
            hop_count: 0,
            max_hops,
            target_recipient_node_id: target_node_id.to_string(),
            sender_public_key_hex: self.local_public_key_hex.clone(),
            payload_len_hex: format!("{:08x}", payload_bytes.len()),
            payload_hex,
        }
    }

    /// Processes a roaming memory delta packet.
    ///
    /// 只做**输入健全性**检查: 非空 source + merkle root 是合法 hex 字符串。
    /// **不验证** Merkle 一致性/bitemporal 真值 (原 doc 宣称的 "Zero-Trust"
    /// 同步未实现; 2026-09-24 审计 L3)。
    pub fn process_roaming_delta(&self, delta: &MemoryRoamingDelta) -> bool {
        !delta.source_node_id.is_empty()
            && !delta.merkle_root_hex.is_empty()
            && delta.merkle_root_hex.chars().all(|c| c.is_ascii_hexdigit())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_p2p_mesh_peer_discovery_and_wrap() {
        let mut controller = P2pMeshController::new("phone_01", "Pixel 9 Pro", "aabbccdd");
        controller.register_peer(MeshNodeDescriptor {
            node_id: "laptop_01".into(),
            device_name: "ThinkPad X1".into(),
            transport: MeshTransportKind::BluetoothLE,
            public_key_hex: "11223344".into(),
            last_seen_secs: 1000,
        });
        assert_eq!(controller.list_peers().len(), 1);
        let packet = controller.wrap_mesh_packet("laptop_01", b"hello_mesh", 3);
        assert_eq!(packet.target_recipient_node_id, "laptop_01");
        assert_eq!(packet.max_hops, 3);
        assert_eq!(packet.hop_count, 0);
    }

    #[test]
    fn test_p2p_mesh_memory_roaming_delta() {
        let controller = P2pMeshController::new("node_a", "Device A", "pub_a");

        let delta = MemoryRoamingDelta {
            source_node_id: "node_b".into(),
            merkle_root_hex: "0123456789abcdef".into(),
            new_fact_ids: vec!["fact_1".into(), "fact_2".into()],
            timestamp_secs: 2000,
        };

        assert!(controller.process_roaming_delta(&delta));
    }

    #[test]
    fn peer_registry_is_bounded() {
        // 2026-09-24 审计 L2: 对端表有界, 不随泛洪无界增长。
        let mut controller = P2pMeshController::new("node", "dev", "key");
        for i in 0..(MAX_DISCOVERED_PEERS + 32) {
            controller.register_peer(MeshNodeDescriptor {
                node_id: format!("peer_{i}"),
                device_name: "d".into(),
                transport: MeshTransportKind::BluetoothLE,
                public_key_hex: "k".into(),
                last_seen_secs: 0,
            });
        }
        assert_eq!(controller.list_peers().len(), MAX_DISCOVERED_PEERS);
    }
}
