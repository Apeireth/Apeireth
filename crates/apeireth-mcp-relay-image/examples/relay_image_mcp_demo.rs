//! # apeireth-mcp-relay-image 端到端 demo
//!
//! 演示 6 步 (R20 阶段 1 skeleton 阶段):
//! 1. 生成测试图片 (1x1 PNG, 67 bytes) + SHA256
//! 2. 写 RelayCache (LRU capacity=10)
//! 3. dedup 验证 (同 SHA256 二次 put 不增长)
//! 4. data URI 往返 (decoded SHA256 与原图一致)
//! 5. list_cached (trait 入口)
//! 6. relay_image skeleton (已知 Err, 不连真上游)
//!
//! 运行: `cargo run -p apeireth-mcp-relay-image --example relay_image_mcp_demo`

use apeireth_mcp_relay_image::{
    compute_sha256, CachePolicy, ImageFormat, ImagePayload, RelayCache, RelayConfig,
    RelayImageError, RelayImageMcpServer, RelayImageMcpServerTrait,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

// 1x1 红色 PNG (per v0.9.21 估最小有效 PNG, 67 bytes)
const TINY_PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== apeireth-mcp-relay-image demo (R20 阶段 1 skeleton) ===\n");

    // --- 步骤 1: 生成测试图片 + SHA256 ---
    let raw_png = BASE64.decode(TINY_PNG_BASE64)?;
    let payload_a = ImagePayload::from_bytes(raw_png.clone(), ImageFormat::Png);
    let hash_a = compute_sha256(&raw_png);
    assert_eq!(hash_a, payload_a.sha256, "ImagePayload::from_bytes 与 compute_sha256 一致");
    println!("[1] 生成测试图片 OK: format={:?} size={}B", payload_a.format, payload_a.size);
    println!("    SHA256 = {}...", &hash_a[..16]);

    // --- 步骤 2: 写 LRU 缓存 (capacity=10) ---
    let mut cache = RelayCache::new(CachePolicy::Lru { capacity: 10 });
    cache.put(payload_a.clone())?;
    assert_eq!(cache.len(), 1);
    println!("[2] RelayCache 写入 OK: cache.len() = {}", cache.len());

    // --- 步骤 3: dedup 验证 ---
    let payload_dup = ImagePayload::from_bytes(raw_png.clone(), ImageFormat::Png);
    cache.put(payload_dup)?;
    assert_eq!(cache.len(), 1, "dedup 应阻止同 SHA256 二次写入");
    println!("[3] Dedup 验证 OK: cache.len() 仍为 1 (LRU 序列更新)");

    // --- 步骤 4: data URI 往返 ---
    let data_uri = payload_a.to_data_uri();
    let server = RelayImageMcpServer::new(RelayConfig::default())?;
    let decoded = server.decode_image(&data_uri).await?;
    assert_eq!(decoded.sha256, payload_a.sha256, "data URI 往返 SHA256 一致");
    assert_eq!(decoded.format, ImageFormat::Png);
    println!("[4] data URI 往返 OK: decoded sha256 = {}...", &decoded.sha256[..16]);

    // --- 步骤 5: list_cached (独立 LRU 验) ---
    // (本示例不通过 server.list_cached, 因为 server 持有独立空 cache;
    //  使用本地 cache 验 list 行为)
    let listed: Vec<_> = cache.list().into_iter().cloned().collect();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].format, ImageFormat::Png);
    println!("[5] cache.list() OK: 1 张 PNG, sha256 = {}...", &listed[0].sha256[..16]);

    // --- 步骤 6: relay_image (skeleton, 期望 Err) ---
    let relay_result = server.relay_image(&payload_a).await;
    match relay_result {
        Err(RelayImageError::ImageRelay(msg)) => {
            println!("[6] relay_image skeleton 已知: {}", msg);
        }
        Ok(_) => panic!("relay_image skeleton 不应成功"),
        Err(other) => panic!("relay_image 错误类型不符: {:?}", other),
    }

    // --- 步骤 7: LRU 容量淘汰 ---
    let mut mini = RelayCache::new(CachePolicy::Lru { capacity: 2 });
    for i in 0..3 {
        mini.put(ImagePayload::from_bytes(
            format!("img-{}", i).into_bytes(),
            ImageFormat::Png,
        ))?;
    }
    assert_eq!(mini.len(), 2, "LRU capacity=2 应淘汰最旧");
    println!("[7] LRU 容量淘汰 OK: capacity=2 写 3 张剩 2 张");

    println!("\n=== 全部 7 步 OK ===");
    Ok(())
}
