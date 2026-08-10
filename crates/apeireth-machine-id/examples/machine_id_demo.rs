//! Machine ID 探测 demo (1:1 翻译 v0.9.21 商业版 getMachineId 调用).
//!
//! 流程: 探测 platform → probe fallback chain → SHA-256 派生 → 输出 result.
//!
//! ## 运行
//!
//! ```bash
//! cargo run -p apeireth-machine-id --example machine_id_demo
//! ```
//!
//! ## 期望输出 (skeleton 阶段, 当前 platform 真实跑)
//!
//! ```text
//! platform: linux
//! source: dmi  (或 dbus / etc / wmi / ioreg / kenv / hostid)
//! raw: AAAAAAAA-BBBB-...
//! hashed: <64 hex chars>
//! cached: <Option<MachineIdResult>>
//! [machine_id_demo] completed (skeleton — R20 阶段 1 真实 fallback chain 实施中)
//! ```
//!
//! ## 6 哲学 anchor 验证
//!
//! - S-1 北极星: 1:1 翻译 v0.9.21 4 平台 getMachineId
//! - S-2 实事求是: 4 平台 17 命令字符串 hardcode, 当前 platform 真跑 fallback
//! - O-2 走在前人肩上: tokio::process + fs_err 标准 API
//! - O-3 干到底: Linux 3 个 fallback, Windows 2 个, BSD 2 个, macOS 1 个
//! - O-4 任何人都能接手: §1-§6 章节 + 4 平台模块
//! - O-5 不假装: skeleton 阶段命令真跑, 失败返 typed error

use apeireth_machine_id::{get_machine_id, hash_machine_id, read_cached, Platform};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1) 平台探测 (per `Platform::detect` 1:1)
    let platform = Platform::detect();
    println!("platform: {}", platform.as_str());

    // 2) 统一接口 (cfg 路由 4 平台, per `get_machine_id` 1:1)
    match get_machine_id().await {
        Ok(result) => {
            println!("source: {}", result.source);
            println!("raw: {}", result.raw);
            println!("hashed: {}", result.hashed);
            println!("detected_at: {:?}", result.detected_at);

            // 3) 验证 hash 一致性 (skeleton 自检: 同样 raw 应得同样 hash)
            let re_hash = hash_machine_id(&result.raw)?;
            assert_eq!(re_hash, result.hashed, "SHA-256 hash 派生必须一致");
            println!("hash self-check: ok");
        }
        Err(e) => {
            eprintln!("[machine_id_demo] get_machine_id 失败: {e}");
            eprintln!("(skeleton 阶段: 当前 platform 可能不在 4 平台内, 或 probe 命令不可用)");
        }
    }

    // 4) 缓存读 (per `read_cached` 1:1, 失败不传播)
    match read_cached().await {
        Ok(Some(cached)) => println!("cached: Some(raw={}, source={})", cached.raw, cached.source),
        Ok(None) => println!("cached: None (首次运行)"),
        Err(e) => eprintln!("cached 读失败: {e}"),
    }

    println!("[machine_id_demo] completed (skeleton — R20 阶段 1 真实 fallback chain 实施中)");
    Ok(())
}
