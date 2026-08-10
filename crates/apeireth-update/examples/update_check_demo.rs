//! # apeireth-update update check demo
//!
//! 1 update 流程完整例子 (per 借鉴文档 §8 P3 第 10-11 项 + 1:1 镜像 cosign.yml verify):
//! 1. 加载 trust public key (K-1 强校验白名单)
//! 2. 注入 mock release (R21+ 真接时改 GitHub API)
//! 3. 创建 DefaultUpdater
//! 4. check_for_update (async, R21 real mode)
//! 5. minisign 真签真验 (用现成 `minisign` crate, 0 重复造轮子)
//! 6. handle_version_request (GET /v1/update/version, 3rd 端点)
//! 7. handle_check_request (GET /v1/update/check)
//! 8. handle_apply_request (POST /v1/update/apply)
//! 9. verify_artifact_mirror_cosign (1:1 镜像 cosign.yml verify 4 步)
//!
//! 跑法: `cargo run -p apeireth-update --example update_check_demo`

use apeireth_update::{
    handle_apply_request, handle_check_request, handle_version_request, load_trusted_public_key,
    sign_minisign, verify_artifact_mirror_cosign, verify_minisign, ApplyRequest, Asset,
    CheckRequest, Channel, DefaultUpdater, LibraryInfo, Release, SignatureAlgorithm, TrustedKey,
    UpdateInfo, Updater, VerifyArtifact, VersionRequest, STUB_MODE,
};
// 例子里直接用 minisign crate (跟测试同模式, 0 重复造轮子)
use minisign::KeyPair;
use sha2::{Digest, Sha256};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化 tracing (可选, 方便看 warn! 日志)
    let _ = tracing_subscriber::fmt()
        .with_env_filter("warn,apeireth_update=info")
        .try_init();

    println!("===========================================");
    println!("  apeireth-update demo (R21 real mode)");
    println!("===========================================");
    println!();

    // §1 Library info
    let info = LibraryInfo::current();
    println!("[§1] Library info:");
    println!("  name:              {}", info.name);
    println!("  schema_version:    {}", info.schema_version);
    println!("  platform:          {}", info.platform);
    println!("  real_mode:         {}", info.real_mode);
    println!("  stub_mode:         {}", info.stub_mode);
    println!("  channel_count:     {}", info.channel_count);
    println!("  trusted_key_count: {}", info.trusted_key_count);
    println!("  endpoint_count:    {}", info.endpoint_count);
    println!("  verify_step_count: {}", info.verify_step_count);
    println!("  tool_whitelist_count: {}", info.tool_whitelist_count);
    println!("  update_error_variant_count: {}", info.update_error_variant_count);
    let _ = STUB_MODE;
    println!();

    // §2 加载 trust public key (K-1 强校验, 真签真验, 0 重复造轮子)
    println!("[§2] Loading trust public key (K-1 强校验白名单, 真签真验)...");
    let keypair = KeyPair::generate_encrypted_keypair(Some("apeireth-demo-password".to_string()))
        .expect("keypair generation must succeed");
    let pk_box_str = keypair.pk.to_box().expect("pk.to_box must succeed").to_string();
    let trusted_key = match load_trusted_public_key(&pk_box_str, TrustedKey::Ephemeral) {
        Ok(k) => {
            println!("  ✓ loaded: kind={:?}, fingerprint={}", k.kind, k.fingerprint);
            k
        }
        Err(e) => {
            eprintln!("  ✗ load_trusted_public_key failed: {}", e);
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }
    };
    println!();

    // §3 构造 mock Release (R21+ 真接时改 GitHub API)
    println!("[§3] Building mock release (R21+ 真接时改 GitHub API)...");
    let data = b"apeireth-v1.0.0 binary content (demo)";

    // 真签 (per minisign crate API, 0 重复造轮子)
    let sk = keypair
        .sk
        .to_box(None)
        .expect("sk.to_box must succeed")
        .into_secret_key(Some("apeireth-demo-password".to_string()))
        .expect("into_secret_key must succeed");
    let signature_b64 = sign_minisign(&sk, data).expect("sign_minisign must succeed");

    // SHA-256 (per cosign.yml verify job 1:1 镜像)
    let mut hasher = Sha256::new();
    hasher.update(data);
    let sha256_hex = hex::encode(hasher.finalize());

    let mock_release = Release {
        tag: "v1.0.0".to_string(),
        version: "1.0.0".to_string(),
        channel: Channel::Stable,
        notes: "Apeireth 1.0.0 release (mock, R21+ 真接时改 GitHub API)".to_string(),
        published_at: "2026-08-06T00:00:00Z".to_string(),
        assets: vec![Asset {
            name: "apeireth-v1.0.0-x86_64-unknown-linux-gnu.tar.gz".to_string(),
            url: "https://github.com/apeireth/apeireth-rust/releases/download/v1.0.0/apeireth-v1.0.0-x86_64-unknown-linux-gnu.tar.gz".to_string(),
            size_bytes: data.len() as u64,
            sha256: sha256_hex.clone(),
            signature_b64: signature_b64.clone(),
            algorithm: SignatureAlgorithm::Ed25519,
        }],
        prerelease: false,
    };
    println!("  ✓ mock_release: tag={}, version={}, channel={}", mock_release.tag, mock_release.version, mock_release.channel.as_str());
    println!();

    // §4 创建 DefaultUpdater
    println!("[§4] Creating DefaultUpdater (GitHub Releases check + minisign verify)...");
    let updater = DefaultUpdater::new(
        "apeireth",
        "apeireth-rust",
        vec![mock_release.clone()],
        trusted_key.clone(),
    )
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    println!("  ✓ updater: owner={}, repo={}, releases={}", updater.owner, updater.repo, updater.release_source.len());
    println!();

    // §5 check_for_update
    println!("[§5] check_for_update (async, R21 real mode)...");
    let current = "0.14.0";
    let result = updater
        .check_for_update(current, Channel::Stable)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    match result {
        Some(info) => {
            println!("  ✓ has_update: true");
            print_update_info(&info);
        }
        None => {
            println!("  ✓ has_update: false (current {} is up to date)", current);
        }
    }
    println!();

    // §6 minisign 真签真验 (用现成 `minisign` crate, 0 重复造轮子)
    println!("[§6] verify_minisign (真签真验, 用现成 minisign crate, 0 重复造轮子)...");
    match verify_minisign(&trusted_key, data, &signature_b64) {
        Ok(()) => {
            println!("  ✓ verify_minisign: ok (真签真验, minisign crate 0 重复造轮子)");
        }
        Err(e) => {
            eprintln!("  ✗ verify_minisign failed: {}", e);
            return Err(Box::new(e));
        }
    }
    println!();

    // §7 handle_version_request (GET /v1/update/version, 3rd 端点, per task spec §3)
    println!("[§7] handle_version_request (GET /v1/update/version, 3rd 端点)...");
    let version_req = VersionRequest { channel: None };
    let version_resp = handle_version_request(version_req, "1.0.0", "99F790EC4BE6E38D")
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    println!("  ✓ version:        {}", version_resp.version);
    println!("  ✓ channel:        {}", version_resp.channel.as_str());
    println!("  ✓ fingerprint:    {}", version_resp.fingerprint);
    println!("  ✓ minisign_required: {}", version_resp.minisign_required);
    println!("  ✓ algorithm:      {}", version_resp.algorithm.as_str());
    println!("  ✓ protocol:       {}", version_resp.protocol);
    println!("  ✓ request_id:     {}", version_resp.request_id);
    println!("  ✓ real_mode:      {}", version_resp.real_mode);
    println!();

    // §8 handle_check_request (GET /v1/update/check)
    println!("[§8] handle_check_request (GET /v1/update/check)...");
    let check_req = CheckRequest {
        current_version: current.to_string(),
        channel: Some(Channel::Stable.as_str().to_string()),
    };
    let check_resp = handle_check_request(&updater, check_req)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    println!("  ✓ has_update:    {}", check_resp.has_update);
    println!("  ✓ request_id:    {}", check_resp.request_id);
    println!("  ✓ real_mode:     {}", check_resp.real_mode);
    if let Some(info) = check_resp.update_info {
        println!("  ✓ update_info:");
        print_update_info(&info);
    }
    println!();

    // §9 handle_apply_request (POST /v1/update/apply)
    println!("[§9] handle_apply_request (POST /v1/update/apply)...");
    let apply_req = ApplyRequest {
        version: "1.0.0".to_string(),
        target_dir: Some("/opt/apeireth".to_string()),
    };
    let apply_resp = handle_apply_request(&updater, apply_req)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    println!("  ✓ outcome.version:        {}", apply_resp.outcome.version);
    println!("  ✓ outcome.success:        {}", apply_resp.outcome.success);
    println!("  ✓ outcome.required_fields: {}", apply_resp.outcome.required_fields_count);
    println!("  ✓ request_id:             {}", apply_resp.request_id);
    println!("  ✓ real_mode:              {}", apply_resp.real_mode);
    println!();

    // §10 verify_artifact_mirror_cosign (1:1 镜像 cosign.yml verify 4 步)
    println!("[§10] verify_artifact_mirror_cosign (1:1 镜像 cosign.yml verify 4 步)...");
    let verify_artifact = VerifyArtifact {
        name: "apeireth-v1.0.0-x86_64-unknown-linux-gnu.tar.gz".to_string(),
        data: data.to_vec(),
        expected_sha256: sha256_hex.clone(),
        signature_b64: signature_b64.clone(),
        algorithm: SignatureAlgorithm::Ed25519,
    };
    let report = verify_artifact_mirror_cosign(&verify_artifact, &trusted_key)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    println!("  ✓ passed:        {}", report.passed);
    println!("  ✓ protocol:      {}", report.protocol);
    println!("  ✓ real_mode:     {}", report.real_mode);
    println!("  ✓ 4 steps:");
    for step in &report.steps {
        let mark = if step.passed { "✓" } else { "✗" };
        println!("    {} {} (passed={})", mark, step.step, step.passed);
    }
    println!();

    println!("===========================================");
    println!("  Demo complete (R21 real mode, R21+ 续真接)");
    println!("===========================================");
    println!();
    println!("⏳ R21+ 续真接清单:");
    println!("  1. 真实 GitHub Releases API (网络依赖 + auth token)");
    println!("  2. 真实 minisign 验签 (替换 TestFixture 占位 + Ephemeral 临时公钥)");
    println!("  3. 真实 asset 下载 (reqwest::Client::get + 校验 SHA-256)");
    println!("  4. 真实 apply (跟 apeireth-upgrade 7 阶段 OTA 集成, 调 UpgradeIntent::new)");
    println!("  5. 真实 HTTP server (axum 0.7+ / warp 0.4+)");
    println!("  6. 真实 cosign ECDSA P-256 key 加载 (per cosign.yml 1:1 镜像, 0.25 升级到 0.8+)");

    Ok(())
}

fn print_update_info(info: &UpdateInfo) {
    println!("    version:              {}", info.version);
    println!("    tag:                  {}", info.tag);
    println!("    channel:              {}", info.channel.as_str());
    println!("    notes:                {}...", info.notes.chars().take(40).collect::<String>());
    println!("    asset.name:           {}", info.asset.name);
    println!("    asset.size_bytes:     {}", info.asset.size_bytes);
    println!("    asset.algorithm:      {}", info.asset.algorithm.as_str());
    println!("    asset.sha256 (head):  {}...", info.asset.sha256.chars().take(8).collect::<String>());
    println!("    asset.signature_b64 (head): {}...", info.asset.signature_b64.chars().take(8).collect::<String>());
    println!("    published_at:         {}", info.published_at);
    println!("    required_fields_count: {}", info.required_fields_count);
}
